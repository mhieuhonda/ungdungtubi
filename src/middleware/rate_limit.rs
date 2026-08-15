//! Rate Limiting middleware (v0.9.24 — Giai đoạn 29).
//!
//! Cơ chế: in-memory token bucket per IP + endpoint.
//! Mỗi IP có budget:
//!   - Auth endpoints (/dang-nhap, /auth/*): 10 req/phút — chống brute-force OAuth
//!   - API endpoints (/api/*): 60 req/phút — chống scraping
//!   - POST endpoints: 30 req/phút — chống spam form submit
//!   - Other: 120 req/phút — normal browsing
//!
//! Khi exceed: trả 429 Too Many Requests + Retry-After header.
//!
//! Note: In-memory rate limit KHÔNG persist across restarts.
//!       Mỗi worker process có state riêng — nếu chạy nhiều worker, limit sẽ × N.
//!       Trong production với 1 container, điều này OK.
//!       Nếu scale horizontal, cần chuyển sang Redis-based rate limit.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Entry trong rate limit map.
#[derive(Clone, Debug)]
struct RateEntry {
    /// Số request trong cửa sổ hiện tại.
    count: u32,
    /// Thời điểm bắt đầu cửa sổ.
    window_start: Instant,
    /// Thời điểm bị block tới (nếu đang bị block).
    blocked_until: Option<Instant>,
}

impl Default for RateEntry {
    fn default() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            blocked_until: None,
        }
    }
}

/// State chia sẻ cho rate limit middleware.
#[derive(Clone, Default)]
pub struct RateLimitState {
    /// Map: (ip, endpoint_group) → RateEntry
    entries: Arc<RwLock<HashMap<(String, &'static str), RateEntry>>>,
}

impl RateLimitState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Phân loại endpoint vào nhóm để áp dụng limit tương ứng.
    fn classify_endpoint(path: &str) -> &'static str {
        if path.starts_with("/auth/") || path == "/dang-nhap" {
            "auth"
        } else if path.starts_with("/api/upload") {
            "upload"
        } else if path.starts_with("/api/") {
            "api"
        } else if path == "/ca-nhan/cap-nhat" || path == "/cai-dat/cap-nhat" {
            "profile_update"
        } else if path.starts_with("/cong-dong/") && (path.ends_with("/tao-chu-de") || path.ends_with("/binh-luan")) {
            "post"
        } else if path.starts_with("/ban-be/") {
            "social"
        } else {
            "general"
        }
    }

    /// Lấy limit (số request) và window (thời gian cửa sổ) cho từng nhóm.
    fn limit_for_group(group: &str) -> (u32, Duration) {
        match group {
            "auth" => (10, Duration::from_secs(60)),           // 10 req/phút
            "upload" => (10, Duration::from_secs(60)),          // 10 upload/phút
            "api" => (60, Duration::from_secs(60)),             // 60 API/phút
            "profile_update" => (10, Duration::from_secs(60)),  // 10 update/phút
            "post" => (30, Duration::from_secs(60)),            // 30 post/phút
            "social" => (60, Duration::from_secs(60)),          // 60 social/phút
            _ => (120, Duration::from_secs(60)),                // 120 general/phút
        }
    }

    /// Kiểm tra và update rate limit. Trả về (allowed, retry_after_secs).
    pub async fn check(&self, ip: &str, path: &str) -> (bool, u64) {
        let group = Self::classify_endpoint(path);
        let (limit, window) = Self::limit_for_group(group);
        let key = (ip.to_string(), group);

        let mut entries = self.entries.write().await;
        let now = Instant::now();
        let entry = entries.entry(key).or_default();

        // Check nếu đang bị block
        if let Some(until) = entry.blocked_until {
            if now < until {
                let retry_after = (until - now).as_secs().max(1);
                return (false, retry_after);
            } else {
                // Hết block — reset
                entry.blocked_until = None;
                entry.count = 0;
                entry.window_start = now;
            }
        }

        // Reset window nếu đã hết thời gian
        if now.duration_since(entry.window_start) >= window {
            entry.count = 0;
            entry.window_start = now;
        }

        // Tăng count
        entry.count += 1;

        // Check limit
        if entry.count > limit {
            // Block thêm 60s
            entry.blocked_until = Some(now + Duration::from_secs(60));
            log::warn!(
                "🚫 Rate limit exceeded: ip={} group={} count={} limit={}",
                ip, group, entry.count, limit
            );
            return (false, 60);
        }

        (true, 0)
    }

    /// Cleanup entries cũ (gọi định kỳ để tránh memory leak).
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        let max_age = Duration::from_secs(600); // 10 phút

        let before = entries.len();
        entries.retain(|_, entry| {
            now.duration_since(entry.window_start) < max_age
        });
        let removed = before.saturating_sub(entries.len());

        if removed > 0 {
            log::debug!("🧹 Rate limit cleanup: removed {} stale entries", removed);
        }
    }
}

/// Middleware function: rate limit per IP + endpoint.
/// v0.9.24: Nhận state qua `State<RateLimitState>` (từ `from_fn_with_state`).
pub async fn rate_limit(
    State(state): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Response {
    // Lấy IP từ X-Forwarded-For (sau Traefik) hoặc ConnectInfo
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let path = req.uri().path().to_string();
    let (allowed, retry_after) = state.check(&ip, &path).await;

    if !allowed {
        log::warn!("🚫 Rate limit blocked: ip={} path={}", ip, path);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("retry-after", retry_after.to_string()),
                ("x-ratelimit-limit", "120".to_string()),
            ],
            format!(
                "429 — Quá nhiều request. Vui lòng thử lại sau {} giây. 🪷",
                retry_after
            ),
        )
            .into_response();
    }

    next.run(req).await
}

/// Background task: cleanup rate limit state mỗi 5 phút.
pub fn spawn_cleanup_task(state: RateLimitState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            state.cleanup().await;
        }
    });
}

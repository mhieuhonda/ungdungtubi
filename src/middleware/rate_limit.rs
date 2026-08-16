//! Rate Limiting middleware (v0.9.24 — Giai đoạn 29; v0.9.37 — overhauled).
//!
//! Cơ chế: in-memory token bucket per IP + endpoint.
//! Mỗi IP có budget:
//!   - Auth endpoints (/dang-nhap, /auth/*): 10 req/phút — chống brute-force OAuth
//!   - Upload endpoints (/api/upload*, /api/nha-nhac/dang-nhac*): 10 req/phút
//!   - API endpoints (/api/*): 180 req/phút — v0.9.37: tăng từ 60 → 180 để fix 429
//!     khi user đổi tab liên tục (notifications poll + chat history fetch + stats API).
//!   - Profile update (/ca-nhan/cap-nhat, /cai-dat/cap-nhat): 10 req/phút
//!   - POST endpoints (/cong-dong/.../tao-chu-de, /binh-luan, /kinh-sach/.../cam-ngo,
//!     /kinh-sach/.../tang-hoa): 60 req/phút — v0.9.37: tăng từ 30 → 60 để fix 429
//!     khi user post nhiều bình luận liên tục; thêm kinh-sách submissions vào nhóm này.
//!   - Social endpoints (/ban-be/*, /api/ban-be/*): 180 req/phút — v0.9.37: tăng từ
//!     60 → 180 và gộp cả /api/ban-be/* vào nhóm social (trước đây /api/ban-be/* bị
//!     gộp vào /api/* generic, gây 429 khi user DM + notification poll cùng lúc).
//!   - Other: 300 req/phút — v0.9.37: tăng từ 120 → 300 để fix 429 khi user đổi
//!     tab liên tục.
//!
//! Khi exceed: trả 429 Too Many Requests + Retry-After header + HTML page có nút
//! "Quay lại" + countdown timer (v0.9.37: thay plain-text response bằng HTML page).
//!
//! Note: In-memory rate limit KHÔNG persist across restarts.
//!       Mỗi worker process có state riêng — nếu chạy nhiều worker, limit sẽ × N.
//!       Trong production với 1 container, điều này OK.
//!       Nếu scale horizontal, cần chuyển sang Redis-based rate limit.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use axum::{
    extract::Request,
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
/// v0.9.24: Dùng global static (OnceLock) để tránh xung đột với router state.
#[derive(Clone, Default)]
pub struct RateLimitState {
    /// Map: (ip, endpoint_group) → RateEntry
    entries: Arc<RwLock<HashMap<(String, &'static str), RateEntry>>>,
}

/// Global static instance — khởi tạo lần đầu khi `get_global()` được gọi.
static GLOBAL_STATE: OnceLock<RateLimitState> = OnceLock::new();

impl RateLimitState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lấy global instance (singleton). Tự khởi tạo nếu chưa có.
    /// Dùng cho middleware function (không cần State extractor).
    pub fn get_global() -> &'static RateLimitState {
        GLOBAL_STATE.get_or_init(|| RateLimitState::new())
    }

    /// Phân loại endpoint vào nhóm để áp dụng limit tương ứng.
    ///
    /// v0.9.37 FIX BUG A: `/api/ban-be/*` trước đây rơi vào nhóm `api` (60/min) —
    /// nhưng đây là social features (DM, notifications, friend requests). User active
    /// DM-ing + notification poll + friend search dễ vượt 60/min. Giờ tách ra `social`.
    ///
    /// v0.9.37 FIX BUG B: `/api/nha-nhac/dang-nhac*` (YouTube URL + audio file upload)
    /// trước đây rơi vào nhóm `api` (60/min) — nhưng đây là upload operation, nên
    /// giờ vào nhóm `upload` (10/min) — strict hơn nhưng đúng semantics.
    ///
    /// v0.9.37 FIX BUG C: `/kinh-sach/{slug}/cam-ngo` + `/kinh-sach/{slug}/tang-hoa`
    /// trước đây rơi vào nhóm `general` (120/min) — quá dễ spam. Giờ vào `post` (60/min).
    fn classify_endpoint(path: &str) -> &'static str {
        // Auth (login, OAuth callback) — strictest
        if path.starts_with("/auth/") || path == "/dang-nhap" {
            return "auth";
        }

        // Upload endpoints (avatar, group cover/logo, music files) — strict
        if path.starts_with("/api/upload") {
            return "upload";
        }
        // v0.9.37 FIX BUG B: music submissions (YouTube URL + audio file) → upload bucket
        if path == "/api/nha-nhac/dang-nhac" || path == "/api/nha-nhac/dang-nhac-file" {
            return "upload";
        }

        // v0.9.37 FIX BUG A: /api/ban-be/* → social bucket (DM, notifications, friend ops)
        // Trước đây bị gộp vào /api/* generic → 429 khi user active social + notification poll.
        if path.starts_with("/api/ban-be/") || path.starts_with("/ban-be/") {
            return "social";
        }

        // Profile update (ca-nhan/cap-nhat, cai-dat/cap-nhat) — strict
        if path == "/ca-nhan/cap-nhat" || path == "/cai-dat/cap-nhat" {
            return "profile_update";
        }

        // POST endpoints (create topic, create comment, kinh-sach submissions) — medium
        if path.starts_with("/cong-dong/") && (path.ends_with("/tao-chu-de") || path.ends_with("/binh-luan")) {
            return "post";
        }
        // v0.9.37 FIX BUG C: kinh-sách submissions (cam-ngo, tang-hoa) → post bucket
        if path.starts_with("/kinh-sach/") && (path.ends_with("/cam-ngo") || path.ends_with("/tang-hoa")) {
            return "post";
        }

        // Generic API (stats, history, prefs, etc.) — v0.9.37: tăng limit
        if path.starts_with("/api/") {
            return "api";
        }

        // Everything else (HTML pages, static files)
        "general"
    }

    /// Lấy limit (số request) và window (thời gian cửa sổ) cho từng nhóm.
    ///
    /// v0.9.37: Tăng đáng kể các limit để fix lỗi 429 khi user đổi tab liên tục.
    /// Trước đây: api=60, social=60, general=120, post=30 → dễ 429.
    /// Giờ: api=180, social=180, general=300, post=60 — gấp 2.5-3x.
    /// Auth/upload/profile_update giữ nguyên (đây là security limits, không nên nới lỏng).
    fn limit_for_group(group: &str) -> (u32, Duration) {
        match group {
            "auth" => (10, Duration::from_secs(60)),           // 10 req/phút — security limit
            "upload" => (10, Duration::from_secs(60)),          // 10 upload/phút — security limit
            "api" => (180, Duration::from_secs(60)),            // 180 API/phút — v0.9.37: 60 → 180
            "profile_update" => (10, Duration::from_secs(60)),  // 10 update/phút — security limit
            "post" => (60, Duration::from_secs(60)),            // 60 post/phút — v0.9.37: 30 → 60
            "social" => (180, Duration::from_secs(60)),         // 180 social/phút — v0.9.37: 60 → 180
            _ => (300, Duration::from_secs(60)),                // 300 general/phút — v0.9.37: 120 → 300
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
            // v0.9.37: Block 30s thay vì 60s — cân bằng giữa chống spam + user experience.
            // 60s block quá dài, user click vài lần nhanh là bị block 1 phút → frustrating.
            let block_secs = 30;
            entry.blocked_until = Some(now + Duration::from_secs(block_secs));
            log::warn!(
                "🚫 Rate limit exceeded: ip={} group={} count={} limit={} (block {}s)",
                ip, group, entry.count, limit, block_secs
            );
            return (false, block_secs);
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

/// v0.9.37 — HTML page thân thiện cho 429 response.
/// Trước đây trả plain-text "429 — Quá nhiều request..." → user thấy trang trắng,
/// không có nav, không có nút quay lại, không có countdown.
/// Giờ: render HTML page với layout tối giản, có nút "Quay lại" + countdown timer
/// + hiển thị IP và nhóm bị rate-limit để user hiểu lý do.
fn render_429_page(retry_after: u64, group: &str, path: &str) -> String {
    let group_label = match group {
        "auth" => "đăng nhập / OAuth",
        "upload" => "upload ảnh / file âm thanh",
        "api" => "API (stats, history, preferences)",
        "profile_update" => "cập nhật hồ sơ / cài đặt",
        "post" => "đăng bài / bình luận / cảm ngộ",
        "social" => "nhắn tin / thông báo / kết bạn",
        _ => "lướt web chung",
    };
    let limit_label = match group {
        "auth" | "upload" | "profile_update" => "10",
        "post" => "60",
        "api" | "social" => "180",
        _ => "300",
    };
    format!(r#"<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>429 — Quá nhiều request · Ứng Dụng Từ Bi</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
       background: linear-gradient(135deg, #f0fdf4 0%, #ecfdf5 50%, #f0fdfa 100%);
       color: #1f2937; min-height: 100vh; display: flex; align-items: center;
       justify-content: center; padding: 1rem; }}
.container {{ max-width: 480px; width: 100%; background: white; border-radius: 1rem;
             box-shadow: 0 20px 25px -5px rgba(0,0,0,0.1), 0 10px 10px -5px rgba(0,0,0,0.04);
             padding: 2.5rem 2rem; text-align: center; border: 1px solid #d1fae5; }}
.emoji {{ font-size: 3rem; margin-bottom: 0.5rem; line-height: 1; }}
h1 {{ font-size: 1.5rem; font-weight: 700; color: #064e3b; margin-bottom: 0.5rem; }}
.subtitle {{ color: #047857; font-size: 0.875rem; margin-bottom: 1.5rem; }}
.countdown-box {{ background: #fef3c7; border: 1px solid #fde68a; border-radius: 0.5rem;
                  padding: 1rem; margin: 1.5rem 0; }}
.countdown-num {{ font-size: 2.5rem; font-weight: 700; color: #92400e; line-height: 1; }}
.countdown-label {{ font-size: 0.75rem; color: #78350f; margin-top: 0.25rem; }}
.info {{ background: #f0fdf4; border-radius: 0.5rem; padding: 0.75rem 1rem;
         font-size: 0.75rem; color: #065f46; margin: 1rem 0; text-align: left; }}
.info-row {{ display: flex; justify-content: space-between; padding: 0.25rem 0; }}
.info-key {{ font-weight: 600; }}
.btn-back {{ display: inline-block; background: #047857; color: white; padding: 0.75rem 1.5rem;
             border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.875rem;
             transition: background 0.2s; margin-top: 0.5rem; border: none; cursor: pointer; }}
.btn-back:hover {{ background: #065f46; }}
.btn-back:disabled {{ background: #9ca3af; cursor: not-allowed; }}
.footer {{ margin-top: 1.5rem; font-size: 0.75rem; color: #6b7280; }}
</style>
</head>
<body>
<div class="container">
  <img src="/static/tubi.png" alt="Ứng Dụng Từ Bi" style="width:64px;height:64px;border-radius:1rem;object-fit:cover;margin:0 auto 1rem;display:block;box-shadow:0 4px 12px rgba(0,0,0,0.1);">
  <h1>429 — Quá nhiều request</h1>
  <p class="subtitle">Bạn đang thao tác quá nhanh. Hãy nghỉ một lát rồi thử lại.</p>
  <div class="countdown-box">
    <div class="countdown-num" id="countdown">{retry_after}</div>
    <div class="countdown-label">giây — vui lòng đợi</div>
  </div>
  <div class="info">
    <div class="info-row"><span class="info-key">📝 Nhóm bị giới hạn:</span><span>{group_label}</span></div>
    <div class="info-row"><span class="info-key">⚡ Giới hạn:</span><span>{limit_label} request/phút</span></div>
    <div class="info-row"><span class="info-key">🔗 Đường dẫn:</span><span style="overflow:hidden;text-overflow:ellipsis;max-width:200px;text-align:right;">{path}</span></div>
  </div>
  <button class="btn-back" id="backBtn" onclick="history.back()" disabled>← Quay lại</button>
  <div class="footer">
    🪷 Ứng Dụng Từ Bi v0.9.38 · Nguyện công đức vô lượng · Nam Mô A Di Đà Phật
  </div>
</div>
<script>
// Countdown timer
(function() {{
  let remaining = {retry_after};
  const numEl = document.getElementById('countdown');
  const btnEl = document.getElementById('backBtn');
  const interval = setInterval(() => {{
    remaining--;
    if (remaining <= 0) {{
      clearInterval(interval);
      numEl.textContent = '0';
      numEl.style.color = '#047857';
      btnEl.disabled = false;
      btnEl.textContent = '← Quay lại (sẵn sàng)';
    }} else {{
      numEl.textContent = remaining;
    }}
  }}, 1000);
}})();
</script>
</body>
</html>"#)
}

/// Middleware function: rate limit per IP + endpoint.
/// v0.9.24: Dùng global static state (OnceLock) — không cần State extractor,
/// tránh xung đột với router's AppState.
/// v0.9.37: Trả HTML page thay vì plain-text khi 429 (better UX).
pub async fn rate_limit(req: Request, next: Next) -> Response {
    let state = RateLimitState::get_global();

    // Lấy IP từ X-Forwarded-For (sau Traefik) hoặc fallback "unknown"
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
        let group = RateLimitState::classify_endpoint(&path);
        log::warn!("🚫 Rate limit blocked: ip={} path={} group={}", ip, path, group);

        // v0.9.37: Nếu request là fetch() API (Accept: application/json hoặc X-Requested-With),
        // trả JSON response để client-side JS có thể handle. Không thì trả HTML page.
        let wants_json = req
            .headers()
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.contains("application/json"))
            || req
                .headers()
                .get("x-requested-with")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|s| s.eq_ignore_ascii_case("xmlhttprequest"));

        if wants_json {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("retry-after", retry_after.to_string()),
                    ("content-type", "application/json; charset=utf-8".to_string()),
                ],
                axum::Json(serde_json::json!({
                    "error": "rate_limited",
                    "message": format!("Quá nhiều request. Vui lòng thử lại sau {} giây. 🪷", retry_after),
                    "retry_after": retry_after,
                    "group": group,
                })).to_string(),
            )
                .into_response();
        }

        // HTML page cho browser navigation
        let html = render_429_page(retry_after, group, &path);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("retry-after", retry_after.to_string()),
                ("content-type", "text/html; charset=utf-8".to_string()),
                ("cache-control", "no-store".to_string()),
            ],
            html,
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

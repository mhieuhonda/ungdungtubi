//! Handlers cho Hoạt Động Cộng Đồng (Activity Feed) — Giai đoạn 50 (v0.9.44).
//!
//! Trang `/cong-dong/hoat-dong` hiển thị các hoạt động gần đây trong cộng đồng:
//!   - 📌 Chủ đề mới (`topics` trong 7 ngày qua)
//!   - 💬 Bình luận mới (`comments` trong 7 ngày qua)
//!   - 👥 Nhóm mới tạo (`groups` trong 7 ngày qua)
//!   - 🎵 Nhạc được duyệt (`user_music_submissions WHERE status='approved'`)
//!   - 🙏 Thành viên mới tham gia nhóm (`group_members WHERE status='active'`)
//!
//! Thiết kế:
//!   * Query SQL UNION ALL 5 nguồn → sort DESC, limit 50.
//!   * Cache 5 phút (`activity_cache` trên `AppState`) tránh truy vấn nặng mỗi page load.
//!   * Yêu cầu đăng nhập — guest redirect `/dang-nhap`.
//!
//! Routes:
//!   - GET /cong-dong/hoat-dong          — Trang activity feed (HTML)
//!   - GET /api/cong-dong/hoat-dong     — JSON API cho activity items

use axum::{
    extract::State,
    response::{Html, IntoResponse, Json, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use std::time::Instant;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── ActivityItem ─────────────────────────────────────────────────────────

/// Một item trong activity feed.
///
/// Mỗi item đại diện cho 1 hoạt động trong cộng đồng (vd: user A tạo chủ đề B).
/// Được fetch từ 5 bảng khác nhau qua UNION ALL.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ActivityItem {
    /// Loại hoạt động: 'topic' | 'comment' | 'group' | 'music' | 'member'.
    pub activity_type: String,
    /// Tên người thực hiện hoạt động (có thể null nếu user đã bị xoá).
    pub actor_name: Option<String>,
    /// URL avatar người thực hiện (null nếu không có).
    pub actor_avatar: Option<String>,
    /// Tên đối tượng (vd: title chủ đề, tên nhóm, tên bài nhạc).
    pub target_name: Option<String>,
    /// Link đến đối tượng (vd: `/cong-dong/chu-de/{id}`).
    pub target_link: Option<String>,
    /// Thời điểm xảy ra hoạt động.
    pub created_at: DateTime<Utc>,
    /// Mô tả câu chữ (vd: "An đã tạo chủ đề \"Xin chào\"").
    pub description: Option<String>,
}

impl ActivityItem {
    /// Icon emoji theo loại hoạt động.
    pub fn icon(&self) -> &'static str {
        match self.activity_type.as_str() {
            "topic" => "📌",
            "comment" => "💬",
            "group" => "👥",
            "music" => "🎵",
            "member" => "🙏",
            _ => "🪷",
        }
    }

    /// Nhãn tiếng Việt cho loại hoạt động.
    pub fn type_label(&self) -> &'static str {
        match self.activity_type.as_str() {
            "topic" => "Chủ đề",
            "comment" => "Bình luận",
            "group" => "Nhóm",
            "music" => "Nhạc",
            "member" => "Thành viên",
            _ => "Khác",
        }
    }

    /// Hiển thị thời gian tương đối ("5 phút trước").
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    /// RFC3339 timestamp cho thuộc tính `title` của thẻ <time>.
    pub fn created_at_rfc3339(&self) -> String {
        self.created_at.to_rfc3339()
    }

    /// Link đích (fallback "#" nếu null).
    pub fn target_link_or_default(&self) -> String {
        self.target_link.clone().unwrap_or_else(|| "#".to_string())
    }

    /// Mô tả hiển thị (fallback "Có hoạt động mới trong cộng đồng.").
    pub fn description_or_default(&self) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| "Có hoạt động mới trong cộng đồng.".to_string())
    }
}

// ─── Cache + SQL ──────────────────────────────────────────────────────────

/// Thời gian sống của cache: 5 phút (300 giây).
const ACTIVITY_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// SQL UNION ALL — gộp 5 nguồn hoạt động trong 7 ngày qua.
///
/// Mỗi subquery trả về 7 cột đồng nhất: activity_type, actor_name, actor_avatar,
/// target_name, target_link, created_at, description. Sau đó sort DESC + limit 50.
const ACTIVITY_FEED_SQL: &str = r#"
SELECT
  'topic'::TEXT AS activity_type,
  u.display_name AS actor_name,
  u.avatar_url AS actor_avatar,
  t.title AS target_name,
  ('/cong-dong/chu-de/' || t.id::TEXT) AS target_link,
  t.created_at AS created_at,
  (u.display_name || ' đã tạo chủ đề "' || t.title || '"') AS description
FROM topics t
LEFT JOIN users u ON u.id = t.author_id
WHERE t.is_active = true AND t.created_at > NOW() - INTERVAL '7 days'

UNION ALL

SELECT
  'comment'::TEXT,
  u.display_name,
  u.avatar_url,
  t.title,
  ('/cong-dong/chu-de/' || t.id::TEXT),
  c.created_at,
  (u.display_name || ' đã bình luận trong "' || COALESCE(t.title, 'chủ đề đã xoá') || '"')
FROM comments c
LEFT JOIN users u ON u.id = c.author_id
LEFT JOIN topics t ON t.id = c.topic_id
WHERE c.is_active = true AND c.created_at > NOW() - INTERVAL '7 days'

UNION ALL

SELECT
  'group'::TEXT,
  u.display_name,
  u.avatar_url,
  g.name,
  ('/cong-dong/nhom/' || g.slug),
  g.created_at,
  (u.display_name || ' đã tạo nhóm "' || g.name || '"')
FROM groups g
LEFT JOIN users u ON u.id = g.owner_id
WHERE g.is_active = true AND g.created_at > NOW() - INTERVAL '7 days'

UNION ALL

SELECT
  'music'::TEXT,
  u.display_name,
  u.avatar_url,
  s.title,
  '/khong-gian/nha-nhac',
  COALESCE(s.reviewed_at, s.created_at),
  (u.display_name || ' đã đóng góp nhạc "' || s.title || '"')
FROM user_music_submissions s
LEFT JOIN users u ON u.id = s.user_id
WHERE s.status = 'approved'
  AND COALESCE(s.reviewed_at, s.created_at) > NOW() - INTERVAL '7 days'

UNION ALL

SELECT
  'member'::TEXT,
  u.display_name,
  u.avatar_url,
  g.name,
  ('/cong-dong/nhom/' || g.slug),
  gm.joined_at,
  (u.display_name || ' đã tham gia nhóm "' || COALESCE(g.name, 'nhóm đã xoá') || '"')
FROM group_members gm
LEFT JOIN users u ON u.id = gm.user_id
LEFT JOIN groups g ON g.id = gm.group_id
WHERE gm.status = 'active' AND gm.joined_at > NOW() - INTERVAL '7 days'

ORDER BY created_at DESC
LIMIT 50
"#;

/// Lấy danh sách activity items, dùng cache nếu còn hạn (5 phút).
///
/// Tránh truy vấn UNION ALL nặng trên mỗi page load — chỉ refetch khi cache expire.
async fn fetch_activities(state: &AppState) -> Vec<ActivityItem> {
    // 1. Check cache first (lock ngắn hạn để release sớm).
    {
        let cache = state.activity_cache.lock().await;
        if let Some((fetched_at, items)) = cache.as_ref() {
            if fetched_at.elapsed() < ACTIVITY_CACHE_TTL {
                return items.clone();
            }
        }
    }

    // 2. Cache miss hoặc expired → truy vấn DB.
    let items: Vec<ActivityItem> = sqlx::query_as::<_, ActivityItem>(ACTIVITY_FEED_SQL)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_else(|e| {
            log::error!("❌ Lỗi truy vấn activity feed: {e}");
            vec![]
        });

    // 3. Update cache.
    let mut cache = state.activity_cache.lock().await;
    *cache = Some((Instant::now(), items.clone()));

    items
}

// ─── Template struct ──────────────────────────────────────────────────────

/// Template cho trang `/cong-dong/hoat-dong`.
#[derive(Template)]
#[template(path = "community/hoat-dong.html")]
pub struct HoatDongTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub activities: Vec<ActivityItem>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /cong-dong/hoat-dong — Trang activity feed (HTML).
///
/// Yêu cầu đăng nhập — guest redirect `/dang-nhap`.
pub async fn hoat_dong_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let activities = fetch_activities(&state).await;

    let html = HoatDongTemplate {
        user: Some(user),
        active_page: "community".into(),
        activities,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (hoat-dong): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /api/cong-dong/hoat-dong — JSON API cho activity items.
///
/// Trả về JSON: `{ items: [...], count: N, cached_minutes: 5 }`.
/// Yêu cầu đăng nhập — guest trả về 401 JSON error.
pub async fn hoat_dong_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(_user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    let activities = fetch_activities(&state).await;
    Json(serde_json::json!({
        "items": activities,
        "count": activities.len(),
        "cached_minutes": 5,
    }))
    .into_response()
}

//! Handlers cho trang Tìm Kiếm toàn cục — Giai đoạn 19 (v0.9.14).
//!
//! Routes:
//!   - GET /tim-kiem?q=... — Trang kết quả tìm kiếm đa loại (users, books, topics, groups)

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Query params ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

// ─── Result models ───────────────────────────────────────────────────────

/// Một kết quả user.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserResult {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub rank: String,
    pub role: String,
    pub phap_danh: Option<String>,
}

/// Một kết quả sách.
/// v0.9.25 FIX (bug B5): đổi `cover_image_url` → `cover_url` để khớp schema migration 012
/// (trước v0.9.25 query fail vì cột `cover_image_url` không tồn tại trong bảng books).
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct BookResult {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub author: Option<String>,
    pub cover_url: Option<String>,
    pub view_count: i64,
}

/// Một kết quả chủ đề.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct TopicResult {
    pub id: Uuid,
    pub title: String,
    pub body_preview: String,
    pub view_count: i64,
    pub group_slug: String,
    pub group_name: String,
    pub author_name: String,
}

/// Một kết quả nhóm.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct GroupResult {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub member_count: i64,
}

/// Tổng hợp kết quả.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub query: String,
    pub users: Vec<UserResult>,
    pub books: Vec<BookResult>,
    pub topics: Vec<TopicResult>,
    pub groups: Vec<GroupResult>,
    pub total: usize,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.users.is_empty() && self.books.is_empty() && self.topics.is_empty() && self.groups.is_empty()
    }
}

// ─── Template ────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "tim-kiem/index.html")]
pub struct TimKiemTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub results: SearchResults,
}

// ─── Handlers ────────────────────────────────────────────────────────────

/// GET /tim-kiem?q=... — Trang tìm kiếm toàn cục.
pub async fn tim_kiem_index(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let q = query.q.unwrap_or_default();
    let q_trimmed = q.trim();

    let results = if q_trimmed.is_empty() {
        SearchResults::default()
    } else if q_trimmed.chars().count() < 2 {
        // Query quá ngắn — không search
        SearchResults {
            query: q_trimmed.to_string(),
            ..Default::default()
        }
    } else {
        search_all(&state.pool, q_trimmed).await
    };

    let html = TimKiemTemplate {
        user,
        active_page: "tim_kiem".into(),
        results,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (tim-kiem): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Tìm kiếm đồng thời users + books + topics + groups.
/// v0.9.27: Escape ILIKE wildcards (% _) trong search query để tránh
/// unintended broad matches (vd. user search "%" → match all rows).
async fn search_all(pool: &sqlx::PgPool, q: &str) -> SearchResults {
    // v0.9.27: Escape % và _ trong user input trước khi wrap với %...%
    let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{escaped}%");

    // Search users (tối đa 10)
    let users = sqlx::query_as::<_, UserResult>(
        "SELECT id, display_name, avatar_url, rank, role, phap_danh
         FROM users
         WHERE is_active = true
           AND (display_name ILIKE $1 ESCAPE '\\' OR phap_danh ILIKE $1 ESCAPE '\\' OR email ILIKE $1 ESCAPE '\\')
         ORDER BY
            CASE WHEN display_name ILIKE $1 ESCAPE '\\' THEN 0 ELSE 1 END,
            display_name
         LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Search users fail: {e}");
        vec![]
    });

    // Search books (tối đa 10)
    // v0.9.25 FIX (bug B5): đổi `cover_image_url` → `cover_url` (schema migration 012).
    let books = sqlx::query_as::<_, BookResult>(
        "SELECT id, slug, title, author, cover_url, view_count
         FROM books
         WHERE is_active = true
           AND (title ILIKE $1 ESCAPE '\\' OR author ILIKE $1 ESCAPE '\\' OR description ILIKE $1 ESCAPE '\\')
         ORDER BY view_count DESC
         LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Search books fail: {e}");
        vec![]
    });

    // Search topics (tối đa 10)
    let topics = sqlx::query_as::<_, TopicResult>(
        "SELECT t.id, t.title, LEFT(t.body, 200) AS body_preview, t.view_count,
                g.slug AS group_slug, g.name AS group_name,
                COALESCE(u.display_name, u.email, 'Ẩn danh') AS author_name
         FROM topics t
         JOIN groups g ON g.id = t.group_id
         LEFT JOIN users u ON u.id = t.author_id
         WHERE t.is_active = true
           AND (t.title ILIKE $1 ESCAPE '\\' OR t.body ILIKE $1 ESCAPE '\\')
         ORDER BY t.view_count DESC
         LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Search topics fail: {e}");
        vec![]
    });

    // Search groups (tối đa 10)
    // v0.9.25 FIX (bug B5): bảng `groups` không có cột `cover_image_url` —
    // dùng subquery join với `images` để lấy `stored_filename` (URL ảnh cover upload).
    // Trước v0.9.25, query fail vì column không tồn tại → search groups luôn trả empty.
    let groups = sqlx::query_as::<_, GroupResult>(
        "SELECT g.id, g.slug, g.name, g.description,
                (SELECT i.stored_filename FROM images i WHERE i.id = g.cover_upload_id) AS cover_image_url,
                COUNT(gm.user_id)::BIGINT AS member_count
         FROM groups g
         LEFT JOIN group_members gm ON gm.group_id = g.id
         WHERE g.is_active = true
           AND (g.name ILIKE $1 ESCAPE '\\' OR g.description ILIKE $1 ESCAPE '\\')
         GROUP BY g.id, g.slug, g.name, g.description, g.cover_upload_id
         ORDER BY member_count DESC
         LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Search groups fail: {e}");
        vec![]
    });

    let total = users.len() + books.len() + topics.len() + groups.len();
    SearchResults {
        query: q.to_string(),
        users,
        books,
        topics,
        groups,
        total,
    }
}

//! Handlers cho Tiến Độ Đọc Sách + Bookmark — Giai đoạn 56 (v0.9.45).
//!
//! Routes:
//!   POST /api/kinh-sach/luu-tien-do        — Lưu tiến độ đọc (JSON API)
//!   GET  /api/kinh-sach/tien-do/{book_id}  — Lấy tiến độ đọc của user cho 1 sách
//!   GET  /api/kinh-sach/tien-do            — Lấy tất cả tiến độ đọc của user
//!   POST /api/kinh-sach/chuong/{chapter_id}/bookmark   — Bookmark chương
//!   POST /api/kinh-sach/chuong/{chapter_id}/huy-bookmark — Xoá bookmark
//!   GET  /api/kinh-sach/bookmarks          — Tất cả bookmark của user
//!
//! Logic:
//!   - Khi user đọc chương: POST /api/kinh-sach/luu-tien-do với {book_id, chapter_id, scroll_pos, reading_seconds}
//!   - Khi user quay lại sách: GET /api/kinh-sach/tien-do/{book_id} → "Tiếp tục đọc từ chương X"

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;

#[derive(Debug, Deserialize)]
pub struct SaveProgressRequest {
    pub book_id: Uuid,
    pub chapter_id: Uuid,
    pub scroll_position: Option<i32>,
    pub reading_seconds: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ReadingProgressRow {
    pub book_id: Uuid,
    pub book_title: String,
    pub book_slug: String,
    pub book_cover_url: Option<String>,
    pub last_chapter_id: Option<Uuid>,
    pub last_chapter_title: Option<String>,
    pub last_chapter_slug: Option<String>,
    pub progress_percent: i16,
    pub chapters_read: i64,
    pub total_reading_seconds: i64,
    pub last_read_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BookmarkRow {
    pub id: i64,
    pub book_id: Uuid,
    pub book_title: String,
    pub book_slug: String,
    pub chapter_id: Uuid,
    pub chapter_title: String,
    pub chapter_slug: String,
    pub note: String,
    pub label: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BookmarkForm {
    pub note: Option<String>,
    pub label: Option<String>,
}

/// POST /api/kinh-sach/luu-tien-do — Lưu tiến độ đọc.
pub async fn api_save_progress(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<SaveProgressRequest>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let scroll_pos = body.scroll_position.unwrap_or(0);
    let reading_seconds = body.reading_seconds.unwrap_or(0).max(0);

    // Lấy book chapter count + current chapters_read
    let book_info: Option<(i64,)> = sqlx::query_as(
        "SELECT chapter_count FROM books WHERE id = $1"
    )
    .bind(body.book_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let chapter_count = book_info.map(|(c,)| c.max(1)).unwrap_or(1);

    // Upsert progress
    let result = sqlx::query(
        "INSERT INTO reading_progress
            (user_id, book_id, last_chapter_id, scroll_position, total_reading_seconds,
             chapters_read, last_read_at, progress_percent)
         VALUES ($1, $2, $3, $4, $5, 1, NOW(), 1)
         ON CONFLICT (user_id, book_id) DO UPDATE SET
            last_chapter_id       = EXCLUDED.last_chapter_id,
            scroll_position       = EXCLUDED.scroll_position,
            total_reading_seconds = reading_progress.total_reading_seconds + EXCLUDED.total_reading_seconds,
            chapters_read         = reading_progress.chapters_read + 1,
            last_read_at          = NOW(),
            updated_at            = NOW()
         RETURNING progress_percent"
    )
    .bind(user.id)
    .bind(body.book_id)
    .bind(body.chapter_id)
    .bind(scroll_pos)
    .bind(reading_seconds)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(_) => {
            // Update progress_percent based on chapters_read / chapter_count
            let _ = sqlx::query(
                "UPDATE reading_progress
                 SET progress_percent = LEAST(100, (chapters_read * 100 / GREATEST($3, 1)))
                 WHERE user_id = $1 AND book_id = $2"
            )
            .bind(user.id)
            .bind(body.book_id)
            .bind(chapter_count as i64)
            .execute(&state.pool)
            .await;

            Json(serde_json::json!({
                "success": true,
                "message": "Đã lưu tiến độ đọc."
            }))
            .into_response()
        }
        Err(e) => {
            log::error!("❌ Lỗi lưu reading_progress: {e}");
            Json(serde_json::json!({
                "success": false, "message": "Không thể lưu tiến độ."
            }))
            .into_response()
        }
    }
}

/// GET /api/kinh-sach/tien-do/{book_id} — Lấy tiến độ đọc cho 1 sách.
pub async fn api_get_progress(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(book_id): Path<Uuid>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let row: Option<ReadingProgressRow> = sqlx::query_as(
        "SELECT rp.book_id, b.title AS book_title, b.slug AS book_slug, b.cover_url AS book_cover_url,
                rp.last_chapter_id,
                bc.title AS last_chapter_title,
                bc.slug AS last_chapter_slug,
                rp.progress_percent, rp.chapters_read, rp.total_reading_seconds, rp.last_read_at
         FROM reading_progress rp
         JOIN books b ON b.id = rp.book_id
         LEFT JOIN book_chapters bc ON bc.id = rp.last_chapter_id
         WHERE rp.user_id = $1 AND rp.book_id = $2"
    )
    .bind(user.id)
    .bind(book_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    Json(serde_json::json!({
        "success": true,
        "progress": row
    }))
    .into_response()
}

/// GET /api/kinh-sach/tien-do — Tất cả tiến độ đọc của user.
pub async fn api_list_progress(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let rows: Vec<ReadingProgressRow> = sqlx::query_as(
        "SELECT rp.book_id, b.title AS book_title, b.slug AS book_slug, b.cover_url AS book_cover_url,
                rp.last_chapter_id,
                bc.title AS last_chapter_title,
                bc.slug AS last_chapter_slug,
                rp.progress_percent, rp.chapters_read, rp.total_reading_seconds, rp.last_read_at
         FROM reading_progress rp
         JOIN books b ON b.id = rp.book_id
         LEFT JOIN book_chapters bc ON bc.id = rp.last_chapter_id
         WHERE rp.user_id = $1
         ORDER BY rp.last_read_at DESC LIMIT 50"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "success": true,
        "progress": rows
    }))
    .into_response()
}

/// POST /api/kinh-sach/chuong/{chapter_id}/bookmark — Bookmark chương.
pub async fn api_add_bookmark(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(chapter_id): Path<Uuid>,
    Json(body): Json<BookmarkForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    // Lấy book_id từ chapter
    let chapter_info: Option<(Uuid,)> = sqlx::query_as("SELECT book_id FROM book_chapters WHERE id = $1")
        .bind(chapter_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    let Some((book_id,)) = chapter_info else {
        return Json(serde_json::json!({
            "success": false, "message": "Chương không tồn tại."
        }))
        .into_response();
    };

    let note = body.note.unwrap_or_default().trim().to_string();
    let label = body.label.unwrap_or_else(|| "bookmark".into());

    let result = sqlx::query(
        "INSERT INTO chapter_bookmarks (user_id, book_id, chapter_id, note, label)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, chapter_id) DO UPDATE SET
            note = EXCLUDED.note,
            label = EXCLUDED.label"
    )
    .bind(user.id)
    .bind(book_id)
    .bind(chapter_id)
    .bind(&note)
    .bind(&label)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi add bookmark: {e}");
        return Json(serde_json::json!({
            "success": false, "message": "Không thể bookmark."
        }))
        .into_response();
    }

    Json(serde_json::json!({
        "success": true,
        "message": "Đã bookmark chương."
    }))
    .into_response()
}

/// POST /api/kinh-sach/chuong/{chapter_id}/huy-bookmark — Xoá bookmark.
pub async fn api_remove_bookmark(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(chapter_id): Path<Uuid>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let _ = sqlx::query(
        "DELETE FROM chapter_bookmarks WHERE user_id = $1 AND chapter_id = $2"
    )
    .bind(user.id)
    .bind(chapter_id)
    .execute(&state.pool)
    .await;

    Json(serde_json::json!({
        "success": true,
        "message": "Đã xoá bookmark."
    }))
    .into_response()
}

/// GET /api/kinh-sach/bookmarks — Tất cả bookmark của user.
pub async fn api_list_bookmarks(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let rows: Vec<BookmarkRow> = sqlx::query_as(
        "SELECT cb.id, cb.book_id, b.title AS book_title, b.slug AS book_slug,
                cb.chapter_id, bc.title AS chapter_title, bc.slug AS chapter_slug,
                cb.note, cb.label, cb.created_at
         FROM chapter_bookmarks cb
         JOIN books b ON b.id = cb.book_id
         JOIN book_chapters bc ON bc.id = cb.chapter_id
         WHERE cb.user_id = $1
         ORDER BY cb.created_at DESC LIMIT 100"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "success": true,
        "bookmarks": rows
    }))
    .into_response()
}

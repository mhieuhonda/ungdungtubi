//! Handlers cho Hệ Thống Cấp Bậc Tự Động — Giai đoạn 54 (v0.9.45).
//!
//! Theo tài liệu "ỨNG DỤNG TỪ BI.docx" mục II.3.b (Hệ thống cấp bậc):
//!   Người Mới → Người Thường → Người Bình Thường → Người Tốt → ...
//!   → Thiện Nhân → Đại Gia
//!
//! Routes:
//!   POST /admin/thanh-vien/tang-cap-tu-dong   — Admin trigger auto-promote tất cả
//!   GET  /admin/thanh-vien/lich-su-tang-cap    — Admin xem lịch sử thay đổi rank
//!   POST /api/users/{user_id}/tang-cap-tu-dong — Auto-promote 1 user (API)
//!
//! Logic:
//!   - Gọi SQL function `calculate_member_rank(user_id)` để tính rank mới
//!   - So sánh với rank hiện tại
//!   - Nếu khác → UPDATE users.rank + INSERT INTO member_rank_history

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::{admin::render_forbidden, get_user_from_session};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct RankHistoryRow {
    pub id: i64,
    pub user_id: Uuid,
    pub from_rank: String,
    pub to_rank: String,
    pub reason: String,
    pub note: String,
    pub created_at: DateTime<Utc>,
    pub user_display_name: String,
    pub changed_by_name: Option<String>,
}

/// Tính rank hiện tại cho user + auto-promote nếu cần.
/// Trả về (old_rank, new_rank, was_promoted).
pub async fn auto_promote_user(pool: &sqlx::PgPool, user_id: Uuid) -> (String, String, bool) {
    // Lấy rank hiện tại
    let current: Option<(String,)> = sqlx::query_as("SELECT rank FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let Some((current_rank,)) = current else {
        return (String::new(), String::new(), false);
    };

    // Gọi SQL function calculate_member_rank
    let new_rank: Option<(String,)> = sqlx::query_as("SELECT calculate_member_rank($1)")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let Some((new_rank,)) = new_rank else {
        return (current_rank.clone(), current_rank, false);
    };

    if new_rank != current_rank {
        // Promote: UPDATE + log history
        let _ = sqlx::query("UPDATE users SET rank = $1, updated_at = NOW() WHERE id = $2")
            .bind(&new_rank)
            .bind(user_id)
            .execute(pool)
            .await;

        let _ = sqlx::query(
            "INSERT INTO member_rank_history (user_id, from_rank, to_rank, reason, note)
             VALUES ($1, $2, $3, 'auto', 'Auto-promote Giai đoạn 54')"
        )
        .bind(user_id)
        .bind(&current_rank)
        .bind(&new_rank)
        .execute(pool)
        .await;

        log::info!(
            "📈 Auto-promote user {}: {} → {}",
            user_id, current_rank, new_rank
        );
        (current_rank, new_rank, true)
    } else {
        (current_rank.clone(), new_rank, false)
    }
}

/// POST /admin/thanh-vien/tang-cap-tu-dong — Admin trigger auto-promote tất cả users.
pub async fn admin_auto_promote_all(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    // Lấy tất cả active user IDs
    let user_ids: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE is_active = true")
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

    let mut promoted = 0u64;
    for (user_id,) in &user_ids {
        let (_, _, was_promoted) = auto_promote_user(&state.pool, *user_id).await;
        if was_promoted {
            promoted += 1;
        }
    }

    log::info!(
        "📈 Admin {} trigger auto-promote all: {} users checked, {} promoted",
        user.id,
        user_ids.len(),
        promoted
    );

    Html(format!(
        "<!DOCTYPE html><html><body>\
         <h2>✅ Đã kiểm tra {} users — {promoted} users được tăng cấp.</h2>\
         <p><a href='/admin/thanh-vien'>← Về trang thành viên</a> | \
         <a href='/admin/thanh-vien/lich-su-tang-cap'>Xem lịch sử tăng cấp</a></p>\
         </body></html>",
        user_ids.len()
    ))
    .into_response()
}

/// GET /admin/thanh-vien/lich-su-tang-cap — Lịch sử thay đổi rank.
pub async fn admin_rank_history(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    let rows: Vec<RankHistoryRow> = sqlx::query_as(
        "SELECT h.id, h.user_id, h.from_rank, h.to_rank, h.reason, h.note, h.created_at,
                u.display_name AS user_display_name,
                admin.display_name AS changed_by_name
         FROM member_rank_history h
         JOIN users u ON u.id = h.user_id
         LEFT JOIN users admin ON admin.id = h.changed_by
         ORDER BY h.created_at DESC
         LIMIT 200"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut html = String::with_capacity(8192);
    html.push_str(&format!(
        "<!DOCTYPE html><html lang='vi'><head><meta charset='UTF-8'>\
         <meta name='viewport' content='width=device-width, initial-scale=1.0'>\
         <title>Lịch Sử Tăng Cấp — Admin</title>\
         <script src='https://cdn.tailwindcss.com'></script></head>\
         <body class='bg-gray-50 min-h-screen p-4'>\
         <div class='max-w-6xl mx-auto'>\
         <h1 class='text-2xl font-bold mb-4'>📈 Lịch Sử Thay Đổi Cấp Bậc</h1>\
         <p class='text-sm text-gray-600 mb-4'>{} entries — Giai đoạn 54 (v0.9.45)</p>\
         <div class='bg-white rounded-xl shadow overflow-hidden'>\
         <table class='min-w-full divide-y divide-gray-200'>\
         <thead class='bg-gray-50'><tr>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Thành viên</th>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Từ</th>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Đến</th>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Lý do</th>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Ngày</th>\
         <th class='px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase'>Thực hiện</th>\
         </tr></thead><tbody>",
        rows.len()
    ));

    for r in &rows {
        html.push_str(&format!(
            "<tr class='border-b hover:bg-gray-50'>\
             <td class='px-4 py-3 text-sm'>{}</td>\
             <td class='px-4 py-3 text-sm text-gray-500'>{}</td>\
             <td class='px-4 py-3 text-sm font-semibold text-emerald-700'>{}</td>\
             <td class='px-4 py-3 text-xs'><span class='px-2 py-1 rounded bg-indigo-100 text-indigo-700'>{}</span></td>\
             <td class='px-4 py-3 text-xs text-gray-500'>{}</td>\
             <td class='px-4 py-3 text-xs text-gray-500'>{}</td>\
             </tr>",
            r.user_display_name,
            r.from_rank,
            r.to_rank,
            r.reason,
            r.created_at.format("%Y-%m-%d %H:%M"),
            r.changed_by_name.as_deref().unwrap_or("(auto)")
        ));
    }
    html.push_str("</tbody></table></div>");
    html.push_str(
        "<div class='mt-4 flex gap-2'>\
         <a href='/admin/thanh-vien/tang-cap-tu-dong' class='bg-emerald-600 text-white px-4 py-2 rounded-lg'>📈 Tăng cấp tự động tất cả</a>\
         <a href='/admin/thanh-vien' class='bg-gray-200 px-4 py-2 rounded-lg'>← Về trang thành viên</a>\
         </div></div></body></html>",
    );

    Html(html).into_response()
}

/// POST /api/users/{user_id}/tang-cap-tu-dong — API trigger auto-promote 1 user.
pub async fn api_auto_promote_user(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return axum::response::Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };
    if !user.is_staff() {
        return axum::response::Json(serde_json::json!({
            "success": false, "message": "Không có quyền."
        }))
        .into_response();
    }

    let (old_rank, new_rank, promoted) = auto_promote_user(&state.pool, user_id).await;

    axum::response::Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "old_rank": old_rank,
        "new_rank": new_rank,
        "promoted": promoted,
        "message": if promoted {
            format!("Đã tăng cấp: {} → {}", old_rank, new_rank)
        } else {
            "Cấp bậc không thay đổi.".into()
        }
    }))
    .into_response()
}

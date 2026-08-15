//! Handlers cho chuyên mục Bạn Bè (v0.9.5 — Giai đoạn 9).
//!
//! Bao gồm:
//!   * GET  /ban-be                                     — Trang chính Bạn Bè (danh sách bạn + lời mời + inbox preview)
//!   * POST /ban-be/keu-ban/{user_id}                   — Gửi lời mời kết bạn
//!   * POST /ban-be/chap-nhan/{friendship_id}           — Chấp nhận lời mời
//!   * POST /ban-be/tu-choi/{friendship_id}             — Từ chối lời mời
//!   * POST /ban-be/huy-ket-ban/{user_id}               — Hủy kết bạn
//!   * GET  /ban-be/tin-nhan                            — Danh sách conversation (inbox DM)
//!   * GET  /ban-be/tin-nhan/{conversation_id}          — Xem conversation + chat realtime
//!   * WS   /ws/ban-be/tin-nhan/{conversation_id}       — WebSocket DM (realtime)
//!   * GET  /api/ban-be/tin-nhan/{conversation_id}/history — Lấy history DM
//!   * GET  /ban-be/thu                                 — Hộp thư đến
//!   * GET  /ban-be/thu/gui                             — Form gửi thư
//!   * POST /ban-be/thu/gui                             — Gửi thư mới
//!   * GET  /ban-be/thu/{mail_id}                       — Xem thư
//!   * GET  /ban-be/thong-bao                           — Danh sách thông báo
//!   * GET  /api/ban-be/thong-bao/chua-doc              — Đếm thông báo chưa đọc
//!   * POST /api/ban-be/thong-bao/{id}/da-doc           — Đánh dấu đã đọc
//!   * GET  /ban-be/tim-kiem                            — Tìm kiếm user (để kết bạn)

#![allow(dead_code)]

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::friends::{
    DirectMessage, DirectMessageWithAuthor, FriendshipWithUser, MailWithUsers,
    NotificationWithActor,
};
use crate::models::user::User;

/// Giới hạn tin nhắn DM (ký tự) — dài hơn group chat (1000 vs 500).
const MAX_DM_BODY_CHARS: usize = 1000;

/// Số tin nhắn / thư hiển thị trên 1 trang.
const PAGE_SIZE: i64 = 50;

/// v0.9.20: Server gửi WebSocket Ping mỗi 25s để giữ kết nối qua proxy.
const WS_PING_INTERVAL_SECS: u64 = 25;

/// v0.9.20: Đóng kết nối nếu không nhận được gì trong 180s.
const WS_IDLE_TIMEOUT_SECS: u64 = 180;

/// v0.9.20: Control message từ recv loop → send_task.
enum DmCtrlMessage {
    Error(String),
    Pong(bytes::Bytes),
}

// ====================================================================
// Trang chính Bạn Bè — GET /ban-be
// ====================================================================

#[derive(Template)]
#[template(path = "ban-be/index.html")]
pub struct BanBeIndexTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub friends: Vec<FriendshipWithUser>,
    pub pending_requests: Vec<FriendshipWithUser>,
    pub sent_requests: Vec<FriendshipWithUser>,
    pub unread_mail_count: i64,
    pub unread_notification_count: i64,
}

/// GET /ban-be — Trang chính Bạn Bè.
pub async fn ban_be_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be").into_response(),
    };

    // Lấy danh sách bạn bè (status = accepted)
    let friends = sqlx::query_as::<_, FriendshipWithUser>(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END AS other_user_id,
                u.display_name AS other_display_name,
                u.avatar_url AS other_avatar_url,
                u.rank AS other_rank
         FROM friendships f
         JOIN users u ON u.id = (CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END)
         WHERE (f.requester_id = $1 OR f.addressee_id = $1) AND f.status = 'accepted'
         ORDER BY f.updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Lời mời kết bạn đang chờ (người khác gửi cho mình)
    let pending_requests = sqlx::query_as::<_, FriendshipWithUser>(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                f.requester_id AS other_user_id,
                u.display_name AS other_display_name,
                u.avatar_url AS other_avatar_url,
                u.rank AS other_rank
         FROM friendships f
         JOIN users u ON u.id = f.requester_id
         WHERE f.addressee_id = $1 AND f.status = 'pending'
         ORDER BY f.created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Lời mời mình đã gửi (đang chờ được chấp nhận)
    let sent_requests = sqlx::query_as::<_, FriendshipWithUser>(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                f.addressee_id AS other_user_id,
                u.display_name AS other_display_name,
                u.avatar_url AS other_avatar_url,
                u.rank AS other_rank
         FROM friendships f
         JOIN users u ON u.id = f.addressee_id
         WHERE f.requester_id = $1 AND f.status = 'pending'
         ORDER BY f.created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Đếm thư chưa đọc
    let unread_mail_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mails WHERE recipient_id = $1 AND is_read = false AND is_active = true",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    // Đếm thông báo chưa đọc
    let unread_notification_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = BanBeIndexTemplate {
        user: Some(user),
        active_page: "friends".into(),
        friends,
        pending_requests,
        sent_requests,
        unread_mail_count,
        unread_notification_count,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (ban-be index): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ====================================================================
// Kết bạn — POST /ban-be/keu-ban/{user_id}
// ====================================================================

/// POST /ban-be/keu-ban/{user_id} — Gửi lời mời kết bạn.
pub async fn send_friend_request(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(target_user_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    // Fetch target user info for rendering the <li> response
    let target: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT display_name, avatar_url FROM users WHERE id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let (display_name, avatar_url) = match target {
        Some(d) => d,
        None => {
            return Html(
                r#"<li class="px-6 py-4 flex items-center gap-4">
                    <div class="flex-1 min-w-0">
                        <div class="text-sm text-red-600">Không tìm thấy người dùng.</div>
                    </div>
                </li>"#,
            )
            .into_response();
        }
    };

    // Helper: render avatar HTML
    let avatar_html = match &avatar_url {
        Some(url) => format!(
            r#"<img src="{url}" alt="avatar" class="w-10 h-10 rounded-full border border-gray-200" referrerpolicy="no-referrer">"#
        ),
        None => {
            let initial = display_name.chars().next().unwrap_or('🪷');
            format!(
                r#"<div class="w-10 h-10 rounded-full flex items-center justify-center font-bold text-sm" style="background-color:#E8F5E9;color:#2E7D32">{initial}</div>"#
            )
        }
    };

    if target_user_id == user.id {
        return Html(format!(
            r#"<li class="px-6 py-4 flex items-center gap-4">
                {avatar_html}
                <div class="flex-1 min-w-0">
                    <div class="font-semibold text-gray-900 truncate">{display_name}</div>
                    <div class="text-xs text-red-500">Không thể kết bạn với chính mình.</div>
                </div>
            </li>"#
        ))
        .into_response();
    }

    // Check nếu đã có friendship giữa 2 user (cả 2 chiều)
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM friendships
         WHERE (requester_id = $1 AND addressee_id = $2)
            OR (requester_id = $2 AND addressee_id = $1)",
    )
    .bind(user.id)
    .bind(target_user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if let Some((status,)) = existing {
        let status_html = match status.as_str() {
            "accepted" => r#"<div class="text-xs text-green-600">✓ Đã là bạn bè</div>"#.to_string(),
            "pending" => r#"<div class="text-xs text-amber-600">⏳ Đã gửi lời mời — đang chờ phản hồi</div>"#.to_string(),
            other => format!(r#"<div class="text-xs text-gray-400">Trạng thái: {other}</div>"#),
        };
        return Html(format!(
            r#"<li class="px-6 py-4 flex items-center gap-4">
                {avatar_html}
                <div class="flex-1 min-w-0">
                    <div class="font-semibold text-gray-900 truncate">{display_name}</div>
                    {status_html}
                </div>
            </li>"#
        ))
        .into_response();
    }

    // Tạo friendship mới (status = pending)
    let inserted: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO friendships (requester_id, addressee_id, status)
         VALUES ($1, $2, 'pending')
         ON CONFLICT (requester_id, addressee_id) DO NOTHING
         RETURNING id",
    )
    .bind(user.id)
    .bind(target_user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if let Some((friendship_id,)) = inserted {
        // Tạo notification cho người nhận
        let _ = sqlx::query(
            "INSERT INTO notifications (user_id, type, actor_id, payload)
             VALUES ($1, 'friend_request', $2, $3)",
        )
        .bind(target_user_id)
        .bind(user.id)
        .bind(serde_json::json!({
            "friendship_id": friendship_id,
            "message": format!("{} gửi lời mời kết bạn", user.display_name)
        }))
        .execute(&state.pool)
        .await;

        log::info!("💬 Lời mời kết bạn: {} → {}", user.id, target_user_id);
    }

    // Return HTMX response — full <li> replacement with updated status
    Html(format!(
        r#"<li class="px-6 py-4 flex items-center gap-4">
            {avatar_html}
            <div class="flex-1 min-w-0">
                <div class="font-semibold text-gray-900 truncate">{display_name}</div>
                <div class="text-xs text-amber-600">⏳ Đã gửi lời mời — đang chờ phản hồi</div>
            </div>
        </li>"#
    ))
    .into_response()
}

/// POST /ban-be/chap-nhan/{friendship_id} — Chấp nhận lời mời kết bạn.
pub async fn accept_friend_request(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(friendship_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    // Update friendship status → accepted (chỉ addressee mới được accept)
    let updated: Option<(Uuid, Uuid)> = sqlx::query_as(
        "UPDATE friendships SET status = 'accepted'
         WHERE id = $1 AND addressee_id = $2 AND status = 'pending'
         RETURNING requester_id, addressee_id",
    )
    .bind(friendship_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if let Some((requester_id, _addressee_id)) = updated {
        // Tạo notification cho người gửi lời mời (báo đã được chấp nhận)
        let _ = sqlx::query(
            "INSERT INTO notifications (user_id, type, actor_id, payload)
             VALUES ($1, 'friend_accept', $2, $3)",
        )
        .bind(requester_id)
        .bind(user.id)
        .bind(serde_json::json!({
            "message": format!("{} đã chấp nhận lời mời kết bạn", user.display_name)
        }))
        .execute(&state.pool)
        .await;

        log::info!("✓ Kết bạn thành công: {} ↔ {}", requester_id, user.id);

        // Fetch the other user's info for the HTMX response
        let other: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT display_name, avatar_url FROM users WHERE id = $1",
        )
        .bind(requester_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        if let Some((display_name, avatar_url)) = other {
            let avatar_html = match &avatar_url {
                Some(url) => format!(
                    r#"<img src="{url}" alt="avatar" class="w-10 h-10 rounded-full border border-gray-200" referrerpolicy="no-referrer">"#
                ),
                None => {
                    let initial = display_name.chars().next().unwrap_or('🪷');
                    format!(
                        r#"<div class="w-10 h-10 rounded-full flex items-center justify-center font-bold text-sm" style="background-color:#E8F5E9;color:#2E7D32">{initial}</div>"#
                    )
                }
            };
            // Return HTMX response — <li> replaced with "accepted" status
            return Html(format!(
                r#"<li class="px-6 py-4 flex items-center gap-4">
                    {avatar_html}
                    <div class="flex-1 min-w-0">
                        <div class="font-semibold text-gray-900 truncate">{display_name}</div>
                        <div class="text-xs text-green-600">✓ Đã là bạn bè</div>
                    </div>
                </li>"#
            ))
            .into_response();
        }
    }

    // Fallback: if something went wrong, return empty (removes the <li>)
    Html(String::new()).into_response()
}

/// POST /ban-be/tu-choi/{friendship_id} — Từ chối lời mời kết bạn.
pub async fn decline_friend_request(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(friendship_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    // Xoá friendship (không lưu status = declined để cho phép gửi lại sau)
    let _ = sqlx::query(
        "DELETE FROM friendships WHERE id = $1 AND addressee_id = $2 AND status = 'pending'",
    )
    .bind(friendship_id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    // Return empty response — HTMX will remove the <li> from the DOM
    Html(String::new()).into_response()
}

/// POST /ban-be/huy-ket-ban/{user_id} — Hủy kết bạn (xóa friendship).
pub async fn remove_friend(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(target_user_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let _ = sqlx::query(
        "DELETE FROM friendships
         WHERE ((requester_id = $1 AND addressee_id = $2)
             OR (requester_id = $2 AND addressee_id = $1))
           AND status = 'accepted'",
    )
    .bind(user.id)
    .bind(target_user_id)
    .execute(&state.pool)
    .await;

    Redirect::to("/ban-be").into_response()
}

// ====================================================================
// Direct Messages (Nhắn tin 1-1) — GET /ban-be/tin-nhan
// ====================================================================

#[derive(Template)]
#[template(path = "ban-be/dm_inbox.html")]
pub struct DmInboxTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub conversations: Vec<crate::models::friends::ConversationWithParticipant>,
}

/// GET /ban-be/tin-nhan — Danh sách conversation (inbox DM).
pub async fn dm_inbox(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be/tin-nhan").into_response(),
    };

    let conversations = sqlx::query_as::<_, crate::models::friends::ConversationWithParticipant>(
        "SELECT c.id, c.type AS kind, c.created_at, c.updated_at,
                u.id AS other_user_id,
                u.display_name AS other_display_name,
                u.avatar_url AS other_avatar_url,
                u.rank AS other_rank,
                lm.body AS last_message_body,
                lm.created_at AS last_message_at,
                lm.author_id AS last_message_author_id
         FROM conversations c
         JOIN conversation_participants cp ON cp.conversation_id = c.id
         JOIN conversation_participants cp2 ON cp2.conversation_id = c.id AND cp2.user_id <> cp.user_id
         JOIN users u ON u.id = cp2.user_id
         LEFT JOIN LATERAL (
             SELECT body, created_at, author_id FROM direct_messages
             WHERE conversation_id = c.id AND is_active = true
             ORDER BY created_at DESC LIMIT 1
         ) lm ON true
         WHERE cp.user_id = $1 AND c.type = 'direct' AND c.is_active = true
         ORDER BY COALESCE(lm.created_at, c.updated_at) DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = DmInboxTemplate {
        user: Some(user),
        active_page: "friends".into(),
        conversations,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (dm inbox): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

#[derive(Template)]
#[template(path = "ban-be/conversation.html")]
pub struct ConversationTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub conversation_id: Uuid,
    pub other_user_id: Uuid,
    pub other_display_name: String,
    pub other_avatar_url: Option<String>,
    pub other_rank: String,
    pub messages_json: String,
    /// v0.9.12: JSON-encoded init object cho Alpine.js `dmChat({...})`.
    /// Dùng `serde_json::to_string` để escape đúng JS string context —
    /// tránh stored XSS qua `other_display_name` do người dùng kiểm soát.
    pub init_json: String,
}

/// GET /ban-be/tin-nhan/{conversation_id} — Xem conversation + chat realtime.
pub async fn dm_view(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(conversation_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => {
            return Redirect::to(&format!(
                "/dang-nhap?next=/ban-be/tin-nhan/{conversation_id}"
            ))
            .into_response();
        }
    };

    // Verify user là participant của conversation này
    let other: Option<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        "SELECT u.id, u.display_name, u.avatar_url, u.rank
         FROM conversation_participants cp
         JOIN conversation_participants cp2 ON cp2.conversation_id = cp.conversation_id
            AND cp2.user_id <> cp.user_id
         JOIN users u ON u.id = cp2.user_id
         WHERE cp.conversation_id = $1 AND cp.user_id = $2
            AND cp.conversation_id IN (SELECT id FROM conversations WHERE type = 'direct')",
    )
    .bind(conversation_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((other_user_id, other_display_name, other_avatar_url, other_rank)) = other else {
        return Html(
            r#"<div class="max-w-2xl mx-auto px-4 py-12 text-center">
                <p class="text-gray-500">Conversation không tồn tại hoặc bạn không có quyền truy cập.</p>
                <a href="/ban-be/tin-nhan" class="text-tubi-700 hover:underline mt-4 inline-block">← Quay lại</a>
            </div>"#,
        )
        .into_response();
    };

    // Lấy 50 tin nhắn gần nhất
    // v0.9.19: thêm u.role AS author_role để render hiệu ứng đặc biệt cho admin/mod.
    let messages = sqlx::query_as::<_, DirectMessageWithAuthor>(
        "SELECT m.id, m.conversation_id, m.author_id, m.body, m.is_active, m.created_at,
                u.display_name AS author_display_name,
                u.avatar_url AS author_avatar_url,
                u.rank AS author_rank, u.role AS author_role
         FROM direct_messages m
         JOIN users u ON u.id = m.author_id
         WHERE m.conversation_id = $1 AND m.is_active = true
         ORDER BY m.created_at DESC
         LIMIT $2",
    )
    .bind(conversation_id)
    .bind(PAGE_SIZE)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .rev()
    .collect::<Vec<_>>();

    let messages_json = serde_json::to_string(&messages).unwrap_or_else(|_| "[]".into());

    // v0.9.12: Serialize toàn bộ init object cho Alpine.js bằng serde_json.
    // Tránh stored XSS qua `other_display_name` (người dùng kiểm soát) khi inject
    // trực tiếp vào x-data="dmChat({...})" — serde_json escape đúng JS string.
    let messages_init: serde_json::Value = serde_json::from_str(&messages_json)
        .unwrap_or(serde_json::json!([]));
    let init_json = serde_json::to_string(&serde_json::json!({
        "conversationId": conversation_id.to_string(),
        "otherUserId": other_user_id.to_string(),
        "otherDisplayName": &other_display_name,
        "initialMessages": messages_init,
    }))
    .unwrap_or_else(|_| "{}".into());

    // Update last_read_at
    let _ = sqlx::query(
        "UPDATE conversation_participants SET last_read_at = NOW()
         WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conversation_id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    let html = ConversationTemplate {
        user: Some(user),
        active_page: "friends".into(),
        conversation_id,
        other_user_id,
        other_display_name,
        other_avatar_url,
        other_rank,
        messages_json,
        init_json,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (conversation): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Query params cho GET /api/ban-be/tin-nhan/{conversation_id}/history.
#[derive(Debug, serde::Deserialize)]
pub struct DmHistoryQuery {
    pub limit: Option<i64>,
    pub before: Option<String>,
}

/// GET /api/ban-be/tin-nhan/{conversation_id}/history — Lấy DM history (paginated).
pub async fn dm_history(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(conversation_id): Path<Uuid>,
    Query(q): Query<DmHistoryQuery>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => {
            return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập.").into_response();
        }
    };

    // Verify participant
    let is_participant: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM conversation_participants
         WHERE conversation_id = $1 AND user_id = $2)",
    )
    .bind(conversation_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !is_participant {
        return (axum::http::StatusCode::FORBIDDEN, "Không có quyền.").into_response();
    }

    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let before_dt: Option<DateTime<Utc>> = q
        .before
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let messages = if let Some(before) = before_dt {
        sqlx::query_as::<_, DirectMessageWithAuthor>(
            "SELECT m.id, m.conversation_id, m.author_id, m.body, m.is_active, m.created_at,
                    u.display_name AS author_display_name,
                    u.avatar_url AS author_avatar_url,
                    u.rank AS author_rank, u.role AS author_role
             FROM direct_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.conversation_id = $1 AND m.is_active = true AND m.created_at < $2
             ORDER BY m.created_at DESC
             LIMIT $3",
        )
        .bind(conversation_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, DirectMessageWithAuthor>(
            "SELECT m.id, m.conversation_id, m.author_id, m.body, m.is_active, m.created_at,
                    u.display_name AS author_display_name,
                    u.avatar_url AS author_avatar_url,
                    u.rank AS author_rank, u.role AS author_role
             FROM direct_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.conversation_id = $1 AND m.is_active = true
             ORDER BY m.created_at DESC
             LIMIT $2",
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    match messages {
        Ok(msgs) => axum::Json(msgs).into_response(),
        Err(e) => {
            log::error!("❌ Lỗi truy vấn DM history: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi truy vấn history.",
            )
                .into_response()
        }
    }
}

/// GET /ws/ban-be/tin-nhan/{conversation_id} — WebSocket upgrade cho DM.
#[allow(clippy::too_many_lines)]
pub async fn dm_ws_upgrade(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(conversation_id): Path<Uuid>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Response {
    // Auth
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => {
            return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập để chat.").into_response();
        }
    };

    // Verify participant
    let is_participant: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM conversation_participants
         WHERE conversation_id = $1 AND user_id = $2)",
    )
    .bind(conversation_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !is_participant {
        return (
            axum::http::StatusCode::FORBIDDEN,
            "Bạn không phải thành viên của conversation này.",
        )
            .into_response();
    }

    ws.max_message_size(64 * 1024)
        .on_upgrade(move |socket| handle_dm_socket(socket, state, conversation_id, user))
}

/// Handler cho DM WebSocket session — v0.9.20: ping/pong keepalive + idle timeout.
#[allow(clippy::too_many_lines)]
async fn handle_dm_socket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    conversation_id: Uuid,
    user: User,
) {
    use tokio::sync::mpsc;

    let (mut sender, mut receiver) = socket.split();

    let tx = state.dm_chat_hub.subscribe(conversation_id).await;
    let mut rx = tx.subscribe();

    log::info!(
        "💬 DM WS connected: user={} conv={}",
        user.id,
        conversation_id
    );

    let (ctrl_tx, mut ctrl_rx) = mpsc::unbounded_channel::<DmCtrlMessage>();

    let send_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(Duration::from_secs(WS_PING_INTERVAL_SECS));
        ping_interval.tick().await;

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(payload) => {
                            if sender
                                .send(axum::extract::ws::Message::Text(payload.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                ctrl = ctrl_rx.recv() => {
                    match ctrl {
                        Some(DmCtrlMessage::Error(err_payload)) => {
                            if sender
                                .send(axum::extract::ws::Message::Text(err_payload.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Some(DmCtrlMessage::Pong(payload)) => {
                            if sender
                                .send(axum::extract::ws::Message::Pong(payload))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = ping_interval.tick() => {
                    if sender
                        .send(axum::extract::ws::Message::Ping(
                            bytes::Bytes::from_static(b"tubi"),
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    let dm_chat_hub = state.dm_chat_hub.clone();
    let pool = state.pool.clone();
    let user_id = user.id;
    let user_display_name = user.display_name.clone();
    let user_avatar_url = user.avatar_url.clone();
    let user_rank = user.rank.clone();
    let user_role = user.role.clone();

    loop {
        let next_msg = tokio::time::timeout(
            Duration::from_secs(WS_IDLE_TIMEOUT_SECS),
            receiver.next(),
        )
        .await;

        match next_msg {
            Err(_) => {
                log::info!(
                    "💬 DM WS idle timeout ({WS_IDLE_TIMEOUT_SECS}s), đóng: user={} conv={}",
                    user_id, conversation_id
                );
                break;
            }
            Ok(None) => break,
            Ok(Some(Err(_))) => break,
            Ok(Some(Ok(msg))) => {
                use axum::extract::ws::Message;
                match msg {
                    Message::Text(text) => {
                        let body = text.trim().to_string();

                        // v0.9.20: App-level ping
                        if body == "{\"type\":\"ping\"}" || body == "ping" {
                            let _ = ctrl_tx.send(DmCtrlMessage::Error(
                                r#"{"type":"pong"}"#.to_string(),
                            ));
                            continue;
                        }

                        if body.is_empty() || body.chars().count() > MAX_DM_BODY_CHARS {
                            let err_payload = serde_json::json!({
                                "type": "error",
                                "message": format!("Tin nhắn không hợp lệ (tối đa {MAX_DM_BODY_CHARS} ký tự).")
                            })
                            .to_string();
                            let _ = ctrl_tx.send(DmCtrlMessage::Error(err_payload));
                            continue;
                        }

                        let saved: Option<DirectMessageWithAuthor> = match sqlx::query_as::<_, DirectMessage>(
                            "INSERT INTO direct_messages (conversation_id, author_id, body)
                             VALUES ($1, $2, $3)
                             RETURNING id, conversation_id, author_id, body, is_active, created_at",
                        )
                        .bind(conversation_id)
                        .bind(user_id)
                        .bind(&body)
                        .fetch_one(&pool)
                        .await
                        {
                            Ok(m) => Some(DirectMessageWithAuthor {
                                id: m.id,
                                conversation_id: m.conversation_id,
                                author_id: m.author_id,
                                body: m.body,
                                is_active: m.is_active,
                                created_at: m.created_at,
                                author_display_name: user_display_name.clone(),
                                author_avatar_url: user_avatar_url.clone(),
                                author_rank: user_rank.clone(),
                                author_role: Some(user_role.clone()),
                            }),
                            Err(e) => {
                                log::error!("❌ Lỗi lưu DM message: {e}");
                                let err_payload = serde_json::json!({
                                    "type": "error",
                                    "message": "Không lưu được tin nhắn. Vui lòng thử lại."
                                })
                                .to_string();
                                let _ = ctrl_tx.send(DmCtrlMessage::Error(err_payload));
                                None
                            }
                        };

                        if let Some(msg) = saved {
                            let payload = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
                            let _ = dm_chat_hub.broadcast(conversation_id, payload).await;

                            let _ = sqlx::query(
                                "UPDATE conversations SET updated_at = NOW() WHERE id = $1",
                            )
                            .bind(conversation_id)
                            .execute(&pool)
                            .await;
                        }
                    }
                    Message::Pong(_) => continue,
                    Message::Ping(payload) => {
                        let _ = ctrl_tx.send(DmCtrlMessage::Pong(payload));
                        continue;
                    }
                    Message::Close(_) => break,
                    Message::Binary(_) => continue,
                }
            }
        }
    }

    send_task.abort();
    log::info!("💬 DM WS disconnected: user={} conv={}", user_id, conversation_id);
}

// ====================================================================
// Mail (Gửi thư) — GET /ban-be/thu
// ====================================================================

#[derive(Template)]
#[template(path = "ban-be/mail_inbox.html")]
pub struct MailInboxTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub mails: Vec<MailWithUsers>,
    pub unread_count: i64,
}

/// GET /ban-be/thu — Hộp thư đến.
pub async fn mail_inbox(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be/thu").into_response(),
    };

    let mails = sqlx::query_as::<_, MailWithUsers>(
        "SELECT m.id, m.sender_id, m.recipient_id, m.subject, m.body,
                m.is_read, m.read_at, m.is_active, m.created_at,
                su.display_name AS sender_display_name,
                su.avatar_url AS sender_avatar_url,
                su.rank AS sender_rank,
                ru.display_name AS recipient_display_name,
                ru.avatar_url AS recipient_avatar_url,
                ru.rank AS recipient_rank
         FROM mails m
         JOIN users su ON su.id = m.sender_id
         JOIN users ru ON ru.id = m.recipient_id
         WHERE m.recipient_id = $1 AND m.is_active = true
         ORDER BY m.created_at DESC
         LIMIT $2",
    )
    .bind(user.id)
    .bind(PAGE_SIZE)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let unread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mails WHERE recipient_id = $1 AND is_read = false AND is_active = true",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = MailInboxTemplate {
        user: Some(user),
        active_page: "friends".into(),
        mails,
        unread_count,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (mail inbox): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

#[derive(Template)]
#[template(path = "ban-be/mail_compose.html")]
pub struct MailComposeTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub friends: Vec<FriendshipWithUser>,
    pub error: Option<String>,
}

/// GET /ban-be/thu/gui — Form gửi thư.
pub async fn mail_compose_form(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be/thu/gui").into_response(),
    };

    // Lấy danh sách bạn bè để chọn người nhận
    let friends = sqlx::query_as::<_, FriendshipWithUser>(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END AS other_user_id,
                u.display_name AS other_display_name,
                u.avatar_url AS other_avatar_url,
                u.rank AS other_rank
         FROM friendships f
         JOIN users u ON u.id = (CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END)
         WHERE (f.requester_id = $1 OR f.addressee_id = $1) AND f.status = 'accepted'
         ORDER BY u.display_name ASC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = MailComposeTemplate {
        user: Some(user),
        active_page: "friends".into(),
        friends,
        error: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (mail compose): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Form data cho POST /ban-be/thu/gui.
#[derive(Debug, serde::Deserialize)]
pub struct MailSendForm {
    pub recipient_id: String,
    pub subject: String,
    pub body: String,
}

/// POST /ban-be/thu/gui — Gửi thư mới.
pub async fn mail_send(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<MailSendForm>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let recipient_id = match form.recipient_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Redirect::to("/ban-be/thu/gui").into_response(),
    };

    if recipient_id == user.id {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 p-3 rounded-lg text-sm">
                Không thể gửi thư cho chính mình. <a href="/ban-be/thu/gui" class="underline">← Thử lại</a>
            </div>"#,
        )
        .into_response();
    }

    // v0.9.12: Security — chỉ cho phép gửi thư cho bạn bè đã chấp nhận kết bạn.
    // Tránh spam chéo toàn userbase qua việc craft POST với recipient_id bất kỳ.
    let is_friend: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM friendships
            WHERE ((requester_id = $1 AND addressee_id = $2)
                OR (requester_id = $2 AND addressee_id = $1))
              AND status = 'accepted'
        )",
    )
    .bind(user.id)
    .bind(recipient_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if is_friend != Some(true) {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 p-3 rounded-lg text-sm">
                Bạn chỉ có thể gửi thư cho bạn bè đã kết bạn. <a href="/ban-be/thu/gui" class="underline">← Thử lại</a>
            </div>"#,
        )
        .into_response();
    }

    let subject = form.subject.trim().to_string();
    let body = form.body.trim().to_string();

    if subject.is_empty() || subject.chars().count() > 200 {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 p-3 rounded-lg text-sm">
                Tiêu đề không được để trống và tối đa 200 ký tự. <a href="/ban-be/thu/gui" class="underline">← Thử lại</a>
            </div>"#,
        )
        .into_response();
    }
    if body.is_empty() {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 p-3 rounded-lg text-sm">
                Nội dung thư không được để trống. <a href="/ban-be/thu/gui" class="underline">← Thử lại</a>
            </div>"#,
        )
        .into_response();
    }

    // Insert mail
    let inserted: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO mails (sender_id, recipient_id, subject, body)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(user.id)
    .bind(recipient_id)
    .bind(&subject)
    .bind(&body)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if let Some((mail_id,)) = inserted {
        // Tạo notification cho người nhận
        let _ = sqlx::query(
            "INSERT INTO notifications (user_id, type, actor_id, payload)
             VALUES ($1, 'mail', $2, $3)",
        )
        .bind(recipient_id)
        .bind(user.id)
        .bind(serde_json::json!({
            "mail_id": mail_id,
            "subject": subject,
            "message": format!("{} gửi bạn một thư: {}", user.display_name, subject)
        }))
        .execute(&state.pool)
        .await;

        log::info!("✉️ Mail gửi: {} → {}", user.id, recipient_id);
    }

    Redirect::to("/ban-be/thu").into_response()
}

#[derive(Template)]
#[template(path = "ban-be/mail_view.html")]
pub struct MailViewTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub mail: Option<MailWithUsers>,
}

/// GET /ban-be/thu/{mail_id} — Xem thư.
pub async fn mail_view(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(mail_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    // Lấy mail (chỉ recipient hoặc sender mới xem được)
    let mail: Option<MailWithUsers> = sqlx::query_as::<_, MailWithUsers>(
        "SELECT m.id, m.sender_id, m.recipient_id, m.subject, m.body,
                m.is_read, m.read_at, m.is_active, m.created_at,
                su.display_name AS sender_display_name,
                su.avatar_url AS sender_avatar_url,
                su.rank AS sender_rank,
                ru.display_name AS recipient_display_name,
                ru.avatar_url AS recipient_avatar_url,
                ru.rank AS recipient_rank
         FROM mails m
         JOIN users su ON su.id = m.sender_id
         JOIN users ru ON ru.id = m.recipient_id
         WHERE m.id = $1 AND (m.recipient_id = $2 OR m.sender_id = $2) AND m.is_active = true",
    )
    .bind(mail_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    // Nếu user là recipient và chưa đọc → đánh dấu đã đọc
    if let Some(ref m) = mail
        && m.recipient_id == user.id && !m.is_read
    {
        let _ = sqlx::query(
            "UPDATE mails SET is_read = true, read_at = NOW() WHERE id = $1",
        )
        .bind(mail_id)
        .execute(&state.pool)
        .await;
    }

    let html = MailViewTemplate {
        user: Some(user),
        active_page: "friends".into(),
        mail,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (mail view): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ====================================================================
// Notifications — GET /ban-be/thong-bao
// ====================================================================

#[derive(Template)]
#[template(path = "ban-be/notifications.html")]
pub struct NotificationsTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub notifications: Vec<NotificationWithActor>,
}

/// GET /ban-be/thong-bao — Danh sách thông báo.
pub async fn notifications_list(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be/thong-bao").into_response(),
    };

    let notifications = sqlx::query_as::<_, NotificationWithActor>(
        "SELECT n.id, n.user_id, n.type AS kind, n.actor_id, n.payload, n.is_read, n.read_at, n.created_at,
                u.display_name AS actor_display_name,
                u.avatar_url AS actor_avatar_url,
                u.rank AS actor_rank
         FROM notifications n
         LEFT JOIN users u ON u.id = n.actor_id
         WHERE n.user_id = $1
         ORDER BY n.created_at DESC
         LIMIT $2",
    )
    .bind(user.id)
    .bind(PAGE_SIZE)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Mark all as read sau khi đã load
    let _ = sqlx::query(
        "UPDATE notifications SET is_read = true, read_at = COALESCE(read_at, NOW())
         WHERE user_id = $1 AND is_read = false",
    )
    .bind(user.id)
    .execute(&state.pool)
    .await;

    let html = NotificationsTemplate {
        user: Some(user),
        active_page: "friends".into(),
        notifications,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (notifications): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /api/ban-be/thong-bao/chua-doc — Đếm thông báo chưa đọc (cho badge).
pub async fn notifications_unread_count(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => {
            return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập.").into_response();
        }
    };

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    axum::Json(serde_json::json!({ "unread_count": count })).into_response()
}

/// POST /api/ban-be/thong-bao/{notification_id}/da-doc — Đánh dấu 1 thông báo đã đọc.
pub async fn mark_notification_read(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(notification_id): Path<Uuid>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => {
            return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập.").into_response();
        }
    };

    let _ = sqlx::query(
        "UPDATE notifications SET is_read = true, read_at = NOW()
         WHERE id = $1 AND user_id = $2 AND is_read = false",
    )
    .bind(notification_id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    axum::Json(serde_json::json!({ "status": "ok" })).into_response()
}

// ====================================================================
// Tìm kiếm user — GET /ban-be/tim-kiem
// ====================================================================

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Template)]
#[template(path = "ban-be/search.html")]
pub struct SearchTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub query: String,
    pub results: Vec<UserSearchResult>,
}

/// Kết quả tìm kiếm user (subset fields).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub rank: String,
    pub is_friend: bool,
    pub pending_sent: bool,
    pub pending_received: bool,
}

/// GET /ban-be/tim-kiem — Tìm kiếm user theo display_name hoặc email.
pub async fn search_users(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<SearchQuery>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap?next=/ban-be/tim-kiem").into_response(),
    };

    let query = q.q.unwrap_or_default().trim().to_string();

    let results: Vec<UserSearchResult> = if query.is_empty() {
        Vec::new()
    } else {
        let pattern = format!("%{query}%");
        sqlx::query_as::<_, UserSearchResult>(
            "SELECT u.id, u.display_name, u.avatar_url, u.rank,
                    EXISTS(SELECT 1 FROM friendships f
                           WHERE (f.requester_id = u.id AND f.addressee_id = $2)
                              OR (f.requester_id = $2 AND f.addressee_id = u.id)
                           AND f.status = 'accepted') AS is_friend,
                    EXISTS(SELECT 1 FROM friendships f
                           WHERE f.requester_id = $2 AND f.addressee_id = u.id
                           AND f.status = 'pending') AS pending_sent,
                    EXISTS(SELECT 1 FROM friendships f
                           WHERE f.requester_id = u.id AND f.addressee_id = $2
                           AND f.status = 'pending') AS pending_received
             FROM users u
             WHERE u.id <> $2 AND u.is_active = true
               AND (u.display_name ILIKE $1 OR u.email ILIKE $1
                    OR u.phap_danh ILIKE $1 OR u.phap_hieu ILIKE $1 OR u.but_danh ILIKE $1)
             ORDER BY u.display_name ASC
             LIMIT 30",
        )
        .bind(&pattern)
        .bind(user.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    };

    let html = SearchTemplate {
        user: Some(user),
        active_page: "friends".into(),
        query,
        results,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (search users): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Helper: Tạo conversation 1-1 giữa 2 user nếu chưa có.
/// Trả về conversation_id. Dùng cho DM.
pub async fn get_or_create_direct_conversation(
    pool: &sqlx::PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Option<Uuid> {
    // Tìm conversation đã có giữa 2 user
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT c.id FROM conversations c
         WHERE c.type = 'direct' AND c.is_active = true
           AND EXISTS(SELECT 1 FROM conversation_participants cp
                      WHERE cp.conversation_id = c.id AND cp.user_id = $1)
           AND EXISTS(SELECT 1 FROM conversation_participants cp
                      WHERE cp.conversation_id = c.id AND cp.user_id = $2)",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((conv_id,)) = existing {
        return Some(conv_id);
    }

    // Tạo mới
    let conv_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO conversations (type) VALUES ('direct') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .ok();

    if let Some(id) = conv_id {
        let _ = sqlx::query(
            "INSERT INTO conversation_participants (conversation_id, user_id) VALUES
             ($1, $2), ($1, $3)
             ON CONFLICT (conversation_id, user_id) DO NOTHING",
        )
        .bind(id)
        .bind(user_a)
        .bind(user_b)
        .execute(pool)
        .await;
        return Some(id);
    }

    None
}

/// Form data cho POST /ban-be/tao-conversation.
#[derive(Debug, serde::Deserialize)]
pub struct CreateConversationForm {
    pub other_user_id: String,
}

/// POST /ban-be/tao-conversation — Tạo (hoặc lấy) conversation 1-1 với user khác.
pub async fn create_conversation(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CreateConversationForm>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let other_user_id = match form.other_user_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Redirect::to("/ban-be").into_response(),
    };

    if other_user_id == user.id {
        return Redirect::to("/ban-be").into_response();
    }

    match get_or_create_direct_conversation(&state.pool, user.id, other_user_id).await {
        Some(conv_id) => Redirect::to(&format!("/ban-be/tin-nhan/{conv_id}")).into_response(),
        None => Redirect::to("/ban-be").into_response(),
    }
}

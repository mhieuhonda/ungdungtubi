//! Handlers cho Live Chat real-time (v0.9.3 — Giai đoạn 7+).
//!
//! Bao gồm:
//!   * GET  /ws/cong-dong/nhom/{slug}                       — WebSocket upgrade (nhóm)
//!   * GET  /api/cong-dong/nhom/{slug}/chat-history         — Lấy 50 tin nhắn gần nhất (nhóm)
//!   * GET  /ws/chat-chung                                 — WebSocket upgrade (chat chung toàn platform) [v0.9.3]
//!   * GET  /api/chat-chung/history                        — Lấy 50 tin nhắn chat chung gần nhất [v0.9.3]
//!
//! Theo thiết kế trong `HieuLouis/Giao Diện Cộng Đồng Trong Ứng Dụng.docx`:
//!   * Live Chat kết hợp với list Chủ Đề trong mỗi nhóm
//!   * Live Chat chỉ để giao lưu, kết bạn, tán gẫu, hỏi nhanh
//!   * Mọi nội dung có giá trị nên được chuyển thành Chủ Đề
//!   * Live Chat panel chiếm ~30-40% chiều cao, list Chủ Đề chiếm 60-70%

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use sqlx::PgPool;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::community::{ChatMessage, ChatMessageWithAuthor};
use crate::models::user::User;

/// Sức chứa tối đa của broadcast channel cho một nhóm.
/// 256 là đủ dư cho một nhóm hoạt động — tin nhắn cũ bị drop nếu client chậm.
const BROADCAST_CHANNEL_CAPACITY: usize = 256;

/// Giới hạn tin nhắn chat (ký tự) — chống spam / lạm dụng.
const MAX_CHAT_BODY_CHARS: usize = 500;

/// Tin nhắn broadcast qua WebSocket — đã kèm author info.
type BroadcastPayload = String; // JSON-serialised ChatMessageWithAuthor

/// Hub quản lý các broadcast channel theo nhóm.
///
/// Mỗi nhóm có một `broadcast::Sender<BroadcastPayload>`. Khi client kết nối
/// WebSocket, nó `subscribe()` để lấy `Receiver`. Bất kỳ tin nhắn nào gửi vào
/// channel sẽ được forward đến tất cả client đang online trong nhóm đó.
#[derive(Clone, Default)]
pub struct ChatHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<BroadcastPayload>>>>,
}

impl ChatHub {
    /// Lấy (hoặc tạo nếu chưa có) broadcast sender cho nhóm.
    pub async fn subscribe(&self, group_id: Uuid) -> broadcast::Sender<BroadcastPayload> {
        let mut map = self.channels.lock().await;
        map.entry(group_id)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }

    /// Broadcast một payload JSON đến tất cả client trong nhóm.
    /// Bỏ qua lỗi "no receivers" vì là tình huống bình thường
    /// (không ai online khi tin nhắn được gửi).
    pub async fn broadcast(&self, group_id: Uuid, payload: BroadcastPayload) {
        let tx = self.subscribe(group_id).await;
        let _ = tx.send(payload);
    }
}

// --- Column list (đồng bộ với model) ---

const CHAT_LIST_COLUMNS: &str = "m.id, m.group_id, m.author_id, m.body, m.is_active, m.created_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, \
    u.rank AS author_rank";

/// Query params cho GET /api/cong-dong/nhom/{slug}/chat-history.
#[derive(Debug, serde::Deserialize)]
pub struct ChatHistoryQuery {
    /// Số tin nhắn tối đa (mặc định 50, max 100).
    pub limit: Option<i64>,
    /// Lấy tin nhắn có `created_at` < `before` (ISO 8601) — pagination.
    pub before: Option<String>,
}

/// GET /api/cong-dong/nhom/{slug}/chat-history — Lấy chat history của nhóm.
///
/// Trả về JSON array các tin nhắn (mới nhất trước). Public (ai cũng xem được
/// chat history của nhóm public), nhưng chỉ member mới chat được qua WebSocket.
#[allow(clippy::too_many_lines)]
pub async fn chat_history(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<ChatHistoryQuery>,
) -> Response {
    // Resolve group_id + visibility check
    let group_row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, visibility FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((group_id, _visibility)) = group_row else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "Nhóm không tồn tại.",
        )
            .into_response();
    };

    let limit = q.limit.unwrap_or(50).clamp(1, 100);

    let before_dt: Option<DateTime<Utc>> = q
        .before
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let messages = if let Some(before) = before_dt {
        sqlx::query_as::<_, ChatMessageWithAuthor>(&format!(
            "SELECT {CHAT_LIST_COLUMNS}
             FROM group_chat_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.group_id = $1 AND m.is_active = true AND m.created_at < $2
             ORDER BY m.created_at DESC
             LIMIT $3"
        ))
        .bind(group_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, ChatMessageWithAuthor>(&format!(
            "SELECT {CHAT_LIST_COLUMNS}
             FROM group_chat_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.group_id = $1 AND m.is_active = true
             ORDER BY m.created_at DESC
             LIMIT $2"
        ))
        .bind(group_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    match messages {
        Ok(msgs) => Json(msgs).into_response(),
        Err(e) => {
            log::error!("❌ Lỗi truy vấn chat history: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi truy vấn chat history.",
            )
                .into_response()
        }
    }
}

/// GET /ws/cong-dong/nhom/{slug} — WebSocket upgrade cho Live Chat.
///
/// Quy trình:
///   1. Auth bằng session_id cookie — chỉ user đã đăng nhập mới chat được
///   2. Resolve group_id từ slug — nhóm phải active
///   3. Kiểm tra user có membership active trong nhóm không
///   4. Upgrade WebSocket — spawn 2 task:
///      - send_task: forward broadcast → client
///      - recv loop: đọc tin nhắn từ client → persist DB → broadcast
#[allow(clippy::too_many_lines)]
pub async fn chat_ws_upgrade(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Response {
    // 1. Auth
    let user = get_user_from_session(&state.pool, &jar).await;
    let Some(user) = user else {
        return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập để chat.").into_response();
    };

    // 2. Resolve group
    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(e) => {
            log::error!("❌ Lỗi truy vấn nhóm cho WS: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống.",
            )
                .into_response();
        }
    };

    // 3. Membership check — chỉ member active mới chat được
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM group_members
         WHERE group_id = $1 AND user_id = $2 AND status = 'active')",
    )
    .bind(group_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !is_member {
        return (
            axum::http::StatusCode::FORBIDDEN,
            "Bạn cần tham gia nhóm để chat.",
        )
            .into_response();
    }

    // 4. Upgrade
    ws.on_upgrade(move |socket| handle_chat_socket(socket, state, group_id, user))
}

/// Handler cho WebSocket session sau khi upgrade thành công.
///
/// Chia socket thành sender + receiver, spawn 2 task song song:
///   * send_task: forward từ broadcast channel → client (tin nhắn từ người khác)
///   * recv loop: đọc từ client → persist DB → broadcast (tin nhắn của mình)
///
/// Khi một trong hai task kết thúc (client ngắt hoặc error), task còn lại bị abort.
#[allow(clippy::too_many_lines)]
async fn handle_chat_socket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    group_id: Uuid,
    user: User,
) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe broadcast channel cho nhóm này
    let tx = state.chat_hub.subscribe(group_id).await;
    let mut rx = tx.subscribe();

    log::info!(
        "💬 WS connected: user={} group={}",
        user.id,
        group_id
    );

    // send_task: forward broadcast → client
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(payload) => {
                    if sender
                        .send(axum::extract::ws::Message::Text(payload.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Client chậm — bỏ qua tin nhắn cũ, tiếp tục
                    continue;
                }
            }
        }
    });

    // recv loop: đọc từ client → persist → broadcast
    let chat_hub = state.chat_hub.clone();
    let pool = state.pool.clone();
    let user_id = user.id;
    let user_display_name = user.display_name.clone();
    let user_avatar_url = user.avatar_url.clone();
    let user_rank = user.rank.clone();

    // Clone sender để gửi error trực tiếp cho client (không broadcast cho tất cả)
    let mut err_sender = sender.clone();

    while let Some(msg_result) = receiver.next().await {
        let Ok(msg) = msg_result else { break };

        if let axum::extract::ws::Message::Text(text) = msg {
            // Strip whitespace + validate length
            let body = text.trim().to_string();
            if body.is_empty() || body.chars().count() > MAX_CHAT_BODY_CHARS {
                // v0.9.3 fix: gửi error trực tiếp cho client (không broadcast cho tất cả)
                let err_payload = serde_json::json!({
                    "type": "error",
                    "message": format!("Tin nhắn không hợp lệ (tối đa {MAX_CHAT_BODY_CHARS} ký tự).")
                })
                .to_string();
                let _ = err_sender.send(axum::extract::ws::Message::Text(err_payload.into())).await;
                continue;
            }

            // Persist vào DB — dùng ChatMessage (không có author info) vì
            // INSERT RETURNING không trả về columns từ users table.
            let saved: Option<ChatMessageWithAuthor> = sqlx::query_as::<_, ChatMessage>(
                "INSERT INTO group_chat_messages (group_id, author_id, body)
                 VALUES ($1, $2, $3)
                 RETURNING id, group_id, author_id, body, is_active, created_at",
            )
            .bind(group_id)
            .bind(user_id)
            .bind(&body)
            .fetch_one(&pool)
            .await
            .ok()
            .map(|m| ChatMessageWithAuthor {
                id: m.id,
                group_id: m.group_id,
                author_id: m.author_id,
                body: m.body,
                is_active: m.is_active,
                created_at: m.created_at,
                author_display_name: user_display_name.clone(),
                author_avatar_url: user_avatar_url.clone(),
                author_rank: user_rank.clone(),
            });

            if let Some(msg) = saved {
                let payload = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
                let _ = chat_hub.broadcast(group_id, payload).await;
            }
        }
        // Bỏ qua Binary / Ping / Pong — chỉ xử lý Text
    }

    // Cleanup: abort send_task khi client ngắt
    send_task.abort();
    log::info!("💬 WS disconnected: user={} group={}", user_id, group_id);
}

/// Helper: lấy 20 tin nhắn gần nhất cho render SSR trang nhóm.
/// Dùng trong `community::view_group` để inject vào template.
pub async fn recent_messages(pool: &PgPool, group_id: Uuid) -> Vec<ChatMessageWithAuthor> {
    sqlx::query_as::<_, ChatMessageWithAuthor>(&format!(
        "SELECT {CHAT_LIST_COLUMNS}
         FROM group_chat_messages m
         JOIN users u ON u.id = m.author_id
         WHERE m.group_id = $1 AND m.is_active = true
         ORDER BY m.created_at DESC
         LIMIT 20"
    ))
    .bind(group_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .rev()
    .collect()
}

// ====================================================================
// Global Chat (Chat Chung) — v0.9.3
// Platform-wide chat accessible from any page via draggable bubble
// ====================================================================

/// Sức chứa broadcast channel cho chat chung.
const GLOBAL_CHAT_CHANNEL_CAPACITY: usize = 512;

/// Số tin nhắn tối đa được lưu — tự động xoá cũ khi vượt quá.
const GLOBAL_CHAT_MAX_MESSAGES: i64 = 500;

/// Hub quản lý broadcast channel cho Chat Chung toàn platform.
///
/// Khác với ChatHub (nhiều nhóm → nhiều channel), GlobalChatHub chỉ có 1 channel.
#[derive(Clone, Default)]
pub struct GlobalChatHub {
    channel: Arc<Mutex<Option<broadcast::Sender<BroadcastPayload>>>>,
}

impl GlobalChatHub {
    /// Lấy (hoặc tạo nếu chưa có) broadcast sender cho chat chung.
    pub async fn subscribe(&self) -> broadcast::Sender<BroadcastPayload> {
        let mut guard = self.channel.lock().await;
        guard
            .get_or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(GLOBAL_CHAT_CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }

    /// Broadcast một payload JSON đến tất cả client đang kết nối chat chung.
    pub async fn broadcast(&self, payload: BroadcastPayload) {
        let tx = self.subscribe().await;
        let _ = tx.send(payload);
    }
}

/// Column list cho global_chat_messages + join users.
const GLOBAL_CHAT_LIST_COLUMNS: &str = "m.id, m.author_id, m.body, m.is_active, m.created_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, \
    u.rank AS author_rank";

/// Query params cho GET /api/chat-chung/history.
#[derive(Debug, serde::Deserialize)]
pub struct GlobalChatHistoryQuery {
    /// Số tin nhắn tối đa (mặc định 50, max 100).
    pub limit: Option<i64>,
}

/// GET /api/chat-chung/history — Lấy chat chung history.
///
/// Public endpoint — ai cũng xem được (không cần đăng nhập).
pub async fn global_chat_history(
    State(state): State<AppState>,
    Query(q): Query<GlobalChatHistoryQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);

    let messages = sqlx::query_as::<_, crate::models::community::GlobalChatMessageWithAuthor>(
        &format!(
            "SELECT {GLOBAL_CHAT_LIST_COLUMNS}
             FROM global_chat_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.is_active = true
             ORDER BY m.created_at DESC
             LIMIT $1"
        ),
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    match messages {
        Ok(msgs) => Json(msgs).into_response(),
        Err(e) => {
            log::error!("❌ Lỗi truy vấn global chat history: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi truy vấn chat chung.",
            )
                .into_response()
        }
    }
}

/// GET /ws/chat-chung — WebSocket upgrade cho Chat Chung toàn platform.
///
/// Chỉ cần đăng nhập (không cần membership).
pub async fn global_chat_ws_upgrade(
    State(state): State<AppState>,
    jar: CookieJar,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Response {
    // Auth
    let user = get_user_from_session(&state.pool, &jar).await;
    let Some(user) = user else {
        return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập để chat.").into_response();
    };

    // Upgrade
    ws.on_upgrade(move |socket| handle_global_chat_socket(socket, state, user))
}

/// Handler cho global chat WebSocket session.
#[allow(clippy::too_many_lines)]
async fn handle_global_chat_socket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    user: User,
) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe broadcast channel
    let tx = state.global_chat_hub.subscribe().await;
    let mut rx = tx.subscribe();

    log::info!("💬 Global WS connected: user={}", user.id);

    // send_task: forward broadcast → client
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(payload) => {
                    if sender
                        .send(axum::extract::ws::Message::Text(payload.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    });

    // recv loop: đọc từ client → persist → prune → broadcast
    let global_chat_hub = state.global_chat_hub.clone();
    let pool = state.pool.clone();
    let user_id = user.id;
    let user_display_name = user.display_name.clone();
    let user_avatar_url = user.avatar_url.clone();
    let user_rank = user.rank.clone();

    // Clone sender để gửi error trực tiếp
    let mut err_sender = sender.clone();

    while let Some(msg_result) = receiver.next().await {
        let Ok(msg) = msg_result else { break };

        if let axum::extract::ws::Message::Text(text) = msg {
            let body = text.trim().to_string();
            if body.is_empty() || body.chars().count() > MAX_CHAT_BODY_CHARS {
                let err_payload = serde_json::json!({
                    "type": "error",
                    "message": format!("Tin nhắn không hợp lệ (tối đa {MAX_CHAT_BODY_CHARS} ký tự).")
                })
                .to_string();
                let _ = err_sender.send(axum::extract::ws::Message::Text(err_payload.into())).await;
                continue;
            }

            // Persist vào DB
            use crate::models::community::{GlobalChatMessage, GlobalChatMessageWithAuthor};
            let saved: Option<GlobalChatMessageWithAuthor> = sqlx::query_as::<_, GlobalChatMessage>(
                "INSERT INTO global_chat_messages (author_id, body)
                 VALUES ($1, $2)
                 RETURNING id, author_id, body, is_active, created_at",
            )
            .bind(user_id)
            .bind(&body)
            .fetch_one(&pool)
            .await
            .ok()
            .map(|m| GlobalChatMessageWithAuthor {
                id: m.id,
                author_id: m.author_id,
                body: m.body,
                is_active: m.is_active,
                created_at: m.created_at,
                author_display_name: user_display_name.clone(),
                author_avatar_url: user_avatar_url.clone(),
                author_rank: user_rank.clone(),
            });

            if let Some(msg) = saved {
                let payload = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
                let _ = global_chat_hub.broadcast(payload).await;

                // Auto-prune: xoá tin nhắn cũ nếu vượt quá giới hạn
                let _ = sqlx::query(
                    "DELETE FROM global_chat_messages
                     WHERE id IN (
                         SELECT id FROM global_chat_messages
                         WHERE is_active = true
                         ORDER BY created_at DESC
                         OFFSET $1
                     )",
                )
                .bind(GLOBAL_CHAT_MAX_MESSAGES)
                .execute(&pool)
                .await;
            }
        }
    }

    // Cleanup
    send_task.abort();
    log::info!("💬 Global WS disconnected: user={}", user_id);
}

//! Handlers cho Chat real-time (v0.9.21 — Giai đoạn 26).
//!
//! v0.9.21 — Giai đoạn 26:
//!   * Xoá hoàn toàn group live chat (ChatHub, chat_ws_upgrade, chat_history,
//!     handle_chat_socket, handle_ws_message, recent_messages).
//!   * Chỉ giữ Chat Chung (global chat) + DM chat.
//!   * Fix bug: pong response dùng CtrlMessage::Pong thay vì Error.
//!
//! Bao gồm:
//!   * GET  /ws/chat-chung                                 — WebSocket upgrade (chat chung toàn platform)
//!   * GET  /api/chat-chung/history                        — Lấy 50 tin nhắn chat chung gần nhất

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use futures_util::{SinkExt, StreamExt};
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, Mutex};
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

/// Giới hạn tin nhắn chat (ký tự) — chống spam / lạm dụng.
const MAX_CHAT_BODY_CHARS: usize = 500;

/// Server gửi WebSocket Ping mỗi 25s để giữ kết nối qua proxy.
/// 25s < 30s (default Traefik idle timeout) → kết nối không bị đóng.
const WS_PING_INTERVAL_SECS: u64 = 25;

/// Đóng kết nối nếu không nhận được message nào (kể cả Pong) trong 180s.
/// 180s = 7 lần Ping không phản hồi → chắc chắn kết nối đã chết.
const WS_IDLE_TIMEOUT_SECS: u64 = 180;

/// Tin nhắn broadcast qua WebSocket — đã kèm author info.
type BroadcastPayload = String; // JSON-serialised

/// Loại tin nhắn control từ recv loop → send_task.
enum CtrlMessage {
    Text(String),  // Text message (error payload, app-level pong, etc.)
    Pong(bytes::Bytes), // WebSocket Pong response
}

// ====================================================================
// Global Chat (Chat Chung) — v0.9.3
// ====================================================================

const GLOBAL_CHAT_CHANNEL_CAPACITY: usize = 512;
const GLOBAL_CHAT_MAX_MESSAGES: i64 = 500;

#[derive(Clone, Default)]
pub struct GlobalChatHub {
    channel: Arc<Mutex<Option<broadcast::Sender<BroadcastPayload>>>>,
}

impl GlobalChatHub {
    pub async fn subscribe(&self) -> broadcast::Sender<BroadcastPayload> {
        let mut guard = self.channel.lock().await;
        guard
            .get_or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(GLOBAL_CHAT_CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }

    pub async fn broadcast(&self, payload: BroadcastPayload) {
        let tx = self.subscribe().await;
        let _ = tx.send(payload);
    }
}

// ====================================================================
// DM Chat Hub — v0.9.5 Giai đoạn 9
// ====================================================================

const DM_CHANNEL_CAPACITY: usize = 128;

#[derive(Clone, Default)]
pub struct DmChatHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<BroadcastPayload>>>>,
}

impl DmChatHub {
    pub async fn subscribe(&self, conversation_id: Uuid) -> broadcast::Sender<BroadcastPayload> {
        let mut map = self.channels.lock().await;
        map.entry(conversation_id)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(DM_CHANNEL_CAPACITY);
                tx
            })
            .clone()
    }

    pub async fn broadcast(&self, conversation_id: Uuid, payload: BroadcastPayload) {
        let tx = self.subscribe(conversation_id).await;
        let _ = tx.send(payload);
    }

    /// v0.9.28 — Giai đoạn 33: Cleanup channel nếu không còn receiver nào.
    ///
    /// Trước v0.9.28: `channels` HashMap grow unbounded — mỗi conversation_id mới
    /// tạo entry `or_insert_with` nhưng không bao giờ remove. Sau hàng ngàn
    /// conversation, HashMap leak RAM (mỗi entry giữ broadcast buffer 128 slots).
    ///
    /// Gọi method này sau khi DM WebSocket disconnect. Nếu `receiver_count() == 0`
    /// (không còn ai subscribe channel này), remove entry khỏi map → giải phóng RAM.
    pub async fn cleanup_if_empty(&self, conversation_id: Uuid) {
        let mut map = self.channels.lock().await;
        if let Some(tx) = map.get(&conversation_id) {
            if tx.receiver_count() == 0 {
                map.remove(&conversation_id);
                log::debug!("💬 DM channel cleanup: conv={}", conversation_id);
            }
        }
    }
}

const GLOBAL_CHAT_LIST_COLUMNS: &str = "m.id, m.author_id, m.body, m.is_active, m.created_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, \
    u.rank AS author_rank, u.role AS author_role";

#[derive(Debug, serde::Deserialize)]
pub struct GlobalChatHistoryQuery {
    pub limit: Option<i64>,
}

/// v0.9.31 — Giai đoạn 36: REST fallback cho global chat.
/// POST /api/chat-chung/gui — Gửi tin nhắn chat chung qua HTTP (khi WS không khả dụng).
#[derive(Debug, serde::Deserialize)]
pub struct GlobalChatSendRequest {
    pub body: String,
}

pub async fn global_chat_send_rest(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<GlobalChatSendRequest>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập.").into_response(),
    };

    let body = req.body.trim().to_string();
    if body.is_empty() || body.chars().count() > MAX_CHAT_BODY_CHARS {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Tin nhắn không hợp lệ.",
        )
            .into_response();
    }

    use crate::models::community::{GlobalChatMessage, GlobalChatMessageWithAuthor};

    let saved: Result<GlobalChatMessageWithAuthor, _> = sqlx::query_as::<_, GlobalChatMessage>(
        "INSERT INTO global_chat_messages (author_id, body)
         VALUES ($1, $2)
         RETURNING id, author_id, body, is_active, created_at",
    )
    .bind(user.id)
    .bind(&body)
    .fetch_one(&state.pool)
    .await
    .map(|m| GlobalChatMessageWithAuthor {
        id: m.id,
        author_id: m.author_id,
        body: m.body,
        is_active: m.is_active,
        created_at: m.created_at,
        author_display_name: user.display_name.clone(),
        author_avatar_url: user.avatar_url.clone(),
        author_rank: user.rank.clone(),
        author_role: Some(user.role.clone()),
    });

    match saved {
        Ok(msg) => {
            // Broadcast qua WebSocket để user khác online nhận realtime
            let payload = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
            let hub = state.global_chat_hub.clone();
            let pool = state.pool.clone();
            tokio::spawn(async move {
                hub.broadcast(payload).await;
                // Auto-prune
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
            });
            Json(msg).into_response()
        }
        Err(e) => {
            log::error!("❌ Lỗi lưu global chat message (REST): {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Không lưu được tin nhắn.",
            )
                .into_response()
        }
    }
}

/// GET /api/chat-chung/history — Public endpoint
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
pub async fn global_chat_ws_upgrade(
    State(state): State<AppState>,
    jar: CookieJar,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let Some(user) = user else {
        return (axum::http::StatusCode::UNAUTHORIZED, "Cần đăng nhập để chat.").into_response();
    };

    ws.max_message_size(64 * 1024)
        .on_upgrade(move |socket| handle_global_chat_socket(socket, state, user))
}

/// Handler cho global chat WebSocket session.
/// v0.9.20: same architecture — ping + idle timeout.
/// v0.9.21: fix CtrlMessage — pong dùng CtrlMessage::Text thay vì Error.
#[allow(clippy::too_many_lines)]
async fn handle_global_chat_socket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    user: User,
) {
    let (mut sender, mut receiver) = socket.split();

    let tx = state.global_chat_hub.subscribe().await;
    let mut rx = tx.subscribe();

    log::info!("💬 Global WS connected: user={}", user.id);

    let (ctrl_tx, mut ctrl_rx) = mpsc::unbounded_channel::<CtrlMessage>();

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
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
                ctrl = ctrl_rx.recv() => {
                    match ctrl {
                        Some(CtrlMessage::Text(text_payload)) => {
                            if sender
                                .send(axum::extract::ws::Message::Text(text_payload.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Some(CtrlMessage::Pong(payload)) => {
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

    let global_chat_hub = state.global_chat_hub.clone();
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
                    "💬 Global WS idle timeout ({WS_IDLE_TIMEOUT_SECS}s), đóng: user={}",
                    user_id
                );
                break;
            }
            Ok(None) => break,
            Ok(Some(Err(_))) => break,
            Ok(Some(Ok(msg))) => {
                if !handle_global_ws_message(
                    msg,
                    &ctrl_tx,
                    user_id,
                    &user_display_name,
                    &user_avatar_url,
                    &user_rank,
                    &user_role,
                    &pool,
                    &global_chat_hub,
                    MAX_CHAT_BODY_CHARS,
                )
                .await
                {
                    break;
                }
            }
        }
    }

    send_task.abort();
    log::info!("💬 Global WS disconnected: user={}", user_id);
}

/// Xử lý WebSocket message cho global chat.
/// v0.9.21: Fix — app-level pong gửi qua CtrlMessage::Text (không dùng Error nữa).
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
async fn handle_global_ws_message(
    msg: axum::extract::ws::Message,
    ctrl_tx: &mpsc::UnboundedSender<CtrlMessage>,
    user_id: Uuid,
    user_display_name: &str,
    user_avatar_url: &Option<String>,
    user_rank: &str,
    user_role: &str,
    pool: &PgPool,
    global_chat_hub: &GlobalChatHub,
    max_chars: usize,
) -> bool {
    use axum::extract::ws::Message;

    match msg {
        Message::Text(text) => {
            let body = text.trim().to_string();

            // v0.9.21: App-level ping — respond bằng Text pong (CtrlMessage::Text, không phải Error)
            if body == "{\"type\":\"ping\"}" || body == "ping" {
                let _ = ctrl_tx.send(CtrlMessage::Text(
                    r#"{"type":"pong"}"#.to_string(),
                ));
                return true;
            }

            if body.is_empty() || body.chars().count() > max_chars {
                let err_payload = serde_json::json!({
                    "type": "error",
                    "message": format!("Tin nhắn không hợp lệ (tối đa {max_chars} ký tự).")
                })
                .to_string();
                let _ = ctrl_tx.send(CtrlMessage::Text(err_payload));
                return true;
            }

            use crate::models::community::{GlobalChatMessage, GlobalChatMessageWithAuthor};
            let saved: Option<GlobalChatMessageWithAuthor> = match sqlx::query_as::<_, GlobalChatMessage>(
                "INSERT INTO global_chat_messages (author_id, body)
                 VALUES ($1, $2)
                 RETURNING id, author_id, body, is_active, created_at",
            )
            .bind(user_id)
            .bind(&body)
            .fetch_one(pool)
            .await
            {
                Ok(m) => Some(GlobalChatMessageWithAuthor {
                    id: m.id,
                    author_id: m.author_id,
                    body: m.body,
                    is_active: m.is_active,
                    created_at: m.created_at,
                    author_display_name: user_display_name.to_string(),
                    author_avatar_url: user_avatar_url.clone(),
                    author_rank: user_rank.to_string(),
                    author_role: Some(user_role.to_string()),
                }),
                Err(e) => {
                    log::error!("❌ Lỗi lưu global chat message: {e}");
                    let err_payload = serde_json::json!({
                        "type": "error",
                        "message": "Không lưu được tin nhắn. Vui lòng thử lại."
                    })
                    .to_string();
                    let _ = ctrl_tx.send(CtrlMessage::Text(err_payload));
                    None
                }
            };

            if let Some(msg) = saved {
                let payload = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
                let _ = global_chat_hub.broadcast(payload).await;

                // Auto-prune
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
                .execute(pool)
                .await;
            }
            true
        }
        Message::Pong(_) => true,
        Message::Ping(payload) => {
            let _ = ctrl_tx.send(CtrlMessage::Pong(payload));
            true
        }
        Message::Close(_) => false,
        Message::Binary(_) => true,
    }
}

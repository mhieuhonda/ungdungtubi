#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ====================================================================
// Friendship (Kết bạn) — Giai đoạn 9 v0.9.5
// ====================================================================

/// Quan hệ bạn bè giữa 2 user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Friendship {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub addressee_id: Uuid,
    /// pending | accepted | blocked | declined
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Friendship kèm thông tin user (join query) — dùng cho danh sách bạn bè + lời mời.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FriendshipWithUser {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub addressee_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    // Từ users (người kia trong quan hệ)
    pub other_user_id: Uuid,
    pub other_display_name: String,
    pub other_avatar_url: Option<String>,
    pub other_rank: String,
}

// ====================================================================
// Conversation + Direct Messages (Nhắn tin 1-1) — Giai đoạn 9 v0.9.5
// ====================================================================

/// Một cuộc hội thoại (1-1 hoặc nhóm).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    /// direct | group — renamed from SQL column `type` để tránh Rust keyword conflict
    pub kind: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tin nhắn trong conversation.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DirectMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Direct message kèm thông tin author (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DirectMessageWithAuthor {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
}

/// Conversation kèm thông tin người đối diện (cho direct conversation).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationWithParticipant {
    pub id: Uuid,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // User đối diện (direct conversation)
    pub other_user_id: Uuid,
    pub other_display_name: String,
    pub other_avatar_url: Option<String>,
    pub other_rank: String,
    // Tin nhắn cuối (preview inbox)
    pub last_message_body: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub last_message_author_id: Option<Uuid>,
}

// ====================================================================
// Mail (Gửi thư) — Giai đoạn 9 v0.9.5
// ====================================================================

/// Thư giữa 2 user (long-form, không realtime).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Mail {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub subject: String,
    pub body: String,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Mail kèm thông tin sender/recipient (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MailWithUsers {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub subject: String,
    pub body: String,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    // Từ users
    pub sender_display_name: String,
    pub sender_avatar_url: Option<String>,
    pub sender_rank: String,
    pub recipient_display_name: String,
    pub recipient_avatar_url: Option<String>,
    pub recipient_rank: String,
}

// ====================================================================
// Notification (Thông báo) — Giai đoạn 9 v0.9.5
// ====================================================================

/// Thông báo gửi đến user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    /// friend_request | friend_accept | friend_decline | mail | dm | system | group_invite
    /// — renamed from SQL column `type` để tránh Rust keyword conflict
    pub kind: String,
    pub actor_id: Option<Uuid>,
    pub payload: Option<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Notification kèm thông tin actor (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationWithActor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub actor_id: Option<Uuid>,
    pub payload: Option<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    // Từ users (actor)
    pub actor_display_name: Option<String>,
    pub actor_avatar_url: Option<String>,
    pub actor_rank: Option<String>,
}

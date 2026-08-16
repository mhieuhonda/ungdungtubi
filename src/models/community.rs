#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Một nhóm cộng đồng.
///
/// Mỗi nhóm chứa nhiều chủ đề (topics) và có nhiều thành viên (`group_members`).
/// Vai trò của thành viên được phân biệt qua bảng `group_members`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub owner_id: Uuid,
    pub cover_upload_id: Option<Uuid>,
    /// v0.9.36 — Giai đoạn 41: Logo riêng (icon vuông nhỏ) của nhóm, khác với cover_upload_id (banner).
    #[sqlx(default)]
    pub logo_upload_id: Option<Uuid>,
    /// public | private | hidden
    pub visibility: String,
    pub require_approval: bool,
    pub member_count: i32,
    pub topic_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Thông tin nhóm kèm category (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupWithCategory {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub owner_id: Uuid,
    pub visibility: String,
    pub require_approval: bool,
    pub member_count: i32,
    pub topic_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Một chủ đề (bài viết) trong nhóm.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Topic {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub comment_count: i32,
    pub view_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Topic kèm thông tin author (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TopicWithAuthor {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub body: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub comment_count: i32,
    pub view_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
}

/// Một bình luận trên chủ đề.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comment kèm thông tin author.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommentWithAuthor {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
}

/// Một thành viên nhóm.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub id: i64,
    pub group_id: Uuid,
    pub user_id: Uuid,
    /// owner | admin | moderator | member
    pub role: String,
    /// active | pending | banned
    pub status: String,
    pub joined_at: DateTime<Utc>,
}

/// Thành viên nhóm kèm thông tin user — dùng cho danh sách thành viên.
/// v0.9.23: Giai đoạn 28 — chủ nhóm quản lý thành viên.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMemberWithUser {
    pub id: i64,
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub status: String,
    pub joined_at: DateTime<Utc>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub rank: String,
}

impl GroupMemberWithUser {
    /// Role hiển thị tiếng Việt.
    pub fn role_display(&self) -> &str {
        match self.role.as_str() {
            "owner" => "Chủ nhóm",
            "admin" => "Quản trị",
            "moderator" => "Điều hành",
            _ => "Thành viên",
        }
    }

    /// Icon cho role.
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "owner" => "👑",
            "admin" => "🛡️",
            "moderator" => "📜",
            _ => "🪷",
        }
    }

    /// Ký tự đầu tiên của tên.
    pub fn initial(&self) -> char {
        self.display_name.chars().next().unwrap_or('🪷')
    }
}

/// Phân loại nhóm.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupCategory {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub icon: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Form tạo nhóm mới.
#[derive(Debug, Deserialize)]
pub struct GroupCreateForm {
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub visibility: String,
    pub require_approval: Option<String>,
}

/// Form tạo chủ đề mới.
#[derive(Debug, Deserialize)]
pub struct TopicCreateForm {
    pub title: String,
    pub body: String,
}

/// Form bình luận mới.
#[derive(Debug, Deserialize)]
pub struct CommentCreateForm {
    pub body: String,
    pub parent_id: Option<String>,
}

/// Một tin nhắn Live Chat trong nhóm (v0.9.2 — Giai đoạn 7).
///
/// Phân biệt với `Comment`: bình luận gắn trên Chủ Đề (lưu trữ tri thức),
/// còn `ChatMessage` là chat real-time trong Nhóm (giao lưu, kết nối).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// ChatMessage kèm thông tin author (join query) — dùng cho render + broadcast.
///
/// v0.9.19 (Giai đoạn 24): thêm `author_role` để frontend render hiệu ứng đặc biệt
/// cho tin nhắn của admin/mod (coder effect cho admin_ky_thuat, khung riêng cho các admin khác).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessageWithAuthor {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
    /// v0.9.19: vai trò author (member | mod | admin_ky_thuat | admin_cong_dong | admin_quan_li).
    /// Dùng cho frontend render hiệu ứng tin nhắn đặc biệt cho admin/mod.
    #[sqlx(default)]
    pub author_role: Option<String>,
}

impl ChatMessageWithAuthor {
    /// Hiển thị thời gian tương đối ("5 phút trước").
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    /// Chữ cái đầu tên author (dùng làm avatar fallback).
    pub fn author_initial(&self) -> char {
        self.author_display_name.chars().next().unwrap_or('🪷')
    }
}

impl Group {
    /// Hiển thị visibility tiếng Việt.
    pub fn visibility_display(&self) -> &str {
        match self.visibility.as_str() {
            "private" => "Riêng tư",
            "hidden" => "Ẩn",
            _ => "Công khai",
        }
    }

    /// Icon cho visibility.
    pub fn visibility_icon(&self) -> &str {
        match self.visibility.as_str() {
            "private" => "🔒",
            "hidden" => "🚫",
            _ => "🌍",
        }
    }
}

impl GroupWithCategory {
    pub fn visibility_display(&self) -> &str {
        match self.visibility.as_str() {
            "private" => "Riêng tư",
            "hidden" => "Ẩn",
            _ => "Công khai",
        }
    }

    pub fn visibility_icon(&self) -> String {
        match self.visibility.as_str() {
            "private" => "🔒".into(),
            "hidden" => "🚫".into(),
            _ => "🌍".into(),
        }
    }

    /// Hiển thị category icon (fallback 🪷).
    pub fn category_icon_or_lotus(&self) -> String {
        self.category_icon.clone().unwrap_or_else(|| "🪷".into())
    }

    /// Hiển thị category name (fallback "Khác").
    pub fn category_name_or_other(&self) -> String {
        self.category_name.clone().unwrap_or_else(|| "Khác".into())
    }
}

impl TopicWithAuthor {
    /// Hiển thị thời gian tương đối ("5 phút trước").
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    /// Trích đoạn body ngắn (max 180 ký tự).
    pub fn body_excerpt(&self) -> String {
        let s = self.body.chars().take(180).collect::<String>();
        if self.body.chars().count() > 180 {
            format!("{s}…")
        } else {
            s
        }
    }

    /// Chữ cái đầu tên author.
    pub fn author_initial(&self) -> char {
        self.author_display_name.chars().next().unwrap_or('🪷')
    }
}

impl CommentWithAuthor {
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    pub fn author_initial(&self) -> char {
        self.author_display_name.chars().next().unwrap_or('🪷')
    }
}

/// Một tin nhắn Chat Chung toàn nền tảng (v0.9.3).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GlobalChatMessage {
    pub id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// GlobalChatMessage kèm thông tin author — dùng cho render + broadcast.
///
/// v0.9.19: thêm `author_role` để render hiệu ứng đặc biệt cho admin/mod messages.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GlobalChatMessageWithAuthor {
    pub id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
    /// v0.9.19: vai trò author — dùng cho frontend render hiệu ứng đặc biệt.
    #[sqlx(default)]
    pub author_role: Option<String>,
}

impl GroupMember {
    /// Hiển thị vai trò tiếng Việt.
    pub fn role_display(&self) -> &str {
        match self.role.as_str() {
            "owner" => "Trưởng Nhóm",
            "admin" => "Quản Trị",
            "moderator" => "Điều Hành",
            _ => "Thành Viên",
        }
    }

    /// Icon cho vai trò.
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "owner" => "👑",
            "admin" => "🛡️",
            "moderator" => "📜",
            _ => "🪷",
        }
    }

    /// Kiểm tra có quyền quản lý nhóm (owner/admin/moderator).
    pub fn is_staff(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "admin" | "moderator")
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Một thành viên của Ứng Dụng Từ Bi.
///
/// Từ v0.3, web chỉ còn đăng nhập/đăng ký bằng Google OAuth,
/// nên `password_hash` chỉ còn áp dụng cho các tài khoản cũ
/// đăng ký bằng email/password trước đây.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    /// Argon2 hash — NULL với tài khoản Google-only.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub rank: String,
    pub a_balance: i64,
    pub k_balance: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Google unique user ID (sub claim).
    pub google_sub: Option<String>,
    /// URL ảnh đại diện từ Google.
    pub avatar_url: Option<String>,
    /// Email đã được Google xác thực.
    pub email_verified: bool,
}

/// Dữ liệu lấy được từ Google userinfo endpoint.
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    pub picture: Option<String>,
}

impl User {
    /// Tên hiển thị cấp bậc theo khoá `rank`.
    pub fn rank_display(&self) -> &str {
        match self.rank.as_str() {
            "new" => "Người Mới",
            "normal" => "Người Thường",
            "common" => "Người Bình Thường",
            "good" => "Người Tốt",
            "very_good" => "Người Khá Tốt",
            "great" => "Người Rất Tốt",
            "excellent" => "Người Cực Kỳ Tốt",
            "benevolent" => "Thiện Nhân",
            "tycoon" => "Đại Gia",
            _ => "Người Mới",
        }
    }

    /// Kiểm tra user có đăng nhập qua Google hay không.
    pub fn is_google_user(&self) -> bool {
        self.google_sub.is_some()
    }
}

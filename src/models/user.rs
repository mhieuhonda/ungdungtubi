#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Một thành viên của Ứng Dụng Từ Bi.
///
/// Từ v0.3, web chỉ còn đăng nhập/đăng ký bằng Google OAuth,
/// nên `password_hash` chỉ còn áp dụng cho các tài khoản cũ
/// đăng ký bằng email/password trước đây.
///
/// Từ v0.4, thêm các trường hồ sơ: phap_danh, phap_hieu, but_danh, gender, bio.
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
    /// Pháp danh — tên Phật giáo khi quy y (tùy chọn).
    pub phap_danh: Option<String>,
    /// Pháp hiệu — tên đạo giáo khi truyền pháp (tùy chọn).
    pub phap_hieu: Option<String>,
    /// Bút danh — tên bút khi viết bài (tùy chọn).
    pub but_danh: Option<String>,
    /// Giới tính: male | female | other.
    pub gender: String,
    /// Tiểu sử / lời giới thiệu ngắn.
    pub bio: Option<String>,
    /// ID ảnh avatar user tự upload (ưu tiên trước Google avatar_url).
    pub avatar_upload_id: Option<Uuid>,
}

/// Dữ liệu lấy được từ Google userinfo endpoint.
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    pub picture: Option<String>,
}

/// Một cấp bậc thành viên.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemberRank {
    pub code: String,
    pub name: String,
    pub description: String,
    pub min_k_balance: i64,
    pub color: String,
    pub icon: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Dữ liệu cập nhật hồ sơ (từ form /ca-nhan/cap-nhat).
#[derive(Debug, Deserialize)]
pub struct ProfileUpdate {
    pub display_name: String,
    pub phap_danh: Option<String>,
    pub phap_hieu: Option<String>,
    pub but_danh: Option<String>,
    pub gender: String,
    pub bio: Option<String>,
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

    /// Emoji đại diện cho cấp bậc.
    pub fn rank_icon(&self) -> &str {
        match self.rank.as_str() {
            "new" => "🌱",
            "normal" => "🍃",
            "common" => "🌿",
            "good" => "🌳",
            "very_good" => "🌲",
            "great" => "🎋",
            "excellent" => "🏆",
            "benevolent" => "🪷",
            "tycoon" => "👑",
            _ => "🌱",
        }
    }

    /// Màu sắc đại diện cho cấp bậc (hex).
    pub fn rank_color(&self) -> &str {
        match self.rank.as_str() {
            "new" => "#9E9E9E",
            "normal" => "#795548",
            "common" => "#558B2F",
            "good" => "#388E3C",
            "very_good" => "#2E7D32",
            "great" => "#1B5E20",
            "excellent" => "#00695C",
            "benevolent" => "#FFB300",
            "tycoon" => "#FF6F00",
            _ => "#9E9E9E",
        }
    }

    /// Kiểm tra user có đăng nhập qua Google hay không.
    pub fn is_google_user(&self) -> bool {
        self.google_sub.is_some()
    }

    /// Tên hiển thị ưu tiên theo thứ tự: pháp danh > pháp hiệu > bút danh > display_name.
    pub fn display_label(&self) -> &str {
        self.phap_danh
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.phap_hieu.as_deref().filter(|s| !s.trim().is_empty()))
            .or_else(|| self.but_danh.as_deref().filter(|s| !s.trim().is_empty()))
            .unwrap_or(&self.display_name)
    }

    /// Giới tính hiển thị tiếng Việt.
    pub fn gender_display(&self) -> &str {
        match self.gender.as_str() {
            "male" => "Nam",
            "female" => "Nữ",
            _ => "Khác",
        }
    }
}

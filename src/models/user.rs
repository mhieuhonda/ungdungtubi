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
/// Từ v0.4, thêm các trường hồ sơ: `phap_danh`, `phap_hieu`, `but_danh`, gender, bio.
///
/// Từ v0.9.7 (Giai đoạn 11), thêm trường `role` cho hệ thống phân quyền:
///   - `member`          — Thành Viên (mặc định)
///   - `admin_ky_thuat`  — Admin Kỹ Thuật
///   - `admin_cong_dong` — Admin Cộng Đồng
///   - `admin_quan_li`   — Admin Quản Lý (quyền cao nhất)
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
    /// ID ảnh avatar user tự upload (ưu tiên trước Google `avatar_url`).
    pub avatar_upload_id: Option<Uuid>,
    /// Vai trò quản trị: member | admin_ky_thuat | admin_cong_dong | admin_quan_li
    /// (v0.9.7 — Giai đoạn 11)
    pub role: String,
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
    pub const fn is_google_user(&self) -> bool {
        self.google_sub.is_some()
    }

    /// Tên hiển thị ưu tiên theo thứ tự: pháp danh > pháp hiệu > bút danh > `display_name`.
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

    // ─── Hệ thống vai trò (v0.9.7 — Giai đoạn 11) ───────────────────────────

    /// Tên hiển thị tiếng Việt của vai trò.
    /// Dùng cho badge trên profile / header.
    pub fn role_display(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "Admin Quản Lý",
            "admin_cong_dong" => "Admin Cộng Đồng",
            "admin_ky_thuat" => "Admin Kỹ Thuật",
            _ => "Thành Viên",
        }
    }

    /// Emoji đại diện cho vai trò.
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "👑",
            "admin_cong_dong" => "🛡️",
            "admin_ky_thuat" => "⚙️",
            _ => "🪷",
        }
    }

    /// Màu sắc đại diện cho vai trò (hex).
    pub fn role_color(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "#FF6F00",   // amber-900 (gold)
            "admin_cong_dong" => "#1565C0",  // blue-800
            "admin_ky_thuat" => "#6A1B9A",   // purple-800
            _ => "#2E7D32",                   // tubi-800 (green)
        }
    }

    /// Cấp độ vai trò (dùng để so sánh quyền):
    ///   - member          → 1
    ///   - admin_ky_thuat  → 2
    ///   - admin_cong_dong → 3
    ///   - admin_quan_li   → 4
    pub const fn role_level(&self) -> u8 {
        match self.role.as_str() {
            "admin_ky_thuat" => 2,
            "admin_cong_dong" => 3,
            "admin_quan_li" => 4,
            _ => 1,
        }
    }

    /// True nếu user là bất kỳ vai trò admin nào (kỹ thuật / cộng đồng / quản lý).
    pub const fn is_admin(&self) -> bool {
        self.role_level() >= 2
    }

    /// True nếu user chính xác là Admin Kỹ Thuật.
    pub const fn is_admin_ky_thuat(&self) -> bool {
        matches!(self.role.as_str(), "admin_ky_thuat")
    }

    /// True nếu user chính xác là Admin Cộng Đồng.
    pub const fn is_admin_cong_dong(&self) -> bool {
        matches!(self.role.as_str(), "admin_cong_dong")
    }

    /// True nếu user chính xác là Admin Quản Lý (super admin — quyền cao nhất).
    pub const fn is_admin_quan_li(&self) -> bool {
        matches!(self.role.as_str(), "admin_quan_li")
    }

    /// True nếu user có quyền kỹ thuật (Admin Kỹ Thuật trở lên).
    /// Dùng cho route /admin, quản lý users, hệ thống.
    pub const fn can_manage_technical(&self) -> bool {
        self.role_level() >= 2
    }

    /// True nếu user có quyền cộng đồng (Admin Cộng Đồng trở lên).
    /// Dùng cho duyệt cảm ngộ, ghim/khoá chủ đề, mod comment.
    pub const fn can_manage_community(&self) -> bool {
        self.role_level() >= 3
    }
}

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
///   - `mod`             — Mod (v0.9.19 — Giai đoạn 24: dưới admin, trên member)
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
    /// Nguyên lực I — phần thưởng từ Tượng Phật (v0.9.9 — Giai đoạn 13).
    pub i_balance: i64,
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
    /// v0.9.19: thêm "Mod" — chức vụ dưới admin, trên thành viên.
    pub fn role_display(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "Admin Quản Lý",
            "admin_cong_dong" => "Admin Cộng Đồng",
            "admin_ky_thuat" => "Admin Kỹ Thuật",
            "mod" => "Mod",
            _ => "Thành Viên",
        }
    }

    /// Emoji đại diện cho vai trò.
    /// v0.9.19: thêm 📜 cho Mod.
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "👑",
            "admin_cong_dong" => "🛡️",
            "admin_ky_thuat" => "⚙️",
            "mod" => "📜",
            _ => "🪷",
        }
    }

    /// Màu sắc đại diện cho vai trò (hex).
    /// v0.9.19: thêm màu cho Mod — teal-700 (#0F766E).
    pub fn role_color(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "#FF6F00",   // amber-900 (gold)
            "admin_cong_dong" => "#1565C0",  // blue-800
            "admin_ky_thuat" => "#6A1B9A",   // purple-800
            "mod" => "#0F766E",              // teal-700 (moderator)
            _ => "#2E7D32",                   // tubi-800 (green)
        }
    }

    /// Cấp độ vai trò (dùng để so sánh quyền):
    ///   - member          → 1
    ///   - mod             → 2  (v0.9.19 — Giai đoạn 24)
    ///   - admin_cong_dong → 3
    ///   - admin_quan_li   → 4
    ///   - admin_ky_thuat  → 5 (CAO NHẤT — v0.9.8)
    ///
    /// v0.9.8: Nâng Admin Kỹ Thuật lên chức vụ cao nhất với toàn bộ 50 quyền.
    /// v0.9.19: Thêm Mod (level 2) — dưới admin, trên thành viên, có quyền quản trị cơ bản.
    /// Hierarchy mới: admin_ky_thuat > admin_quan_li > admin_cong_dong > mod > member
    ///
    /// (Không thể là `const fn` vì Rust 1.97 chưa ổn định `PartialEq` cho `&str`
    /// trong const context — xem issue rust-lang/rust#143874.)
    pub fn role_level(&self) -> u8 {
        match self.role.as_str() {
            "mod" => 2,
            "admin_cong_dong" => 3,
            "admin_quan_li" => 4,
            "admin_ky_thuat" => 5,  // CAO NHẤT — v0.9.8
            _ => 1,
        }
    }

    /// True nếu user là bất kỳ vai trò admin nào (kỹ thuật / cộng đồng / quản lý).
    /// v0.9.19: Mod KHÔNG phải là admin — Mod là chức vụ riêng (dưới admin, trên member).
    /// Dùng `is_staff()` để kiểm tra "admin HOẶC mod".
    pub fn is_admin(&self) -> bool {
        matches!(
            self.role.as_str(),
            "admin_ky_thuat" | "admin_quan_li" | "admin_cong_dong"
        )
    }

    /// True nếu user là Mod (v0.9.19 — Giai đoạn 24).
    /// Mod có quyền quản trị cơ bản: duyệt cảm ngộ, mod bình luận, chat trong mọi nhóm.
    /// Nhưng không được đổi role user, không được ban user.
    pub fn is_mod(&self) -> bool {
        matches!(self.role.as_str(), "mod")
    }

    /// True nếu user là staff (admin hoặc mod) — v0.9.19.
    /// Dùng cho các route quản trị cơ bản (xem danh sách, mod content, chat mọi nhóm).
    pub fn is_staff(&self) -> bool {
        self.is_admin() || self.is_mod()
    }

    /// True nếu user chính xác là Admin Kỹ Thuật.
    pub fn is_admin_ky_thuat(&self) -> bool {
        matches!(self.role.as_str(), "admin_ky_thuat")
    }

    /// True nếu user chính xác là Admin Cộng Đồng.
    pub fn is_admin_cong_dong(&self) -> bool {
        matches!(self.role.as_str(), "admin_cong_dong")
    }

    /// True nếu user chính xác là Admin Quản Lý (super admin — quyền cao nhất).
    pub fn is_admin_quan_li(&self) -> bool {
        matches!(self.role.as_str(), "admin_quan_li")
    }

    /// True nếu user có quyền kỹ thuật (Admin Kỹ Thuật trở lên).
    /// Dùng cho route /admin, quản lý users, hệ thống.
    /// v0.9.19: Mod (level 2) không có quyền technical.
    pub fn can_manage_technical(&self) -> bool {
        self.role_level() >= 3 // admin_cong_dong (3) trở lên — Mod (2) không có
    }

    /// True nếu user có quyền cộng đồng (Mod trở lên).
    /// Dùng cho duyệt cảm ngộ, ghim/khoá chủ đề, mod comment.
    /// v0.9.19: Mod (level 2) CÓ quyền community — đây là quyền cốt lõi của Mod.
    pub fn can_manage_community(&self) -> bool {
        self.role_level() >= 2 // Mod (2) trở lên
    }

    /// True nếu user có quyền quản trị (Admin Quản Lý trở lên).
    /// Dùng cho đổi role, quản lý users, cấu hình hệ thống.
    /// v0.9.19: Mod (level 2) không có quyền admin.
    pub fn can_manage_admin(&self) -> bool {
        self.role_level() >= 4 // admin_quan_li (4) trở lên
    }

    /// True nếu user được chat trong BẤT KỲ nhóm nào (không cần membership).
    /// v0.9.19: Admin + Mod có quyền này — fix bug admin không chat được trong nhóm
    /// mà họ chưa tham gia.
    pub fn can_chat_any_group(&self) -> bool {
        self.is_staff()
    }

    // ─── Hệ thống 150 quyền chi tiết (v0.9.14 — Giai đoạn 19) ─────────────
    //
    // Mở rộng từ 50 → 150 quyền, chia 15 nhóm × 10 quyền:
    //   system(10) + users(10) + content(10) + community(10) + kinh_sach(10)  → 50 (cũ)
    //   fund(10) + achievements(10) + security(10) + navigation(10) + analytics(10)
    //   + media(10) + friends(10) + mail(10) + events(10) + shop(10)         → 100 (mới)
    //
    // Phân bổ:
    //   admin_ky_thuat:  150 (TẤT CẢ)
    //   admin_quan_li:   100 (cũ 30 + mới 70)
    //   admin_cong_dong:  75 (cũ 20 + mới 55)
    //   member:            0

    /// Kiểm tra user có quyền cụ thể không.
    /// Dùng cho permission gate trong handlers.
    /// Note: Kiểm tra thực tế nên query DB qua `user_has_permission()` SQL function,
    /// nhưng method này cho phép kiểm tra nhanh ở template logic.
    pub fn has_permission_code(&self, code: &str) -> bool {
        // Admin Kỹ Thuật có TẤT CẢ 150 quyền
        if self.is_admin_ky_thuat() {
            return true;
        }
        // Các role khác — kiểm tra theo nhóm quyền đã gán
        match self.role.as_str() {
            "admin_quan_li" => {
                // 100 quyền: 30 cũ + 70 mới
                matches!(code,
                    // === 30 cũ ===
                    // Users (10)
                    "users_view_list" | "users_view_detail" | "users_edit_profile" |
                    "users_change_role" | "users_activate" | "users_delete" |
                    "users_ban" | "users_view_sessions" | "users_manage_oauth" | "users_export_data" |
                    // Content (10)
                    "content_view_pending" | "content_approve" | "content_edit_any" |
                    "content_delete_any" | "content_pin_lock" | "content_manage_cat" |
                    "content_manage_tags" | "content_mod_comments" | "content_mod_reviews" | "content_feature" |
                    // Community (10)
                    "community_view_stats" | "community_manage_grp" | "community_create_off" |
                    "community_manage_evt" | "community_manage_chat" | "community_manage_mem" |
                    "community_broadcast" | "community_manage_inv" | "community_archive" | "community_merge" |
                    // === 70 mới ===
                    // Fund (10)
                    "fund_view_all" | "fund_approve" | "fund_create_campaign" | "fund_manage_expenses" |
                    "fund_export" | "fund_refund" | "fund_view_anonymous" | "fund_manage_categories" |
                    "fund_set_goal" | "fund_audit_log" |
                    // Achievements (10)
                    "ach_view_all" | "ach_create" | "ach_edit" | "ach_grant" | "ach_revoke" |
                    "ach_view_progress" | "ach_manage_rewards" | "ach_view_history" | "ach_export" | "ach_delete" |
                    // Analytics (10)
                    "an_view_dashboard" | "an_view_user_stats" | "an_view_content_stats" |
                    "an_view_revenue" | "an_export_reports" | "an_view_funnel" | "an_view_cohort" |
                    "an_set_kpi" | "an_view_realtime" | "an_integrate_tool" |
                    // Shop (10)
                    "shop_view_all" | "shop_add_product" | "shop_edit_any" | "shop_delete" |
                    "shop_approve" | "shop_view_orders" | "shop_refund" | "shop_manage_categories" |
                    "shop_set_featured" | "shop_export" |
                    // Events (10)
                    "evt_create" | "evt_edit_any" | "evt_delete" | "evt_manage_attendance" |
                    "evt_broadcast" | "evt_view_stats" | "evt_manage_schedule" |
                    "evt_manage_recording" | "evt_set_capacity" | "evt_export" |
                    // Media (5/10)
                    "media_view_all" | "media_view_storage" | "media_delete_any" |
                    "media_moderate" | "media_restore" |
                    // Navigation (5/10)
                    "nav_edit_announce" | "nav_manage_home" | "nav_edit_meta" |
                    "nav_view_settings_log" | "nav_manage_features"
                )
            }
            "admin_cong_dong" => {
                // 75 quyền: 20 cũ + 55 mới
                matches!(code,
                    // === 20 cũ ===
                    // Content (10)
                    "content_view_pending" | "content_approve" | "content_edit_any" |
                    "content_delete_any" | "content_pin_lock" | "content_manage_cat" |
                    "content_manage_tags" | "content_mod_comments" | "content_mod_reviews" | "content_feature" |
                    // Community (10)
                    "community_view_stats" | "community_manage_grp" | "community_create_off" |
                    "community_manage_evt" | "community_manage_chat" | "community_manage_mem" |
                    "community_broadcast" | "community_manage_inv" | "community_archive" | "community_merge" |
                    // === 55 mới ===
                    // Friends (10)
                    "fr_view_all_friends" | "fr_view_all_dm" | "fr_delete_message" | "fr_mute_user" |
                    "fr_manage_blocklist" | "fr_force_unfriend" | "fr_view_dm_reports" |
                    "fr_export_dm" | "fr_manage_groups" | "fr_reset_conversation" |
                    // Mail (10)
                    "mail_view_all" | "mail_delete_any" | "mail_broadcast" | "mail_template" |
                    "mail_view_queue" | "notif_send_all" | "notif_template" | "notif_view_stats" |
                    "notif_delete_any" | "mail_manage_filters" |
                    // Events (10)
                    "evt_create" | "evt_edit_any" | "evt_delete" | "evt_manage_attendance" |
                    "evt_broadcast" | "evt_view_stats" | "evt_manage_schedule" |
                    "evt_manage_recording" | "evt_set_capacity" | "evt_export" |
                    // Achievements (10)
                    "ach_view_all" | "ach_view_progress" | "ach_view_history" | "ach_grant" |
                    "ach_export" | "ach_create" | "ach_edit" | "ach_manage_rewards" |
                    "ach_revoke" | "ach_delete" |
                    // Media (5)
                    "media_view_all" | "media_approve" | "media_moderate" |
                    "media_delete_any" | "media_view_storage" |
                    // Fund (5)
                    "fund_view_all" | "fund_approve" | "fund_view_anonymous" |
                    "fund_view_audit_log" | "fund_audit_log" |
                    // Security (5)
                    "sec_view_audit" | "sec_view_login_log" | "sec_session_revoke" |
                    "sec_spam_filter" | "sec_report_manage"
                )
            }
            _ => false,
        }
    }

    /// Số quyền có giao diện UI thực tế (cho badge/hiển thị).
    /// v0.9.15: đồng bộ với system_permission_count — admin_ky_thuat có 150 quyền
    /// toàn diện trên UI (đã bổ sung nav tiles cho tất cả 10 nhóm chức năng).
    pub fn permission_count(&self) -> u16 {
        match self.role.as_str() {
            "admin_ky_thuat" => 150,
            "admin_quan_li" => 100,
            "admin_cong_dong" => 75,
            _ => 0,
        }
    }

    /// Tổng số quyền hệ thống (permission codes trong has_permission_code).
    /// Đây là potential permissions, không phải UI-accessible.
    /// Dùng cho health check và debug.
    ///
    /// v0.9.14 (Giai đoạn 19): 50 → 150 quyền.
    pub fn system_permission_count(&self) -> u16 {
        match self.role.as_str() {
            "admin_ky_thuat" => 150,
            "admin_quan_li" => 100,
            "admin_cong_dong" => 75,
            _ => 0,
        }
    }

    /// Tên trang admin dashboard tương ứng với role.
    /// v0.9.19: Mod redirect về /admin/thanh-vien (mod không có dashboard riêng).
    pub fn admin_dashboard_path(&self) -> &str {
        match self.role.as_str() {
            "admin_ky_thuat" => "/admin/ky-thuat",
            "admin_cong_dong" => "/admin/cong-dong",
            "admin_quan_li" => "/admin/quan-li",
            "mod" => "/admin/thanh-vien", // Mod không có dashboard riêng — xem danh sách thành viên
            _ => "/admin",
        }
    }
}

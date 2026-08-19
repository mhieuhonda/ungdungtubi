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
/// Từ v0.9.7 (Giai đoạn 11), thêm trường `role` cho hệ thống phân quyền.
///
/// **v0.9.24 (Giai đoạn 29) — REDESIGN PHÂN QUYỀN:**
///   - Tất cả admin NGANG HÀNG nhau (cùng level 3) — không còn hierarchy.
///   - Mỗi admin có quyền khác nhau theo phần mình phụ trách:
///     - `admin_ky_thuat`  — phụ trách kỹ thuật (system, security, infrastructure)
///     - `admin_quan_li`   — phụ trách quản lý (users, content, community, fund)
///     - `admin_cong_dong` — phụ trách cộng đồng (content, community, friends, mail, events)
///   - `mod`             — Mod (level 2, dưới admin, trên member) — moderation cơ bản
///   - `member`          — Thành Viên (mặc định, 0 quyền admin)
///
/// **v0.9.30 (Giai đoạn 35) — THÊM ROLE admin_phat_trien:**
///   - `admin_phat_trien` — Admin Phát Triển (level 3, 39 quyền:
///     system + development + deployment + analytics + navigation + api).
///     Phụ trách định hướng phát triển sản phẩm, CI/CD, roadmap, kỹ thuật xây dựng.
///     NGANG HÀNH với 3 admin kia — không phân cấp.
///
/// Nguyên tắc: "Các admin đều bằng nhau ngang hàng,
///              nhưng mỗi người phụ trách một mảng khác nhau."
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
    /// Bi balance — tiền Từ Bi, loại cao cấp nhất (v0.9.42 — Giai đoạn 46).
    /// Kiếm qua cống hiến đặc biệt hoặc quy đổi từ K. 100 K = 1 Bi.
    #[sqlx(default)]
    pub bi_balance: i64,
    /// Tinh Khí Thần — chỉ số chơi game cấp 100 (v0.9.46 — Giai đoạn 64).
    /// Tăng bằng cách nuốt Tinh Thể (tối đa 10/cấp).
    #[sqlx(default)]
    pub tinh_khi_than: i16,
    /// Max Tinh Khí Thần — giới hạn trên (v0.9.46 — Giai đoạn 64).
    #[sqlx(default)]
    pub max_tinh_khi_than: i16,
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
    /// Vai trò quản trị: member | mod | admin_ky_thuat | admin_cong_dong | admin_quan_li | admin_phat_trien
    /// v0.9.24: tất cả admin NGANG HÀNH (level 3), khác nhau ở permission scope.
    /// v0.9.30: thêm admin_phat_trien (Admin Phát Triển — cấp 3, 39 quyền).
    pub role: String,
    /// Tu Sĩ rank 1-5 sao (NULL nếu chưa duyệt) — v0.9.45 Giai đoạn 53.
    #[sqlx(default)]
    pub tu_si_rank: Option<i16>,
    /// Thời điểm admin duyệt Tu Sĩ — v0.9.45 Giai đoạn 53.
    #[sqlx(default)]
    pub tu_si_approved_at: Option<DateTime<Utc>>,
    /// Lần hoạt động cuối (heartbeat) — v0.9.39 Giai đoạn 43.
    #[sqlx(default)]
    pub last_seen_at: Option<DateTime<Utc>>,
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

    // ─── Hệ thống vai trò (v0.9.24 — Giai đoạn 29: Admin ngang hàng) ───────
    //
    // NGUYÊN TẮC MỚI (v0.9.24):
    //   - Tất cả 3 admin role NGANG HÀNH nhau (cùng level 3)
    //   - Mod ở level 2 (dưới admin, trên member)
    //   - Member ở level 1
    //   - Không còn hierarchy admin_ky_thuat > admin_quan_li > admin_cong_dong
    //   - Mỗi admin có scope quyền riêng theo phần phụ trách

    /// Tên hiển thị tiếng Việt của vai trò.
    /// Dùng cho badge trên profile / header.
    /// v0.9.30: thêm admin_phat_trien → "Admin Phát Triển".
    pub fn role_display(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "Admin Quản Lý",
            "admin_cong_dong" => "Admin Cộng Đồng",
            "admin_ky_thuat" => "Admin Kỹ Thuật",
            "admin_phat_trien" => "Admin Phát Triển",
            "mod" => "Mod",
            _ => "Thành Viên",
        }
    }

    /// Emoji đại diện cho vai trò.
    /// v0.9.30: admin_phat_trien → 🧭 (la bàn — định hướng phát triển).
    pub fn role_icon(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "👑",
            "admin_cong_dong" => "🛡️",
            "admin_ky_thuat" => "⚙️",
            "admin_phat_trien" => "🧭",
            "mod" => "📜",
            _ => "🪷",
        }
    }

    /// Màu sắc đại diện cho vai trò (hex).
    /// v0.9.24: 3 admin dùng 3 màu khác nhau nhưng cùng level — không "cao hơn" nhau.
    /// v0.9.30: admin_phat_trien → indigo-700 (violet đậm — phát triển/sáng tạo).
    pub fn role_color(&self) -> &str {
        match self.role.as_str() {
            "admin_quan_li" => "#FF6F00",      // amber-900 (gold) — quản lý
            "admin_cong_dong" => "#1565C0",     // blue-800 — cộng đồng
            "admin_ky_thuat" => "#6A1B9A",      // purple-800 — kỹ thuật
            "admin_phat_trien" => "#312E81",    // indigo-900 — phát triển
            "mod" => "#0F766E",                 // teal-700 (moderator)
            _ => "#2E7D32",                      // tubi-800 (green)
        }
    }

    /// Cấp độ vai trò (dùng để so sánh quyền):
    ///   - member          → 1
    ///   - mod             → 2
    ///   - admin_*         → 3 (TẤT CẢ admin NGANG HÀNG — v0.9.24)
    ///
    /// v0.9.24: Bỏ hierarchy cũ (admin_ky_thuat=5 > admin_quan_li=4 > admin_cong_dong=3).
    ///          Giờ tất cả admin đều = 3. Phân quyền theo scope phụ trách,
    ///          không theo level.
    ///
    /// Lưu ý: Không dùng `role_level()` để check quyền cụ thể nữa —
    ///        dùng `has_permission_code(code)` hoặc `can_manage_*()` thay thế.
    pub fn role_level(&self) -> u8 {
        match self.role.as_str() {
            "mod" => 2,
            "admin_ky_thuat" | "admin_quan_li" | "admin_cong_dong" | "admin_phat_trien" => 3, // NGANG HÀNG
            _ => 1,
        }
    }

    /// True nếu user là bất kỳ vai trò admin nào (kỹ thuật / cộng đồng / quản lý / phát triển).
    /// v0.9.24: Tất cả 3 admin đều ngang hàng — không phân cấp.
    /// v0.9.30: Thêm admin_phat_trien — 4 admin ngang hàng.
    /// v0.9.19: Mod KHÔNG phải là admin — Mod là chức vụ riêng (dưới admin, trên member).
    /// Dùng `is_staff()` để kiểm tra "admin HOẶC mod".
    pub fn is_admin(&self) -> bool {
        matches!(
            self.role.as_str(),
            "admin_ky_thuat" | "admin_quan_li" | "admin_cong_dong" | "admin_phat_trien"
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

    /// True nếu user chính xác là Admin Quản Lý.
    pub fn is_admin_quan_li(&self) -> bool {
        matches!(self.role.as_str(), "admin_quan_li")
    }

    /// True nếu user chính xác là Admin Phát Triển (v0.9.30 — Giai đoạn 35).
    /// Admin Phát Triển phụ trách định hướng phát triển sản phẩm, CI/CD, roadmap.
    /// NGANG HÀNG với 3 admin kia (cùng cấp 3).
    pub fn is_admin_phat_trien(&self) -> bool {
        matches!(self.role.as_str(), "admin_phat_trien")
    }

    /// True nếu user có quyền kỹ thuật (Admin Kỹ Thuật).
    /// v0.9.24: Dùng permission check thay vì role_level — chỉ admin_ky_thuat có quyền system.
    /// Mod (level 2) không có quyền technical.
    pub fn can_manage_technical(&self) -> bool {
        self.has_permission_code("system_view_status")
    }

    /// True nếu user có quyền cộng đồng (duyệt cảm ngộ, mod comment, etc.).
    /// v0.9.24: Dùng permission check — admin_cong_dong, admin_quan_li, admin_ky_thuat, mod đều có.
    pub fn can_manage_community(&self) -> bool {
        self.has_permission_code("content_mod_reviews")
            || self.has_permission_code("content_mod_comments")
    }

    /// True nếu user có quyền quản trị (đổi role user).
    /// v0.9.24: Dùng permission check — chỉ admin_quan_li và admin_ky_thuat có users_change_role.
    /// (admin_cong_dong và mod KHÔNG có — scope của họ là cộng đồng, không phải user management.)
    pub fn can_manage_admin(&self) -> bool {
        self.has_permission_code("users_change_role")
    }

    /// True nếu user được phép ban/kích hoạt user.
    /// v0.9.24: Ai có quyền users_ban thì được ban/activate.
    /// admin_ky_thuat, admin_quan_li đều có users_ban. admin_cong_dong và mod không có.
    pub fn can_ban_user(&self) -> bool {
        self.has_permission_code("users_ban")
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
    // v0.9.24 REDESIGN: Phân bổ quyền theo scope phụ trách (admin ngang hàng):
    //   admin_ky_thuat:  40 quyền (system + security + technical users + media + analytics + nav + api)
    //   admin_quan_li:   40 quyền (users + content + community + fund + mail/notif)
    //   admin_cong_dong: 45 quyền (content + community + friends + mail + events + achievements + media)
    //   mod:             15 quyền (content moderation + community moderation + friends + security reporting)
    //   member:           0 quyền admin

    /// Kiểm tra user có quyền cụ thể không.
    /// Dùng cho permission gate trong handlers.
    ///
    /// v0.9.24: Phân bổ theo scope phụ trách — không còn "admin_ky_thuat có TẤT CẢ quyền".
    /// Mỗi admin chỉ có quyền trong lĩnh vực mình phụ trách.
    ///
    /// Note: Kiểm tra thực tế nên query DB qua `user_has_permission()` SQL function,
    /// nhưng method này cho phép kiểm tra nhanh ở template logic.
    pub fn has_permission_code(&self, code: &str) -> bool {
        // Member không có quyền admin nào
        if self.role == "member" {
            return false;
        }

        match self.role.as_str() {
            // ════════════════════════════════════════════════════════════════
            // admin_ky_thuat — Phụ trách KỸ THUẬT (40 quyền)
            // Scope: system, security, technical users, media storage, analytics, nav, api
            // ════════════════════════════════════════════════════════════════
            "admin_ky_thuat" => matches!(code,
                // System (10) — toàn quyền hệ thống
                "system_view_status" | "system_manage_config" | "system_manage_migrate" |
                "system_view_logs" | "system_manage_cache" | "system_restart_server" |
                "system_manage_cron" | "system_view_metrics" | "system_manage_backup" |
                "system_debug_mode" |
                // Users — xem + kỹ thuật + change_role (v0.9.25 fix B6: đồng bộ với comment
                // trong admin.rs và migration 021 — admin_ky_thuat CÓ đổi role được)
                "users_view_list" | "users_view_detail" | "users_view_sessions" |
                "users_change_role" | "users_activate" | "users_ban" | "users_export_data" |
                // Security (5) — chuyên môn admin_ky_thuat
                "sec_view_audit" | "sec_view_login_log" | "sec_session_revoke" |
                "sec_spam_filter" | "sec_report_manage" |
                // Media (5) — technical storage
                "media_view_all" | "media_view_storage" | "media_delete_any" |
                "media_moderate" | "media_restore" |
                // Analytics (6)
                "an_view_dashboard" | "an_view_user_stats" | "an_view_content_stats" |
                "an_view_revenue" | "an_export_reports" | "an_view_realtime" |
                // Navigation (5) — technical config
                "nav_edit_announce" | "nav_manage_home" | "nav_edit_meta" |
                "nav_view_settings_log" | "nav_manage_features" |
                // API keys
                "api_manage_keys"
            ),

            // ════════════════════════════════════════════════════════════════
            // admin_quan_li — Phụ trách QUẢN LÝ (40 quyền)
            // Scope: users, content, community, fund, mail/notif
            // ════════════════════════════════════════════════════════════════
            "admin_quan_li" => matches!(code,
                // Users (10) — quản lý thành viên đầy đủ (bao gồm change_role)
                "users_view_list" | "users_view_detail" | "users_edit_profile" |
                "users_change_role" | "users_activate" | "users_delete" |
                "users_ban" | "users_view_sessions" | "users_manage_oauth" | "users_export_data" |
                // Content (10) — kiểm duyệt nội dung
                "content_view_pending" | "content_approve" | "content_edit_any" |
                "content_delete_any" | "content_pin_lock" | "content_manage_cat" |
                "content_manage_tags" | "content_mod_comments" | "content_mod_reviews" | "content_feature" |
                // Community (10) — quản lý cộng đồng
                "community_view_stats" | "community_manage_grp" | "community_create_off" |
                "community_manage_evt" | "community_manage_chat" | "community_manage_mem" |
                "community_broadcast" | "community_manage_inv" | "community_archive" | "community_merge" |
                // Fund (5) — quản lý quỹ từ bi
                "fund_view_all" | "fund_approve" | "fund_view_anonymous" |
                "fund_audit_log" | "fund_export" |
                // Mail/Notif (5) — thông báo hệ thống
                "mail_view_all" | "notif_send_all" | "mail_broadcast" |
                "notif_template" | "mail_view_queue"
            ),

            // ════════════════════════════════════════════════════════════════
            // admin_cong_dong — Phụ trách CỘNG ĐỒNG (45 quyền)
            // Scope: content, community, friends, mail, events, achievements, media mod
            // ════════════════════════════════════════════════════════════════
            "admin_cong_dong" => matches!(code,
                // Content (10) — kiểm duyệt nội dung cộng đồng
                "content_view_pending" | "content_approve" | "content_edit_any" |
                "content_delete_any" | "content_pin_lock" | "content_manage_cat" |
                "content_manage_tags" | "content_mod_comments" | "content_mod_reviews" | "content_feature" |
                // Community (10) — quản lý cộng đồng
                "community_view_stats" | "community_manage_grp" | "community_create_off" |
                "community_manage_evt" | "community_manage_chat" | "community_manage_mem" |
                "community_broadcast" | "community_manage_inv" | "community_archive" | "community_merge" |
                // Friends (5) — quản lý kết bạn
                "fr_view_all_friends" | "fr_view_all_dm" | "fr_delete_message" |
                "fr_view_dm_reports" | "fr_manage_groups" |
                // Mail (5) — quản lý thư
                "mail_view_all" | "mail_delete_any" | "mail_broadcast" |
                "mail_view_queue" | "mail_manage_filters" |
                // Events (5) — quản lý sự kiện cộng tu
                "evt_create" | "evt_edit_any" | "evt_manage_attendance" |
                "evt_broadcast" | "evt_view_stats" |
                // Achievements (5) — quản lý thành tích
                "ach_view_all" | "ach_view_progress" | "ach_view_history" |
                "ach_grant" | "ach_export" |
                // Media moderation (5)
                "media_view_all" | "media_approve" | "media_moderate" |
                "media_delete_any" | "media_view_storage"
            ),

            // ════════════════════════════════════════════════════════════════
            // admin_phat_trien — Phụ trách PHÁT TRIỂN (39 quyền) — v0.9.30 Giai đoạn 35
            // Scope: system, development, deployment, analytics, navigation, api
            // Giao thoa với admin_ky_thuat nhưng tập trung vào phát triển sản phẩm
            // ════════════════════════════════════════════════════════════════
            "admin_phat_trien" => matches!(code,
                // System (10) — toàn quyền hệ thống (giao thoa admin_ky_thuat)
                "system_view_status" | "system_manage_config" | "system_manage_migrate" |
                "system_view_logs" | "system_manage_cache" | "system_restart_server" |
                "system_manage_cron" | "system_view_metrics" | "system_manage_backup" |
                "system_debug_mode" |
                // Users (7) — xem + đổi role + kỹ thuật
                "users_view_list" | "users_view_detail" | "users_view_sessions" |
                "users_change_role" | "users_activate" | "users_ban" | "users_export_data" |
                // Security (5) — chuyên môn kỹ thuật
                "sec_view_audit" | "sec_view_login_log" | "sec_session_revoke" |
                "sec_spam_filter" | "sec_report_manage" |
                // Media (5) — technical storage
                "media_view_all" | "media_view_storage" | "media_delete_any" |
                "media_moderate" | "media_restore" |
                // Analytics (6) — theo dõi phát triển sản phẩm
                "an_view_dashboard" | "an_view_user_stats" | "an_view_content_stats" |
                "an_view_revenue" | "an_export_reports" | "an_view_realtime" |
                // Navigation (5) — định hướng UI/UX phát triển
                "nav_edit_announce" | "nav_manage_home" | "nav_edit_meta" |
                "nav_view_settings_log" | "nav_manage_features" |
                // API keys
                "api_manage_keys"
            ),

            // ════════════════════════════════════════════════════════════════
            // mod — Moderator cơ bản (15 quyền)
            // Scope: content moderation, chat moderation, basic community
            // ════════════════════════════════════════════════════════════════
            "mod" => matches!(code,
                // Content moderation (5)
                "content_view_pending" | "content_approve" | "content_mod_comments" |
                "content_mod_reviews" | "content_pin_lock" |
                // Community moderation (5)
                "community_view_stats" | "community_manage_chat" | "community_manage_mem" |
                "community_broadcast" | "community_archive" |
                // Friends/DM moderation (3)
                "fr_view_dm_reports" | "fr_delete_message" | "fr_manage_groups" |
                // Security reporting (2)
                "sec_view_audit" | "sec_report_manage"
            ),

            _ => false,
        }
    }

    /// Số quyền có giao diện UI thực tế (cho badge/hiển thị).
    /// v0.9.30: Đồng bộ với migration 022 (+admin_phat_trien → 39 quyền).
    /// v0.9.25: Đồng bộ với migration 021 (+users_change_role cho admin_ky_thuat → 41 quyền).
    /// v0.9.24: Đồng bộ với migration 021 — admin ngang hàng, mỗi role có scope riêng.
    pub fn permission_count(&self) -> u16 {
        match self.role.as_str() {
            "admin_ky_thuat" => 41,
            "admin_quan_li" => 40,
            "admin_cong_dong" => 45,
            "admin_phat_trien" => 39,
            "mod" => 15,
            _ => 0,
        }
    }

    /// Tổng số quyền hệ thống (permission codes trong has_permission_code).
    /// Đồng bộ với `permission_count()` từ v0.9.24.
    pub fn system_permission_count(&self) -> u16 {
        self.permission_count()
    }

    /// Tên trang admin dashboard tương ứng với role.
    /// v0.9.24: Mỗi admin có dashboard riêng theo scope phụ trách.
    /// v0.9.19: Mod redirect về /admin/thanh-vien (mod không có dashboard riêng).
    /// v0.9.32: admin_phat_trien giờ có dashboard riêng /admin/phat-trien.
    pub fn admin_dashboard_path(&self) -> &str {
        match self.role.as_str() {
            "admin_ky_thuat" => "/admin/ky-thuat",
            "admin_cong_dong" => "/admin/cong-dong",
            "admin_quan_li" => "/admin/quan-li",
            // v0.9.32: admin_phat_trien có dashboard riêng (indigo, vision, roadmap, CI/CD).
            "admin_phat_trien" => "/admin/phat-trien",
            "mod" => "/admin/thanh-vien",
            _ => "/admin",
        }
    }
}

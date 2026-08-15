//! Handlers cho trang Quản Trị (Giai đoạn 12 — v0.9.14).
//!
//! Hệ thống vai trò (v0.9.14 — hierarchy mới):
//!   - `admin_ky_thuat`  — Admin Kỹ Thuật (CAO NHẤT — 6/50 quyền: UI/hệ thống)
//!   - `admin_quan_li`   — Admin Quản Lý (4/30 quyền: UI/hệ thống)
//!   - `admin_cong_dong` — Admin Cộng Đồng (4/20 quyền: UI/hệ thống)
//!   - `member`          — Thành Viên (mặc định, 0 quyền admin)
//!
//! 3 giao diện admin riêng biệt:
//!   - /admin/ky-thuat    — Phong cách coder/terminal (tối, ngầu, Matrix)
//!   - /admin/cong-dong   — Phong cách community mod (xanh, social)
//!   - /admin/quan-li     — Phong cách executive (vàng, premium)
//!
//! Routes:
//!   - GET  /admin                       — Redirect đến dashboard tương ứng role
//!   - GET  /admin/ky-thuat             — Dashboard Admin Kỹ Thuật (terminal style)
//!   - GET  /admin/ky-thuat/nhat-ky     — Full audit log page (paginated)
//!   - GET  /admin/cong-dong            — Dashboard Admin Cộng Đồng (mod style)
//!   - GET  /admin/cong-dong/cam-ngo    — Content moderation (pending reviews)
//!   - POST /admin/cong-dong/cam-ngo/{id}/duyet    — Approve review
//!   - POST /admin/cong-dong/cam-ngo/{id}/tu-choi  — Reject review
//!   - GET  /admin/quan-li              — Dashboard Admin Quản Lý (exec style)
//!   - GET  /admin/thanh-vien           — List users + roles (shared)
//!   - POST /admin/thanh-vien/{id}/role — Đổi role user (admin_ky_thuat + admin_quan_li)
//!   - POST /admin/thanh-vien/{id}/ban  — Ban user (admin_ky_thuat only)
//!   - POST /admin/thanh-vien/{id}/kich-hoat — Activate user (admin_ky_thuat only)

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

/// Row data cho danh sách thành viên trong trang /admin/thanh-vien.
///
/// v0.9.9: thêm `last_session_at` (lấy từ MAX(sessions.created_at)) để hiển thị
/// "hoạt động gần nhất" thay vì "Ngày tham gia" — giống phong cách hiển thị trong ảnh
/// (online dot hoặc "6 ngày trước").
#[allow(dead_code)] // một số field dành cho future UI, giữ lại để tránh drift với DB schema.
#[derive(Debug, Clone, FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub rank: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub k_balance: i64,
    pub a_balance: i64,
    /// Nguyên lực I — phần thưởng từ Tượng Phật (v0.9.10).
    pub i_balance: i64,
    /// Thời gian đăng nhập gần nhất (lấy từ sessions). NULL nếu chưa từng đăng nhập.
    pub last_session_at: Option<DateTime<Utc>>,
}

impl AdminUserRow {
    /// Màu đại diện cho vai trò (dùng cho avatar background + role badge).
    pub fn role_color_hint(&self) -> &'static str {
        match self.role.as_str() {
            "admin_quan_li" => "#FF6F00",
            "admin_cong_dong" => "#1565C0",
            "admin_ky_thuat" => "#6A1B9A",
            "mod" => "#0F766E",  // v0.9.19: teal-700 cho Mod
            _ => "#2E7D32",
        }
    }

    /// Handle dùng cho dòng `@username` — ưu tiên phần localpart của email.
    pub fn handle(&self) -> String {
        self.email.split('@').next().unwrap_or("").to_string()
    }

    /// HTML cho role badge (top-right của card).
    /// v0.9.19: thêm option "Mod".
    pub fn role_badge_html(&self) -> String {
        let (icon, label) = match self.role.as_str() {
            "admin_quan_li" => ("👑", "Admin Quản Lý"),
            "admin_cong_dong" => ("🛡️", "Admin Cộng Đồng"),
            "admin_ky_thuat" => ("⚙️", "Admin Kỹ Thuật"),
            "mod" => ("📜", "Mod"),
            _ => ("🪷", "Thành Viên"),
        };
        let color = self.role_color_hint();
        format!(
            r#"<span class="shrink-0 inline-flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-semibold" style="background-color: {color}22; color: {color}"><span>{icon}</span><span>{label}</span></span>"#
        )
    }

    /// Text trạng thái "online/hoạt động gần đây" cho footer của card.
    /// Trả về (css_class, dot_color, text).
    /// v0.9.9: nhận `&DateTime<Utc>` vì Askama truyền field bằng reference.
    pub fn last_seen_text(&self, now: &DateTime<Utc>) -> (String, String, String) {
        if !self.is_active {
            return (
                "text-red-600".into(),
                "bg-red-500".into(),
                "Bị khóa".into(),
            );
        }
        match self.last_session_at {
            None => (
                "text-gray-400".into(),
                "bg-gray-300".into(),
                "chưa đăng nhập".into(),
            ),
            Some(last) => {
                let mins_ago = (now.timestamp() - last.timestamp()) / 60;
                if mins_ago < 5 {
                    ("text-green-600".into(), "bg-green-500".into(), "Đang hoạt động".into())
                } else if mins_ago < 60 {
                    ("text-gray-500".into(), "bg-gray-300".into(), format!("{mins_ago} phút trước"))
                } else if mins_ago < 1440 {
                    ("text-gray-500".into(), "bg-gray-300".into(), format!("{} giờ trước", mins_ago / 60))
                } else {
                    ("text-gray-500".into(), "bg-gray-300".into(), format!("{} ngày trước", mins_ago / 1440))
                }
            }
        }
    }
}

/// Stats cho dashboard /admin (chung cho cả 3 kiểu).
#[derive(Debug, Clone, Default, FromRow)]
pub struct AdminStats {
    pub total_users: i64,
    pub active_users: i64,
    pub admin_count: i64,
    pub total_groups: i64,
    pub total_topics: i64,
    pub total_comments: i64,
    pub total_books: i64,
    pub total_mails: i64,
    pub pending_reviews: i64,
}

// Template structs (Askama).

/// Audit log entry for admin actions
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: i64,
    pub created_at: String,
    pub action: String,
    pub user_handle: String,
    pub detail: String,
}

/// Permission group summary for admin dashboard.
/// v0.9.15: hiển thị 10 nhóm quyền × 10 quyền mỗi nhóm = 150 quyền tổng.
#[derive(Debug, Clone)]
pub struct PermGroup {
    pub code: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    /// Số quyền admin_ky_thuat có trong nhóm này (luôn = 10).
    pub ky_thuat: u16,
}

/// Trả về 10 nhóm quyền (mỗi nhóm 10 quyền) cho dashboard.
fn all_perm_groups() -> Vec<PermGroup> {
    vec![
        PermGroup { code: "system",      label: "Hệ thống",      icon: "⚙️",  ky_thuat: 10 },
        PermGroup { code: "users",       label: "Thành viên",    icon: "👥",  ky_thuat: 10 },
        PermGroup { code: "content",     label: "Nội dung",      icon: "📝",  ky_thuat: 10 },
        PermGroup { code: "community",   label: "Cộng đồng",     icon: "🏛️",  ky_thuat: 10 },
        PermGroup { code: "kinh_sach",   label: "Kinh Sách",     icon: "📚",  ky_thuat: 10 },
        PermGroup { code: "fund",        label: "Quỹ Từ Bi",     icon: "🪷",  ky_thuat: 10 },
        PermGroup { code: "achievements",label: "Thành tích",    icon: "🎖️",  ky_thuat: 10 },
        PermGroup { code: "security",    label: "Bảo mật",       icon: "🔒",  ky_thuat: 10 },
        PermGroup { code: "media",       label: "Media",         icon: "🖼️",  ky_thuat: 10 },
        PermGroup { code: "analytics",   label: "Phân tích",     icon: "📊",  ky_thuat: 10 },
    ]
}

/// Admin Kỹ Thuật dashboard — terminal/coder style (KHÔNG extends layout.html)
#[derive(Template)]
#[template(path = "admin/ky-thuat/index.html")]
pub struct AdminKyThuatTemplate {
    pub user: Option<User>,
    pub stats: AdminStats,
    pub audit_logs: Vec<AuditLog>,
    pub perm_groups: Vec<PermGroup>,
}

/// Admin Cộng Đồng dashboard — community mod style
#[derive(Template)]
#[template(path = "admin/cong-dong/index.html")]
pub struct AdminCongDongTemplate {
    pub user: Option<User>,
    pub stats: AdminStats,
}

/// Admin Quản Lý dashboard — executive/premium style
#[derive(Template)]
#[template(path = "admin/quan-li/index.html")]
pub struct AdminQuanLiTemplate {
    pub user: Option<User>,
    pub stats: AdminStats,
}

/// Shared users list template (extends layout.html — phong cách web chính)
#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsersTemplate {
    pub user: Option<User>,
    pub users: Vec<AdminUserRow>,
    pub active_page: String,
    pub error: Option<String>,
    pub success: Option<String>,
    /// v0.9.9: thời điểm render — dùng để tính "X phút trước" cho last_seen.
    pub now: DateTime<Utc>,
}

/// Form đổi role user.
#[derive(Debug, Deserialize)]
pub struct RoleChangeForm {
    pub role: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /admin — Redirect đến dashboard tương ứng với role.
///
/// - admin_ky_thuat → /admin/ky-thuat
/// - admin_cong_dong → /admin/cong-dong
/// - admin_quan_li → /admin/quan-li
pub async fn admin_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // v0.9.19: Cho phép Mod xem /admin — redirect về /admin/thanh-vien.
    // (Trước đây chỉ admin mới vào được /admin, mod bị 403.)
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    // Redirect đến dashboard riêng của role
    // (Mod redirect về /admin/thanh-vien — không có dashboard riêng)
    Redirect::to(user.admin_dashboard_path()).into_response()
}

/// GET /admin/ky-thuat — Dashboard Admin Kỹ Thuật (terminal/coder style).
///
/// Chỉ admin_ky_thuat mới vào được trang này.
/// Phong cách: tối, terminal, Matrix-like, cực ngầu.
pub async fn admin_ky_thuat_dashboard(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission check — chỉ admin_ky_thuat
    if !user.is_admin_ky_thuat() {
        return render_forbidden(&user);
    }

    let stats = fetch_admin_stats_or_default(&state.pool).await;
    let audit_logs = fetch_audit_logs(&state.pool, 20).await;
    let perm_groups = all_perm_groups();

    let html = AdminKyThuatTemplate {
        user: Some(user),
        stats,
        audit_logs,
        perm_groups,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin ky-thuat): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /admin/cong-dong — Dashboard Admin Cộng Đồng (community mod style).
///
/// Chỉ admin_cong_dong mới vào được trang này.
/// Phong cách: xanh dương, social, ấm áp.
pub async fn admin_cong_dong_dashboard(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission check — chỉ admin_cong_dong
    if !user.is_admin_cong_dong() {
        return render_forbidden(&user);
    }

    let stats = fetch_admin_stats_or_default(&state.pool).await;

    let html = AdminCongDongTemplate {
        user: Some(user),
        stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin cong-dong): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /admin/quan-li — Dashboard Admin Quản Lý (executive/premium style).
///
/// Chỉ admin_quan_li mới vào được trang này.
/// Phong cách: vàng, premium, executive dashboard.
pub async fn admin_quan_li_dashboard(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission check — chỉ admin_quan_li
    if !user.is_admin_quan_li() {
        return render_forbidden(&user);
    }

    let stats = fetch_admin_stats_or_default(&state.pool).await;

    let html = AdminQuanLiTemplate {
        user: Some(user),
        stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin quan-li): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /admin/ky-thuat/users — Redirect sang /admin/thanh-vien.
///
/// v0.9.12: Fix 404 — sidebar admin kỹ thuật có link "Quản lý thành viên" cũ
/// trỏ tới `/admin/ky-thuat/users` (không tồn tại). Redirect sang route thật
/// `/admin/thanh-vien` (shared user list) để không còn 404 nữa.
pub async fn admin_ky_thuat_users_redirect(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin_ky_thuat() {
        return render_forbidden(&user);
    }
    Redirect::to("/admin/thanh-vien").into_response()
}

/// GET /admin/thanh-vien — Danh sách thành viên + role.
///
/// Chỉ admin mới xem được. Chỉ admin_ky_thuat và admin_quan_li mới đổi role được.
pub async fn admin_users_list(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // v0.9.19: Mod được xem danh sách thành viên (nhưng không đổi role được).
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    let users = fetch_users_list(&state.pool).await;

    let html = AdminUsersTemplate {
        user: Some(user),
        users,
        active_page: "admin".into(),
        error: None,
        success: None,
        now: Utc::now(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin users): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /admin/thanh-vien/{id}/role — Đổi role của một user.
///
/// **v0.9.8**: Admin Kỹ Thuật (cao nhất) + Admin Quản Lý mới được đổi role.
/// Admin Cộng Đồng có thể xem nhưng không thể đổi.
pub async fn admin_change_role(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
    Form(form): Form<RoleChangeForm>,
) -> Response {
    let Some(actor) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission: admin_ky_thuat (level 4) + admin_quan_li (level 3) mới đổi role được
    if !actor.can_manage_admin() {
        return render_forbidden(&actor);
    }

    // Validate role
    // v0.9.19: thêm 'mod' vào danh sách role hợp lệ.
    let new_role = form.role.trim().to_string();
    if !matches!(
        new_role.as_str(),
        "member" | "mod" | "admin_ky_thuat" | "admin_cong_dong" | "admin_quan_li"
    ) {
        return render_users_error(
            &state.pool,
            &actor,
            "Role không hợp lệ. Phải là: member | mod | admin_ky_thuat | admin_cong_dong | admin_quan_li",
        )
        .await;
    }

    // Không cho admin tự demote chính mình (tránh khoá mình ra khỏi hệ thống)
    if actor.id == user_id && actor.role != new_role {
        return render_users_error(
            &state.pool,
            &actor,
            "Bạn không thể tự đổi vai trò của chính mình.",
        )
        .await;
    }

    // Admin Quản Lý không được nâng ai lên admin_ky_thuat (chỉ admin_ky_thuat mới được)
    // v0.9.19: Cũng không được nâng lên admin_ky_thuat từ mod hoặc member.
    if actor.is_admin_quan_li() && new_role == "admin_ky_thuat" {
        return render_users_error(
            &state.pool,
            &actor,
            "Admin Quản Lý không thể nâng ai lên Admin Kỹ Thuật. Chỉ Admin Kỹ Thuật mới có quyền này.",
        )
        .await;
    }

    // v0.9.19: Admin Cộng Đồng không được đổi role user khác — chỉ xem được.
    // (admin_cong_dong có thể xem /admin/thanh-vien nhưng không đổi role được.)
    // Quyền đổi role: chỉ admin_ky_thuat (level 5) và admin_quan_li (level 4).
    // admin_cong_dong (level 3) và mod (level 2) không được đổi role.
    // Note: `can_manage_admin()` đã trả false cho admin_cong_dong + mod, nên check
    // này đã được handle ở trên. Nhưng để safe, thêm check rõ ràng:
    if actor.is_admin_cong_dong() || actor.is_mod() {
        return render_users_error(
            &state.pool,
            &actor,
            "Vai trò của bạn không có quyền đổi role user khác. Chỉ Admin Kỹ Thuật và Admin Quản Lý mới có quyền này.",
        )
        .await;
    }

    // v0.9.23: Chống leo thang đặc quyền — không cho nâng user lên role cao hơn hoặc bằng actor
    // Admin Quản Lý (level 4) chỉ được đặt role ≤ admin_quan_li (không được đặt admin_ky_thuat)
    // Admin Kỹ Thuật (level 5) được đặt mọi role
    let new_role_level: u8 = match new_role.as_str() {
        "admin_ky_thuat" => 5,
        "admin_quan_li" => 4,
        "admin_cong_dong" => 3,
        "mod" => 2,
        _ => 1,
    };
    if new_role_level >= actor.role_level() {
        return render_users_error(
            &state.pool,
            &actor,
            "Bạn không thể nâng ai lên vai trò cao hơn hoặc bằng vai trò của mình.",
        )
        .await;
    }

    // Update
    match sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_role)
        .bind(user_id)
        .execute(&state.pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return render_users_error(
                    &state.pool,
                    &actor,
                    "Không tìm thấy user với ID đã cho.",
                )
                .await;
            }
            log::info!(
                "🔧 Admin {} ({}) đổi role user {} → {}",
                actor.display_name,
                actor.email,
                user_id,
                new_role
            );
            // Audit log
            let detail = format!("{{\"target_user_id\": \"{}\", \"new_role\": \"{}\"}}", user_id, new_role);
            let _ = sqlx::query(
                "INSERT INTO audit_log (actor_id, action, category, details) VALUES ($1, 'change_role', 'permission', $2::jsonb)"
            )
            .bind(actor.id)
            .bind(&detail)
            .execute(&state.pool)
            .await;
        }
        Err(e) => {
            log::error!("❌ Lỗi đổi role user: {e}");
            return render_users_error(
                &state.pool,
                &actor,
                &format!("Lỗi database: {e}"),
            )
            .await;
        }
    }

    // Re-render với success message
    render_users_success(&state.pool, &actor, &format!("Đã cập nhật vai trò → {new_role}")).await
}

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Lấy admin stats, trả về default nếu lỗi.
async fn fetch_admin_stats_or_default(pool: &sqlx::PgPool) -> AdminStats {
    match fetch_admin_stats(pool).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("❌ Lỗi fetch admin stats: {e}");
            AdminStats::default()
        }
    }
}

/// Lấy các thống kê cho dashboard admin.
async fn fetch_admin_stats(pool: &sqlx::PgPool) -> Result<AdminStats, sqlx::Error> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(*)::BIGINT AS total_users,
            COUNT(*) FILTER (WHERE is_active)::BIGINT AS active_users,
            COUNT(*) FILTER (WHERE role != 'member')::BIGINT AS admin_count
         FROM users",
    )
    .fetch_one(pool)
    .await?;

    let total_groups: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM groups")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_topics: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM topics")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_comments: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM comments")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_books: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM books")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_mails: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM mails")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let pending_reviews: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM book_reviews WHERE status = 'pending'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Ok(AdminStats {
        total_users: row.0,
        active_users: row.1,
        admin_count: row.2,
        total_groups,
        total_topics,
        total_comments,
        total_books,
        total_mails,
        pending_reviews,
    })
}

/// Fetch users list — hierarchy mới (v0.9.8): admin_ky_thuat cao nhất.
///
/// v0.9.9: LEFT JOIN sessions để lấy `last_session_at` (MAX(created_at)) — dùng
/// cho hiển thị "hoạt động gần nhất" trên card.
/// v0.9.19: Thêm 'mod' vào ORDER BY (sau admin_cong_dong, trước member).
async fn fetch_users_list(pool: &sqlx::PgPool) -> Vec<AdminUserRow> {
    sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.email, u.display_name, u.role, u.rank, u.is_active,
                u.email_verified, u.created_at, u.k_balance, u.a_balance,
                COALESCE(u.i_balance, 0) AS i_balance,
                (SELECT MAX(s.created_at) FROM sessions s WHERE s.user_id = u.id) AS last_session_at
         FROM users u
         ORDER BY
            CASE u.role
                WHEN 'admin_ky_thuat'  THEN 1
                WHEN 'admin_quan_li'   THEN 2
                WHEN 'admin_cong_dong' THEN 3
                WHEN 'mod'             THEN 4
                ELSE 5
            END,
            u.created_at DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::error!("❌ Lỗi fetch users list: {e}");
        vec![]
    })
}

/// Render trang 403 Forbidden — user không có quyền.
/// v0.9.19: Cập nhật hierarchy để bao gồm Mod.
fn render_forbidden(user: &User) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="vi"><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>403 — Không có quyền — Ứng Dụng Từ Bi</title>
<script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-50 min-h-screen flex items-center justify-center px-4">
<div class="max-w-md w-full bg-white rounded-2xl p-8 shadow-lg text-center">
  <div class="text-5xl mb-4">🚫</div>
  <h1 class="text-xl font-bold text-red-600 mb-2">403 — Không có quyền truy cập</h1>
  <p class="text-gray-600 text-sm mb-2">Trang Quản Trị chỉ dành cho Admin và Mod.</p>
  <p class="text-gray-500 text-xs mb-4">Vai trò hiện tại: <strong>{role_icon} {role_display}</strong> ({perm_count} quyền UI / {sys_perm_count} quyền hệ thống)</p>
  <p class="text-gray-400 text-[10px] mb-6">Hierarchy: Admin Kỹ Thuật (150/150) &gt; Admin Quản Lý (100/100) &gt; Admin Cộng Đồng (75/75) &gt; Mod (15) &gt; Thành Viên (0)</p>
  <a href="/" class="inline-block text-white px-6 py-2 rounded-xl transition" style="background-color:#2E7D32">← Về trang chủ</a>
</div>
</body></html>"#,
        role_icon = user.role_icon(),
        role_display = user.role_display(),
        perm_count = user.permission_count(),
        sys_perm_count = user.system_permission_count(),
    );
    Html(html).into_response()
}

/// Render trang /admin/thanh-vien với error message.
async fn render_users_error(pool: &sqlx::PgPool, actor: &User, error: &str) -> Response {
    let users = fetch_users_list(pool).await;

    let html = AdminUsersTemplate {
        user: Some(actor.clone()),
        users,
        active_page: "admin".into(),
        error: Some(error.into()),
        success: None,
        now: Utc::now(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin users error): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Render trang /admin/thanh-vien với success message.
async fn render_users_success(pool: &sqlx::PgPool, actor: &User, success: &str) -> Response {
    let users = fetch_users_list(pool).await;

    let html = AdminUsersTemplate {
        user: Some(actor.clone()),
        users,
        active_page: "admin".into(),
        error: None,
        success: Some(success.into()),
        now: Utc::now(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin users success): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Fetch audit log entries from the `audit_log` table.
///
/// Joins with `users` to resolve `actor_id → display_name/email`.
/// If the table or columns don't exist yet, returns empty vec gracefully.
async fn fetch_audit_logs(pool: &sqlx::PgPool, limit: i64) -> Vec<AuditLog> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, DateTime<Utc>)>(
        "SELECT al.id, al.action, al.category,
                COALESCE(u.display_name, u.email, 'system') AS user_handle,
                al.details::text AS detail,
                al.created_at
         FROM audit_log al
         LEFT JOIN users u ON u.id = al.actor_id
         ORDER BY al.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch audit logs (table có thể chưa tồn tại): {e}");
        vec![]
    });

    rows.into_iter()
        .map(|(id, action, category, user_handle, detail, created_at)| {
            // Combine category + action for display, format detail nicely
            let display_action = format!("[{category}] {action}");
            let display_detail = detail
                .filter(|d| d != "{}")
                .unwrap_or_default();
            AuditLog {
                id,
                action: display_action,
                user_handle,
                detail: display_detail,
                created_at: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            }
        })
        .collect()
}

// ─── Audit Log Page ─────────────────────────────────────────────────────

/// Audit log full-page template (paginated).
#[derive(Template)]
#[template(path = "admin/ky-thuat/audit-log.html")]
pub struct AuditLogPageTemplate {
    pub user: Option<User>,
    pub audit_logs: Vec<AuditLog>,
    pub page: i64,
    pub has_more: bool,
}

/// GET /admin/ky-thuat/nhat-ky — Full audit log page (paginated).
pub async fn admin_audit_log_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !user.is_admin_ky_thuat() {
        return render_forbidden(&user);
    }

    // Fetch 51 to detect if there are more pages
    let mut logs = fetch_audit_logs(&state.pool, 51).await;
    let has_more = logs.len() > 50;
    if has_more {
        logs.truncate(50);
    }

    let html = AuditLogPageTemplate {
        user: Some(user),
        audit_logs: logs,
        page: 1,
        has_more,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (audit log page): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ─── Content Moderation (Admin Cộng Đồng) ──────────────────────────────

/// Row for pending book review.
#[derive(Debug, Clone, FromRow)]
pub struct PendingReview {
    pub id: Uuid,
    pub book_title: String,
    pub user_name: String,
    pub body_preview: String,
    pub created_at: DateTime<Utc>,
}

/// Content moderation (cam ngo) page template.
#[derive(Template)]
#[template(path = "admin/cong-dong/cam-ngo.html")]
pub struct CamNgoTemplate {
    pub user: Option<User>,
    pub pending_reviews: Vec<PendingReview>,
    pub error: Option<String>,
    pub success: Option<String>,
}

/// GET /admin/cong-dong/cam-ngo — List pending book reviews.
pub async fn admin_cam_ngo_list(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Admin Cộng Đồng and above can moderate
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    let pending_reviews = fetch_pending_reviews(&state.pool).await;

    let html = CamNgoTemplate {
        user: Some(user),
        pending_reviews,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cam-ngo list): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /admin/cong-dong/cam-ngo/{review_id}/duyet — Approve a review.
pub async fn admin_cam_ngo_duyet(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(review_id): Path<Uuid>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !user.is_staff() {
        return render_forbidden(&user);
    }

    match sqlx::query("UPDATE book_reviews SET status = 'approved', updated_at = NOW() WHERE id = $1")
        .bind(review_id)
        .execute(&state.pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return render_cam_ngo_result(&state.pool, &user, Some("Không tìm thấy cảm ngộ."), None).await;
            }
            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_log (actor_id, action, category, details) VALUES ($1, 'approve_review', 'admin', '{}')"
            )
            .bind(user.id)
            .execute(&state.pool)
            .await;
            log::info!("✅ Admin {} duyệt cảm ngộ {}", user.display_name, review_id);
        }
        Err(e) => {
            log::error!("❌ Lỗi duyệt cảm ngộ: {e}");
            return render_cam_ngo_result(&state.pool, &user, Some(&format!("Lỗi database: {e}")), None).await;
        }
    }

    render_cam_ngo_result(&state.pool, &user, None, Some("Đã duyệt cảm ngộ.")).await
}

/// POST /admin/cong-dong/cam-ngo/{review_id}/tu-choi — Reject a review.
pub async fn admin_cam_ngo_tu_choi(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(review_id): Path<Uuid>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !user.is_staff() {
        return render_forbidden(&user);
    }

    match sqlx::query("UPDATE book_reviews SET status = 'rejected', updated_at = NOW() WHERE id = $1")
        .bind(review_id)
        .execute(&state.pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return render_cam_ngo_result(&state.pool, &user, Some("Không tìm thấy cảm ngộ."), None).await;
            }
            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_log (actor_id, action, category, details) VALUES ($1, 'reject_review', 'admin', '{}')"
            )
            .bind(user.id)
            .execute(&state.pool)
            .await;
            log::info!("❌ Admin {} từ chối cảm ngộ {}", user.display_name, review_id);
        }
        Err(e) => {
            log::error!("❌ Lỗi từ chối cảm ngộ: {e}");
            return render_cam_ngo_result(&state.pool, &user, Some(&format!("Lỗi database: {e}")), None).await;
        }
    }

    render_cam_ngo_result(&state.pool, &user, None, Some("Đã từ chối cảm ngộ.")).await
}

/// Fetch pending book reviews for moderation.
async fn fetch_pending_reviews(pool: &sqlx::PgPool) -> Vec<PendingReview> {
    sqlx::query_as::<_, PendingReview>(
        "SELECT br.id,
                COALESCE(b.title, 'Sách không tên') AS book_title,
                COALESCE(u.display_name, u.email, 'Ẩn danh') AS user_name,
                LEFT(br.body, 120) AS body_preview,
                br.created_at
         FROM book_reviews br
         JOIN books b ON b.id = br.book_id
         JOIN users u ON u.id = br.user_id
         WHERE br.status = 'pending' AND br.is_active
         ORDER BY br.created_at ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch pending reviews: {e}");
        vec![]
    })
}

/// Render cam-ngo page with result message.
async fn render_cam_ngo_result(
    pool: &sqlx::PgPool,
    actor: &User,
    error: Option<&str>,
    success: Option<&str>,
) -> Response {
    let pending_reviews = fetch_pending_reviews(pool).await;

    let html = CamNgoTemplate {
        user: Some(actor.clone()),
        pending_reviews,
        error: error.map(String::from),
        success: success.map(String::from),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cam-ngo result): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ─── User Ban / Activate (Admin Kỹ Thuật) ──────────────────────────────

/// POST /admin/thanh-vien/{user_id}/ban — Ban user (set is_active = false).
pub async fn admin_ban_user(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
) -> Response {
    let Some(actor) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !actor.is_admin_ky_thuat() {
        return render_forbidden(&actor);
    }

    // Don't ban yourself
    if actor.id == user_id {
        return render_users_error(
            &state.pool,
            &actor,
            "Bạn không thể khóa chính mình.",
        )
        .await;
    }

    match sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return render_users_error(&state.pool, &actor, "Không tìm thấy user.").await;
            }
            // Audit log
            let detail = format!("{{\"target_user_id\": \"{}\"}}", user_id);
            let _ = sqlx::query(
                "INSERT INTO audit_log (actor_id, action, category, details) VALUES ($1, 'ban_user', 'admin', $2::jsonb)"
            )
            .bind(actor.id)
            .bind(&detail)
            .execute(&state.pool)
            .await;
            log::info!("🔒 Admin {} khóa user {}", actor.display_name, user_id);
        }
        Err(e) => {
            log::error!("❌ Lỗi khóa user: {e}");
            return render_users_error(&state.pool, &actor, &format!("Lỗi database: {e}")).await;
        }
    }

    render_users_success(&state.pool, &actor, "Đã khóa tài khoản.").await
}

/// POST /admin/thanh-vien/{user_id}/kich-hoat — Activate user (set is_active = true).
pub async fn admin_activate_user(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
) -> Response {
    let Some(actor) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !actor.is_admin_ky_thuat() {
        return render_forbidden(&actor);
    }

    match sqlx::query("UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return render_users_error(&state.pool, &actor, "Không tìm thấy user.").await;
            }
            // Audit log
            let detail = format!("{{\"target_user_id\": \"{}\"}}", user_id);
            let _ = sqlx::query(
                "INSERT INTO audit_log (actor_id, action, category, details) VALUES ($1, 'activate_user', 'admin', $2::jsonb)"
            )
            .bind(actor.id)
            .bind(&detail)
            .execute(&state.pool)
            .await;
            log::info!("✅ Admin {} kích hoạt user {}", actor.display_name, user_id);
        }
        Err(e) => {
            log::error!("❌ Lỗi kích hoạt user: {e}");
            return render_users_error(&state.pool, &actor, &format!("Lỗi database: {e}")).await;
        }
    }

    render_users_success(&state.pool, &actor, "Đã kích hoạt tài khoản.").await
}

// ════════════════════════════════════════════════════════════════════════════
// v0.9.17 — Giai đoạn 22: Admin Nav Fix
// ════════════════════════════════════════════════════════════════════════════
//
// Trước đây các nav tile trong admin dashboard trỏ tới user pages (/cong-dong,
// /kinh-sach, /quy-tu-bi, /bang-xep-hang, /thanh-tich) — khiến admin click vào
// rồi bị redirect ra khỏi admin context. User report bug: "tôi vào quản lí
// cộng đồng thì nó không vào phần quản lí mà nó lại vào phần Cộng đồng bình
// thường của user".
//
// Fix: tạo các route admin riêng cho các module chưa có UI quản trị đầy đủ.
// Mỗi route hiển thị:
//   1. Header với icon + tên module
//   2. Stats tổng quan (số lượng items)
//   3. Thông báo "Module đang được phát triển" với danh sách tính năng sắp ra
//   4. Danh sách items gần đây (read-only) để admin có cái nhìn tổng quan
//   5. Nút "Trở về dashboard"
//
// 4 module mới:
//   - GET /admin/cong-dong/nhom  — Quản lý Nhóm Cộng Đồng (read-only list)
//   - GET /admin/kinh-sach       — Quản lý Kinh Sách (read-only list)
//   - GET /admin/binh-luan       — Quản lý Bình luận (read-only list)
//   - GET /admin/quy-tu-bi       — Quản lý Quỹ Từ Bi (read-only list)
//
// Permission: tất cả admin role đều có quyền xem (admin_ky_thuat, admin_quan_li,
// admin_cong_dong). Module moderation đầy đủ sẽ ra mắt ở các phiên bản sau.

/// Row data cho danh sách nhóm trong admin placeholder page.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct AdminGroupRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub visibility: String,
    pub is_active: bool,
    pub member_count: i64,
    pub topic_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Row data cho danh sách sách trong admin placeholder page.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct AdminBookRow {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category: String,
    pub language: String,
    pub view_count: i64,
    pub flower_count: i64,
    pub review_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Row data cho danh sách bình luận trong admin placeholder page.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct AdminCommentRow {
    pub id: i64,
    pub body: String,
    pub author_name: String,
    pub topic_id: i64,
    pub topic_title: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Row data cho danh sách đóng góp quỹ trong admin placeholder page.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct AdminFundRow {
    pub id: i64,
    pub donor_name: Option<String>,
    pub fund_type: String,
    pub amount: i64,
    pub is_anonymous: bool,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Shared template cho 4 admin placeholder pages.
#[derive(Template)]
#[template(path = "admin/placeholder.html")]
pub struct AdminPlaceholderTemplate {
    pub user: Option<User>,
    /// Tên module: "Quản lý Nhóm", "Quản lý Kinh Sách", ...
    pub module_title: String,
    /// Icon emoji: "🏛️", "📚", "💬", "🪷"
    pub module_icon: String,
    /// Mô tả ngắn gọn module làm gì
    pub module_description: String,
    /// Dashboard path để "Trở về" button — ví dụ "/admin/cong-dong"
    pub back_path: String,
    /// Tên dashboard để hiển thị trên nút back — ví dụ "Admin Cộng Đồng"
    pub back_label: String,
    /// Stats tổng quan (reuse AdminStats để có mọi field)
    pub stats: AdminStats,
    /// Danh sách nhóm (chỉ dùng cho module groups)
    pub groups: Vec<AdminGroupRow>,
    /// Danh sách sách (chỉ dùng cho module kinh sách)
    pub books: Vec<AdminBookRow>,
    /// Danh sách bình luận (chỉ dùng cho module bình luận)
    pub comments: Vec<AdminCommentRow>,
    /// Danh sách quỹ (chỉ dùng cho module quỹ từ bi)
    pub funds: Vec<AdminFundRow>,
    /// Module key — "groups" | "books" | "comments" | "fund"
    pub module_key: String,
}

/// GET /admin/cong-dong/nhom — Placeholder Quản lý Nhóm.
///
/// Hiển thị danh sách 20 nhóm mới nhất (read-only) + stats tổng quan.
pub async fn admin_groups_placeholder(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }
    let stats = fetch_admin_stats_or_default(&state.pool).await;
    let groups = fetch_admin_groups_list(&state.pool, 20).await;
    // v0.9.19: back_path/back_label theo role THỰC TẾ của user — tránh 403 khi admin_ky_thuat
    // hoặc admin_quan_li click back từ placeholder groups (trước đây hardcode /admin/cong-dong).
    let (back_path, back_label) = user_admin_dashboard_back(&user);
    let html = AdminPlaceholderTemplate {
        user: Some(user),
        module_title: "Quản lý Nhóm Cộng Đồng".into(),
        module_icon: "🏛️".into(),
        module_description: "Tổng quan tất cả nhóm trong hệ thống — xem số thành viên, chủ đề, trạng thái.".into(),
        back_path,
        back_label,
        stats,
        groups,
        books: vec![],
        comments: vec![],
        funds: vec![],
        module_key: "groups".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin groups placeholder): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// GET /admin/kinh-sach — Placeholder Quản lý Kinh Sách.
pub async fn admin_kinh_sach_placeholder(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }
    let stats = fetch_admin_stats_or_default(&state.pool).await;
    let books = fetch_admin_books_list(&state.pool, 20).await;
    // v0.9.19: back_path/back_label theo role THỰC TẾ của user.
    let (back_path, back_label) = user_admin_dashboard_back(&user);
    let html = AdminPlaceholderTemplate {
        user: Some(user),
        module_title: "Quản lý Kinh Sách".into(),
        module_icon: "📚".into(),
        module_description: "Tổng quan tất cả sách trong thư viện — xem lượt đọc, cảm ngộ, tặng hoa.".into(),
        back_path,
        back_label,
        stats,
        groups: vec![],
        books,
        comments: vec![],
        funds: vec![],
        module_key: "books".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin kinh sach placeholder): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// GET /admin/binh-luan — Placeholder Quản lý Bình luận.
pub async fn admin_binh_luan_placeholder(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }
    let stats = fetch_admin_stats_or_default(&state.pool).await;
    let comments = fetch_admin_comments_list(&state.pool, 20).await;
    // v0.9.19: back_path/back_label theo role THỰC TẾ của user.
    let (back_path, back_label) = user_admin_dashboard_back(&user);
    let html = AdminPlaceholderTemplate {
        user: Some(user),
        module_title: "Quản lý Bình luận".into(),
        module_icon: "💬".into(),
        module_description: "Tổng quan bình luận gần đây trong Cộng Đồng — kiểm duyệt nội dung.".into(),
        back_path,
        back_label,
        stats,
        groups: vec![],
        books: vec![],
        comments,
        funds: vec![],
        module_key: "comments".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin binh luan placeholder): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// GET /admin/quy-tu-bi — Placeholder Quản lý Quỹ Từ Bi.
pub async fn admin_quy_tu_bi_placeholder(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }
    let stats = fetch_admin_stats_or_default(&state.pool).await;
    let funds = fetch_admin_funds_list(&state.pool, 20).await;
    // v0.9.19: back_path/back_label theo role THỰC TẾ của user — tránh 403 khi admin_ky_thuat
    // hoặc admin_cong_dong click back từ placeholder quỹ (trước đây hardcode /admin/quan-li).
    let (back_path, back_label) = user_admin_dashboard_back(&user);
    let html = AdminPlaceholderTemplate {
        user: Some(user),
        module_title: "Quản lý Quỹ Từ Bi".into(),
        module_icon: "🪷".into(),
        module_description: "Tổng quan đóng góp và chi tiêu quỹ — công khai, minh bạch.".into(),
        back_path,
        back_label,
        stats,
        groups: vec![],
        books: vec![],
        comments: vec![],
        funds,
        module_key: "fund".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin quy tu bi placeholder): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

// ─── Helpers for placeholder pages ─────────────────────────────────────────

/// Helper: quyết định back_path/back_label dựa trên role THỰC TẾ của user.
///
/// v0.9.19: FIX BUG USER REPORT — trước đây back_path/back_label được hardcode
/// theo "module owner" (ví dụ /admin/cong-dong/nhom → luôn back về /admin/cong-dong).
/// Điều này gây ra 403 Forbidden khi admin_ky_thuat (hoặc admin_quan_li) click back
/// từ placeholder page: họ bị redirect tới dashboard của role khác → không có quyền.
///
/// Giờ back_path luôn trỏ về dashboard của CHÍNH user đang login:
///   - admin_ky_thuat  → /admin/ky-thuat  (label "Admin Kỹ Thuật")
///   - admin_cong_dong → /admin/cong-dong (label "Admin Cộng Đồng")
///   - admin_quan_li   → /admin/quan-li   (label "Admin Quản Lý")
///   - member          → /admin           (sẽ redirect tiếp tới /dang-nhap)
fn user_admin_dashboard_back(user: &User) -> (String, String) {
    (
        user.admin_dashboard_path().to_string(),
        user.role_display().to_string(),
    )
}

/// Fetch 20 nhóm mới nhất — for admin groups placeholder.
async fn fetch_admin_groups_list(pool: &sqlx::PgPool, limit: i64) -> Vec<AdminGroupRow> {
    sqlx::query_as::<_, AdminGroupRow>(
        "SELECT g.id, g.name, g.slug, g.visibility, g.is_active,
                (SELECT COUNT(*)::BIGINT FROM group_members gm WHERE gm.group_id = g.id) AS member_count,
                (SELECT COUNT(*)::BIGINT FROM topics t WHERE t.group_id = g.id) AS topic_count,
                g.created_at
         FROM groups g
         ORDER BY g.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch admin groups list: {e}");
        vec![]
    })
}

/// Fetch 20 sách mới nhất — for admin kinh sach placeholder.
async fn fetch_admin_books_list(pool: &sqlx::PgPool, limit: i64) -> Vec<AdminBookRow> {
    sqlx::query_as::<_, AdminBookRow>(
        "SELECT b.id, b.title, b.slug, c.slug AS category, b.language,
                b.view_count::BIGINT,
                (SELECT COUNT(*)::BIGINT FROM book_flowers bf WHERE bf.book_id = b.id) AS flower_count,
                (SELECT COUNT(*)::BIGINT FROM book_reviews br WHERE br.book_id = b.id) AS review_count,
                b.created_at
         FROM books b
         LEFT JOIN book_categories c ON c.id = b.category_id
         ORDER BY b.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch admin books list: {e}");
        vec![]
    })
}

/// Fetch 20 bình luận mới nhất — for admin binh luan placeholder.
async fn fetch_admin_comments_list(pool: &sqlx::PgPool, limit: i64) -> Vec<AdminCommentRow> {
    sqlx::query_as::<_, AdminCommentRow>(
        "SELECT c.id, c.body, u.display_name AS author_name, t.id AS topic_id,
                t.title AS topic_title, c.is_active, c.created_at
         FROM comments c
         JOIN users u ON u.id = c.author_id
         JOIN topics t ON t.id = c.topic_id
         ORDER BY c.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch admin comments list: {e}");
        vec![]
    })
}

/// Fetch 20 đóng góp quỹ mới nhất — for admin quỹ từ bi placeholder.
async fn fetch_admin_funds_list(pool: &sqlx::PgPool, limit: i64) -> Vec<AdminFundRow> {
    sqlx::query_as::<_, AdminFundRow>(
        "SELECT id, donor_name, fund_type, amount::BIGINT AS amount,
                is_anonymous, message, created_at
         FROM fund_donations
         ORDER BY created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch admin funds list: {e}");
        vec![]
    })
}

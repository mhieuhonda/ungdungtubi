//! Handlers cho trang Quản Trị (Giai đoạn 12 — v0.9.8).
//!
//! Hệ thống vai trò (v0.9.8 — hierarchy mới):
//!   - `admin_ky_thuat`  — Admin Kỹ Thuật (CAO NHẤT — 50 quyền, toàn quyền hệ thống)
//!   - `admin_quan_li`   — Admin Quản Lý (30 quyền — users + content + community)
//!   - `admin_cong_dong` — Admin Cộng Đồng (20 quyền — content + community)
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
//!   - GET  /admin/cong-dong            — Dashboard Admin Cộng Đồng (mod style)
//!   - GET  /admin/quan-li              — Dashboard Admin Quản Lý (exec style)
//!   - GET  /admin/thanh-vien           — List users + roles (shared)
//!   - POST /admin/thanh-vien/{id}/role — Đổi role user (admin_ky_thuat + admin_quan_li)

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
            _ => "#2E7D32",
        }
    }

    /// Handle dùng cho dòng `@username` — ưu tiên phần localpart của email.
    pub fn handle(&self) -> String {
        self.email.split('@').next().unwrap_or("").to_string()
    }

    /// HTML cho role badge (top-right của card).
    pub fn role_badge_html(&self) -> String {
        let (icon, label) = match self.role.as_str() {
            "admin_quan_li" => ("👑", "Admin Quản Lý"),
            "admin_cong_dong" => ("🛡️", "Admin Cộng Đồng"),
            "admin_ky_thuat" => ("⚙️", "Admin Kỹ Thuật"),
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

/// Admin Kỹ Thuật dashboard — terminal/coder style (KHÔNG extends layout.html)
#[derive(Template)]
#[template(path = "admin/ky-thuat/index.html")]
pub struct AdminKyThuatTemplate {
    pub user: Option<User>,
    pub stats: AdminStats,
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

    if !user.is_admin() {
        return render_forbidden(&user);
    }

    // Redirect đến dashboard riêng của role
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

    let html = AdminKyThuatTemplate {
        user: Some(user),
        stats,
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

    if !user.is_admin() {
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
    let new_role = form.role.trim().to_string();
    if !matches!(
        new_role.as_str(),
        "member" | "admin_ky_thuat" | "admin_cong_dong" | "admin_quan_li"
    ) {
        return render_users_error(
            &state.pool,
            &actor,
            "Role không hợp lệ. Phải là: member | admin_ky_thuat | admin_cong_dong | admin_quan_li",
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
    if actor.is_admin_quan_li() && new_role == "admin_ky_thuat" {
        return render_users_error(
            &state.pool,
            &actor,
            "Admin Quản Lý không thể nâng ai lên Admin Kỹ Thuật. Chỉ Admin Kỹ Thuật mới có quyền này.",
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
                ELSE 4
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
  <p class="text-gray-600 text-sm mb-2">Trang Quản Trị chỉ dành cho Admin.</p>
  <p class="text-gray-500 text-xs mb-4">Vai trò hiện tại: <strong>{role_icon} {role_display}</strong> ({perm_count} quyền)</p>
  <p class="text-gray-400 text-[10px] mb-6">Hierarchy: Admin Kỹ Thuật (50) &gt; Admin Quản Lý (30) &gt; Admin Cộng Đồng (20) &gt; Thành Viên (0)</p>
  <a href="/" class="inline-block text-white px-6 py-2 rounded-xl transition" style="background-color:#2E7D32">← Về trang chủ</a>
</div>
</body></html>"#,
        role_icon = user.role_icon(),
        role_display = user.role_display(),
        perm_count = user.permission_count(),
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

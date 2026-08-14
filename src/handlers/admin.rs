//! Handlers cho trang Quản Trị (Giai đoạn 11 — v0.9.7).
//!
//! Hệ thống vai trò:
//!   - `member`          — Thành Viên (mặc định)
//!   - `admin_ky_thuat`  — Admin Kỹ Thuật
//!   - `admin_cong_dong` — Admin Cộng Đồng
//!   - `admin_quan_li`   — Admin Quản Lý (super admin — quyền cao nhất)
//!
//! Routes:
//!   - GET  /admin                       — Dashboard (stats + quick links)
//!   - GET  /admin/thanh-vien            — List users + roles
//!   - POST /admin/thanh-vien/{id}/role  — Đổi role user (chỉ admin_quan_li)

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
}

/// Stats cho dashboard /admin.
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

/// Template structs (Askama).
#[derive(Template)]
#[template(path = "admin/index.html")]
pub struct AdminIndexTemplate {
    pub user: Option<User>,
    pub stats: AdminStats,
    pub active_page: String,
}

#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsersTemplate {
    pub user: Option<User>,
    pub users: Vec<AdminUserRow>,
    pub active_page: String,
    pub error: Option<String>,
    pub success: Option<String>,
}

/// Form đổi role user.
#[derive(Debug, Deserialize)]
pub struct RoleChangeForm {
    pub role: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /admin — Dashboard quản trị.
///
/// Yêu cầu: user đã đăng nhập và có role admin (kỹ thuật / cộng đồng / quản lý).
pub async fn admin_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission check — chỉ admin mới vào được
    if !user.is_admin() {
        return render_forbidden(&user);
    }

    let stats = match fetch_admin_stats(&state.pool).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("❌ Lỗi fetch admin stats: {e}");
            AdminStats::default()
        }
    };

    let html = AdminIndexTemplate {
        user: Some(user),
        stats,
        active_page: "admin".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin index): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /admin/thanh-vien — Danh sách thành viên + role.
pub async fn admin_users_list(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    if !user.is_admin() {
        return render_forbidden(&user);
    }

    let users = sqlx::query_as::<_, AdminUserRow>(
        "SELECT id, email, display_name, role, rank, is_active, email_verified, created_at,
                k_balance, a_balance
         FROM users
         ORDER BY
            CASE role
                WHEN 'admin_quan_li'   THEN 1
                WHEN 'admin_cong_dong' THEN 2
                WHEN 'admin_ky_thuat'  THEN 3
                ELSE 4
            END,
            created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_else(|e| {
        log::error!("❌ Lỗi fetch users list: {e}");
        vec![]
    });

    let html = AdminUsersTemplate {
        user: Some(user),
        users,
        active_page: "admin".into(),
        error: None,
        success: None,
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
/// **Chỉ Admin Quản Lý (super admin) mới được phép đổi role user khác.**
/// Admin kỹ thuật / cộng đồng có thể xem nhưng không thể đổi.
pub async fn admin_change_role(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
    Form(form): Form<RoleChangeForm>,
) -> Response {
    let Some(actor) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Permission: chỉ admin_quan_li mới đổi role được
    if !actor.is_admin_quan_li() {
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
    if actor.id == user_id && new_role != "admin_quan_li" {
        return render_users_error(
            &state.pool,
            &actor,
            "Bạn không thể tự hạ cấp vai trò của chính mình khi đang là Admin Quản Lý.",
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

/// Lấy các thống kê cho dashboard admin.
async fn fetch_admin_stats(pool: &sqlx::PgPool) -> Result<AdminStats, sqlx::Error> {
    // total_users + active_users + admin_count trong 1 query
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
  <p class="text-gray-600 text-sm mb-2">Trang Quản Trị chỉ dành cho Admin (Kỹ Thuật / Cộng Đồng / Quản Lý).</p>
  <p class="text-gray-500 text-xs mb-6">Vai trò hiện tại của bạn: <strong>{role_icon} {role_display}</strong></p>
  <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition" style="background-color:#2E7D32">← Về trang chủ</a>
</div>
</body></html>"#,
        role_icon = user.role_icon(),
        role_display = user.role_display(),
    );
    Html(html).into_response()
}

/// Render trang /admin/thanh-vien với error message.
async fn render_users_error(pool: &sqlx::PgPool, actor: &User, error: &str) -> Response {
    let users = sqlx::query_as::<_, AdminUserRow>(
        "SELECT id, email, display_name, role, rank, is_active, email_verified, created_at,
                k_balance, a_balance
         FROM users
         ORDER BY
            CASE role
                WHEN 'admin_quan_li'   THEN 1
                WHEN 'admin_cong_dong' THEN 2
                WHEN 'admin_ky_thuat'  THEN 3
                ELSE 4
            END,
            created_at DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let html = AdminUsersTemplate {
        user: Some(actor.clone()),
        users,
        active_page: "admin".into(),
        error: Some(error.into()),
        success: None,
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
    let users = sqlx::query_as::<_, AdminUserRow>(
        "SELECT id, email, display_name, role, rank, is_active, email_verified, created_at,
                k_balance, a_balance
         FROM users
         ORDER BY
            CASE role
                WHEN 'admin_quan_li'   THEN 1
                WHEN 'admin_cong_dong' THEN 2
                WHEN 'admin_ky_thuat'  THEN 3
                ELSE 4
            END,
            created_at DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let html = AdminUsersTemplate {
        user: Some(actor.clone()),
        users,
        active_page: "admin".into(),
        error: None,
        success: Some(success.into()),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin users success): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

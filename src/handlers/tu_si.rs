//! Handlers cho Hệ Thống Tu Sĩ — Giai đoạn 53 (v0.9.45).
//!
//! Theo tài liệu "ỨNG DỤNG TỪ BI.docx" mục II.3.c (Hệ thống Tu Sĩ):
//!   Thành viên có thể đăng ký trở thành Tu Sĩ — được xét duyệt bởi hệ thống
//!   và đội ngũ quản lý. Cấp bậc Tu Sĩ:
//!     Tu Sĩ Một Sao: hỗ trợ từ 100 K/tháng.
//!     Tu Sĩ Hai Sao: hỗ trợ từ 200 K/tháng.
//!     Tu Sĩ Ba Sao: hỗ trợ từ 500 K/tháng.
//!     Tu Sĩ Bốn Sao: hỗ trợ từ 1000 K/tháng.
//!     Tu Sĩ Năm Sao: hỗ trợ từ 5000 K/tháng.
//!
//! Routes:
//!   GET  /tu-si                     — Trang Tu Sĩ (hiển thị form + thông tin)
//!   POST /tu-si/dang-ky             — Gửi đơn đăng ký Tu Sĩ
//!   POST /tu-si/huy-don/{app_id}    — User rút đơn
//!   GET  /admin/tu-si               — Admin duyệt đơn
//!   POST /admin/tu-si/{app_id}/duyet   — Admin duyệt + cấp rank
//!   POST /admin/tu-si/{app_id}/tu-choi — Admin từ chối

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use askama::Template;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Constants ────────────────────────────────────────────────────────────

/// Mức cam kết K/tháng tối thiểu theo rank (1-5 sao).
pub fn min_k_pledge_for_rank(rank: i16) -> i64 {
    match rank {
        1 => 100,
        2 => 200,
        3 => 500,
        4 => 1000,
        5 => 5000,
        _ => 0,
    }
}

/// Tên hiển thị cấp bậc Tu Sĩ.
pub fn tu_si_rank_name(rank: i16) -> &'static str {
    match rank {
        1 => "Tu Sĩ Một Sao",
        2 => "Tu Sĩ Hai Sao",
        3 => "Tu Sĩ Ba Sao",
        4 => "Tu Sĩ Bốn Sao",
        5 => "Tu Sĩ Năm Sao",
        _ => "Chưa đăng ký",
    }
}

/// Emoji ngôi sao cho cấp bậc.
pub fn tu_si_rank_stars(rank: i16) -> &'static str {
    match rank {
        1 => "⭐",
        2 => "⭐⭐",
        3 => "⭐⭐⭐",
        4 => "⭐⭐⭐⭐",
        5 => "⭐⭐⭐⭐⭐",
        _ => "",
    }
}

// ─── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct TuSiApplication {
    pub id: i64,
    pub user_id: Uuid,
    pub requested_rank: i16,
    pub monthly_k_pledge: i64,
    pub motivation: String,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_note: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TuSiApplicationWithUser {
    pub id: i64,
    pub user_id: Uuid,
    pub requested_rank: i16,
    pub monthly_k_pledge: i64,
    pub motivation: String,
    pub status: String,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_note: String,
    pub created_at: DateTime<Utc>,
    pub user_display_name: String,
    pub user_avatar_url: Option<String>,
    pub user_email: String,
    pub user_k_balance: i64,
}

#[derive(Debug, Deserialize)]
pub struct TuSiApplyForm {
    pub requested_rank: i16,
    pub monthly_k_pledge: i64,
    pub motivation: String,
}

#[derive(Debug, Deserialize)]
pub struct TuSiReviewForm {
    pub review_note: String,
}

// ─── Templates ────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "tu-si/index.html")]
pub struct TuSiIndexTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub current_rank: Option<i16>,
    pub current_rank_display: String,
    pub current_rank_stars: String,
    pub current_application: Option<TuSiApplication>,
    pub recent_applications: Vec<TuSiApplication>,
    pub rank_tiers: Vec<(i16, i64, &'static str)>,
    pub error: Option<String>,
    pub success: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/tu-si/index.html")]
pub struct AdminTuSiTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub pending: Vec<TuSiApplicationWithUser>,
    pub approved: Vec<TuSiApplicationWithUser>,
    pub rejected: Vec<TuSiApplicationWithUser>,
    pub total_tu_si: i64,
    pub error: Option<String>,
    pub success: Option<String>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /tu-si — Trang Tu Sĩ (hiển thị form + thông tin cấp bậc hiện tại).
pub async fn tu_si_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    if user.is_none() {
        return Redirect::to("/dang-nhap?next=/tu-si").into_response();
    }
    let user = user.unwrap();

    // Lấy current Tu Sĩ rank của user
    let current_rank: Option<i16> = sqlx::query_scalar(
        "SELECT tu_si_rank FROM users WHERE id = $1"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Lấy đơn đăng ký gần nhất của user
    let current_application: Option<TuSiApplication> = sqlx::query_as(
        "SELECT id, user_id, requested_rank, monthly_k_pledge, motivation, status,
                reviewed_by, reviewed_at, review_note, created_at, updated_at
         FROM tu_si_applications
         WHERE user_id = $1
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Lịch sử đăng ký
    let recent_applications: Vec<TuSiApplication> = sqlx::query_as(
        "SELECT id, user_id, requested_rank, monthly_k_pledge, motivation, status,
                reviewed_by, reviewed_at, review_note, created_at, updated_at
         FROM tu_si_applications
         WHERE user_id = $1
         ORDER BY created_at DESC LIMIT 10"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let rank_tiers: Vec<(i16, i64, &'static str)> = vec![
        (1, 100, "⭐ Tu Sĩ Một Sao"),
        (2, 200, "⭐⭐ Tu Sĩ Hai Sao"),
        (3, 500, "⭐⭐⭐ Tu Sĩ Ba Sao"),
        (4, 1000, "⭐⭐⭐⭐ Tu Sĩ Bốn Sao"),
        (5, 5000, "⭐⭐⭐⭐⭐ Tu Sĩ Năm Sao"),
    ];

    let html = TuSiIndexTemplate {
        user: Some(user),
        active_page: "tu-si".into(),
        current_rank,
        current_rank_display: current_rank.map(tu_si_rank_name).unwrap_or_default().to_string(),
        current_rank_stars: current_rank.map(tu_si_rank_stars).unwrap_or_default().to_string(),
        current_application,
        recent_applications,
        rank_tiers,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (tu-si index): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /tu-si/dang-ky — Gửi đơn đăng ký Tu Sĩ.
pub async fn tu_si_apply(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<TuSiApplyForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/tu-si").into_response();
    };

    // Validate rank
    if form.requested_rank < 1 || form.requested_rank > 5 {
        return render_tu_si_error(&state, &user, "Cấp bậc phải từ 1 đến 5 sao.").await;
    }

    // Validate pledge >= min
    let min_pledge = min_k_pledge_for_rank(form.requested_rank);
    if form.monthly_k_pledge < min_pledge {
        return render_tu_si_error(
            &state,
            &user,
            &format!(
                "Mức cam kết tối thiểu cho Tu Sĩ {} sao là {} K/tháng.",
                form.requested_rank, min_pledge
            ),
        )
        .await;
    }

    // Validate motivation length
    let motivation = form.motivation.trim().to_string();
    if motivation.chars().count() < 20 {
        return render_tu_si_error(
            &state,
            &user,
            "Vui lòng viết tâm nguyện ít nhất 20 ký tự.",
        )
        .await;
    }
    if motivation.chars().count() > 2000 {
        return render_tu_si_error(
            &state,
            &user,
            "Tâm nguyện quá dài (tối đa 2000 ký tự).",
        )
        .await;
    }

    // Check pending application
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM tu_si_applications WHERE user_id = $1 AND status = 'pending'"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if existing.is_some() {
        return render_tu_si_error(
            &state,
            &user,
            "Bạn đã có đơn đang chờ xét duyệt. Vui lòng rút đơn cũ trước khi đăng ký lại.",
        )
        .await;
    }

    // INSERT
    let result = sqlx::query(
        "INSERT INTO tu_si_applications (user_id, requested_rank, monthly_k_pledge, motivation, status)
         VALUES ($1, $2, $3, $4, 'pending')"
    )
    .bind(user.id)
    .bind(form.requested_rank)
    .bind(form.monthly_k_pledge)
    .bind(&motivation)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi INSERT tu_si_applications: {e}");
        return render_tu_si_error(&state, &user, "Không thể gửi đơn. Vui lòng thử lại sau.").await;
    }

    log::info!("📝 User {} đăng ký Tu Sĩ {} sao (pledge {} K/tháng)", user.id, form.requested_rank, form.monthly_k_pledge);
    render_tu_si_success(&state, &user, "Đơn đăng ký Tu Sĩ đã được gửi. Đội ngũ quản lý sẽ xét duyệt sớm.").await
}

/// POST /tu-si/huy-don/{app_id} — User rút đơn đăng ký.
pub async fn tu_si_withdraw(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(app_id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/tu-si").into_response();
    };

    let result = sqlx::query(
        "UPDATE tu_si_applications
         SET status = 'withdrawn', updated_at = NOW()
         WHERE id = $1 AND user_id = $2 AND status = 'pending'"
    )
    .bind(app_id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi rút đơn Tu Sĩ: {e}");
        return render_tu_si_error(&state, &user, "Không thể rút đơn. Vui lòng thử lại.").await;
    }

    render_tu_si_success(&state, &user, "Đã rút đơn đăng ký.").await
}

/// GET /admin/tu-si — Admin duyệt đơn Tu Sĩ.
pub async fn admin_tu_si_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/admin/tu-si").into_response();
    };
    if !user.is_staff() {
        return crate::handlers::admin::render_forbidden(&user);
    }

    let pending: Vec<TuSiApplicationWithUser> = sqlx::query_as(
        "SELECT a.id, a.user_id, a.requested_rank, a.monthly_k_pledge, a.motivation,
                a.status, a.reviewed_at, a.review_note, a.created_at,
                u.display_name AS user_display_name, u.avatar_url AS user_avatar_url,
                u.email AS user_email, u.k_balance AS user_k_balance
         FROM tu_si_applications a
         JOIN users u ON u.id = a.user_id
         WHERE a.status = 'pending'
         ORDER BY a.created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let approved: Vec<TuSiApplicationWithUser> = sqlx::query_as(
        "SELECT a.id, a.user_id, a.requested_rank, a.monthly_k_pledge, a.motivation,
                a.status, a.reviewed_at, a.review_note, a.created_at,
                u.display_name AS user_display_name, u.avatar_url AS user_avatar_url,
                u.email AS user_email, u.k_balance AS user_k_balance
         FROM tu_si_applications a
         JOIN users u ON u.id = a.user_id
         WHERE a.status = 'approved'
         ORDER BY a.reviewed_at DESC LIMIT 20"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let rejected: Vec<TuSiApplicationWithUser> = sqlx::query_as(
        "SELECT a.id, a.user_id, a.requested_rank, a.monthly_k_pledge, a.motivation,
                a.status, a.reviewed_at, a.review_note, a.created_at,
                u.display_name AS user_display_name, u.avatar_url AS user_avatar_url,
                u.email AS user_email, u.k_balance AS user_k_balance
         FROM tu_si_applications a
         JOIN users u ON u.id = a.user_id
         WHERE a.status = 'rejected'
         ORDER BY a.reviewed_at DESC LIMIT 20"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let total_tu_si: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE tu_si_rank IS NOT NULL"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = AdminTuSiTemplate {
        user: Some(user),
        active_page: "admin-tu-si".into(),
        pending,
        approved,
        rejected,
        total_tu_si,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin tu-si): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /admin/tu-si/{app_id}/duyet — Admin duyệt + cấp Tu Sĩ rank.
pub async fn admin_tu_si_approve(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(app_id): Path<i64>,
    Form(form): Form<TuSiReviewForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return crate::handlers::admin::render_forbidden(&user);
    }

    // Lấy đơn đăng ký
    let app: Option<(Uuid, i16)> = sqlx::query_as(
        "SELECT user_id, requested_rank FROM tu_si_applications WHERE id = $1 AND status = 'pending'"
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((target_user_id, requested_rank)) = app else {
        return Redirect::to("/admin/tu-si").into_response();
    };

    // Update đơn
    let _ = sqlx::query(
        "UPDATE tu_si_applications
         SET status = 'approved', reviewed_by = $1, reviewed_at = NOW(), review_note = $2,
             updated_at = NOW()
         WHERE id = $3"
    )
    .bind(user.id)
    .bind(&form.review_note)
    .bind(app_id)
    .execute(&state.pool)
    .await;

    // Update user tu_si_rank + tu_si_approved_at
    let _ = sqlx::query(
        "UPDATE users SET tu_si_rank = $1, tu_si_approved_at = NOW(), updated_at = NOW()
         WHERE id = $2"
    )
    .bind(requested_rank)
    .bind(target_user_id)
    .execute(&state.pool)
    .await;

    log::info!(
        "✅ Admin {} duyệt Tu Sĩ {} sao cho user {}",
        user.id, requested_rank, target_user_id
    );

    Redirect::to("/admin/tu-si").into_response()
}

/// POST /admin/tu-si/{app_id}/tu-choi — Admin từ chối đơn.
pub async fn admin_tu_si_reject(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(app_id): Path<i64>,
    Form(form): Form<TuSiReviewForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return crate::handlers::admin::render_forbidden(&user);
    }

    let _ = sqlx::query(
        "UPDATE tu_si_applications
         SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW(), review_note = $2,
             updated_at = NOW()
         WHERE id = $3 AND status = 'pending'"
    )
    .bind(user.id)
    .bind(&form.review_note)
    .bind(app_id)
    .execute(&state.pool)
    .await;

    Redirect::to("/admin/tu-si").into_response()
}

// ─── Helpers ──────────────────────────────────────────────────────────────

async fn render_tu_si_error(state: &AppState, user: &User, msg: &str) -> Response {
    let current_rank: Option<i16> = sqlx::query_scalar(
        "SELECT tu_si_rank FROM users WHERE id = $1"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let html = TuSiIndexTemplate {
        user: Some(user.clone()),
        active_page: "tu-si".into(),
        current_rank,
        current_rank_display: current_rank.map(tu_si_rank_name).unwrap_or_default().to_string(),
        current_rank_stars: current_rank.map(tu_si_rank_stars).unwrap_or_default().to_string(),
        current_application: None,
        recent_applications: Vec::new(),
        rank_tiers: vec![
            (1, 100, "⭐ Tu Sĩ Một Sao"),
            (2, 200, "⭐⭐ Tu Sĩ Hai Sao"),
            (3, 500, "⭐⭐⭐ Tu Sĩ Ba Sao"),
            (4, 1000, "⭐⭐⭐⭐ Tu Sĩ Bốn Sao"),
            (5, 5000, "⭐⭐⭐⭐⭐ Tu Sĩ Năm Sao"),
        ],
        error: Some(msg.to_string()),
        success: None,
    }
    .render()
    .unwrap_or_else(|e| format!("Lỗi render: {e}"));

    Html(html).into_response()
}

async fn render_tu_si_success(state: &AppState, user: &User, msg: &str) -> Response {
    let current_rank: Option<i16> = sqlx::query_scalar(
        "SELECT tu_si_rank FROM users WHERE id = $1"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let html = TuSiIndexTemplate {
        user: Some(user.clone()),
        active_page: "tu-si".into(),
        current_rank,
        current_rank_display: current_rank.map(tu_si_rank_name).unwrap_or_default().to_string(),
        current_rank_stars: current_rank.map(tu_si_rank_stars).unwrap_or_default().to_string(),
        current_application: None,
        recent_applications: Vec::new(),
        rank_tiers: vec![
            (1, 100, "⭐ Tu Sĩ Một Sao"),
            (2, 200, "⭐⭐ Tu Sĩ Hai Sao"),
            (3, 500, "⭐⭐⭐ Tu Sĩ Ba Sao"),
            (4, 1000, "⭐⭐⭐⭐ Tu Sĩ Bốn Sao"),
            (5, 5000, "⭐⭐⭐⭐⭐ Tu Sĩ Năm Sao"),
        ],
        error: None,
        success: Some(msg.to_string()),
    }
    .render()
    .unwrap_or_else(|e| format!("Lỗi render: {e}"));

    Html(html).into_response()
}

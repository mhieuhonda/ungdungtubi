//! Handlers cho Mục Tiêu Tu Học + Streak Bảo Vệ — Giai đoạn 58 (v0.9.45).
//!
//! Routes:
//!   GET  /khong-gian/muc-tieu                    — Trang mục tiêu tu học
//!   POST /khong-gian/muc-tieu/tao                 — Tạo mục tiêu mới
//!   POST /khong-gian/muc-tieu/{id}/xoa           — Xoá mục tiêu
//!   POST /khong-gian/muc-tieu/{id}/hoan-thanh    — Đánh dấu hoàn thành
//!   GET  /api/muc-tieu/list                       — JSON API lấy mục tiêu
//!   POST /api/streak-freeze/ap-dung               — Áp dụng streak freeze (1 freeze / ngày bỏ lỡ)
//!   POST /api/streak-freeze/mua                    — Mua streak freeze (100 A / cái)
//!   GET  /api/streak-freeze/quota                 — Quota tháng này + đã dùng

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use askama::Template;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

const STREAK_FREEZE_COST_A: i64 = 100;
const STREAK_FREEZE_MONTHLY_QUOTA: i16 = 2;

#[derive(Debug, Clone, FromRow)]
pub struct TuHocGoal {
    pub id: i64,
    pub user_id: Uuid,
    pub goal_type: String,
    pub target_value: i64,
    pub target_unit: String,
    pub title: String,
    pub status: String,
    pub deadline: Option<chrono::NaiveDate>,
    pub current_value: i64,
    pub last_reset_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoalForm {
    pub goal_type: String,
    pub target_value: i64,
    pub target_unit: String,
    pub title: String,
    pub deadline: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct StreakFreezeQuota {
    pub year_month: i32,
    pub used_count: i16,
    pub remaining: i16,
}

#[derive(Template)]
#[template(path = "khong-gian/muc-tieu.html")]
pub struct MucTieuTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub goals: Vec<TuHocGoal>,
    pub streak_freeze_quota: i16,
    pub streak_freeze_used: i16,
    pub streak_freeze_purchased: i64,
    pub error: Option<String>,
    pub success: Option<String>,
}

/// GET /khong-gian/muc-tieu — Trang mục tiêu tu học.
pub async fn muc_tieu_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/muc-tieu").into_response();
    };

    let goals: Vec<TuHocGoal> = sqlx::query_as(
        "SELECT id, user_id, goal_type, target_value, target_unit, title, status,
                deadline, current_value, last_reset_at, created_at
         FROM tu_hoc_goals
         WHERE user_id = $1 AND status IN ('active', 'completed')
         ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Streak freeze quota tháng này
    let now = Utc::now();
    let year_month = now.year() * 100 + now.month() as i32;
    let quota: StreakFreezeQuota = sqlx::query_as(
        "SELECT $2 AS year_month,
                COALESCE(used_count, 0) AS used_count,
                GREATEST(0, $3 - COALESCE(used_count, 0)) AS remaining
         FROM streak_freeze_quota
         WHERE user_id = $1 AND year_month = $2"
    )
    .bind(user.id)
    .bind(year_month)
    .bind(STREAK_FREEZE_MONTHLY_QUOTA)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(StreakFreezeQuota {
        year_month,
        used_count: 0,
        remaining: STREAK_FREEZE_MONTHLY_QUOTA,
    });

    let purchased: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM streak_freezes WHERE user_id = $1 AND source = 'purchased' AND applied = false"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = MucTieuTemplate {
        user: Some(user),
        active_page: "muc-tieu".into(),
        goals,
        streak_freeze_quota: STREAK_FREEZE_MONTHLY_QUOTA,
        streak_freeze_used: quota.used_count,
        streak_freeze_purchased: purchased,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (muc-tieu): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /khong-gian/muc-tieu/tao — Tạo mục tiêu mới.
pub async fn muc_tieu_create(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CreateGoalForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate
    let valid_types = [
        "daily_niem",
        "weekly_niem",
        "monthly_niem",
        "daily_read",
        "weekly_read",
        "daily_thien",
        "custom",
    ];
    if !valid_types.contains(&form.goal_type.as_str()) {
        return render_muc_tieu_error(&state, &user, "Loại mục tiêu không hợp lệ.").await;
    }

    if form.target_value <= 0 || form.target_value > 100_000 {
        return render_muc_tieu_error(&state, &user, "Mục tiêu số phải từ 1 đến 100.000.").await;
    }

    let title = form.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 200 {
        return render_muc_tieu_error(&state, &user, "Tiêu đề mục tiêu không được để trống và tối đa 200 ký tự.").await;
    }

    let deadline = form
        .deadline
        .as_ref()
        .filter(|s| !s.is_empty())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let valid_units = ["count", "chapter", "minute"];
    let unit = if valid_units.contains(&form.target_unit.as_str()) {
        form.target_unit.clone()
    } else {
        "count".to_string()
    };

    let result = sqlx::query(
        "INSERT INTO tu_hoc_goals (user_id, goal_type, target_value, target_unit, title, status, deadline)
         VALUES ($1, $2, $3, $4, $5, 'active', $6)"
    )
    .bind(user.id)
    .bind(&form.goal_type)
    .bind(form.target_value)
    .bind(&unit)
    .bind(&title)
    .bind(deadline)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi INSERT tu_hoc_goals: {e}");
        return render_muc_tieu_error(&state, &user, "Không thể tạo mục tiêu. Vui lòng thử lại.").await;
    }

    render_muc_tieu_success(&state, &user, "Đã tạo mục tiêu tu học. Cố lên! 🪷").await
}

/// POST /khong-gian/muc-tieu/{id}/xoa — Xoá mục tiêu.
pub async fn muc_tieu_delete(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(goal_id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let _ = sqlx::query("DELETE FROM tu_hoc_goals WHERE id = $1 AND user_id = $2")
        .bind(goal_id)
        .bind(user.id)
        .execute(&state.pool)
        .await;

    Redirect::to("/khong-gian/muc-tieu").into_response()
}

/// POST /khong-gian/muc-tieu/{id}/hoan-thanh — Đánh dấu hoàn thành.
pub async fn muc_tieu_complete(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(goal_id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let _ = sqlx::query(
        "UPDATE tu_hoc_goals SET status = 'completed', updated_at = NOW()
         WHERE id = $1 AND user_id = $2"
    )
    .bind(goal_id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    Redirect::to("/khong-gian/muc-tieu").into_response()
}

/// POST /api/streak-freeze/mua — Mua 1 streak freeze với 100 A.
pub async fn api_streak_freeze_buy(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    // Check A balance
    let balance: Option<(i64,)> = sqlx::query_as("SELECT a_balance FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    let Some((current_a,)) = balance else {
        return Json(serde_json::json!({
            "success": false, "message": "Không tìm thấy user."
        }))
        .into_response();
    };

    if current_a < STREAK_FREEZE_COST_A {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("Không đủ A. Cần {} A, bạn có {} A.", STREAK_FREEZE_COST_A, current_a)
        }))
        .into_response();
    }

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ Lỗi tx streak freeze mua: {e}");
            return Json(serde_json::json!({
                "success": false, "message": "Lỗi hệ thống."
            }))
            .into_response();
        }
    };

    let new_balance = current_a - STREAK_FREEZE_COST_A;

    let _ = sqlx::query("UPDATE users SET a_balance = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_balance)
        .bind(user.id)
        .execute(&mut *tx)
        .await;

    let _ = sqlx::query(
        "INSERT INTO balance_transactions (user_id, currency, amount, balance_after, tx_type, description)
         VALUES ($1, 'a', $2, $3, 'other', 'Mua streak freeze')"
    )
    .bind(user.id)
    .bind(-STREAK_FREEZE_COST_A)
    .bind(new_balance)
    .execute(&mut *tx)
    .await;

    let _ = sqlx::query(
        "INSERT INTO streak_freezes (user_id, freeze_date, source, cost_a)
         VALUES ($1, CURRENT_DATE, 'purchased', $2)"
    )
    .bind(user.id)
    .bind(STREAK_FREEZE_COST_A)
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        log::error!("❌ Lỗi commit tx streak freeze mua: {e}");
        return Json(serde_json::json!({
            "success": false, "message": "Không thể mua streak freeze."
        }))
        .into_response();
    }

    Json(serde_json::json!({
        "success": true,
        "new_balance_a": new_balance,
        "message": format!("Đã mua 1 streak freeze với {} A.", STREAK_FREEZE_COST_A)
    }))
    .into_response()
}

/// GET /api/streak-freeze/quota — Quota tháng này.
pub async fn api_streak_freeze_quota(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let now = Utc::now();
    let year_month = now.year() * 100 + now.month() as i32;

    let used: i16 = sqlx::query_scalar(
        "SELECT COALESCE(used_count, 0)::SMALLINT FROM streak_freeze_quota
         WHERE user_id = $1 AND year_month = $2"
    )
    .bind(user.id)
    .bind(year_month)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let purchased: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM streak_freezes
         WHERE user_id = $1 AND source = 'purchased' AND applied = false"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    Json(serde_json::json!({
        "success": true,
        "year_month": year_month,
        "monthly_quota": STREAK_FREEZE_MONTHLY_QUOTA,
        "monthly_used": used,
        "monthly_remaining": STREAK_FREEZE_MONTHLY_QUOTA - used,
        "purchased_available": purchased,
        "purchased_cost_a": STREAK_FREEZE_COST_A
    }))
    .into_response()
}

// ─── Helpers ──────────────────────────────────────────────────────────────

async fn render_muc_tieu_error(state: &AppState, user: &User, msg: &str) -> Response {
    let goals: Vec<TuHocGoal> = sqlx::query_as(
        "SELECT id, user_id, goal_type, target_value, target_unit, title, status,
                deadline, current_value, last_reset_at, created_at
         FROM tu_hoc_goals
         WHERE user_id = $1 AND status IN ('active', 'completed')
         ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = MucTieuTemplate {
        user: Some(user.clone()),
        active_page: "muc-tieu".into(),
        goals,
        streak_freeze_quota: STREAK_FREEZE_MONTHLY_QUOTA,
        streak_freeze_used: 0,
        streak_freeze_purchased: 0,
        error: Some(msg.to_string()),
        success: None,
    }
    .render()
    .unwrap_or_else(|e| format!("Lỗi render: {e}"));
    Html(html).into_response()
}

async fn render_muc_tieu_success(state: &AppState, user: &User, msg: &str) -> Response {
    let goals: Vec<TuHocGoal> = sqlx::query_as(
        "SELECT id, user_id, goal_type, target_value, target_unit, title, status,
                deadline, current_value, last_reset_at, created_at
         FROM tu_hoc_goals
         WHERE user_id = $1 AND status IN ('active', 'completed')
         ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = MucTieuTemplate {
        user: Some(user.clone()),
        active_page: "muc-tieu".into(),
        goals,
        streak_freeze_quota: STREAK_FREEZE_MONTHLY_QUOTA,
        streak_freeze_used: 0,
        streak_freeze_purchased: 0,
        error: None,
        success: Some(msg.to_string()),
    }
    .render()
    .unwrap_or_else(|e| format!("Lỗi render: {e}"));
    Html(html).into_response()
}

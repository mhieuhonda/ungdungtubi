//! Handlers cho Quỹ Từ Bi — Giai đoạn 15 (v0.9.11).
//!
//! Theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục VI:
//!   * Quỹ Từ Bi là quỹ chung của toàn bộ cộng đồng
//!   * Nguồn: đóng góp thành viên, mạnh thường quân, lợi nhuận dự án
//!   * Nguyên tắc: Công khai · Minh bạch · Cùng quản lý · Cùng phát triển
//!
//! Routes:
//!   - GET  /quy-tu-bi                — Trang Quỹ Từ Bi (stats + donations + form)
//!   - POST /quy-tu-bi/dong-gop       — Đóng góp K vào quỹ
//!   - GET  /api/quy-tu-bi/stats      — JSON tổng quan

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::quy_tu_bi::{
    DonationForm, DonationType, FundDonationWithUser, FundExpense, FundSummary, TopDonor,
};
use crate::models::user::User;

/// Template cho trang /quy-tu-bi.
#[derive(Template)]
#[template(path = "quy-tu-bi/index.html")]
pub struct QuyTuBiTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub summary: FundSummary,
    /// Tổng K đang lưu thông trong hệ thống (sum users.k_balance).
    pub total_k_in_system: i64,
    pub total_a_in_system: i64,
    pub total_i_in_system: i64,
    pub recent_donations: Vec<FundDonationWithUser>,
    pub top_donors: Vec<TopDonor>,
    pub recent_expenses: Vec<FundExpense>,
    /// Số dư K của user (nếu đã đăng nhập) — để hiển thị trong form.
    pub user_k_balance: Option<i64>,
    /// Thông báo lỗi / thành công từ form submit.
    pub error: Option<String>,
    pub success: Option<String>,
}

/// GET /quy-tu-bi — Trang Quỹ Từ Bi.
pub async fn quy_tu_bi_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    render_quy_tu_bi_page(&state.pool, jar, None, None).await
}

/// POST /quy-tu-bi/dong-gop — Đóng góp K vào quỹ.
pub async fn quy_tu_bi_dong_gop(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<DonationForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/quy-tu-bi").into_response();
    };

    // Validate form
    if let Err(e) = form.validate() {
        return render_quy_tu_bi_page(&state.pool, jar, Some(e), None).await;
    }

    // Validate user balance
    if user.k_balance < form.amount_k {
        return render_quy_tu_bi_page(
            &state.pool,
            jar,
            Some(format!(
                "Số dư K không đủ. Bạn có {} K, đang cố đóng góp {} K.",
                user.k_balance, form.amount_k
            )),
            None,
        )
        .await;
    }

    // Transaction: deduct K from user + insert donation
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("❌ quy_tu_bi_dong_gop: begin tx fail: {e}");
            return render_quy_tu_bi_page(
                &state.pool,
                jar,
                Some("Lỗi kết nối DB — vui lòng thử lại.".into()),
                None,
            )
            .await;
        }
    };

    // 1. Trừ K từ user
    let deduct = sqlx::query(
        "UPDATE users SET k_balance = k_balance - $1, updated_at = NOW() WHERE id = $2 AND k_balance >= $1",
    )
    .bind(form.amount_k)
    .bind(user.id)
    .execute(&mut *tx)
    .await;

    match deduct {
        Ok(res) if res.rows_affected() == 1 => {}
        Ok(_) => {
            // rows_affected = 0 → không đủ K (race condition)
            let _ = tx.rollback().await;
            return render_quy_tu_bi_page(
                &state.pool,
                jar,
                Some("Số dư K không đủ (có thể vừa có giao dịch khác). Vui lòng thử lại.".into()),
                None,
            )
            .await;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            log::error!("❌ quy_tu_bi_dong_gop: deduct K fail: {e}");
            return render_quy_tu_bi_page(
                &state.pool,
                jar,
                Some("Lỗi khi trừ K. Vui lòng thử lại.".into()),
                None,
            )
            .await;
        }
    }

    // 2. Insert donation record
    let msg = form
        .message
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let insert = sqlx::query(
        "INSERT INTO fund_donations (user_id, amount_k, donation_type, message, is_anonymous, status)
         VALUES ($1, $2, $3, $4, $5, 'completed')",
    )
    .bind(user.id)
    .bind(form.amount_k)
    .bind(&form.donation_type)
    .bind(&msg)
    .bind(form.anonymous())
    .execute(&mut *tx)
    .await;

    if let Err(e) = insert {
        let _ = tx.rollback().await;
        log::error!("❌ quy_tu_bi_dong_gop: insert donation fail: {e}");
        return render_quy_tu_bi_page(
            &state.pool,
            jar,
            Some("Lỗi khi ghi đóng góp. K chưa bị trừ, vui lòng thử lại.".into()),
            None,
        )
        .await;
    }

    // 3. Commit
    if let Err(e) = tx.commit().await {
        log::error!("❌ quy_tu_bi_dong_gop: commit fail: {e}");
        return render_quy_tu_bi_page(
            &state.pool,
            jar,
            Some("Lỗi khi hoàn tất giao dịch. Vui lòng kiểm tra số dư và thử lại nếu cần.".into()),
            None,
        )
        .await;
    }

    log::info!(
        "✅ Donation: user={} amount={}K type={}",
        user.id,
        form.amount_k,
        form.donation_type
    );

    // Ghi nhận notification cho admins (best-effort, không fail nếu lỗi)
    let _ = notify_admins_of_donation(&state.pool, user.id, form.amount_k, &form.donation_type).await;

    render_quy_tu_bi_page(
        &state.pool,
        jar,
        None,
        Some(format!(
            "🪷 Cảm ơn đạo hữu đã đóng góp {} K vào {}. Nguyện công đức vô lượng.",
            form.amount_k,
            DonationType::from_str(&form.donation_type).label()
        )),
    )
    .await
}

/// GET /api/quy-tu-bi/stats — JSON tổng quan.
pub async fn quy_tu_bi_stats_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let _user = get_user_from_session(&state.pool, &jar).await;

    let summary = fetch_summary(&state.pool).await;
    let total_k = fetch_total_k_in_system(&state.pool).await;
    let total_a = fetch_total_a_in_system(&state.pool).await;
    let total_i = fetch_total_i_in_system(&state.pool).await;

    Json(serde_json::json!({
        "summary": {
            "total_income_k": summary.total_income_k,
            "total_expense_k": summary.total_expense_k,
            "balance_k": summary.balance_k,
            "total_donations_count": summary.total_donations_count,
            "unique_donors": summary.unique_donors,
        },
        "by_type": {
            "general": summary.fund_general,
            "sach": summary.fund_sach,
            "tu": summary.fund_tu,
            "qua": summary.fund_qua,
            "thien_nguyen": summary.fund_thien_nguyen,
        },
        "system": {
            "total_k_in_system": total_k,
            "total_a_in_system": total_a,
            "total_i_in_system": total_i,
        },
    }))
    .into_response()
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Render trang Quỹ Từ Bi với error/success tùy chọn.
async fn render_quy_tu_bi_page(
    pool: &PgPool,
    jar: CookieJar,
    error: Option<String>,
    success: Option<String>,
) -> Response {
    let user = get_user_from_session(pool, &jar).await;
    let user_k_balance = user.as_ref().map(|u| u.k_balance);

    let summary = fetch_summary(pool).await;
    let total_k_in_system = fetch_total_k_in_system(pool).await;
    let total_a_in_system = fetch_total_a_in_system(pool).await;
    let total_i_in_system = fetch_total_i_in_system(pool).await;
    let recent_donations = fetch_recent_donations(pool, 20).await;
    let top_donors = fetch_top_donors(pool, 10).await;
    let recent_expenses = fetch_recent_expenses(pool, 10).await;

    let html = QuyTuBiTemplate {
        user,
        active_page: "quy_tu_bi".into(),
        summary,
        total_k_in_system,
        total_a_in_system,
        total_i_in_system,
        recent_donations,
        top_donors,
        recent_expenses,
        user_k_balance,
        error,
        success,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (quy-tu-bi): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Fetch tổng quan quỹ từ view v_fund_summary.
/// Trả về defaults nếu view chưa tồn tại (migration 016 chưa chạy).
async fn fetch_summary(pool: &PgPool) -> FundSummary {
    sqlx::query_as::<_, FundSummary>(
        "SELECT
            COALESCE(total_income_k, 0)::BIGINT AS total_income_k,
            COALESCE(total_expense_k, 0)::BIGINT AS total_expense_k,
            COALESCE(balance_k, 0)::BIGINT AS balance_k,
            COALESCE(total_donations_count, 0)::BIGINT AS total_donations_count,
            COALESCE(unique_donors, 0)::BIGINT AS unique_donors,
            COALESCE(fund_general, 0)::BIGINT AS fund_general,
            COALESCE(fund_sach, 0)::BIGINT AS fund_sach,
            COALESCE(fund_tu, 0)::BIGINT AS fund_tu,
            COALESCE(fund_qua, 0)::BIGINT AS fund_qua,
            COALESCE(fund_thien_nguyen, 0)::BIGINT AS fund_thien_nguyen
         FROM v_fund_summary LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}

async fn fetch_total_k_in_system(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(k_balance), 0)::BIGINT FROM users WHERE is_active = true"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

async fn fetch_total_a_in_system(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(a_balance), 0)::BIGINT FROM users WHERE is_active = true"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

async fn fetch_total_i_in_system(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(i_balance), 0)::BIGINT FROM users WHERE is_active = true"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

/// Lấy N đóng góp gần nhất (join với users để lấy display_name + avatar).
async fn fetch_recent_donations(pool: &PgPool, limit: i64) -> Vec<FundDonationWithUser> {
    // v0.9.22: Fix SQL injection — bind limit as $1
    let sql =
        "SELECT d.id, d.user_id, d.amount_k, d.donation_type, d.message,
                d.is_anonymous, d.status, d.created_at,
                CASE WHEN d.is_anonymous THEN NULL ELSE u.display_name END AS display_name,
                CASE WHEN d.is_anonymous THEN NULL ELSE u.avatar_url END AS avatar_url
         FROM fund_donations d
         LEFT JOIN users u ON u.id = d.user_id
         WHERE d.status = 'completed'
         ORDER BY d.created_at DESC
         LIMIT $1"
    ;
    sqlx::query_as::<_, FundDonationWithUser>(sql)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| {
            // Bảng fund_donations có thể chưa tồn tại (migration 016 chưa chạy)
            log::debug!("fetch_recent_donations: {e}");
            vec![]
        })
}

/// Lấy top N nhà hảo tâm.
async fn fetch_top_donors(pool: &PgPool, limit: i64) -> Vec<TopDonor> {
    // v0.9.22: Fix SQL injection — bind limit as $1
    let sql =
        "SELECT d.user_id, u.display_name, u.avatar_url,
                SUM(d.amount_k)::BIGINT AS total_k,
                COUNT(*)::BIGINT AS donation_count
         FROM fund_donations d
         JOIN users u ON u.id = d.user_id
         WHERE d.status = 'completed' AND d.is_anonymous = false AND u.is_active = true
         GROUP BY d.user_id, u.display_name, u.avatar_url
         ORDER BY total_k DESC
         LIMIT $1"
    ;
    sqlx::query_as::<_, TopDonor>(sql)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| {
            log::debug!("fetch_top_donors: {e}");
            vec![]
        })
}

/// Lấy N khoản chi tiêu gần nhất.
async fn fetch_recent_expenses(pool: &PgPool, limit: i64) -> Vec<FundExpense> {
    // v0.9.22: Fix SQL injection — bind limit as $1
    let sql =
        "SELECT id, amount_k, expense_type, description, receipt_url,
                spent_at, approved_by, is_public, created_at
         FROM fund_expenses
         WHERE is_public = true
         ORDER BY spent_at DESC, created_at DESC
         LIMIT $1"
    ;
    sqlx::query_as::<_, FundExpense>(sql)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| {
            log::debug!("fetch_recent_expenses: {e}");
            vec![]
        })
}

/// Gửi notification cho admins khi có donation mới (best-effort).
async fn notify_admins_of_donation(
    pool: &PgPool,
    donor_id: Uuid,
    amount_k: i64,
    donation_type: &str,
) -> Result<(), sqlx::Error> {
    // Lấy danh sách admin users
    let admin_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE role IN ('admin_ky_thuat', 'admin_quan_li', 'admin_cong_dong') AND is_active = true"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let payload = serde_json::json!({
        "type": "fund_donation",
        "donor_id": donor_id.to_string(),
        "amount_k": amount_k,
        "donation_type": donation_type,
    });

    for admin_id in admin_ids {
        let _ = sqlx::query(
            "INSERT INTO notifications (user_id, type, actor_id, payload)
             VALUES ($1, 'system', $2, $3)"
        )
        .bind(admin_id)
        .bind(donor_id)
        .bind(&payload)
        .execute(pool)
        .await;
    }

    Ok(())
}

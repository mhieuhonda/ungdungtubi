//! Handlers cho Không Gian Cá Nhân — Giai đoạn 13 (v0.9.9).
//!
//! Routes:
//!   - GET  /khong-gian                  — Trang Không Gian cá nhân (Niệm Phật + Tượng Phật + Nhật ký)
//!   - POST /api/niem-phat               — Tăng 1 lần niệm Phật (+1 A)
//!   - POST /tuong-phat/cau-nguyen       — Tạo vow Cầu Nguyện (+1 I)
//!   - POST /tuong-phat/sam-hoi          — Tạo vow Sám Hối (+2 I)
//!   - POST /tuong-phat/hoi-huong        — Tạo vow Hồi Hướng (+3 I)
//!   - GET  /api/khong-gian/stats        — JSON stats cho dashboard

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::Utc;
use serde::Serialize;
use sqlx::PgPool;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::khong_gian::{
    BuddhaVowForm, DailyNiem, KhongGianStats, PublicVow, VowType,
};
use crate::models::user::User;

/// Template cho trang /khong-gian.
#[derive(Template)]
#[template(path = "khong-gian/index.html")]
pub struct KhongGianTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub stats: KhongGianStats,
    pub daily_niem: Vec<DailyNiem>,
    /// Giá trị niem_count lớn nhất trong `daily_niem` — dùng để scale bar chart heights.
    pub daily_max_niem: i64,
    pub public_vows: Vec<PublicVow>,
    pub now: chrono::DateTime<Utc>,
}

/// GET /khong-gian — Trang Không Gian cá nhân.
pub async fn khong_gian_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian").into_response();
    };

    let stats = fetch_khong_gian_stats(&state.pool, user.id).await;
    let daily_niem = fetch_daily_niem(&state.pool, user.id, 7).await;
    let daily_max_niem = daily_niem.iter().map(|d| d.niem_count).max().unwrap_or(1);
    let public_vows = fetch_public_vows(&state.pool, 20).await;

    let html = KhongGianTemplate {
        user: Some(user),
        active_page: "khong_gian".into(),
        stats,
        daily_niem,
        daily_max_niem,
        public_vows,
        now: Utc::now(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (khong-gian): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /api/niem-phat — Tăng 1 lần niệm Phật (+1 A).
///
/// Dùng HTMX: trả về HTML partial cập nhật counter.
pub async fn niem_phat(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Transaction: upsert practice_logs + increment a_balance.
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("❌ niem_phat: begin tx fail: {e}");
            return Html(
                r#"<span class="text-red-600 text-sm">Lỗi kết nối DB — vui lòng thử lại.</span>"#
            )
            .into_response();
        }
    };

    // Upsert practice_logs (1 row/user/day).
    let _ = sqlx::query(
        "INSERT INTO practice_logs (user_id, log_date, niem_count, last_niem_at)
         VALUES ($1, CURRENT_DATE, 1, NOW())
         ON CONFLICT (user_id, log_date)
         DO UPDATE SET niem_count = practice_logs.niem_count + 1,
                       last_niem_at = NOW()",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await;

    // Increment a_balance.
    let new_a: i64 = match sqlx::query_scalar(
        "UPDATE users SET a_balance = a_balance + 1, updated_at = NOW()
         WHERE id = $1 RETURNING a_balance",
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            log::error!("❌ niem_phat: update a_balance fail: {e}");
            let _ = tx.rollback().await;
            return Html(
                r#"<span class="text-red-600 text-sm">Lỗi ghi nhận — vui lòng thử lại.</span>"#
            )
            .into_response();
        }
    };

    if let Err(e) = tx.commit().await {
        log::error!("❌ niem_phat: commit fail: {e}");
        return Html(
            r#"<span class="text-red-600 text-sm">Lỗi commit — vui lòng thử lại.</span>"#
        )
        .into_response();
    }

    // Return HTMX partial: updated counter + small celebration.
    let html = format!(
        r#"<div id="niem-counter" hx-target="this" hx-swap="outerHTML">
            <div class="text-5xl md:text-6xl font-bold text-tubi-800 tabular-nums">{new_a}</div>
            <div class="text-xs text-gray-500 mt-1">Niệm Lực A</div>
        </div>"#
    );
    Html(html).into_response()
}

/// POST /tuong-phat/cau-nguyen — Tạo vow Cầu Nguyện (+1 I).
pub async fn tuong_phat_cau_nguyen(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<BuddhaVowForm>,
) -> Response {
    create_vow(&state, &jar, form, VowType::Prayer).await
}

/// POST /tuong-phat/sam-hoi — Tạo vow Sám Hối (+2 I).
pub async fn tuong_phat_sam_hoi(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<BuddhaVowForm>,
) -> Response {
    create_vow(&state, &jar, form, VowType::Repentance).await
}

/// POST /tuong-phat/hoi-huong — Tạo vow Hồi Hướng (+3 I).
pub async fn tuong_phat_hoi_huong(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<BuddhaVowForm>,
) -> Response {
    create_vow(&state, &jar, form, VowType::Dedication).await
}

/// Helper: tạo vow + thưởng I (Nguyên lực) trong transaction.
async fn create_vow(
    state: &AppState,
    jar: &CookieJar,
    form: BuddhaVowForm,
    expected_type: VowType,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian").into_response();
    };

    // Validate form (vow_type phải khớp endpoint, content 10–2000 ký tự).
    let (vow_type, content) = match form.validate() {
        Some((vt, c)) if vt.db_value() == expected_type.db_value() => (vt, c),
        _ => {
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Nội dung không hợp lệ. Vui lòng viết từ 10–2000 ký tự.
                </div>"#,
            )
            .into_response();
        }
    };
    let is_public = form.is_public_bool();
    let i_reward = vow_type.i_reward();

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("❌ create_vow: begin tx fail: {e}");
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi kết nối DB — vui lòng thử lại.
                </div>"#,
            )
            .into_response();
        }
    };

    let _ = sqlx::query(
        "INSERT INTO buddha_vows (user_id, vow_type, content, is_public)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user.id)
    .bind(vow_type.db_value())
    .bind(&content)
    .bind(is_public)
    .execute(&mut *tx)
    .await;

    let new_i: i64 = match sqlx::query_scalar(
        "UPDATE users SET i_balance = i_balance + $1, updated_at = NOW()
         WHERE id = $2 RETURNING i_balance",
    )
    .bind(i_reward)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            log::error!("❌ create_vow: update i_balance fail: {e}");
            let _ = tx.rollback().await;
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi ghi nhận — vui lòng thử lại.
                </div>"#,
            )
            .into_response();
        }
    };

    if let Err(e) = tx.commit().await {
        log::error!("❌ create_vow: commit fail: {e}");
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Lỗi commit — vui lòng thử lại.
            </div>"#,
        )
        .into_response();
    }

    // Return HTMX partial: success message + new I balance.
    let icon = vow_type.icon();
    let label = vow_type.display();
    let color = vow_type.color();
    let html = format!(
        r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
            ✅ {icon} <strong>{label}</strong> đã được ghi nhận. Bạn nhận được <strong>+{i_reward} I</strong> (Nguyên lực).
            <br>Tổng I hiện tại: <strong>{new_i}</strong>
        </div>"#
    );
    // Note: không hiển thị content trong response để giữ riêng tư nếu is_public=false.
    let _ = color; // color available for future UI enhancement
    Html(html).into_response()
}

/// JSON response cho /api/khong-gian/stats.
#[derive(Serialize)]
struct KhongGianStatsResponse {
    today_niem: i64,
    total_niem: i64,
    streak_days: i32,
    total_vows: i64,
    a_balance: i64,
    k_balance: i64,
    i_balance: i64,
}

/// GET /api/khong-gian/stats — JSON stats cho dashboard.
pub async fn khong_gian_stats_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"}))
            .into_response();
    };

    let stats = fetch_khong_gian_stats(&state.pool, user.id).await;
    let resp = KhongGianStatsResponse {
        today_niem: stats.today_niem,
        total_niem: stats.total_niem,
        streak_days: stats.streak_days,
        total_vows: stats.total_vows,
        a_balance: user.a_balance,
        k_balance: user.k_balance,
        i_balance: user.i_balance,
    };
    Json(resp).into_response()
}

// ─── Internal helpers ───────────────────────────────────────────────────────

/// Lấy stats Không Gian cho user.
async fn fetch_khong_gian_stats(pool: &PgPool, user_id: uuid::Uuid) -> KhongGianStats {
    // Today's niem count.
    let today_niem = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(niem_count, 0) FROM practice_logs
         WHERE user_id = $1 AND log_date = CURRENT_DATE",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    // Total niem count (all-time).
    let total_niem = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(niem_count), 0) FROM practice_logs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Streak: số ngày liên tiếp có niem_count > 0, tính từ today backwards.
    let streak_days = compute_streak(pool, user_id).await;

    // Total vows.
    let total_vows = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM buddha_vows WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    KhongGianStats {
        today_niem,
        total_niem,
        streak_days,
        total_vows,
    }
}

/// Tính số ngày liên tiếp tu học (streak).
/// Bắt đầu từ today, lùi lại cho đến khi gặp ngày không có niệm.
async fn compute_streak(pool: &PgPool, user_id: uuid::Uuid) -> i32 {
    // Lấy 30 ngày gần nhất có practice_log.
    let rows: Vec<DailyNiem> = match sqlx::query_as::<_, DailyNiem>(
        "SELECT log_date, niem_count FROM practice_logs
         WHERE user_id = $1 AND log_date >= CURRENT_DATE - INTERVAL '30 days'
         ORDER BY log_date DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => return 0,
    };

    if rows.is_empty() {
        return 0;
    }

    // Streak logic: nếu today có niệm, đếm lùi; nếu today chưa niệm nhưng yesterday có, đếm từ yesterday.
    let today = chrono::Local::now().date_naive();
    let mut streak = 0i32;
    let mut cursor = rows[0].log_date;

    // Nếu record mới nhất là today hoặc yesterday → bắt đầu đếm.
    let days_diff = (today - cursor).num_days();
    if days_diff > 1 {
        return 0; // Streak đã đứt.
    }

    for row in &rows {
        if row.log_date == cursor && row.niem_count > 0 {
            streak += 1;
            cursor = cursor.pred_opt().unwrap_or(cursor);
        } else if row.log_date < cursor {
            // Gap detected.
            break;
        }
    }
    streak
}

/// Lấy N ngày gần nhất cho biểu đồ tu học.
async fn fetch_daily_niem(pool: &PgPool, user_id: uuid::Uuid, days: i32) -> Vec<DailyNiem> {
    sqlx::query_as::<_, DailyNiem>(
        "SELECT log_date, niem_count FROM practice_logs
         WHERE user_id = $1
           AND log_date >= CURRENT_DATE - ($2 || ' days')::INTERVAL
         ORDER BY log_date ASC",
    )
    .bind(user_id)
    .bind(days.to_string())
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Lấy N vow công khai gần nhất cho bảng Kính Nguyện.
async fn fetch_public_vows(pool: &PgPool, limit: i64) -> Vec<PublicVow> {
    sqlx::query_as::<_, PublicVow>(
        "SELECT v.id, v.vow_type, v.content, v.created_at,
                u.display_name AS author_name,
                u.avatar_url   AS author_avatar
         FROM buddha_vows v
         JOIN users u ON u.id = v.user_id
         WHERE v.is_public = true AND u.is_active = true
         ORDER BY v.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

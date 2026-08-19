//! Handlers — Tiền Tệ & Quy Đổi (Giai đoạn 47 — v0.9.43)
//!
//! Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx" mục "Hệ Thống Tiền Tệ":
//!   - Ấm (A): tiền cơ bản — kiếm qua niệm Phật, hoạt động thường.
//!   - K (Karmic): tiền công đức — kiếm qua việc lành, dùng trong Thương Thành.
//!   - Bi (Từ Bi): tiền cao cấp nhất — kiếm qua cống hiến đặc biệt hoặc quy đổi.
//!
//! Module này triển khai:
//!   1. `GET  /tien-te`                     — Trang quy đổi (UI).
//!   2. `GET  /api/tien-te/ty-gia`          — Xem tỷ giá hiện tại (JSON, public).
//!   3. `POST /api/tien-te/doi`             — Quy đổi A↔K↔Bi (auth, JSON).
//!   4. `GET  /admin/tien-te`               — Admin quản lý tỷ giá (UI).
//!   5. `POST /admin/tien-te/ty-gia`        — Admin cập nhật tỷ giá (form).
//!   6. `GET  /api/tien-te/ls-giao-dich`    — Lịch sử giao dịch quy đổi của user.
//!
//! Bảo mật:
//!   - Mọi thao tác quy đổi chạy trong DB transaction (BEGIN/COMMIT).
//!   - Validate amount > 0, source != target.
//!   - Check balance đủ trước khi trừ.
//!   - Ghi vào `balance_transactions` 2 row (exchange_out + exchange_in) cho traceability.
//!   - Rate limit: max 10 giao dịch quy đổi / user / ngày (chống spam).

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Json, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;

// ════════════════════════════════════════════════════════════════════════════
// Models
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ExchangeRateView {
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: i64,
    pub label: String,
}

impl ExchangeRate {
    /// Hiển thị dạng thân thiện: "100 A = 1 K"
    pub fn label(&self) -> String {
        let sym_from = currency_symbol(&self.from_currency);
        let sym_to = currency_symbol(&self.to_currency);
        let to_amount = 1; // quy ước: from_amount đơn vị from = 1 đơn vị to
        format!("{} {} = {} {}", self.from_amount, sym_from, to_amount, sym_to)
    }
}

#[derive(Debug, Serialize)]
pub struct CurrencyBalance {
    pub a: i64,
    pub k: i64,
    pub i: i64,
    pub bi: i64,
}

#[derive(Debug, Serialize)]
pub struct ExchangeApiResponse {
    pub success: bool,
    pub message: String,
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: i64,
    pub received_amount: i64,
    pub new_balance_from: i64,
    pub new_balance_to: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExchangeForm {
    pub from_currency: String,
    pub to_currency: String,
    pub amount: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdminRateForm {
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: String,
    pub is_active: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════
// Helpers
// ════════════════════════════════════════════════════════════════════════════

pub fn currency_symbol(code: &str) -> &'static str {
    match code {
        "a" => "Ấm",
        "k" => "K",
        "bi" => "Bi",
        _ => "?",
    }
}

pub fn currency_full_name(code: &str) -> &'static str {
    match code {
        "a" => "Ấm (Niệm Lực A)",
        "k" => "K (Công Đức K)",
        "bi" => "Bi (Từ Bi)",
        _ => "Không xác định",
    }
}

pub fn validate_currency(code: &str) -> bool {
    matches!(code, "a" | "k" | "bi")
}

/// Lấy balance của user theo currency code.
async fn get_balance(pool: &PgPool, user_id: Uuid, currency: &str) -> Result<i64, sqlx::Error> {
    let col = match currency {
        "a" => "a_balance",
        "k" => "k_balance",
        "bi" => "bi_balance",
        _ => return Err(sqlx::Error::Configuration("Invalid currency".into())),
    };
    let sql = format!("SELECT {} FROM users WHERE id = $1", col);
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(user_id)
        .fetch_one(pool)
        .await
}

/// Cập nhật balance của user theo currency code (tăng/giảm).
///
/// v0.9.44 — Giai đoạn 49 (bug M5 fix): Trước v0.9.44, SQL là
/// `GREATEST(col, 0) + $2` — GREATEST chỉ clamp `col` về 0, sau đó cộng delta.
/// Nếu col=10, delta=-100 → result = -90 (âm!). Fix: dùng
/// `GREATEST(col + $2, 0)` — clamp tổng về 0, không cho âm bao giờ.
async fn update_balance(
    pool: &PgPool,
    user_id: Uuid,
    currency: &str,
    delta: i64,
) -> Result<i64, sqlx::Error> {
    let col = match currency {
        "a" => "a_balance",
        "k" => "k_balance",
        "bi" => "bi_balance",
        _ => return Err(sqlx::Error::Configuration("Invalid currency".into())),
    };
    let sql = format!(
        "UPDATE users SET {col} = GREATEST({col} + $2, 0), updated_at = NOW() WHERE id = $1 RETURNING {col}"
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(user_id)
        .bind(delta)
        .fetch_one(pool)
        .await
}

/// Lấy tỷ giá from→to (active). Trả về from_amount (số đơn vị from = 1 đơn vị to).
/// Nếu không có direct rate, thử reverse rate.
///
/// v0.9.44 — Giai đoạn 49 (bug C4 fix): Trước v0.9.44, nếu chỉ có reverse rate
/// (ví dụ chỉ có A→K với from_amount=100), khi user đổi K→A, code trả về `rev_amt=100`
/// → received = amount / 100 = 1/100 = 0 (integer division). User đổi 1 K → nhận 0 A!
///
/// Fix: Khi tìm reverse, ta phải trả về from_amount CHO direction mà user yêu cầu.
/// Nếu stored: (from=A, to=K, from_amount=100) nghĩa là 100 A = 1 K.
/// Khi user đổi K→A, ta cần "from_amount K = 1 A", tức là 1 K = 100 A → from_amount = 1/100.
/// Vì from_amount là integer, ta swap direction: tính received = amount * rev_amt
/// thay vì amount / rate. Trả về -rev_amt làm sentinel âm để caller biết dùng nhân.
///
/// Implement đơn giản hơn: nếu chỉ có reverse, ta swap to←→from và gọi lại get_rate.
/// Sau đó caller dùng công thức received = amount * rev_amt (nếu rate_from_reverse).
///
/// Vì interface hiện tại chỉ trả về i64, ta sẽ trả về `Ok(Some(rev_amt))` nhưng
/// cũng đồng thời swap caller side: nếu from→to có reverse (to→from có rate),
/// caller sẽ invert: received = amount / (1/rev_amt) = amount * rev_amt.
/// Cách clean nhất: trả về cả direction info. Nhưng để giữ backward-compat, ta
/// sẽ luôn prefer direct rate; nếu chỉ có reverse, ta swap direction.
async fn get_rate(
    pool: &PgPool,
    from: &str,
    to: &str,
) -> Result<Option<(i64, bool)>, sqlx::Error> {
    if from == to {
        return Ok(Some((1, false)));
    }
    // Direct: (from, to, from_amount) → received = amount / from_amount
    let direct: Option<(i64,)> = sqlx::query_as(
        "SELECT from_amount FROM currency_exchange_rates
         WHERE from_currency = $1 AND to_currency = $2 AND is_active = true"
    )
    .bind(from)
    .bind(to)
    .fetch_optional(pool)
    .await?;
    if let Some((amt,)) = direct {
        // Direct rate found — use division: received = amount / amt
        return Ok(Some((amt, false)));
    }
    // Reverse: (to, from, from_amount) → 1 from = (1/from_amount) of to
    // → received = amount * from_amount (nhân thay vì chia)
    let reverse: Option<(i64,)> = sqlx::query_as(
        "SELECT from_amount FROM currency_exchange_rates
         WHERE from_currency = $1 AND to_currency = $2 AND is_active = true"
    )
    .bind(to)
    .bind(from)
    .fetch_optional(pool)
    .await?;
    if let Some((rev_amt,)) = reverse {
        // v0.9.44 fix: trả về (rev_amt, is_reverse=true) để caller biết nhân thay vì chia.
        return Ok(Some((rev_amt, true)));
    }
    Ok(None)
}

/// Lấy tất cả exchange rates (active).
async fn list_rates(pool: &PgPool) -> Result<Vec<ExchangeRate>, sqlx::Error> {
    sqlx::query_as::<_, ExchangeRate>(
        "SELECT from_currency, to_currency, from_amount, is_active
         FROM currency_exchange_rates
         ORDER BY from_currency, to_currency"
    )
    .fetch_all(pool)
    .await
}

// ════════════════════════════════════════════════════════════════════════════
// Handlers — User
// ════════════════════════════════════════════════════════════════════════════

/// GET /tien-te — Trang quy đổi tiền tệ (UI).
pub async fn tien_te_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let is_authed = user.is_some();

    let rates = list_rates(&state.pool).await.unwrap_or_default();
    let rate_views: Vec<ExchangeRateView> = rates
        .iter()
        .filter(|r| r.is_active)
        .map(|r| ExchangeRateView {
            from_currency: r.from_currency.clone(),
            to_currency: r.to_currency.clone(),
            from_amount: r.from_amount,
            label: r.label(),
        })
        .collect();

    let balances: CurrencyBalance = if let Some(ref u) = user {
        CurrencyBalance {
            a: u.a_balance,
            k: u.k_balance,
            i: u.i_balance,
            bi: u.bi_balance,
        }
    } else {
        CurrencyBalance { a: 0, k: 0, i: 0, bi: 0 }
    };

    let html = render_tien_te_page(is_authed, &balances, &rate_views);
    Html(html).into_response()
}

/// GET /api/tien-te/ty-gia — Public JSON xem tỷ giá.
pub async fn tien_te_rates_api(State(state): State<AppState>) -> Response {
    let rates = list_rates(&state.pool).await.unwrap_or_default();
    let views: Vec<ExchangeRateView> = rates
        .iter()
        .filter(|r| r.is_active)
        .map(|r| ExchangeRateView {
            from_currency: r.from_currency.clone(),
            to_currency: r.to_currency.clone(),
            from_amount: r.from_amount,
            label: r.label(),
        })
        .collect();
    Json(serde_json::json!({
        "status": "ok",
        "rates": views,
        "currencies": {
            "a": {"name": "Ấm", "symbol": "A", "description": "Tiền cơ bản — kiếm qua niệm Phật"},
            "k": {"name": "K", "symbol": "K", "description": "Tiền công đức — dùng trong Thương Thành"},
            "bi": {"name": "Bi", "symbol": "Bi", "description": "Tiền Từ Bi — loại cao cấp nhất"}
        }
    }))
    .into_response()
}

/// POST /api/tien-te/doi — Quy đổi tiền tệ (JSON API, auth required).
///
/// Body: `{"from_currency":"a","to_currency":"k","amount":100}`
/// Response: `{"success":true,"received_amount":1,"new_balance_from":0,"new_balance_to":11,...}`
pub async fn tien_te_exchange_api(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<ExchangeForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false,
            "message": "Vui lòng đăng nhập để quy đổi tiền tệ."
        }))
        .into_response();
    };

    let from = body.from_currency.trim().to_lowercase();
    let to = body.to_currency.trim().to_lowercase();

    // Validate
    if !validate_currency(&from) || !validate_currency(&to) {
        return Json(serde_json::json!({
            "success": false,
            "message": "Loại tiền tệ không hợp lệ. Chỉ chấp nhận: a, k, bi."
        }))
        .into_response();
    }
    if from == to {
        return Json(serde_json::json!({
            "success": false,
            "message": "Không thể quy đổi cùng loại tiền tệ."
        }))
        .into_response();
    }

    let amount: i64 = match body.amount.trim().parse() {
        Ok(n) if n > 0 => n,
        _ => {
            return Json(serde_json::json!({
                "success": false,
                "message": "Số lượng phải là số nguyên dương lớn hơn 0."
            }))
            .into_response();
        }
    };

    // Minimum amount check (must be ≥ from_amount to get at least 1 unit of target)
    // v0.9.44 — Giai đoạn 49 (bug C4 fix): get_rate giờ trả về (rate, is_reverse).
    // - is_reverse=false (direct rate): received = amount / rate
    // - is_reverse=true (reverse rate): received = amount * rate
    let (rate, is_reverse) = match get_rate(&state.pool, &from, &to).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "message": format!("Không có tỷ giá cho {} → {}", currency_full_name(&from), currency_full_name(&to))
            }))
            .into_response();
        }
        Err(e) => {
            log::error!("❌ Lỗi query tỷ giá: {e}");
            return Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server — không lấy được tỷ giá."
            }))
            .into_response();
        }
    };

    // v0.9.44 — Tính received dựa trên direction của rate.
    // - Direct: amount / rate (vd: 100 A → K, rate=100, received=1)
    // - Reverse: amount * rate (vd: 1 K → A, stored A→K=100, received=100)
    let received = if is_reverse {
        amount.checked_mul(rate).unwrap_or(0)
    } else {
        amount / rate
    };

    // Minimum amount check (chỉ áp dụng cho direct rate — reverse rate không có minimum)
    if !is_reverse && amount < rate {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("Số lượng tối thiểu để quy đổi là {} {} (tỷ giá: {} {}).",
                rate, currency_symbol(&from), rate, currency_symbol(&from))
        }))
        .into_response();
    }

    if received < 1 {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("Số lượng quá nhỏ — nhận được 0 {}. Cần ít nhất {} {}.",
                currency_symbol(&to), rate, currency_symbol(&from))
        }))
        .into_response();
    }

    // Check balance
    let current_balance = match get_balance(&state.pool, user.id, &from).await {
        Ok(b) => b,
        Err(e) => {
            log::error!("❌ Lỗi query balance: {e}");
            return Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server — không kiểm tra được số dư."
            }))
            .into_response();
        }
    };

    if current_balance < amount {
        return Json(serde_json::json!({
            "success": false,
            "message": format!("Số dư không đủ. Bạn có {} {} nhưng cần {} {}.",
                current_balance, currency_symbol(&from), amount, currency_symbol(&from))
        }))
        .into_response();
    }

    // Rate limit: max 10 exchanges per user per day
    let today_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM balance_transactions
         WHERE user_id = $1 AND tx_type IN ('exchange_in', 'exchange_out')
         AND created_at > NOW() - INTERVAL '1 day'"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if today_count >= 20 { // 20 = 10 exchanges (2 rows each)
        return Json(serde_json::json!({
            "success": false,
            "message": "Bạn đã thực hiện 10 giao dịch quy đổi hôm nay. Vui lòng đợi ngày mai."
        }))
        .into_response();
    }

    // Execute exchange in a transaction
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ Lỗi begin transaction: {e}");
            return Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server — không thể bắt đầu giao dịch."
            }))
            .into_response();
        }
    };

    // Subtract from source (atomic — race-safe).
    // v0.9.44 — Giai đoạn 49 (bug C5 fix): Trước v0.9.44, code kiểm tra balance rồi mới
    // UPDATE → race condition: 2 request concurrent có thể cùng pass check, rồi cùng trừ
    // → balance âm. Fix: dùng `WHERE {col} >= $2` trong UPDATE để atomic check-and-subtract.
    // Nếu rows_affected = 0 → balance không đủ (race).
    let col_from = match from.as_str() { "a" => "a_balance", "k" => "k_balance", _ => "bi_balance" };
    let new_balance_from: i64 = match sqlx::query_scalar::<_, i64>(
        &format!(
            "UPDATE users SET {col_from} = {col_from} - $2, updated_at = NOW()
             WHERE id = $1 AND {col_from} >= $2
             RETURNING {col_from}"
        )
    )
    .bind(user.id)
    .bind(amount)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(b) => b,
        Err(sqlx::Error::RowNotFound) => {
            // Race condition: balance không đủ khi UPDATE (request khác đã trừ trước)
            log::warn!("⚠️ Race: user {} không đủ {} {} khi UPDATE", user.id, amount, currency_symbol(&from));
            let _ = tx.rollback().await;
            return Json(serde_json::json!({
                "success": false,
                "message": "Số dư không đủ — có giao dịch khác đang xử lý đồng thời. Vui lòng thử lại."
            }))
            .into_response();
        }
        Err(e) => {
            log::error!("❌ Lỗi trừ balance nguồn: {e}");
            let _ = tx.rollback().await;
            return Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server — không trừ được tiền nguồn."
            }))
            .into_response();
        }
    };

    // Add to target (use atomic add — no race issue here since adding is always valid)
    let col_to = match to.as_str() { "a" => "a_balance", "k" => "k_balance", _ => "bi_balance" };
    let new_balance_to: i64 = match sqlx::query_scalar::<_, i64>(
        &format!(
            "UPDATE users SET {col_to} = {col_to} + $2, updated_at = NOW() WHERE id = $1 RETURNING {col_to}"
        )
    )
    .bind(user.id)
    .bind(received)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(b) => b,
        Err(e) => {
            log::error!("❌ Lỗi cộng balance đích: {e}");
            let _ = tx.rollback().await;
            return Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server — không cộng được tiền đích."
            }))
            .into_response();
        }
    };

    // v0.9.44 — Giai đoạn 49 (bug C3 fix): Trước v0.9.44, 2 INSERTs vào
    // balance_transactions dùng `let _ =` → nếu fail, balance đã thay đổi
    // nhưng không có audit log. Fix: propagate error, rollback transaction.
    let desc_out = format!(
        "Quy đổi {} {} → {} {} (tỷ giá {} {} = 1 {}{})",
        amount, currency_symbol(&from),
        received, currency_symbol(&to),
        rate, currency_symbol(&from), currency_symbol(&to),
        if is_reverse { " [reverse rate]" } else { "" }
    );
    if let Err(e) = sqlx::query(
        "INSERT INTO balance_transactions
            (user_id, currency, amount, balance_after, tx_type, description, reference_id)
         VALUES ($1, $2, $3, $4, 'exchange_out', $5, $6)"
    )
    .bind(user.id)
    .bind(&from)
    .bind(-amount)
    .bind(new_balance_from)
    .bind(&desc_out)
    .bind(format!("exchange:{}->{}", from, to))
    .execute(&mut *tx)
    .await
    {
        log::error!("❌ Lỗi INSERT balance_transactions (out): {e}");
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "success": false,
            "message": "Lỗi server — không ghi được log giao dịch. Vui lòng thử lại."
        }))
        .into_response();
    }

    let desc_in = format!(
        "Nhận từ quy đổi {} {} → {} {}",
        amount, currency_symbol(&from),
        received, currency_symbol(&to)
    );
    if let Err(e) = sqlx::query(
        "INSERT INTO balance_transactions
            (user_id, currency, amount, balance_after, tx_type, description, reference_id)
         VALUES ($1, $2, $3, $4, 'exchange_in', $5, $6)"
    )
    .bind(user.id)
    .bind(&to)
    .bind(received)
    .bind(new_balance_to)
    .bind(&desc_in)
    .bind(format!("exchange:{}->{}", from, to))
    .execute(&mut *tx)
    .await
    {
        log::error!("❌ Lỗi INSERT balance_transactions (in): {e}");
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "success": false,
            "message": "Lỗi server — không ghi được log giao dịch. Vui lòng thử lại."
        }))
        .into_response();
    }

    if let Err(e) = tx.commit().await {
        log::error!("❌ Lỗi commit exchange transaction: {e}");
        return Json(serde_json::json!({
            "success": false,
            "message": "Lỗi server — không hoàn tất được giao dịch."
        }))
        .into_response();
    }

    log::info!(
        "💱 User {} quy đổi {} {} → {} {} (rate={})",
        user.id, amount, currency_symbol(&from),
        received, currency_symbol(&to), rate
    );

    Json(ExchangeApiResponse {
        success: true,
        message: format!(
            "✅ Đã quy đổi {} {} → {} {} thành công!",
            amount, currency_symbol(&from),
            received, currency_symbol(&to)
        ),
        from_currency: from,
        to_currency: to,
        from_amount: amount,
        received_amount: received,
        new_balance_from,
        new_balance_to,
    })
    .into_response()
}

/// GET /api/tien-te/ls-giao-dich — Lịch sử giao dịch quy đổi của user.
pub async fn tien_te_history_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false,
            "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let rows: Result<Vec<(String, i64, i64, String, String, chrono::DateTime<chrono::Utc>)>, sqlx::Error> =
        sqlx::query_as(
            "SELECT currency, amount, balance_after, tx_type, description, created_at
             FROM balance_transactions
             WHERE user_id = $1 AND tx_type IN ('exchange_in', 'exchange_out')
             ORDER BY created_at DESC LIMIT 50"
        )
        .bind(user.id)
        .fetch_all(&state.pool)
        .await;

    match rows {
        Ok(history) => {
            let formatted: Vec<serde_json::Value> = history
                .into_iter()
                .map(|(cur, amt, bal_after, tx_type, desc, ts)| {
                    serde_json::json!({
                        "currency": cur,
                        "amount": amt,
                        "balance_after": bal_after,
                        "tx_type": tx_type,
                        "description": desc,
                        "timestamp": ts.to_rfc3339()
                    })
                })
                .collect();
            Json(serde_json::json!({
                "success": true,
                "history": formatted
            }))
            .into_response()
        }
        Err(e) => {
            log::error!("❌ Lỗi query history: {e}");
            Json(serde_json::json!({
                "success": false,
                "message": "Lỗi server."
            }))
            .into_response()
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Handlers — Admin
// ════════════════════════════════════════════════════════════════════════════

/// GET /admin/tien-te — Admin quản lý tỷ giá (UI).
pub async fn admin_tien_te_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let rates = list_rates(&state.pool).await.unwrap_or_default();
    let html = render_admin_tien_te_page(&user, &rates);
    Html(html).into_response()
}

/// POST /admin/tien-te/ty-gia — Admin cập nhật tỷ giá (form).
pub async fn admin_tien_te_update_rate(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AdminRateForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let from = form.from_currency.trim().to_lowercase();
    let to = form.to_currency.trim().to_lowercase();

    if !validate_currency(&from) || !validate_currency(&to) {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Loại tiền tệ không hợp lệ. Chỉ chấp nhận: a, k, bi.
            </div>"#
        )
        .into_response();
    }
    if from == to {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Không thể đặt tỷ giá cùng loại tiền tệ.
            </div>"#
        )
        .into_response();
    }

    let from_amount: i64 = match form.from_amount.trim().parse() {
        Ok(n) if n > 0 => n,
        _ => {
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Số lượng phải là số nguyên dương lớn hơn 0.
                </div>"#
            )
            .into_response();
        }
    };

    let is_active = form.is_active.as_deref().is_some_and(|v| v == "on" || v == "true" || v == "1");

    let result = sqlx::query(
        "INSERT INTO currency_exchange_rates (from_currency, to_currency, from_amount, is_active, updated_by)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (from_currency, to_currency)
         DO UPDATE SET from_amount = $3, is_active = $4, updated_by = $5, updated_at = NOW()"
    )
    .bind(&from)
    .bind(&to)
    .bind(from_amount)
    .bind(is_active)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            log::info!(
                "💱 Admin {} cập nhật tỷ giá {} {} = 1 {} (active={})",
                user.id, from_amount, currency_symbol(&from), currency_symbol(&to), is_active
            );
            Html(
                r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                    ✅ Đã cập nhật tỷ giá. <a href="/admin/tien-te" class="underline">Xem lại danh sách</a>
                </div>"#
            )
            .into_response()
        }
        Err(e) => {
            log::error!("❌ Lỗi update tỷ giá: {e}");
            Html(format!(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi server khi cập nhật tỷ giá: {e}
                </div>"#
            ))
            .into_response()
        }
    }
}

/// POST /admin/tien-te/ty-gia/{from}/{to}/toggle — Bật/tắt tỷ giá.
pub async fn admin_tien_te_toggle_rate(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((from, to)): Path<(String, String)>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let from = from.to_lowercase();
    let to = to.to_lowercase();
    if !validate_currency(&from) || !validate_currency(&to) {
        return Redirect::to("/admin/tien-te").into_response();
    }

    let _ = sqlx::query(
        "UPDATE currency_exchange_rates
         SET is_active = NOT is_active, updated_by = $3, updated_at = NOW()
         WHERE from_currency = $1 AND to_currency = $2"
    )
    .bind(&from)
    .bind(&to)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    Redirect::to("/admin/tien-te").into_response()
}

// ════════════════════════════════════════════════════════════════════════════
// Templates (inline HTML — simple)
// ════════════════════════════════════════════════════════════════════════════

fn render_tien_te_page(is_authed: bool, balances: &CurrencyBalance, rates: &[ExchangeRateView]) -> String {
    let mut rate_cards = String::new();
    for r in rates {
        let sym_from = currency_symbol(&r.from_currency);
        let sym_to = currency_symbol(&r.to_currency);
        let name_from = currency_full_name(&r.from_currency);
        let name_to = currency_full_name(&r.to_currency);
        rate_cards.push_str(&format!(
            r#"<div class="bg-white rounded-xl p-5 shadow-sm border border-amber-100">
                <div class="flex items-center justify-between mb-2">
                    <span class="text-2xl font-bold text-amber-700">{} {}</span>
                    <span class="text-2xl text-amber-700">→</span>
                    <span class="text-2xl font-bold text-amber-700">1 {}</span>
                </div>
                <div class="text-xs text-gray-500">{name_from} → {name_to}</div>
            </div>"#,
            r.from_amount, sym_from, sym_to
        ));
    }

    let balance_section = if is_authed {
        format!(
            r#"<div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
                <div class="bg-gradient-to-br from-emerald-50 to-emerald-100 rounded-xl p-4 border border-emerald-200">
                    <div class="text-xs text-emerald-700 font-medium">Ấm (A)</div>
                    <div class="text-2xl font-bold text-emerald-900">{}</div>
                    <div class="text-[10px] text-emerald-600 mt-1">Niệm Lực A</div>
                </div>
                <div class="bg-gradient-to-br from-amber-50 to-amber-100 rounded-xl p-4 border border-amber-200">
                    <div class="text-xs text-amber-700 font-medium">K (Karmic)</div>
                    <div class="text-2xl font-bold text-amber-900">{}</div>
                    <div class="text-[10px] text-amber-600 mt-1">Công Đức K</div>
                </div>
                <div class="bg-gradient-to-br from-violet-50 to-violet-100 rounded-xl p-4 border border-violet-200">
                    <div class="text-xs text-violet-700 font-medium">I (Nguyên Lực)</div>
                    <div class="text-2xl font-bold text-violet-900">{}</div>
                    <div class="text-[10px] text-violet-600 mt-1">Từ Tượng Phật</div>
                </div>
                <div class="bg-gradient-to-br from-pink-50 to-pink-100 rounded-xl p-4 border border-pink-200">
                    <div class="text-xs text-pink-700 font-medium">Bi (Từ Bi)</div>
                    <div class="text-2xl font-bold text-pink-900">{}</div>
                    <div class="text-[10px] text-pink-600 mt-1">Loại cao cấp nhất</div>
                </div>
            </div>"#,
            balances.a, balances.k, balances.i, balances.bi
        )
    } else {
        r#"<div class="bg-amber-50 border border-amber-200 rounded-xl p-4 mb-6 text-center">
            <a href="/dang-nhap?next=/tien-te" class="text-amber-700 underline font-medium">→ Đăng nhập để xem số dư và quy đổi tiền tệ</a>
        </div>"#.to_string()
    };

    let exchange_form = if is_authed {
        r#"<div class="bg-white rounded-xl p-6 shadow-sm border border-gray-100 mb-6">
            <h2 class="text-lg font-bold text-gray-800 mb-4">💱 Quy đổi tiền tệ</h2>
            <form id="exchange-form" class="space-y-4" onsubmit="return false;">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">Từ loại tiền</label>
                        <select id="from-currency" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                            <option value="a">Ấm (A) — Niệm Lực</option>
                            <option value="k">K (Karmic) — Công Đức</option>
                            <option value="bi">Bi (Từ Bi) — Cao cấp</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">Số lượng</label>
                        <input id="amount" type="number" min="1" placeholder="100" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" />
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">Sang loại tiền</label>
                        <select id="to-currency" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                            <option value="k">K (Karmic) — Công Đức</option>
                            <option value="bi">Bi (Từ Bi) — Cao cấp</option>
                            <option value="a">Ấm (A) — Niệm Lực</option>
                        </select>
                    </div>
                </div>
                <button type="submit" id="exchange-btn" class="w-full bg-amber-600 hover:bg-amber-700 text-white font-bold py-2.5 rounded-lg transition">
                    💱 Quy đổi ngay
                </button>
            </form>
            <div id="exchange-result" class="mt-4 hidden"></div>
        </div>

        <script>
        document.getElementById('exchange-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const btn = document.getElementById('exchange-btn');
            const result = document.getElementById('exchange-result');
            const from = document.getElementById('from-currency').value;
            const to = document.getElementById('to-currency').value;
            const amount = document.getElementById('amount').value;
            if (!amount || amount <= 0) {
                result.className = 'mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm';
                result.textContent = '⚠️ Vui lòng nhập số lượng hợp lệ.';
                result.classList.remove('hidden');
                return;
            }
            btn.disabled = true;
            btn.textContent = '⏳ Đang xử lý...';
            try {
                const resp = await fetch('/api/tien-te/doi', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ from_currency: from, to_currency: to, amount: amount })
                });
                const data = await resp.json();
                result.className = 'mt-4 p-3 ' + (data.success ? 'bg-green-50 border border-green-200 text-green-700' : 'bg-red-50 border border-red-200 text-red-700') + ' rounded-lg text-sm';
                result.textContent = data.message;
                result.classList.remove('hidden');
                if (data.success) {
                    setTimeout(() => window.location.reload(), 1500);
                }
            } catch (err) {
                result.className = 'mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm';
                result.textContent = '⚠️ Lỗi kết nối: ' + err.message;
                result.classList.remove('hidden');
            } finally {
                btn.disabled = false;
                btn.textContent = '💱 Quy đổi ngay';
            }
        });
        </script>"#.to_string()
    } else {
        String::new()
    };

    let history_section = if is_authed {
        r#"<div class="bg-white rounded-xl p-6 shadow-sm border border-gray-100">
            <h2 class="text-lg font-bold text-gray-800 mb-4">📜 Lịch sử quy đổi</h2>
            <div id="history-list" class="space-y-2">
                <div class="text-center text-gray-500 text-sm py-4">Đang tải...</div>
            </div>
        </div>
        <script>
        (async () => {
            try {
                const resp = await fetch('/api/tien-te/ls-giao-dich');
                const data = await resp.json();
                const list = document.getElementById('history-list');
                if (!data.success || !data.history || data.history.length === 0) {
                    list.innerHTML = '<div class="text-center text-gray-500 text-sm py-4">Chưa có giao dịch nào.</div>';
                    return;
                }
                list.innerHTML = data.history.map(h => {
                    const isPos = h.amount > 0;
                    const color = isPos ? 'text-green-700 bg-green-50' : 'text-red-700 bg-red-50';
                    const sign = isPos ? '+' : '';
                    const time = new Date(h.timestamp).toLocaleString('vi-VN');
                    return `<div class="flex items-center justify-between p-3 ${color} rounded-lg text-sm">
                        <span>${h.description}</span>
                        <span class="font-bold">${sign}${h.amount} ${h.currency.toUpperCase()}</span>
                    </div>`;
                }).join('');
            } catch (e) {
                document.getElementById('history-list').innerHTML = '<div class="text-center text-red-500 text-sm py-4">Lỗi tải lịch sử.</div>';
            }
        })();
        </script>"#.to_string()
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>💱 Tiền Tệ & Quy Đổi — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="icon" href="/static/tubi.png">
    <link rel="stylesheet" href="/static/css/app.css">
</head>
<body class="bg-gradient-to-br from-amber-50 via-orange-50 to-pink-50 min-h-screen">
    <header class="bg-white/80 backdrop-blur border-b border-amber-100 sticky top-0 z-10">
        <div class="max-w-4xl mx-auto px-4 py-3 flex items-center justify-between">
            <a href="/" class="flex items-center gap-2">
                <img src="/static/tubi.png" alt="🪷" class="w-8 h-8" />
                <span class="font-bold text-gray-800">Ứng Dụng Từ Bi</span>
            </a>
            <a href="/tong-quan" class="text-sm text-amber-700 hover:underline">← Quay lại Tổng Quan</a>
        </div>
    </header>

    <main class="max-w-4xl mx-auto px-4 py-8">
        <div class="text-center mb-8">
            <h1 class="text-3xl font-bold bg-gradient-to-r from-amber-600 to-pink-600 bg-clip-text text-transparent mb-2">💱 Tiền Tệ & Quy Đổi</h1>
            <p class="text-sm text-gray-600">Hệ thống 3 tiền tệ: Ấm (A) · K (Karmic) · Bi (Từ Bi)</p>
        </div>

        {balance_section}

        {exchange_form}

        <div class="bg-white rounded-xl p-6 shadow-sm border border-gray-100 mb-6">
            <h2 class="text-lg font-bold text-gray-800 mb-4">📊 Tỷ giá hiện tại</h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                {rate_cards}
            </div>
            <p class="text-xs text-gray-500 mt-4">
                💡 Tỷ giá được admin quản lý. Tiền Bi (Bi) là loại cao cấp nhất, kiếm qua cống hiến đặc biệt hoặc quy đổi từ K.
            </p>
        </div>

        {history_section}
    </main>

    <footer class="text-center py-6 text-xs text-gray-500">
        🪷 Ứng Dụng Từ Bi v0.9.43 · Giai đoạn 47 · Nguyện công đức vô lượng
    </footer>
</body>
</html>"#
    )
}

fn render_admin_tien_te_page(user: &crate::models::user::User, rates: &[ExchangeRate]) -> String {
    let mut rate_rows = String::new();
    for r in rates {
        let badge = if r.is_active {
            r#"<span class="px-2 py-0.5 bg-green-100 text-green-800 text-xs rounded-full font-semibold">Đang dùng</span>"#
        } else {
            r#"<span class="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs rounded-full font-semibold">Tắt</span>"#
        };
        let sym_from = currency_symbol(&r.from_currency);
        let sym_to = currency_symbol(&r.to_currency);
        rate_rows.push_str(&format!(
            r#"<tr class="border-b border-gray-100">
                <td class="px-4 py-3 text-sm">{from} → {to}</td>
                <td class="px-4 py-3 text-sm font-mono">{amount} {sym_from} = 1 {sym_to}</td>
                <td class="px-4 py-3 text-sm">{badge}</td>
                <td class="px-4 py-3 text-sm">
                    <form method="POST" action="/admin/tien-te/ty-gia/{from}/{to}/toggle" class="inline">
                        <button type="submit" class="text-xs px-2 py-1 bg-amber-100 text-amber-700 rounded hover:bg-amber-200">
                            {toggle_label}
                        </button>
                    </form>
                </td>
            </tr>"#,
            from = r.from_currency.to_uppercase(),
            to = r.to_currency.to_uppercase(),
            amount = r.from_amount,
            sym_from = sym_from,
            sym_to = sym_to,
            badge = badge,
            toggle_label = if r.is_active { "Tắt" } else { "Bật" }
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>💱 Quản Lý Tỷ Giá — Admin</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="icon" href="/static/tubi.png">
    <link rel="stylesheet" href="/static/css/app.css">
</head>
<body class="bg-gray-50 min-h-screen">
    <header class="bg-white border-b border-gray-200">
        <div class="max-w-5xl mx-auto px-4 py-3 flex items-center justify-between">
            <a href="/admin" class="flex items-center gap-2">
                <img src="/static/tubi.png" alt="🪷" class="w-8 h-8" />
                <span class="font-bold text-gray-800">Admin — Quản Lý Tỷ Giá</span>
            </a>
            <a href="/admin" class="text-sm text-gray-600 hover:underline">← Về Dashboard</a>
        </div>
    </header>

    <main class="max-w-5xl mx-auto px-4 py-8">
        <h1 class="text-2xl font-bold text-gray-800 mb-6">💱 Quản Lý Tỷ Giá Tiền Tệ</h1>

        <div class="bg-white rounded-xl p-6 shadow-sm border border-gray-100 mb-6">
            <h2 class="text-lg font-bold text-gray-800 mb-4">➕ Cập nhật tỷ giá</h2>
            <form method="POST" action="/admin/tien-te/ty-gia" class="grid grid-cols-1 md:grid-cols-4 gap-3 items-end">
                <div>
                    <label class="block text-xs font-medium text-gray-600 mb-1">Từ</label>
                    <select name="from_currency" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                        <option value="a">Ấm (A)</option>
                        <option value="k">K (Karmic)</option>
                        <option value="bi">Bi (Từ Bi)</option>
                    </select>
                </div>
                <div>
                    <label class="block text-xs font-medium text-gray-600 mb-1">Sang</label>
                    <select name="to_currency" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                        <option value="k">K (Karmic)</option>
                        <option value="bi">Bi (Từ Bi)</option>
                        <option value="a">Ấm (A)</option>
                    </select>
                </div>
                <div>
                    <label class="block text-xs font-medium text-gray-600 mb-1">Số lượng (từ = 1 đến)</label>
                    <input name="from_amount" type="number" min="1" placeholder="100" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" />
                </div>
                <div class="flex items-center gap-3">
                    <label class="flex items-center gap-2 text-sm">
                        <input type="checkbox" name="is_active" checked class="rounded" /> Kích hoạt
                    </label>
                    <button type="submit" class="bg-amber-600 hover:bg-amber-700 text-white font-bold py-2 px-4 rounded-lg text-sm">
                        💾 Lưu
                    </button>
                </div>
            </form>
        </div>

        <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
            <table class="w-full">
                <thead class="bg-gray-50 border-b border-gray-200">
                    <tr>
                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase">Cặp</th>
                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase">Tỷ giá</th>
                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase">Trạng thái</th>
                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-600 uppercase">Hành động</th>
                    </tr>
                </thead>
                <tbody>
                    {rate_rows}
                </tbody>
            </table>
        </div>

        <div class="mt-6 bg-amber-50 border border-amber-200 rounded-xl p-4 text-sm text-amber-800">
            💡 <strong>Lưu ý:</strong> Tỷ giá mặc định: 100 Ấm = 1 K · 100 K = 1 Bi · 10000 Ấm = 1 Bi.
            Thay đổi tỷ giá có thể ảnh hưởng đến giao dịch quy đổi của người dùng.
        </div>
    </main>

    <footer class="text-center py-6 text-xs text-gray-500">
        🪷 Ứng Dụng Từ Bi v0.9.43 · Admin: {admin_name} · Nguyện công đức vô lượng
    </footer>
</body>
</html>"#,
        rate_rows = rate_rows,
        admin_name = user.display_name
    )
}

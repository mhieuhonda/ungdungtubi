//! Handlers cho Phần Thưởng Đăng Nhập Hàng Ngày — Giai đoạn 57 (v0.9.45).
//!
//! Routes:
//!   GET  /api/daily-login/status      — Trạng thái streak + phần thưởng hôm nay
//!   POST /api/daily-login/nhan        — Nhận phần thưởng hôm nay
//!   GET  /api/daily-login/ls           — Lịch sử nhận thưởng 30 ngày gần nhất
//!
//! Reward schedule (cycle 7 ngày):
//!   Ngày 1: +10 A
//!   Ngày 2: +15 A
//!   Ngày 3: +20 A
//!   Ngày 4: +25 A
//!   Ngày 5: +30 A
//!   Ngày 6: +40 A
//!   Ngày 7: +100 A (bonus đặc biệt)
//!
//! Streak rules:
//!   - Streak = số ngày liên tiếp đăng nhập (reset về 0 nếu bỏ ngày)
//!   - last_login_date < hôm nay - 1 → streak reset về 0 trước khi +1
//!   - Đã nhận hôm nay → không nhận lại

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;

const REWARD_SCHEDULE: [(i16, i64, bool); 7] = [
    (1, 10, false),
    (2, 15, false),
    (3, 20, false),
    (4, 25, false),
    (5, 30, false),
    (6, 40, false),
    (7, 100, true),
];

#[derive(Debug, Serialize, FromRow)]
pub struct StreakStatus {
    pub current_streak: i16,
    pub max_streak: i16,
    pub last_login_date: Option<chrono::NaiveDate>,
    pub total_days_claimed: i64,
    pub total_a_earned: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyLoginStatus {
    pub streak: StreakStatus,
    pub today_reward_a: i64,
    pub today_streak_day: i16,
    pub today_is_bonus: bool,
    pub today_already_claimed: bool,
    pub next_bonus_in_days: i16,
    pub reward_schedule: Vec<(i16, i64, bool)>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RewardHistoryRow {
    pub reward_date: chrono::NaiveDate,
    pub streak_day: i16,
    pub reward_a: i64,
    pub is_bonus: bool,
    pub balance_after: i64,
    pub claimed_at: DateTime<Utc>,
}

/// Lấy ngày địa phương (Asia/Ho_Chi_Minh) của user — dùng cho daily login reset.
fn local_today() -> NaiveDate {
    // UTC+7 = Vietnam/Saigon timezone
    let now_utc = Utc::now();
    let now_local = now_utc + chrono::Duration::hours(7);
    now_local.date_naive()
}

/// Tính reward cho ngày hôm nay dựa trên streak hiện tại.
fn today_reward(streak_day_next: i16) -> (i16, i64, bool) {
    // streak_day_next: số ngày sẽ nhận (1-7, wrap around)
    let idx = ((streak_day_next - 1).rem_euclid(7)) as usize;
    REWARD_SCHEDULE[idx]
}

/// GET /api/daily-login/status — Trạng thái streak + phần thưởng hôm nay.
pub async fn api_daily_login_status(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let streak: StreakStatus = sqlx::query_as(
        "SELECT current_streak, max_streak, last_login_date::DATE, total_days_claimed, total_a_earned
         FROM user_login_streaks WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(StreakStatus {
        current_streak: 0,
        max_streak: 0,
        last_login_date: None,
        total_days_claimed: 0,
        total_a_earned: 0,
    });

    let today = local_today();
    let already_claimed = streak.last_login_date == Some(today);

    // Tính streak_day_next: nếu last_login = hôm qua → streak + 1, nếu đã claim hôm nay → giữ nguyên
    let streak_day_next = if already_claimed {
        streak.current_streak
    } else if streak.last_login_date.is_none()
        || streak
            .last_login_date
            .unwrap_or(today)
            .num_days_from_ce()
            < today.num_days_from_ce()
    {
        // Chưa claim + (lần đầu HOẶC last_login < hôm nay) → streak +1 (nếu yesterday thì +1, nếu xa hơn thì reset về 1)
        let yesterday = today - chrono::Duration::days(1);
        if streak.last_login_date == Some(yesterday) {
            streak.current_streak + 1
        } else {
            1
        }
    } else {
        streak.current_streak
    };

    let (today_streak_day, today_reward_a, today_is_bonus) = today_reward(streak_day_next);
    let next_bonus_in_days = if today_is_bonus {
        0
    } else {
        (7 - (streak_day_next - 1).rem_euclid(7) - 1).max(0) as i16 + 1
    };

    let status = DailyLoginStatus {
        streak,
        today_reward_a,
        today_streak_day,
        today_is_bonus,
        today_already_claimed: already_claimed,
        next_bonus_in_days,
        reward_schedule: REWARD_SCHEDULE.to_vec(),
    };

    Json(serde_json::json!({
        "success": true,
        "status": status
    }))
    .into_response()
}

/// POST /api/daily-login/nhan — Nhận phần thưởng hôm nay.
pub async fn api_daily_login_claim(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let today = local_today();

    // Lấy streak hiện tại
    let streak: StreakStatus = sqlx::query_as(
        "SELECT current_streak, max_streak, last_login_date::DATE, total_days_claimed, total_a_earned
         FROM user_login_streaks WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(StreakStatus {
        current_streak: 0,
        max_streak: 0,
        last_login_date: None,
        total_days_claimed: 0,
        total_a_earned: 0,
    });

    // Đã claim hôm nay?
    if streak.last_login_date == Some(today) {
        return Json(serde_json::json!({
            "success": false,
            "message": "Bạn đã nhận phần thưởng hôm nay. Quay lại vào ngày mai nhé 🪷"
        }))
        .into_response();
    }

    // Tính streak_day mới
    let yesterday = today - chrono::Duration::days(1);
    let new_streak = if streak.last_login_date == Some(yesterday) {
        // Streak continues
        streak.current_streak + 1
    } else {
        // Reset về 1 (hoặc bắt đầu mới)
        1
    };

    let (streak_day, reward_a, is_bonus) = today_reward(new_streak);

    // Transaction: INSERT daily_login_rewards + UPDATE user_login_streaks + UPDATE users.a_balance + INSERT balance_transactions
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ Lỗi bắt đầu tx daily login: {e}");
            return Json(serde_json::json!({
                "success": false, "message": "Lỗi hệ thống."
            }))
            .into_response();
        }
    };

    // Lock user row để tránh race condition
    let user_row: Option<(i64,)> = sqlx::query_as("SELECT a_balance FROM users WHERE id = $1 FOR UPDATE")
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await
        .ok()
        .flatten();

    let Some((current_a,)) = user_row else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "success": false, "message": "User không tồn tại."
        }))
        .into_response();
    };

    let new_balance = current_a + reward_a;

    // 1. INSERT daily_login_rewards — MUST check rows_affected, vì ON CONFLICT DO NOTHING
    // có thể trả về 0 rows nếu user đã claim hôm nay (race condition giữa check và INSERT).
    let insert_result = sqlx::query(
        "INSERT INTO daily_login_rewards (user_id, reward_date, streak_day, reward_a, is_bonus, balance_after)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, reward_date) DO NOTHING"
    )
    .bind(user.id)
    .bind(today)
    .bind(streak_day)
    .bind(reward_a)
    .bind(is_bonus)
    .bind(new_balance)
    .execute(&mut *tx)
    .await;

    let rows_affected = insert_result.map(|r| r.rows_affected()).unwrap_or(0);
    if rows_affected == 0 {
        // Đã có reward cho hôm nay → KHÔNG được cộng a_balance nữa.
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "success": false,
            "message": "Bạn đã nhận phần thưởng hôm nay rồi. Vui lòng quay lại vào ngày mai."
        }))
        .into_response();
    }

    // 2. UPSERT user_login_streaks
    let _ = sqlx::query(
        "INSERT INTO user_login_streaks
            (user_id, current_streak, max_streak, last_login_date, total_days_claimed, total_a_earned)
         VALUES ($1, $2, GREATEST($2, $3), $4, 1, $5)
         ON CONFLICT (user_id) DO UPDATE SET
            current_streak      = EXCLUDED.current_streak,
            max_streak          = GREATEST(user_login_streaks.max_streak, EXCLUDED.max_streak),
            last_login_date     = EXCLUDED.last_login_date,
            total_days_claimed  = user_login_streaks.total_days_claimed + 1,
            total_a_earned      = user_login_streaks.total_a_earned + EXCLUDED.total_a_earned,
            updated_at          = NOW()"
    )
    .bind(user.id)
    .bind(new_streak)
    .bind(streak.max_streak)
    .bind(today)
    .bind(reward_a)
    .execute(&mut *tx)
    .await;

    // 3. UPDATE users.a_balance
    let _ = sqlx::query("UPDATE users SET a_balance = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_balance)
        .bind(user.id)
        .execute(&mut *tx)
        .await;

    // 4. INSERT balance_transactions
    let _ = sqlx::query(
        "INSERT INTO balance_transactions (user_id, currency, amount, balance_after, tx_type, description)
         VALUES ($1, 'a', $2, $3, 'daily_login', $4)"
    )
    .bind(user.id)
    .bind(reward_a)
    .bind(new_balance)
    .bind(if is_bonus {
        format!("Phần thưởng đăng nhập ngày thứ 7 (streak {})", new_streak)
    } else {
        format!("Phần thưởng đăng nhập ngày {} (streak {})", streak_day, new_streak)
    })
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        log::error!("❌ Lỗi commit daily login tx: {e}");
        return Json(serde_json::json!({
            "success": false, "message": "Không thể nhận phần thưởng."
        }))
        .into_response();
    }

    log::info!(
        "🎁 User {} nhận daily login reward: +{} A (streak {}, day {})",
        user.id, reward_a, new_streak, streak_day
    );

    Json(serde_json::json!({
        "success": true,
        "reward_a": reward_a,
        "new_balance_a": new_balance,
        "streak_day": streak_day,
        "new_streak": new_streak,
        "is_bonus": is_bonus,
        "message": if is_bonus {
            format!("🎉 Phần thưởng ngày thứ 7! +{} A (streak {}). Nguyện công đức vô lượng!", reward_a, new_streak)
        } else {
            format!("🪷 Nhận được +{} A (ngày {}, streak {}).", reward_a, streak_day, new_streak)
        }
    }))
    .into_response()
}

/// GET /api/daily-login/ls — Lịch sử nhận thưởng 30 ngày gần nhất.
pub async fn api_daily_login_history(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let rows: Vec<RewardHistoryRow> = sqlx::query_as(
        "SELECT reward_date::DATE, streak_day, reward_a, is_bonus, balance_after, claimed_at
         FROM daily_login_rewards
         WHERE user_id = $1
         ORDER BY reward_date DESC LIMIT 30"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "success": true,
        "history": rows
    }))
    .into_response()
}

/// Background: Tự động award daily login khi user login thành công.
/// Được gọi từ `auth::google_callback` sau khi tạo session.
pub async fn try_auto_award_on_login(pool: &sqlx::PgPool, user_id: Uuid) {
    let today = local_today();

    // Check if already claimed today
    let already: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM daily_login_rewards WHERE user_id = $1 AND reward_date = $2"
    )
    .bind(user_id)
    .bind(today)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if already.is_some() {
        return;
    }

    // Streak hiện tại
    let streak: Option<(i16, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT current_streak, last_login_date::DATE FROM user_login_streaks WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (current_streak, last_login) = streak.unwrap_or((0, None));

    let yesterday = today - chrono::Duration::days(1);
    let new_streak = if last_login == Some(yesterday) {
        current_streak + 1
    } else {
        1
    };

    let (streak_day, reward_a, _is_bonus) = today_reward(new_streak);

    // Insert reward (with balance update in same tx)
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return,
    };

    let user_row: Option<(i64,)> = sqlx::query_as("SELECT a_balance FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .ok()
        .flatten();

    let Some((current_a,)) = user_row else {
        let _ = tx.rollback().await;
        return;
    };

    let new_balance = current_a + reward_a;

    let _ = sqlx::query(
        "INSERT INTO daily_login_rewards (user_id, reward_date, streak_day, reward_a, is_bonus, balance_after)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, reward_date) DO NOTHING"
    )
    .bind(user_id)
    .bind(today)
    .bind(streak_day)
    .bind(reward_a)
    .bind(_is_bonus)
    .bind(new_balance)
    .execute(&mut *tx)
    .await;

    let _ = sqlx::query(
        "INSERT INTO user_login_streaks
            (user_id, current_streak, max_streak, last_login_date, total_days_claimed, total_a_earned)
         VALUES ($1, $2, $2, $3, 1, $4)
         ON CONFLICT (user_id) DO UPDATE SET
            current_streak      = EXCLUDED.current_streak,
            max_streak          = GREATEST(user_login_streaks.max_streak, EXCLUDED.max_streak),
            last_login_date     = EXCLUDED.last_login_date,
            total_days_claimed  = user_login_streaks.total_days_claimed + 1,
            total_a_earned      = user_login_streaks.total_a_earned + EXCLUDED.total_a_earned,
            updated_at          = NOW()"
    )
    .bind(user_id)
    .bind(new_streak)
    .bind(today)
    .bind(reward_a)
    .execute(&mut *tx)
    .await;

    let _ = sqlx::query("UPDATE users SET a_balance = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_balance)
        .bind(user_id)
        .execute(&mut *tx)
        .await;

    let _ = sqlx::query(
        "INSERT INTO balance_transactions (user_id, currency, amount, balance_after, tx_type, description)
         VALUES ($1, 'a', $2, $3, 'daily_login', $4)"
    )
    .bind(user_id)
    .bind(reward_a)
    .bind(new_balance)
    .bind(format!("Daily login reward — Ngày {} (streak {})", streak_day, new_streak))
    .execute(&mut *tx)
    .await;

    let _ = tx.commit().await;
    log::info!(
        "🎁 Auto-award daily login for user {}: +{} A (streak {}, day {})",
        user_id, reward_a, new_streak, streak_day
    );
}

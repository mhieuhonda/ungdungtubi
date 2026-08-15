//! Handlers cho trang Thành Tích — Giai đoạn 19 (v0.9.14).
//!
//! Routes:
//!   - GET /thanh-tich          — Trang thành tích cá nhân (đã đạt + tiến độ)
//!   - GET /api/thanh-tich/stats — JSON tổng quan thành tích

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Models ──────────────────────────────────────────────────────────────

/// Một thành tích user đã đạt.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct EarnedAchievement {
    pub id: i64,
    pub achievement_id: i32,
    pub code: String,
    pub name_vi: String,
    pub description_vi: Option<String>,
    pub icon: String,
    pub period: String,
    pub category: String,
    pub rarity: String,
    pub achievement_points: i32,
    pub reward_a: i64,
    pub reward_i: i64,
    pub reward_k: i64,
    pub period_key: Option<String>,
    pub progress_value: i64,
    pub achieved_at: DateTime<Utc>,
}

impl EarnedAchievement {
    /// CSS class cho rarity badge.
    pub fn rarity_class(&self) -> &'static str {
        match self.rarity.as_str() {
            "common" => "bg-gray-100 text-gray-700 border-gray-300",
            "rare" => "bg-blue-100 text-blue-700 border-blue-300",
            "epic" => "bg-purple-100 text-purple-700 border-purple-300",
            "legendary" => "bg-amber-100 text-amber-800 border-amber-300",
            "mythic" => "bg-gradient-to-r from-pink-500 to-violet-500 text-white border-pink-400",
            _ => "bg-gray-100 text-gray-700 border-gray-300",
        }
    }

    /// Tên hiển thị độ hiếm tiếng Việt.
    pub fn rarity_label(&self) -> &'static str {
        match self.rarity.as_str() {
            "common" => "Phổ Thông",
            "rare" => "Hiếm",
            "epic" => "Sử Thi",
            "legendary" => "Huyền Thoại",
            "mythic" => "Thần Thoại",
            _ => "Phổ Thông",
        }
    }

    /// Nhãn phần thưởng (vd: "+50 A · +5 I · +1 K").
    pub fn reward_label(&self) -> String {
        let mut parts = Vec::new();
        if self.reward_a > 0 {
            parts.push(format!("+{} A", self.reward_a));
        }
        if self.reward_i > 0 {
            parts.push(format!("+{} I", self.reward_i));
        }
        if self.reward_k > 0 {
            parts.push(format!("+{} K", self.reward_k));
        }
        if parts.is_empty() {
            "Không có phần thưởng".into()
        } else {
            parts.join(" · ")
        }
    }
}

/// Một thành tích đang tiến triển (chưa đạt).
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct InProgressAchievement {
    pub achievement_id: i32,
    pub code: String,
    pub name_vi: String,
    pub description_vi: Option<String>,
    pub icon: String,
    pub period: String,
    pub category: String,
    pub rarity: String,
    pub achievement_points: i32,
    pub target_value: i64,
    pub current_value: i64,
    pub percent_complete: i32,
}

impl InProgressAchievement {
    /// CSS class cho progress bar theo độ hoàn thành.
    pub fn progress_bar_class(&self) -> &'static str {
        if self.percent_complete >= 75 {
            "bg-tubi-600"
        } else if self.percent_complete >= 50 {
            "bg-tubi-500"
        } else if self.percent_complete >= 25 {
            "bg-amber-400"
        } else {
            "bg-gray-400"
        }
    }

    pub fn rarity_label(&self) -> &'static str {
        match self.rarity.as_str() {
            "common" => "Phổ Thông",
            "rare" => "Hiếm",
            "epic" => "Sử Thi",
            "legendary" => "Huyền Thoại",
            "mythic" => "Thần Thoại",
            _ => "Phổ Thông",
        }
    }
}

/// Thống kê tổng quan thành tích của user.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AchievementStats {
    pub total_earned: i64,
    pub total_points: i64,
    pub total_reward_a: i64,
    pub total_reward_i: i64,
    pub total_reward_k: i64,
    /// Phân loại theo rarity.
    pub by_rarity: Vec<(String, i64)>,
    /// Phân loại theo category.
    pub by_category: Vec<(String, i64)>,
}

// ─── Template ────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "thanh-tich/index.html")]
pub struct ThanhTichTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub earned: Vec<EarnedAchievement>,
    pub in_progress: Vec<InProgressAchievement>,
    pub stats: AchievementStats,
}

// ─── Handlers ────────────────────────────────────────────────────────────

/// GET /thanh-tich — Trang thành tích cá nhân.
pub async fn thanh_tich_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/thanh-tich").into_response();
    };

    let earned = fetch_earned_achievements(&state.pool, user.id).await;
    let in_progress = fetch_in_progress_achievements(&state.pool, user.id).await;
    let stats = fetch_achievement_stats(&state.pool, user.id).await;

    let html = ThanhTichTemplate {
        user: Some(user),
        active_page: "thanh_tich".into(),
        earned,
        in_progress,
        stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (thanh-tich): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /api/thanh-tich/stats — JSON tổng quan.
pub async fn thanh_tich_stats_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let stats = match user {
        Some(u) => fetch_achievement_stats(&state.pool, u.id).await,
        None => AchievementStats::default(),
    };
    Json(serde_json::json!({
        "total_earned": stats.total_earned,
        "total_points": stats.total_points,
        "total_reward_a": stats.total_reward_a,
        "total_reward_i": stats.total_reward_i,
        "total_reward_k": stats.total_reward_k,
    }))
    .into_response()
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Lấy danh sách thành tích user đã đạt (mới nhất trước).
async fn fetch_earned_achievements(pool: &sqlx::PgPool, user_id: Uuid) -> Vec<EarnedAchievement> {
    sqlx::query_as::<_, EarnedAchievement>(
        "SELECT ua.id, ua.achievement_id, a.code, a.name_vi, a.description_vi,
                a.icon, a.period, a.category, a.rarity, a.achievement_points,
                a.reward_a, a.reward_i, a.reward_k,
                ua.period_key, ua.progress_value, ua.achieved_at
         FROM user_achievements ua
         JOIN achievements a ON a.id = ua.achievement_id
         WHERE ua.user_id = $1
         ORDER BY ua.achieved_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch earned achievements: {e}");
        vec![]
    })
}

/// Lấy danh sách thành tích chưa đạt (one_time + total — không tính periodic đã đạt).
async fn fetch_in_progress_achievements(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Vec<InProgressAchievement> {
    // Lấy tất cả achievements active mà user chưa đạt (one_time/total)
    // kèm current_value tính từ các bảng metric.
    //
    // Vì việc tính current_value chính xác cho từng metric phức tạp, ở v0.9.14
    // ta chỉ hiển thị target_value + 0 làm current_value cho các achievement
    // chưa có progress. Khi user trigger hành động, Rust handler sẽ upsert
    // vào achievement_progress và current_value sẽ được populate qua view
    // v_user_achievement_progress.
    sqlx::query_as::<_, InProgressAchievement>(
        "SELECT a.id AS achievement_id, a.code, a.name_vi, a.description_vi,
                a.icon, a.period, a.category, a.rarity, a.achievement_points,
                COALESCE(ap.target_value,
                    (a.criteria->>'value')::BIGINT
                ) AS target_value,
                COALESCE(ap.current_value, 0) AS current_value,
                CASE WHEN COALESCE(ap.target_value, (a.criteria->>'value')::BIGINT) > 0
                     THEN LEAST(100,
                        (COALESCE(ap.current_value, 0) * 100 /
                         COALESCE(ap.target_value, (a.criteria->>'value')::BIGINT))::INT)
                     ELSE 0
                END AS percent_complete
         FROM achievements a
         LEFT JOIN achievement_progress ap
              ON ap.achievement_id = a.id AND ap.user_id = $1
         WHERE a.is_active = true
           AND a.period IN ('one_time', 'total')
           AND NOT EXISTS (
               SELECT 1 FROM user_achievements ua
               WHERE ua.achievement_id = a.id AND ua.user_id = $1
           )
         ORDER BY
            CASE a.rarity
                WHEN 'mythic' THEN 1
                WHEN 'legendary' THEN 2
                WHEN 'epic' THEN 3
                WHEN 'rare' THEN 4
                ELSE 5
            END,
            a.sort_order",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        log::warn!("⚠️ Lỗi fetch in-progress achievements: {e}");
        vec![]
    })
}

/// Tính stats tổng quan cho user.
async fn fetch_achievement_stats(pool: &sqlx::PgPool, user_id: Uuid) -> AchievementStats {
    let total_earned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM user_achievements WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let (total_points, total_reward_a, total_reward_i, total_reward_k): (i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT COALESCE(SUM(a.achievement_points), 0)::BIGINT,
                    COALESCE(SUM(a.reward_a), 0)::BIGINT,
                    COALESCE(SUM(a.reward_i), 0)::BIGINT,
                    COALESCE(SUM(a.reward_k), 0)::BIGINT
             FROM user_achievements ua
             JOIN achievements a ON a.id = ua.achievement_id
             WHERE ua.user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0, 0, 0, 0));

    let by_rarity: Vec<(String, i64)> = sqlx::query_as(
        "SELECT a.rarity, COUNT(*)::BIGINT
         FROM user_achievements ua
         JOIN achievements a ON a.id = ua.achievement_id
         WHERE ua.user_id = $1
         GROUP BY a.rarity
         ORDER BY COUNT(*) DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let by_category: Vec<(String, i64)> = sqlx::query_as(
        "SELECT a.category, COUNT(*)::BIGINT
         FROM user_achievements ua
         JOIN achievements a ON a.id = ua.achievement_id
         WHERE ua.user_id = $1
         GROUP BY a.category
         ORDER BY COUNT(*) DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    AchievementStats {
        total_earned,
        total_points,
        total_reward_a,
        total_reward_i,
        total_reward_k,
        by_rarity,
        by_category,
    }
}

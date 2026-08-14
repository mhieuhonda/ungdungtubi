//! Handlers cho Bảng Xếp Hạng — Giai đoạn 14 (v0.9.10).
//!
//! Routes:
//!   - GET /bang-xep-hang              — Trang Bảng Xếp Hạng (5 tabs: A, I, K, Hôm Nay, Streak)
//!   - GET /api/bang-xep-hang/stats    — JSON tổng quan cho dashboard

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Query params ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct TabQuery {
    pub tab: Option<String>,
}

// ─── Models ─────────────────────────────────────────────────────────────

/// Một dòng trong bảng xếp hạng (top user).
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct LeaderboardEntry {
    /// Thứ hạng (1, 2, 3, ...).
    pub rank: i64,
    /// User ID.
    pub user_id: Uuid,
    /// Tên hiển thị.
    pub display_name: String,
    /// URL avatar (có thể NULL).
    pub avatar_url: Option<String>,
    /// Giá trị điểm tùy loại bảng (A, I, K, niem_today, streak).
    pub score: i64,
    /// Cấp bậc hiện tại.
    pub user_rank: String,
    /// Vai trò (member/admin_ky_thuat/etc.).
    pub role: String,
}

impl LeaderboardEntry {
    /// Huy chương cho top 3.
    pub fn medal(&self) -> &'static str {
        match self.rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "",
        }
    }

    /// CSS class cho hàng top 3.
    pub fn row_class(&self) -> &'static str {
        match self.rank {
            1 => "bg-amber-50 border-amber-200",
            2 => "bg-gray-50 border-gray-200",
            3 => "bg-orange-50 border-orange-200",
            _ => "bg-white border-gray-100",
        }
    }

    /// Icon cấp bậc.
    pub fn rank_icon(&self) -> &'static str {
        match self.user_rank.as_str() {
            "normal" => "🍃",
            "common" => "🌿",
            "good" => "🌳",
            "very_good" => "🌲",
            "great" => "🎋",
            "excellent" => "🏆",
            "benevolent" => "🪷",
            "tycoon" => "👑",
            _ => "🌱",
        }
    }
}

/// Thống kê tổng quan cho trang Bảng Xếp Hạng.
#[derive(Debug, Clone, Default)]
pub struct LeaderboardSummary {
    pub total_users: i64,
    pub total_niem_all: i64,
    pub total_a: i64,
    pub total_i: i64,
    pub total_k: i64,
}

// ─── Template ───────────────────────────────────────────────────────────

/// Template cho trang /bang-xep-hang.
#[derive(Template)]
#[template(path = "bang-xep-hang/index.html")]
pub struct BangXepHangTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub entries: Vec<LeaderboardEntry>,
    /// Loại bảng đang xem: "a" | "i" | "k" | "today" | "streak".
    pub tab: String,
    pub summary: LeaderboardSummary,
}

// ─── Handlers ───────────────────────────────────────────────────────────

/// GET /bang-xep-hang — Trang Bảng Xếp Hạng.
pub async fn bang_xep_hang_index(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TabQuery>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let tab = query.tab.unwrap_or_else(|| "a".to_string());

    let entries = fetch_leaderboard(&state.pool, &tab, 50).await;
    let summary = fetch_summary(&state.pool).await;

    let html = BangXepHangTemplate {
        user,
        active_page: "bang_xep_hang".into(),
        entries,
        tab,
        summary,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (bang-xep-hang): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /api/bang-xep-hang/stats — JSON tổng quan.
pub async fn bang_xep_hang_stats_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let _user = get_user_from_session(&state.pool, &jar).await;

    let summary = fetch_summary(&state.pool).await;
    Json(serde_json::json!({
        "total_users": summary.total_users,
        "total_niem_all": summary.total_niem_all,
        "total_a": summary.total_a,
        "total_i": summary.total_i,
        "total_k": summary.total_k,
    }))
    .into_response()
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Lấy top N users cho leaderboard theo loại.
async fn fetch_leaderboard(pool: &sqlx::PgPool, tab: &str, limit: i64) -> Vec<LeaderboardEntry> {
    let sql = match tab {
        "i" => format!(
            "SELECT ROW_NUMBER() OVER (ORDER BY u.i_balance DESC)::BIGINT AS rank,
                    u.id AS user_id, u.display_name, u.avatar_url,
                    u.i_balance AS score, u.rank AS user_rank, u.role
             FROM users u WHERE u.is_active = true AND u.i_balance > 0
             ORDER BY u.i_balance DESC LIMIT {limit}"
        ),
        "k" => format!(
            "SELECT ROW_NUMBER() OVER (ORDER BY u.k_balance DESC)::BIGINT AS rank,
                    u.id AS user_id, u.display_name, u.avatar_url,
                    u.k_balance AS score, u.rank AS user_rank, u.role
             FROM users u WHERE u.is_active = true AND u.k_balance > 0
             ORDER BY u.k_balance DESC LIMIT {limit}"
        ),
        "today" => format!(
            "SELECT ROW_NUMBER() OVER (ORDER BY p.niem_count DESC)::BIGINT AS rank,
                    u.id AS user_id, u.display_name, u.avatar_url,
                    p.niem_count AS score, u.rank AS user_rank, u.role
             FROM users u
             JOIN practice_logs p ON p.user_id = u.id AND p.log_date = CURRENT_DATE
             WHERE u.is_active = true AND p.niem_count > 0
             ORDER BY p.niem_count DESC LIMIT {limit}"
        ),
        "streak" => {
            return fetch_streak_leaderboard(pool, limit).await;
        }
        _ => format!(
            "SELECT ROW_NUMBER() OVER (ORDER BY u.a_balance DESC)::BIGINT AS rank,
                    u.id AS user_id, u.display_name, u.avatar_url,
                    u.a_balance AS score, u.rank AS user_rank, u.role
             FROM users u WHERE u.is_active = true AND u.a_balance > 0
             ORDER BY u.a_balance DESC LIMIT {limit}"
        ),
    };

    sqlx::query_as::<_, LeaderboardEntry>(&sql)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| {
            log::error!("❌ Lỗi fetch leaderboard (tab={tab}): {e}");
            vec![]
        })
}

/// Tính streak leaderboard.
async fn fetch_streak_leaderboard(pool: &sqlx::PgPool, limit: i64) -> Vec<LeaderboardEntry> {
    #[derive(Debug, sqlx::FromRow)]
    struct UserWithStreak {
        user_id: Uuid,
        display_name: String,
        avatar_url: Option<String>,
        user_rank: String,
        role: String,
        streak: i64,
    }

    let sql = format!(
        "WITH daily AS (
            SELECT DISTINCT user_id, log_date
            FROM practice_logs
            WHERE niem_count > 0
              AND log_date >= CURRENT_DATE - INTERVAL '60 days'
        ),
        grouped AS (
            SELECT user_id, log_date,
                   log_date - (ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY log_date))::INT AS grp
            FROM daily
        ),
        streaks AS (
            SELECT user_id, grp, COUNT(*)::BIGINT AS streak_len,
                   MAX(log_date) AS last_day
            FROM grouped
            GROUP BY user_id, grp
        ),
        current_streaks AS (
            SELECT user_id, streak_len
            FROM streaks
            WHERE last_day >= CURRENT_DATE - INTERVAL '1 day'
        )
        SELECT cs.user_id, u.display_name, u.avatar_url, u.rank AS user_rank, u.role,
               cs.streak_len AS streak
        FROM current_streaks cs
        JOIN users u ON u.id = cs.user_id AND u.is_active = true
        ORDER BY cs.streak_len DESC
        LIMIT {limit}"
    );

    let rows: Vec<UserWithStreak> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| {
            log::error!("❌ Lỗi fetch streak leaderboard: {e}");
            vec![]
        });

    rows.into_iter()
        .enumerate()
        .map(|(i, r)| LeaderboardEntry {
            rank: (i + 1) as i64,
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.streak,
            user_rank: r.user_rank,
            role: r.role,
        })
        .collect()
}

/// Lấy thống kê tổng quan.
async fn fetch_summary(pool: &sqlx::PgPool) -> LeaderboardSummary {
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM users WHERE is_active = true")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_a: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(a_balance), 0)::BIGINT FROM users WHERE is_active = true")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_i: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(i_balance), 0)::BIGINT FROM users WHERE is_active = true")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_k: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(k_balance), 0)::BIGINT FROM users WHERE is_active = true")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_niem_all: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(niem_count), 0)::BIGINT FROM practice_logs")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    LeaderboardSummary {
        total_users,
        total_niem_all,
        total_a,
        total_i,
        total_k,
    }
}

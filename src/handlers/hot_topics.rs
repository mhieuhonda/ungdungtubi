//! Handlers cho Chủ Đề Nổi Bật + Trang Khám Phá — Giai đoạn 59 (v0.9.45).
//!
//! Routes:
//!   GET  /cong-dong/kham-pha         — Trang khám phá: hot topics + nhóm nổi bật + sách + nhạc
//!   POST /admin/cong-dong/tinh-hot-score — Admin trigger recalculate hot_score cho tất cả topics
//!   GET  /api/cong-dong/chu-de-noi-bat  — JSON API top hot topics
//!
//! Hot score algorithm (calculate_topic_hot_score SQL function trong migration 038):
//!   score = (comments_24h * 4 + comments_7d * 2 + group_size * 0.5) / (age_hours + 2)^0.5
//!   Topics có score cao → hiển thị đầu /cong-dong/kham-pha.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use askama::Template;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::{admin::render_forbidden, get_user_from_session};
use crate::models::user::User;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct HotTopicRow {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub group_id: Uuid,
    pub group_name: String,
    pub group_slug: String,
    pub group_logo_url: Option<String>,
    pub hot_score: f64,
    pub is_hot: bool,
    pub comment_count: i64,
    pub view_count: i64,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FeaturedGroupRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub cover_url: Option<String>,
    pub member_count: i64,
    pub topic_count: i64,
    pub is_featured: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PopularBookRow {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub author: Option<String>,
    pub cover_url: Option<String>,
    pub view_count: i64,
    pub flower_count: i64,
    pub review_count: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct RecentMusicRow {
    pub id: i64,
    pub title: String,
    pub youtube_id: Option<String>,
    pub audio_file_url: Option<String>,
    pub source_type: String,
    pub submitter_name: String,
    pub submitter_avatar: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
}

#[derive(Template)]
#[template(path = "cong-dong/kham-pha.html")]
pub struct KhamPhaTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub hot_topics: Vec<HotTopicRow>,
    pub featured_groups: Vec<FeaturedGroupRow>,
    pub popular_books: Vec<PopularBookRow>,
    pub recent_music: Vec<RecentMusicRow>,
}

/// GET /cong-dong/kham-pha — Trang khám phá.
pub async fn kham_pha_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    // Hot topics (top 10)
    let hot_topics: Vec<HotTopicRow> = sqlx::query_as(
        "SELECT t.id, t.title, t.body, t.author_id,
                u.display_name AS author_name, u.avatar_url AS author_avatar_url,
                t.group_id, g.name AS group_name, g.slug AS group_slug, g.logo_url AS group_logo_url,
                t.hot_score, t.is_hot,
                COALESCE(t.comment_count, 0)::BIGINT AS comment_count,
                COALESCE(t.view_count, 0)::BIGINT AS view_count,
                t.created_at, t.last_activity_at
         FROM topics t
         JOIN users u ON u.id = t.author_id
         JOIN groups g ON g.id = t.group_id
         WHERE t.is_active = true AND g.is_active = true
         ORDER BY t.is_hot DESC, t.hot_score DESC, t.created_at DESC
         LIMIT 10"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Featured groups
    let featured_groups: Vec<FeaturedGroupRow> = sqlx::query_as(
        "SELECT g.id, g.name, g.slug, g.description, g.logo_url, g.cover_url,
                (SELECT COUNT(*)::BIGINT FROM group_members gm WHERE gm.group_id = g.id AND gm.status = 'approved') AS member_count,
                (SELECT COUNT(*)::BIGINT FROM topics t WHERE t.group_id = g.id AND t.is_active = true) AS topic_count,
                COALESCE(g.is_featured, false) AS is_featured
         FROM groups g
         WHERE g.is_active = true
         ORDER BY g.is_featured DESC, member_count DESC, g.created_at DESC
         LIMIT 6"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Popular books (top 5 by view_count)
    let popular_books: Vec<PopularBookRow> = sqlx::query_as(
        "SELECT id, title, slug, author, cover_url,
                view_count, flower_count, review_count
         FROM books
         ORDER BY view_count DESC LIMIT 5"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Recent approved music
    let recent_music: Vec<RecentMusicRow> = sqlx::query_as(
        "SELECT ums.id, ums.title, ums.youtube_id, ums.audio_file_url, ums.source_type,
                u.display_name AS submitter_name, u.avatar_url AS submitter_avatar,
                ums.approved_at
         FROM user_music_submissions ums
         JOIN users u ON u.id = ums.user_id
         WHERE ums.status = 'approved' AND ums.approved_at IS NOT NULL
         ORDER BY ums.approved_at DESC LIMIT 5"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = KhamPhaTemplate {
        user,
        active_page: "kham-pha".into(),
        hot_topics,
        featured_groups,
        popular_books,
        recent_music,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kham-pha): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /admin/cong-dong/tinh-hot-score — Admin trigger recalculate hot_score.
pub async fn admin_recalculate_hot_scores(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_staff() {
        return render_forbidden(&user);
    }

    let topic_ids: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM topics WHERE is_active = true"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let total = topic_ids.len();
    let mut updated = 0u64;

    for (topic_id,) in &topic_ids {
        let score_result: Option<(f64,)> =
            sqlx::query_as("SELECT calculate_topic_hot_score($1)")
                .bind(topic_id)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();

        if let Some((score,)) = score_result {
            let _ = sqlx::query(
                "UPDATE topics SET hot_score = $1, hot_score_at = NOW(), updated_at = NOW() WHERE id = $2"
            )
            .bind(score)
            .bind(topic_id)
            .execute(&state.pool)
            .await;
            updated += 1;
        }
    }

    // Mark top 10 as is_hot=true
    let _ = sqlx::query("UPDATE topics SET is_hot = false WHERE is_hot = true").execute(&state.pool).await;
    let _ = sqlx::query(
        "UPDATE topics SET is_hot = true
         WHERE id IN (
             SELECT id FROM topics
             WHERE is_active = true AND hot_score > 0
             ORDER BY hot_score DESC LIMIT 10
         )"
    )
    .execute(&state.pool)
    .await;

    log::info!(
        "🔥 Admin {} trigger recalculate hot_score: {} topics, {} updated, top 10 marked hot",
        user.id, total, updated
    );

    Html(format!(
        "<!DOCTYPE html><html><body>\
         <h2>🔥 Đã tính lại hot_score cho {} topics ({} updated). Top 10 đã được mark là HOT.</h2>\
         <p><a href='/cong-dong/kham-pha'>Xem trang Khám Phá</a> | \
         <a href='/admin/cong-dong'>← Về dashboard</a></p>\
         </body></html>",
        total, updated
    ))
    .into_response()
}

/// GET /api/cong-dong/chu-de-noi-bat — JSON API top hot topics.
pub async fn api_hot_topics(State(state): State<AppState>, _jar: CookieJar) -> Response {
    let hot_topics: Vec<HotTopicRow> = sqlx::query_as(
        "SELECT t.id, t.title, t.body, t.author_id,
                u.display_name AS author_name, u.avatar_url AS author_avatar_url,
                t.group_id, g.name AS group_name, g.slug AS group_slug, g.logo_url AS group_logo_url,
                t.hot_score, t.is_hot,
                COALESCE(t.comment_count, 0)::BIGINT AS comment_count,
                COALESCE(t.view_count, 0)::BIGINT AS view_count,
                t.created_at, t.last_activity_at
         FROM topics t
         JOIN users u ON u.id = t.author_id
         JOIN groups g ON g.id = t.group_id
         WHERE t.is_active = true AND g.is_active = true
         ORDER BY t.is_hot DESC, t.hot_score DESC LIMIT 10"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    axum::response::Json(serde_json::json!({
        "success": true,
        "hot_topics": hot_topics
    }))
    .into_response()
}

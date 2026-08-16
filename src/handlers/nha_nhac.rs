//! Handlers cho Nhà Nhạc — Giai đoạn 40 (v0.9.35).
//!
//! Nhà Nhạc (Music House) — phòng KG-03 trong Không Gian.
//! Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx":
//!   - 5 thư mục nhạc: Niem · Thien · Dao · KhongLoi · CaNhan
//!   - 5 chế độ phát: SingleRepeat · Shuffle · RepeatAll · Loop · SleepTimer
//!   - Khi mở nhạc, thành viên trong Không Gian có thể nghe cùng
//!   - Nhạc Cộng Đồng: user submit YouTube links, admin approve/reject
//!
//! Routes:
//!   - GET  /khong-gian/nha-nhac                         — Trang Nhà Nhạc (player UI)
//!   - GET  /khong-gian/nha-nhac/{category}              — Lọc theo category
//!   - GET  /api/nha-nhac/tracks                         — JSON danh sách track
//!   - GET  /api/nha-nhac/tracks/{category}              — JSON track theo category
//!   - GET  /api/nha-nhac/preferences                    — JSON preferences của user
//!   - POST /api/nha-nhac/preferences                     — Update preferences (HTMX)
//!   - POST /api/nha-nhac/ca-nhan/them                    — Add track → playlist Cá Nhân
//!   - POST /api/nha-nhac/ca-nhan/xoa/{track_id}         — Remove track khỏi Cá Nhân
//!   - POST /api/nha-nhac/track/{track_id}/play          — Tăng play_count (analytics)
//!   - GET  /api/nha-nhac/stats                          — JSON stats cho dashboard
//!   - POST /api/nha-nhac/dang-nhac                      — User submit music (YouTube link)
//!   - GET  /admin/nha-nhac/dang-cho-duyet               — Admin view pending submissions
//!   - POST /admin/nha-nhac/dang-cho-duyet/{id}          — Admin approve/reject
//!   - GET  /api/nha-nhac/submissions                    — User's own submissions
//!   - GET  /api/nha-nhac/submissions/approved           — All approved community music
//!   - POST /api/nha-nhac/submission/{id}/play           — Increment play count

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Json, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use sqlx::PgPool;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::nha_nhac::{
    AddPersonalTrackForm, CategoryTab, MusicCategory, MusicPrefsForm, MusicTrack, NhaNhacStats,
    PlaybackMode, UserMusicPrefs,
    SubmitMusicForm, ReviewSubmissionForm, UserMusicSubmission, SubmissionWithUser,
};
use crate::models::user::User;

// ─── Template structs ─────────────────────────────────────────────────────

/// Template cho trang /khong-gian/nha-nhac.
#[derive(Template)]
#[template(path = "khong-gian/nha-nhac.html")]
pub struct NhaNhacTemplate {
    pub user: Option<User>,
    pub active_page: String,
    /// Category hiện tại (default: niem).
    pub current_category: String,
    /// Tracks theo category hiện tại (hoặc tất cả nếu category = "all").
    pub tracks: Vec<MusicTrack>,
    /// Preferences của user (hoặc default nếu chưa có).
    pub prefs: UserMusicPrefs,
    /// Stats Nhà Nhạc.
    pub stats: NhaNhacStats,
    /// Track đang play gần nhất (restore state).
    pub last_track: Option<MusicTrack>,
    /// Số track trong playlist Cá Nhân.
    pub personal_count: i64,
    // ─── Pre-computed cho template (tránh Askama `|` filter conflict) ───
    /// JSON serialization của tracks (cho Alpine x-data).
    pub tracks_json: String,
    /// ID track cuối cùng user nghe (0 nếu chưa có).
    pub last_track_id: i64,
    /// Playback mode string (single_repeat / shuffle / repeat_all / loop).
    pub playback_mode: String,
    /// Volume 0–100.
    pub volume: i32,
    /// Sleep timer minutes (0 nếu tắt).
    pub sleep_timer_minutes: i32,
    /// 5 category tabs (pre-computed icon/label/is_current).
    pub category_tabs: Vec<CategoryTab>,
    /// Icon của category hiện tại (cho header).
    pub current_category_icon: String,
    /// Display name của category hiện tại.
    pub current_category_display: String,
    /// Description của category hiện tại.
    pub current_category_description: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /khong-gian/nha-nhac — Trang Nhà Nhạc (mặc định show category niem).
pub async fn nha_nhac_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    render_nha_nhac(&state, &jar, MusicCategory::Niem).await
}

/// GET /khong-gian/nha-nhac/{category} — Lọc theo category.
pub async fn nha_nhac_category(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(category): Path<String>,
) -> Response {
    let cat = match MusicCategory::from_str(&category) {
        Some(c) => c,
        None => {
            return Redirect::to("/khong-gian/nha-nhac").into_response();
        }
    };
    render_nha_nhac(&state, &jar, cat).await
}

/// GET /api/nha-nhac/tracks — JSON tất cả track.
pub async fn nha_nhac_tracks_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };
    let _ = user; // auth only
    let tracks = fetch_all_tracks(&state.pool).await;
    Json(tracks).into_response()
}

/// GET /api/nha-nhac/tracks/{category} — JSON track theo category.
pub async fn nha_nhac_tracks_by_category_api(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(category): Path<String>,
) -> Response {
    let Some(_user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };
    let tracks = match MusicCategory::from_str(&category) {
        Some(MusicCategory::CaNhan) => {
            // CaNhan cần user context — trả empty ở đây, frontend sẽ gọi /api/nha-nhac/ca-nhan
            Vec::new()
        }
        Some(cat) => fetch_tracks_by_category(&state.pool, cat).await,
        None => Vec::new(),
    };
    Json(tracks).into_response()
}

/// GET /api/nha-nhac/preferences — JSON preferences của user.
pub async fn nha_nhac_prefs_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };
    let prefs = fetch_prefs(&state.pool, user.id).await;
    Json(prefs).into_response()
}

/// POST /api/nha-nhac/preferences — Update preferences (HTMX: trả OOB partial).
pub async fn nha_nhac_prefs_update(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<MusicPrefsForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    let (mode, volume, sleep, last_track) = match form.validate() {
        Some(v) => v,
        None => {
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Preferences không hợp lệ.
                </div>"#,
            )
            .into_response();
        }
    };

    if let Err(e) = upsert_prefs(&state.pool, user.id, mode, volume, sleep, last_track).await {
        log::error!("❌ nha_nhac_prefs_update: {e}");
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Lỗi lưu preferences — vui lòng thử lại.
            </div>"#,
        )
        .into_response();
    }

    // Trả về HTMX partial: success message + OOB update cho prefs indicator.
    let mode_label = mode
        .map(|m| format!("{} {}", m.icon(), m.display()))
        .unwrap_or_else(|| "—".to_string());
    let vol = volume.unwrap_or(70);
    let sleep_label = match sleep.flatten() {
        Some(mins) => format!("{mins} phút"),
        None => "Tắt".to_string(),
    };
    let html = format!(
        r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
            ✅ Đã cập nhật preferences: <strong>{mode_label}</strong> · Âm lượng <strong>{vol}%</strong> · Hẹn giờ <strong>{sleep_label}</strong>
        </div>"#
    );
    Html(html).into_response()
}

/// POST /api/nha-nhac/ca-nhan/them — Add track vào playlist Cá Nhân.
pub async fn nha_nhac_ca_nhan_add(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AddPersonalTrackForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    // Verify track tồn tại + active + public.
    let track_exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM music_tracks WHERE id = $1 AND is_active = true AND is_public = true",
    )
    .bind(form.track_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if track_exists.is_none() {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Track không tồn tại hoặc không công khai.
            </div>"#,
        )
        .into_response();
    }

    // Insert vào user_personal_tracks (UNIQUE constraint → idempotent).
    let result = sqlx::query(
        "INSERT INTO user_personal_tracks (user_id, track_id, sort_order)
         VALUES ($1, $2, COALESCE(
            (SELECT MAX(sort_order) + 1 FROM user_personal_tracks WHERE user_id = $1),
            0
         ))
         ON CONFLICT (user_id, track_id) DO NOTHING",
    )
    .bind(user.id)
    .bind(form.track_id)
    .execute(&state.pool)
    .await;

    let added = matches!(result, Ok(ref r) if r.rows_affected() > 0);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_personal_tracks WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = if added {
        format!(
            r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                ✅ Đã thêm vào Cá Nhân. Playlist hiện có <strong>{count}</strong> bài.
            </div>"#
        )
    } else {
        format!(
            r#"<div class="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-xl text-sm">
                ℹ️ Bài này đã có trong Cá Nhân rồi. Playlist hiện có <strong>{count}</strong> bài.
            </div>"#
        )
    };
    Html(html).into_response()
}

/// POST /api/nha-nhac/ca-nhan/xoa/{track_id} — Remove track khỏi Cá Nhân.
pub async fn nha_nhac_ca_nhan_remove(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(track_id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    let _ = sqlx::query(
        "DELETE FROM user_personal_tracks WHERE user_id = $1 AND track_id = $2",
    )
    .bind(user.id)
    .bind(track_id)
    .execute(&state.pool)
    .await;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_personal_tracks WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = format!(
        r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
            🗑️ Đã xoá khỏi Cá Nhân. Playlist hiện có <strong>{count}</strong> bài.
        </div>"#
    );
    Html(html).into_response()
}

/// POST /api/nha-nhac/track/{track_id}/play — Tăng play_count (analytics).
pub async fn nha_nhac_track_play(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(track_id): Path<i64>,
) -> Response {
    let Some(_user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    let _ = sqlx::query(
        "UPDATE music_tracks SET play_count = play_count + 1, updated_at = NOW()
         WHERE id = $1 AND is_active = true",
    )
    .bind(track_id)
    .execute(&state.pool)
    .await;

    Json(serde_json::json!({"ok": true, "track_id": track_id})).into_response()
}

/// GET /api/nha-nhac/stats — JSON stats cho dashboard.
pub async fn nha_nhac_stats_api(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };
    let stats = fetch_stats(&state.pool, Some(user.id)).await;
    Json(stats).into_response()
}

// ─── Internal helpers ─────────────────────────────────────────────────────

/// Render Nhà Nhạc template cho category đã cho.
async fn render_nha_nhac(
    state: &AppState,
    jar: &CookieJar,
    category: MusicCategory,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    // Tracks theo category (CaNhan → personal playlist).
    let tracks = if matches!(category, MusicCategory::CaNhan) {
        fetch_personal_tracks(&state.pool, user.id).await
    } else {
        fetch_tracks_by_category(&state.pool, category).await
    };

    let prefs = fetch_prefs(&state.pool, user.id).await;
    let stats = fetch_stats(&state.pool, Some(user.id)).await;

    // Restore last_track (nếu có).
    let last_track: Option<MusicTrack> = if let Some(track_id) = prefs.last_track_id {
        sqlx::query_as::<_, MusicTrack>(
            "SELECT * FROM music_tracks WHERE id = $1 AND is_active = true",
        )
        .bind(track_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let personal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_personal_tracks WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = NhaNhacTemplate {
        user: Some(user),
        active_page: "khong_gian".into(),
        current_category: category.db_value().to_string(),
        tracks: tracks.clone(),
        prefs: prefs.clone(),
        stats,
        last_track: last_track.clone(),
        personal_count,
        // Pre-serialize JSON để tránh Askama `|` filter conflict với Rust closure.
        tracks_json: serde_json::to_string(&tracks).unwrap_or_else(|_| "[]".to_string()),
        last_track_id: prefs.last_track_id.unwrap_or(0),
        playback_mode: prefs.playback_mode_enum().db_value().to_string(),
        volume: prefs.volume,
        sleep_timer_minutes: prefs.sleep_timer_minutes.unwrap_or(0),
        // Pre-compute category tabs + current category metadata.
        category_tabs: CategoryTab::all_tabs(category.db_value()),
        current_category_icon: category.icon().to_string(),
        current_category_display: category.display().to_string(),
        current_category_description: category.description().to_string(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (nha-nhac): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Fetch tất cả track (active + public).
async fn fetch_all_tracks(pool: &PgPool) -> Vec<MusicTrack> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT * FROM music_tracks
         WHERE is_active = true AND is_public = true
         ORDER BY category, sort_order, id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch track theo category (active + public).
async fn fetch_tracks_by_category(pool: &PgPool, cat: MusicCategory) -> Vec<MusicTrack> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT * FROM music_tracks
         WHERE is_active = true AND is_public = true AND category = $1
         ORDER BY sort_order, id",
    )
    .bind(cat.db_value())
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch playlist Cá Nhân của user — join music_tracks để lấy full info.
async fn fetch_personal_tracks(pool: &PgPool, user_id: uuid::Uuid) -> Vec<MusicTrack> {
    sqlx::query_as::<_, MusicTrack>(
        "SELECT t.* FROM music_tracks t
         JOIN user_personal_tracks pt ON pt.track_id = t.id
         WHERE pt.user_id = $1 AND t.is_active = true
         ORDER BY pt.sort_order, pt.added_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Fetch preferences của user — nếu chưa có, tạo default row (lazy insert).
async fn fetch_prefs(pool: &PgPool, user_id: uuid::Uuid) -> UserMusicPrefs {
    // Try fetch first.
    if let Some(p) = sqlx::query_as::<_, UserMusicPrefs>(
        "SELECT * FROM user_music_prefs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    {
        return p;
    }

    // Default — lazy insert (race-safe với ON CONFLICT).
    let _ = sqlx::query(
        "INSERT INTO user_music_prefs (user_id, playback_mode, volume, sleep_timer_minutes)
         VALUES ($1, 'repeat_all', 70, NULL)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await;

    // Fetch again.
    sqlx::query_as::<_, UserMusicPrefs>(
        "SELECT * FROM user_music_prefs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| UserMusicPrefs {
        user_id,
        playback_mode: PlaybackMode::default().db_value().to_string(),
        volume: 70,
        sleep_timer_minutes: None,
        last_track_id: None,
        updated_at: chrono::Utc::now(),
    })
}

/// Upsert preferences — update fields được cung cấp.
async fn upsert_prefs(
    pool: &PgPool,
    user_id: uuid::Uuid,
    mode: Option<PlaybackMode>,
    volume: Option<i32>,
    sleep: Option<Option<i32>>,
    last_track: Option<i64>,
) -> Result<(), sqlx::Error> {
    // Build dynamic SET clause — chỉ update fields được cung cấp.
    // Ưu tiên: insert default row nếu chưa có, sau đó update.
    sqlx::query(
        "INSERT INTO user_music_prefs (user_id, playback_mode, volume, sleep_timer_minutes, last_track_id)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id) DO UPDATE SET
            playback_mode       = COALESCE($6, user_music_prefs.playback_mode),
            volume              = COALESCE($7, user_music_prefs.volume),
            sleep_timer_minutes = COALESCE($8, user_music_prefs.sleep_timer_minutes),
            last_track_id       = COALESCE($9, user_music_prefs.last_track_id),
            updated_at          = NOW()",
    )
    .bind(user_id)
    .bind(mode.unwrap_or_default().db_value())
    .bind(volume.unwrap_or(70))
    .bind(sleep.flatten())
    .bind(last_track)
    // Args 6-9: explicit values for UPDATE (NULL → keep existing via COALESCE)
    .bind(mode.map(|m| m.db_value().to_string()))
    .bind(volume)
    .bind(sleep.flatten())
    .bind(last_track)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch stats Nhà Nhạc.
async fn fetch_stats(pool: &PgPool, user_id: Option<uuid::Uuid>) -> NhaNhacStats {
    let total_tracks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM music_tracks WHERE is_active = true AND is_public = true",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total_plays: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(play_count), 0) FROM music_tracks WHERE is_active = true",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Tracks by category.
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT category, COUNT(*)::BIGINT FROM music_tracks
         WHERE is_active = true AND is_public = true
         GROUP BY category ORDER BY category",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let personal_tracks: i64 = if let Some(uid) = user_id {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_personal_tracks WHERE user_id = $1",
        )
        .bind(uid)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    } else {
        0
    };

    NhaNhacStats {
        total_tracks,
        tracks_by_category: rows,
        personal_tracks,
        total_plays,
    }
}

// ─── User Music Submission Handlers ───────────────────────────────────

/// Template cho /admin/nha-nhac/dang-cho-duyet.
#[derive(Template)]
#[template(path = "admin/nha-nhac-pending.html")]
pub struct AdminMusicPendingTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub submissions: Vec<SubmissionWithUser>,
}

/// POST /api/nha-nhac/dang-nhac — User submits music (YouTube link).
pub async fn nha_nhac_submit_music(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<SubmitMusicForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    let (title, artist, cat, youtube_id, description) = match form.validate() {
        Ok(v) => v,
        Err(e) => {
            return Html(format!(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">⚠️ {e}</div>"#
            )).into_response();
        }
    };

    // Rate limit: max 5 submissions per user per day
    let today_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_music_submissions WHERE user_id = $1 AND created_at > NOW() - INTERVAL '1 day'"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if today_count >= 5 {
        return Html(
            r#"<div class="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Bạn đã đăng 5 bài hôm nay. Vui lòng đợi ngày mai.
            </div>"#
        ).into_response();
    }

    // Check duplicate YouTube ID by same user
    let dup: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_music_submissions WHERE user_id = $1 AND youtube_id = $2)"
    )
    .bind(user.id)
    .bind(&youtube_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if dup {
        return Html(
            r#"<div class="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-xl text-sm">
                ℹ️ Bạn đã đăng link này rồi.
            </div>"#
        ).into_response();
    }

    // Insert submission
    let result = sqlx::query(
        "INSERT INTO user_music_submissions (user_id, title, artist, category, youtube_url, youtube_id, description, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')"
    )
    .bind(user.id)
    .bind(&title)
    .bind(&artist)
    .bind(cat.db_value())
    .bind(form.youtube_url.trim())
    .bind(&youtube_id)
    .bind(&description)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Html(
            r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                ✅ Đã gửi bài hát! Vui lòng chờ admin duyệt.
            </div>"#
        ).into_response(),
        Err(e) => {
            log::error!("❌ nha_nhac_submit_music: {e}");
            Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi gửi bài — vui lòng thử lại.
                </div>"#
            ).into_response()
        }
    }
}

/// GET /admin/nha-nhac/dang-cho-duyet — Admin view pending submissions.
pub async fn admin_music_pending(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let submissions = sqlx::query_as::<_, SubmissionWithUser>(
        "SELECT ms.*, u.display_name AS submitter_name, u.avatar_url AS submitter_avatar
         FROM user_music_submissions ms
         JOIN users u ON ms.user_id = u.id
         WHERE ms.status = 'pending'
         ORDER BY ms.created_at ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = AdminMusicPendingTemplate {
        user: Some(user),
        active_page: "admin".into(),
        submissions,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin-music-pending): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// POST /admin/nha-nhac/dang-cho-duyet/{id} — Admin approve/reject.
pub async fn admin_music_review(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(sub_id): Path<i64>,
    Form(form): Form<ReviewSubmissionForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let (new_status, note) = match form.action.as_str() {
        "approve" => ("approved", form.note.unwrap_or_default()),
        "reject" => ("rejected", form.note.unwrap_or_default()),
        _ => return Redirect::to("/admin/nha-nhac/dang-cho-duyet").into_response(),
    };

    // Update submission status
    let result = sqlx::query(
        "UPDATE user_music_submissions SET status = $1, reviewed_by = $2, review_note = $3, reviewed_at = NOW(), updated_at = NOW()
         WHERE id = $4 AND status = 'pending'"
    )
    .bind(new_status)
    .bind(user.id)
    .bind(if note.is_empty() { None } else { Some(&note) })
    .bind(sub_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            // If approved, also add to music_tracks table so it appears in the main music library
            if new_status == "approved" {
                let _ = sqlx::query(
                    "INSERT INTO music_tracks (title, category, description, artist, audio_url, duration_seconds, is_public, upload_user_id, sort_order, is_active)
                     SELECT title, category, description, artist, youtube_url, 0, true, user_id,
                            COALESCE((SELECT MAX(sort_order) + 1 FROM music_tracks WHERE category = ms.category), 0),
                            true
                     FROM user_music_submissions ms WHERE ms.id = $1"
                )
                .bind(sub_id)
                .execute(&state.pool)
                .await;
            }
            log::info!("✅ Music submission #{sub_id} {new_status} by {}", user.display_name);
        }
        Ok(_) => log::warn!("⚠️ Music submission #{sub_id} not pending or not found"),
        Err(e) => log::error!("❌ Error reviewing music submission #{sub_id}: {e}"),
    }

    Redirect::to("/admin/nha-nhac/dang-cho-duyet").into_response()
}

/// GET /api/nha-nhac/submissions — User's own submissions.
pub async fn nha_nhac_my_submissions_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    let submissions: Vec<UserMusicSubmission> = sqlx::query_as(
        "SELECT * FROM user_music_submissions WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(submissions).into_response()
}

/// GET /api/nha-nhac/submissions/approved — All approved community music (for browsing).
pub async fn nha_nhac_community_music_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let Some(_user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    let submissions: Vec<UserMusicSubmission> = sqlx::query_as(
        "SELECT * FROM user_music_submissions WHERE status = 'approved' ORDER BY play_count DESC, created_at DESC LIMIT 50"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(submissions).into_response()
}

/// POST /api/nha-nhac/submission/{id}/play — Increment play count.
pub async fn nha_nhac_submission_play(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(sub_id): Path<i64>,
) -> Response {
    let Some(_user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    let _ = sqlx::query(
        "UPDATE user_music_submissions SET play_count = play_count + 1, updated_at = NOW() WHERE id = $1 AND status = 'approved'"
    )
    .bind(sub_id)
    .execute(&state.pool)
    .await;

    Json(serde_json::json!({"ok": true, "id": sub_id})).into_response()
}

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
    extract::{Multipart, Path, State},
    response::{Html, IntoResponse, Json, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use sqlx::PgPool;

// v0.9.43 — Giai đoạn 47: trace_id generation cho music submit error logging
// (build trace ID để admin dễ trace trong log khi user báo lỗi)
use chrono::Utc;

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
    // ─── v0.9.44 — Giai đoạn 48: My Submissions + auto-show community section ───
    /// JSON serialization của my_submissions (5 gần nhất của user, cho Alpine x-data).
    /// Sửa bug "admin duyệt nhưng user không biết" — giờ user thấy status trực tiếp
    /// trên trang Nhà Nhạc mà không cần vào trang khác.
    pub my_submissions_json: String,
    /// Số community tracks đã được admin duyệt. Nếu > 0, template auto-show
    /// nút "Nhạc Cộng Đồng" + section ngay khi load trang (trước v0.9.44 ẩn mặc định).
    pub approved_community_count: i64,
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

    // v0.9.44 — Giai đoạn 48: Check result, không nuốt im lặng (m13 fix).
    let result = sqlx::query(
        "DELETE FROM user_personal_tracks WHERE user_id = $1 AND track_id = $2",
    )
    .bind(user.id)
    .bind(track_id)
    .execute(&state.pool)
    .await;

    let removed = match result {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            log::error!("❌ nha_nhac_ca_nhan_remove: user={} track={track_id} error={e}", user.id);
            false
        }
    };

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_personal_tracks WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = if removed {
        format!(
            r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                🗑️ Đã xoá khỏi Cá Nhân. Playlist hiện có <strong>{count}</strong> bài.
            </div>"#
        )
    } else {
        format!(
            r#"<div class="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-xl text-sm">
                ℹ️ Bài này không có trong Cá Nhân (hoặc đã bị xoá). Playlist hiện có <strong>{count}</strong> bài.
            </div>"#
        )
    };
    Html(html).into_response()
}

/// POST /api/nha-nhac/track/{track_id}/play — Tăng play_count (analytics).
///
/// v0.9.44 — Giai đoạn 48 (M9 fix): Trước v0.9.44, handler này không validate
/// track_id có tồn tại hay không, và không có rate limit — user (hoặc bot) có
/// thể spam POST để inflate play_count. Giờ: validate track tồn tại + rate
/// limit 1 play/track/user/minute (dedup bằng IP+user+track trong Redis hoặc
/// memory cache). Vì chưa có Redis, dùng simple in-memory LRU cache (per-instance).
pub async fn nha_nhac_track_play(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(track_id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"error": "unauthorized"})).into_response();
    };

    // v0.9.44 — Validate track exists + active. Trước v0.9.44, `let _ =` nuốt error
    // nếu track_id không tồn tại → play_count không tăng nhưng client vẫn nhận `ok:true`.
    let track_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM music_tracks WHERE id = $1 AND is_active = true)"
    )
    .bind(track_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !track_exists {
        return Json(serde_json::json!({
            "error": "track_not_found",
            "message": "Track không tồn tại hoặc đã bị ẩn."
        })).into_response();
    }

    // v0.9.44 — Rate limit: 1 play/track/user/minute (simple in-memory).
    // Trước v0.9.44: không có rate limit → bot có thể spam.
    let key = (user.id, track_id);
    let now = std::time::Instant::now();
    let mut cache = state.play_count_cache.lock().await;
    if let Some(last) = cache.get(&key) {
        if now.duration_since(*last).as_secs() < 60 {
            // Rate limited — silent accept (không log user, không tăng count).
            return Json(serde_json::json!({
                "ok": true,
                "track_id": track_id,
                "rate_limited": true,
                "message": "Đã ghi lượt nghe trong vòng 1 phút qua."
            })).into_response();
        }
    }
    cache.insert(key, now);

    // Drop lock before DB call.
    drop(cache);

    let result = sqlx::query(
        "UPDATE music_tracks SET play_count = play_count + 1, updated_at = NOW()
         WHERE id = $1 AND is_active = true",
    )
    .bind(track_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "track_id": track_id})).into_response(),
        Err(e) => {
            log::error!("❌ nha_nhac_track_play #{track_id}: {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "db_error",
                "message": "Lỗi ghi lượt nghe."
            })).into_response()
        }
    }
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
///
/// v0.9.44 — Giai đoạn 48: Thêm `my_submissions` (3 gần nhất của user) và
/// `approved_community_count` để template auto-show community section nếu
/// có bài đã duyệt (trước v0.9.44, section này ẩn mặc định → user không biết
/// bấm vào để xem nhạc cộng đồng đã được duyệt).
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

    // v0.9.44 — Giai đoạn 48: 3 submissions gần nhất của user (status badges).
    // Fix bug "admin duyệt nhưng user không biết" — giờ user thấy status trực tiếp
    // trên trang Nhà Nhạc.
    let my_submissions: Vec<UserMusicSubmission> = sqlx::query_as(
        "SELECT * FROM user_music_submissions WHERE user_id = $1
         ORDER BY created_at DESC LIMIT 5"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // v0.9.44 — Đếm số community tracks đã duyệt để template biết có nên
    // auto-show community section hay không.
    let approved_community_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_music_submissions WHERE status = 'approved'"
    )
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
        // v0.9.44 — Giai đoạn 48: extra context for "My Submissions" + auto-show community.
        my_submissions_json: serde_json::to_string(&my_submissions)
            .unwrap_or_else(|_| "[]".to_string()),
        approved_community_count,
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
///
/// v0.9.44 — Giai đoạn 48: thêm `query_ok`, `query_err`, `sub_id_param` để
/// hiển thị banner kết quả sau khi admin approve/reject (redirect về với
/// ?ok=...&action=... hoặc ?err=...&sub=...).
#[derive(Template)]
#[template(path = "admin/nha-nhac-pending.html")]
pub struct AdminMusicPendingTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub submissions: Vec<SubmissionWithUser>,
    /// v0.9.44 — "approve" hoặc "reject" nếu redirect về thành công.
    pub query_ok: Option<String>,
    /// v0.9.44 — error code nếu redirect về với lỗi.
    pub query_err: Option<String>,
    /// v0.9.44 — ID của submission vừa được xử lý (cho banner).
    pub sub_id_param: Option<i64>,
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

    // v0.9.42 — Giai đoạn 46: Forbidden words auto-check
    let fw_result = crate::db::check_forbidden_words_multi(&state.pool, &[&title, &description]).await;
    if fw_result.should_block {
        log::warn!("🚫 Nhạc đăng bị chặn (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
        return Html(format!(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">⚠️ Nội dung chứa từ ngữ không phù hợp. Vui lòng sửa lại.</div>"#
        )).into_response();
    }
    if fw_result.should_flag {
        log::warn!("🚩 Nhạc đăng flagged (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
    }

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
    // v0.9.42 — Giai đoạn 46: Explicitly include source_type='youtube' instead of
    // relying on DB DEFAULT. Root cause fix cho "Lỗi gửi bài — không thể lưu bài hát
    // vào cơ sở dữ liệu" — nếu DB column chưa có DEFAULT (partial migration), INSERT
    // sẽ fail. Tường minh an toàn hơn.
    //
    // v0.9.43 — Giai đoạn 47: INSERT retry fallback. Nếu INSERT với source_type fail
    // (ColumnNotFound — migration 026 chưa chạy + safety schema chưa ensure được),
    // retry với INSERT không có source_type (rely on DB DEFAULT ''). Nếu DEFAULT
    // chưa được set, will fail nhưng sẽ log error chi tiết với trace ID để admin debug.
    let trace_id = format!("yt-{}", Utc::now().format("%Y%m%d%H%M%S"));
    let result = sqlx::query(
        "INSERT INTO user_music_submissions (user_id, title, artist, category, youtube_url, youtube_id, description, status, source_type)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', 'youtube')"
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

    // v0.9.43 — Giai đoạn 47: Retry fallback nếu source_type column not found
    let result = match result {
        Ok(r) => Ok(r),
        Err(sqlx::Error::ColumnNotFound(col)) if col.contains("source_type") => {
            log::warn!("⚠️ [trace={}] source_type column not found, retrying INSERT without source_type. Migration 026 chưa chạy — chạy safety schema sẽ fix.", trace_id);
            sqlx::query(
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
            .await
        }
        Err(e) => Err(e),
    };

    match result {
        Ok(_) => Html(
            r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                ✅ Đã gửi bài hát! Vui lòng chờ admin duyệt.
            </div>"#
        ).into_response(),
        Err(e) => {
            // v0.9.41 — Giai đoạn 45: Log error chi tiết + return JSON error code cho frontend
            // để dễ debug. Trước v0.9.41: chỉ hiển thị generic message, không có cách debug.
            // v0.9.41: log DB error reason, phân loại lỗi (ColumnNotFound / Database / Decode),
            // hiển thị error code cho user (frontend có thể expand để report admin).
            // v0.9.43 — Giai đoạn 47: Thêm trace_id để admin dễ trace trong log.
            log::error!("❌ [trace={}] nha_nhac_submit_music (YouTube) INSERT fail: {e}", trace_id);
            log::error!("   [trace={}] user_id={}, title='{}', category='{}', youtube_id='{}'",
                trace_id, user.id, title, cat.db_value(), youtube_id);
            let err_kind = match &e {
                sqlx::Error::ColumnNotFound(col) => format!("Thiếu cột DB: {col}"),
                sqlx::Error::Database(db_err) => {
                    let msg = db_err.message();
                    if msg.contains("violates unique constraint") {
                        "Bài hát đã tồn tại (trùng youtube_id)".to_string()
                    } else if msg.contains("violates check constraint") {
                        format!("Vi phạm ràng buộc DB: {msg}")
                    } else if msg.contains("relation") && msg.contains("does not exist") {
                        format!("Bảng DB không tồn tại: {msg}")
                    } else {
                        format!("Lỗi database: {msg}")
                    }
                }
                _ => format!("Lỗi không xác định: {e}"),
            };
            Html(format!(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi gửi bài — không thể lưu bài hát vào cơ sở dữ liệu.<br>
                    <span class="text-xs text-red-600">Chi tiết: {err_kind}</span><br>
                    <span class="text-xs text-red-500">Mã trace: {trace_id} — báo admin kèm mã này để được hỗ trợ nhanh.</span><br>
                    <span class="text-xs text-red-500">Vui lòng thử lại sau ít phút. Nếu lỗi tiếp diễn, liên hệ admin kỹ thuật kèm thông báo trên.</span>
                </div>"#
            )).into_response()
        }
    }
}

/// GET /admin/nha-nhac/dang-cho-duyet — Admin view pending submissions.
///
/// v0.9.44 — Giai đoạn 48: parse query string `?ok=...&action=approve` hoặc
/// `?err=...&sub=123` để hiển thị banner kết quả cho admin.
pub async fn admin_music_pending(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let submissions = sqlx::query_as::<_, SubmissionWithUser>(
        "SELECT ms.*, u.display_name AS submitter_name, u.avatar_url AS submitter_avatar,
                af.stored_filename AS audio_stored_filename
         FROM user_music_submissions ms
         JOIN users u ON ms.user_id = u.id
         LEFT JOIN audio_files af ON af.id = ms.audio_file_upload_id
         WHERE ms.status = 'pending'
         ORDER BY ms.created_at ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // v0.9.44 — Parse query params cho banner.
    let query_ok = params.get("ok").cloned();
    let query_err = params.get("err").cloned();
    let sub_id_param = params.get("sub").and_then(|s| s.parse::<i64>().ok());

    let html = AdminMusicPendingTemplate {
        user: Some(user),
        active_page: "admin".into(),
        submissions,
        query_ok,
        query_err,
        sub_id_param,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (admin-music-pending): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// POST /admin/nha-nhac/dang-cho-duyet/{id} — Admin approve/reject.
///
/// v0.9.44 — Giai đoạn 48: CRITICAL FIX (bug user complaint "admin duyệt nhưng
/// vào nhà nhạc không thấy nhạc nào được đăng").
///
/// Root cause được fix:
/// 1. Trước v0.9.44: `INSERT INTO music_tracks` được wrap trong `let _ =` →
///    nếu INSERT fail (constraint, schema drift, NULL violation, etc.), submission
///    vẫn được mark `approved` nhưng không có track nào xuất hiện trong music_tracks.
///    User vào lại Nhà Nhạc → không thấy gì. Admin cũng không biết INSERT fail.
/// 2. v0.9.44: Wrap UPDATE + INSERT trong cùng transaction. Nếu INSERT fail,
///    ROLLBACK → submission vẫn `pending`. Log error chi tiết + redirect với
///    banner error để admin biết. Nếu INSERT thành công, COMMIT + gửi notification
///    cho user biết bài đã được duyệt (or bị từ chối).
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

    // Fetch submission BEFORE mutation (for notification + validation).
    // Lấy user_id của submitter để gửi notification sau khi duyệt.
    let submission_info: Option<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT user_id, title, artist FROM user_music_submissions WHERE id = $1 AND status = 'pending'"
    )
    .bind(sub_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((submitter_id, sub_title, sub_artist)) = submission_info else {
        log::warn!("⚠️ Music submission #{sub_id} not pending or not found");
        return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=not_found").into_response();
    };

    // v0.9.44 — Transaction: UPDATE submission + INSERT music_tracks (nếu approve).
    // Nếu bất kỳ bước nào fail → ROLLBACK toàn bộ.
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ [admin_music_review] BEGIN tx fail: {e}");
            return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=db").into_response();
        }
    };

    // Step 1: UPDATE submission status.
    let update_result = sqlx::query(
        "UPDATE user_music_submissions SET status = $1, reviewed_by = $2, review_note = $3, reviewed_at = NOW(), updated_at = NOW()
         WHERE id = $4 AND status = 'pending'"
    )
    .bind(new_status)
    .bind(user.id)
    .bind(if note.is_empty() { None } else { Some(&note) })
    .bind(sub_id)
    .execute(&mut *tx)
    .await;

    let updated_rows = match update_result {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            log::error!("❌ [admin_music_review #{sub_id}] UPDATE fail: {e}");
            let _ = tx.rollback().await;
            return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=db").into_response();
        }
    };

    if updated_rows == 0 {
        log::warn!("⚠️ Music submission #{sub_id} not pending (race?)");
        let _ = tx.rollback().await;
        return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=not_pending").into_response();
    }

    // Step 2: Nếu approve, INSERT vào music_tracks (cùng transaction).
    if new_status == "approved" {
        // Fetch audio_url + duration từ submission (đã UPDATE trong cùng tx).
        let row: Option<(String, Option<String>, Option<i32>)> = sqlx::query_as(
            "SELECT ms.youtube_url, af.stored_filename, ms.audio_duration_seconds
             FROM user_music_submissions ms
             LEFT JOIN audio_files af ON af.id = ms.audio_file_upload_id
             WHERE ms.id = $1"
        )
        .bind(sub_id)
        .fetch_optional(&mut *tx)
        .await
        .ok()
        .flatten();

        let (audio_url, duration_secs) = match row {
            Some((_yt_url, Some(stored_filename), duration)) => {
                // audio_file: dùng URL file local
                let local_url = format!("{}/{stored_filename}", state.config.upload_url_prefix);
                (local_url, duration.unwrap_or(0))
            }
            Some((yt_url, None, _duration)) => {
                // youtube: dùng YouTube URL (trường hợp cũ)
                (yt_url, 0)
            }
            None => {
                // Submission không tồn tại (race?) — rollback
                log::error!("❌ [admin_music_review #{sub_id}] submission vanished after UPDATE");
                let _ = tx.rollback().await;
                return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=vanish").into_response();
            }
        };

        // INSERT vào music_tracks với error propagation (KHÔNG dùng `let _ =`).
        let insert_result = sqlx::query(
            "INSERT INTO music_tracks (title, category, description, artist, audio_url, duration_seconds, is_public, upload_user_id, sort_order, is_active)
             SELECT title, category, description, artist, $2, $3, true, user_id,
                    COALESCE((SELECT MAX(sort_order) + 1 FROM music_tracks WHERE category = ms.category), 0),
                    true
             FROM user_music_submissions ms WHERE ms.id = $1"
        )
        .bind(sub_id)
        .bind(&audio_url)
        .bind(duration_secs)
        .execute(&mut *tx)
        .await;

        if let Err(e) = insert_result {
            // v0.9.44 CRITICAL FIX: Log error + rollback, KHÔNG nuốt im lặng.
            log::error!(
                "❌ [admin_music_review #{sub_id}] INSERT music_tracks FAIL — rolling back approval.\
                \n   audio_url='{}' (len={})\
                \n   duration_secs={}\
                \n   error: {}",
                audio_url,
                audio_url.len(),
                duration_secs,
                e
            );
            let _ = tx.rollback().await;
            return Redirect::to(&format!(
                "/admin/nha-nhac/dang-cho-duyet?err=insert_fail&sub={sub_id}"
            )).into_response();
        }
    }

    // COMMIT transaction.
    if let Err(e) = tx.commit().await {
        log::error!("❌ [admin_music_review #{sub_id}] COMMIT fail: {e}");
        return Redirect::to("/admin/nha-nhac/dang-cho-duyet?err=commit").into_response();
    }

    // v0.9.44 — Giai đoạn 48: Gửi notification cho user khi bài được duyệt hoặc bị từ chối.
    // Trước v0.9.44, user không biết bài của mình đã được xử lý đến khi tự vào lại Nhà Nhạc.
    let notif_type = if new_status == "approved" { "music_approved" } else { "music_rejected" };
    let notif_msg = if new_status == "approved" {
        format!("🎵 Bài nhạc \"{}\" của bạn đã được duyệt!", sub_title)
    } else {
        let reason = if note.is_empty() { "không có lý do cụ thể" } else { note.as_str() };
        format!("🎵 Bài nhạc \"{}\" của bạn bị từ chối. Lý do: {}", sub_title, reason)
    };

    let _ = sqlx::query(
        "INSERT INTO notifications (user_id, type, actor_id, payload)
         VALUES ($1, $2, $3, $4)"
    )
    .bind(submitter_id)
    .bind(notif_type)
    .bind(user.id)
    .bind(serde_json::json!({
        "submission_id": sub_id,
        "submission_title": sub_title,
        "submission_artist": sub_artist,
        "status": new_status,
        "reviewer_name": user.display_name,
        "note": if note.is_empty() { None } else { Some(note.as_str()) },
        "message": notif_msg
    }))
    .execute(&state.pool)
    .await;

    log::info!(
        "✅ Music submission #{sub_id} {new_status} by {} (submitter={submitter_id}, title=\"{sub_title}\")",
        user.display_name
    );

    Redirect::to(&format!("/admin/nha-nhac/dang-cho-duyet?ok={new_status}&sub={sub_id}")).into_response()
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

// ─── v0.9.36 — Giai đoạn 41: Audio File Upload ───────────────────────────

/// POST /api/nha-nhac/dang-nhac-file — User uploads an audio file (MP3/M4A/OGG/WAV/FLAC).
///
/// Multipart form fields:
///   - `file` (binary): audio file (max 20 MB)
///   - `title` (text): song title (required, max 200 chars)
///   - `artist` (text): artist name (required, max 100 chars)
///   - `category` (text): one of niem/thien/dao/khong_loi (required)
///   - `description` (text): optional, max 500 chars
///
/// Returns HTMX partial (success or error message).
pub async fn nha_nhac_submit_music_file(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/khong-gian/nha-nhac").into_response();
    };

    // 1. Read multipart: file (binary) + text fields (title, artist, category, description)
    let max_audio_bytes = crate::handlers::uploads::MAX_AUDIO_BYTES;
    let (file_bytes, original_name, detected_mime, text_fields) =
        match crate::handlers::uploads::read_multipart_audio_file(&mut multipart, max_audio_bytes).await {
            Ok(result) => result,
            Err(resp) => return resp,
        };

    if file_bytes.is_empty() {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Không nhận được file âm thanh. Vui lòng chọn file (MP3, M4A, OGG, WAV, FLAC).
            </div>"#,
        )
        .into_response();
    }

    // 2. Validate MIME type (audio only)
    let mime = detected_mime
        .as_deref()
        .map_or_else(String::new, |m| crate::handlers::uploads::parse_mime(m).unwrap_or_default());
    let Some(ext) = crate::handlers::uploads::audio_mime_to_ext(&mime) else {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Định dạng không được hỗ trợ. Chỉ chấp nhận: MP3, M4A, OGG, WAV, FLAC.
            </div>"#,
        )
        .into_response();
    };

    // 3. Validate text fields (title, artist, category)
    let title = text_fields.get("title").map(|s| s.trim().to_string()).unwrap_or_default();
    if title.is_empty() {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Tiêu đề không được để trống.
            </div>"#,
        )
        .into_response();
    }
    if title.chars().count() > 200 {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Tiêu đề tối đa 200 ký tự.
            </div>"#,
        )
        .into_response();
    }

    let artist = text_fields.get("artist").map(|s| s.trim().to_string()).unwrap_or_default();
    if artist.is_empty() {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Nghệ sĩ không được để trống.
            </div>"#,
        )
        .into_response();
    }
    if artist.chars().count() > 100 {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Nghệ sĩ tối đa 100 ký tự.
            </div>"#,
        )
        .into_response();
    }

    let category_str = text_fields.get("category").map(|s| s.trim().to_string()).unwrap_or_default();
    let Some(cat) = MusicCategory::from_str(&category_str) else {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Thư mục nhạc không hợp lệ. Chọn: niem, thien, dao, khong_loi.
            </div>"#,
        )
        .into_response();
    };
    if matches!(cat, MusicCategory::CaNhan) {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Không thể đăng vào thư mục Cá Nhân.
            </div>"#,
        )
        .into_response();
    }

    let description = text_fields
        .get("description")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if description.chars().count() > 500 {
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Mô tả tối đa 500 ký tự.
            </div>"#,
        )
        .into_response();
    }

    // 4. Rate limit: max 5 submissions per user per day (same as YouTube)
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
            </div>"#,
        )
        .into_response();
    }

    // 5. Check duplicate by SHA-256 (same audio file already submitted by same user)
    let sha256_str = crate::handlers::uploads::compute_sha256(&file_bytes);
    let dup: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM user_music_submissions ms
            JOIN audio_files af ON af.id = ms.audio_file_upload_id
            WHERE ms.user_id = $1 AND af.sha256 = $2
        )"
    )
    .bind(user.id)
    .bind(&sha256_str)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if dup {
        return Html(
            r#"<div class="bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-xl text-sm">
                ℹ️ Bạn đã đăng file này rồi.
            </div>"#,
        )
        .into_response();
    }

    // 6. Ensure upload_dir exists
    if let Err(e) = std::fs::create_dir_all(&state.config.upload_dir) {
        log::error!("❌ Không tạo được upload_dir (audio): {e}");
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Lỗi server — không tạo được thư mục lưu trữ.
            </div>"#,
        )
        .into_response();
    }

    // 7. Save audio file to filesystem: <uuid>.<ext>
    let file_id = uuid::Uuid::new_v4();
    let stored_filename = format!("{file_id}.{ext}");
    let file_path = state.config.upload_dir.join(&stored_filename);

    if let Err(e) = std::fs::write(&file_path, &file_bytes) {
        log::error!("❌ Lỗi ghi file âm thanh: {e}");
        return Html(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                ⚠️ Lỗi server — không lưu được file âm thanh.
            </div>"#,
        )
        .into_response();
    }

    // 8. Estimate duration (simple bitrate-based estimate)
    let duration_seconds = crate::handlers::uploads::estimate_audio_duration_seconds(file_bytes.len(), &mime);

    // 9. Insert into audio_files table
    let audio_file_id = match crate::handlers::uploads::insert_audio_metadata(
        &state.pool,
        file_id,
        user.id,
        original_name.as_ref(),
        &stored_filename,
        &mime,
        &file_bytes,
        &sha256_str,
        duration_seconds,
        "music_submission",
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("❌ Lỗi insert audio_files: {e}");
            // Cleanup file
            let _ = std::fs::remove_file(&file_path);
            return Html(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi server — không ghi được metadata file âm thanh.
                </div>"#,
            )
            .into_response();
        }
    };

    // 10. Insert submission with source_type='audio_file'
    // youtube_url = local URL (for backward compat with music_tracks insert on approval)
    // youtube_id = "LOCAL-{file_id}" (placeholder, 11+ chars, no clash with real YouTube IDs)
    let local_url = format!("{}/{stored_filename}", state.config.upload_url_prefix);
    let placeholder_id = format!("LOCAL-{file_id}");

    let result = sqlx::query(
        "INSERT INTO user_music_submissions
            (user_id, title, artist, category, youtube_url, youtube_id, description, status,
             source_type, audio_file_upload_id, audio_duration_seconds)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', 'audio_file', $8, $9)"
    )
    .bind(user.id)
    .bind(&title)
    .bind(&artist)
    .bind(cat.db_value())
    .bind(&local_url)
    .bind(&placeholder_id)
    .bind(&description)
    .bind(audio_file_id)
    .bind(duration_seconds)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            log::info!("🎵 User {} uploaded audio file: {} ({} bytes, {})", user.id, stored_filename, file_bytes.len(), mime);
            Html(
                r#"<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-xl text-sm">
                    ✅ Đã tải lên file âm thanh! Vui lòng chờ admin duyệt.
                </div>"#,
            )
            .into_response()
        }
        Err(e) => {
            // v0.9.41 — Giai đoạn 45: Log error chi tiết + phân loại + hiển thị error
            // chi tiết cho user (frontend có thể report admin kèm thông tin).
            // Trước v0.9.41: chỉ hiển thị generic message, không có cách debug.
            log::error!("❌ Lỗi insert user_music_submissions (audio): {e}");
            log::error!("   user_id={}, title='{}', category='{}', audio_file_id={}",
                user.id, title, cat.db_value(), audio_file_id);
            let err_kind = match &e {
                sqlx::Error::ColumnNotFound(col) => format!("Thiếu cột DB: {col}"),
                sqlx::Error::Database(db_err) => {
                    let msg = db_err.message();
                    if msg.contains("violates unique constraint") {
                        "File âm thanh đã tồn tại".to_string()
                    } else if msg.contains("violates check constraint") {
                        format!("Vi phạm ràng buộc DB: {msg}")
                    } else if msg.contains("relation") && msg.contains("does not exist") {
                        format!("Bảng DB không tồn tại: {msg}")
                    } else {
                        format!("Lỗi database: {msg}")
                    }
                }
                _ => format!("Lỗi không xác định: {e}"),
            };
            // Cleanup file + audio_files row
            let _ = std::fs::remove_file(&file_path);
            let _ = sqlx::query("DELETE FROM audio_files WHERE id = $1")
                .bind(audio_file_id)
                .execute(&state.pool)
                .await;
            Html(format!(
                r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">
                    ⚠️ Lỗi gửi bài — không thể lưu bài hát vào cơ sở dữ liệu.<br>
                    <span class="text-xs text-red-600">Chi tiết: {err_kind}</span><br>
                    <span class="text-xs text-red-500">Vui lòng thử lại sau ít phút. Nếu lỗi tiếp diễn, liên hệ admin kỹ thuật kèm thông báo trên.</span>
                </div>"#,
            ))
            .into_response()
        }
    }
}

//! Handlers cho trang Admin Analytics Dashboard — Giai đoạn 52 (v0.9.44).
//!
//! Trang `/admin/thong-ke` hiển thị 6 metric chính để admin theo dõi sức khoẻ
//! nền tảng:
//!   1. 📊 DAU 30 ngày               — số user active mỗi ngày (last_seen_at)
//!   2. ✨ Signups 30 ngày            — số user đăng ký mới mỗi ngày
//!   3. 🎵 Top 10 tracks phát nhiều nhất — từ music_tracks.play_count
//!   4. 👥 Top 10 nhóm hoạt động       — từ groups.topic_count
//!   5. 🎨 Danh mục nhạc cộng đồng    — từ user_music_submissions WHERE approved
//!   6. 💱 Khối lượng quy đổi 7 ngày   — từ balance_transactions exchange_in/out
//!
//! Thiết kế:
//!   * Mỗi chart dùng **pure CSS bar/line** (không JS chart lib) → giữ bundle nhỏ.
//!   * Mọi SQL dùng `generate_series` để đảm bảo series đầy đủ 30/7 ngày
//!     kể cả ngày không có dữ liệu (zero count) — chart luôn có đủ thanh.
//!   * Mọi query dùng `bind()` parameterised — không string-concat user input.
//!   * Dùng `chrono::Local::now()` cho date math (TZ=Asia/Ho_Chi_Minh per
//!     v0.9.39 fix — đồng bộ với Postgres CURRENT_DATE).
//!   * Admin-only: `user.is_admin()`. Non-admin redirect về `/`.
//!
//! Routes:
//!   - GET /admin/thong-ke                  — Dashboard 6 cards (HTML)
//!   - GET /admin/thong-ke/csv/{metric}     — Tải CSV theo metric
//!     metric ∈ {dau, signups, top_tracks, top_groups, music_categories, exchange}

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;
use sqlx::FromRow;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Data rows ────────────────────────────────────────────────────────────

/// Row cho các query `date_str + count` (DAU, signups, exchange volume).
#[derive(Debug, Clone, FromRow)]
struct DateCountRow {
    date_str: String,
    user_count: i64,
}

/// Row cho các query `name + count` (top tracks, top groups, music categories).
#[derive(Debug, Clone, FromRow)]
struct NameCountRow {
    name: String,
    cnt: i64,
}

// ─── SQL constants ────────────────────────────────────────────────────────

/// DAU 30 ngày — đếm user có `last_seen_at` trong ngày đó.
/// `generate_series` đảm bảo 30 dòng kể cả ngày 0 user.
const SQL_DAU_30: &str = r#"
SELECT to_char(d.day, 'DD/MM') AS date_str,
       COUNT(u.id) AS user_count
FROM generate_series(
    (CURRENT_DATE - INTERVAL '29 days')::timestamptz,
    CURRENT_DATE::timestamptz,
    INTERVAL '1 day'
) AS d(day)
LEFT JOIN users u
  ON date_trunc('day', u.last_seen_at) = d.day
GROUP BY d.day
ORDER BY d.day ASC
"#;

/// New user signups 30 ngày — đếm user có `created_at` trong ngày đó.
const SQL_SIGNUPS_30: &str = r#"
SELECT to_char(d.day, 'DD/MM') AS date_str,
       COUNT(u.id) AS user_count
FROM generate_series(
    (CURRENT_DATE - INTERVAL '29 days')::timestamptz,
    CURRENT_DATE::timestamptz,
    INTERVAL '1 day'
) AS d(day)
LEFT JOIN users u
  ON date_trunc('day', u.created_at) = d.day
GROUP BY d.day
ORDER BY d.day ASC
"#;

/// Top 10 tracks phát nhiều nhất — từ `music_tracks.play_count`.
const SQL_TOP_TRACKS: &str = r#"
SELECT title AS name, play_count AS cnt
FROM music_tracks
WHERE is_active = true
ORDER BY play_count DESC
LIMIT 10
"#;

/// Top 10 nhóm hoạt động — từ `groups.topic_count` (counter denormalised).
const SQL_TOP_GROUPS: &str = r#"
SELECT name, topic_count AS cnt
FROM groups
WHERE is_active = true
ORDER BY topic_count DESC
LIMIT 10
"#;

/// Danh mục nhạc cộng đồng — count theo `category` của submissions approved.
const SQL_MUSIC_CATEGORIES: &str = r#"
SELECT category AS name, COUNT(*) AS cnt
FROM user_music_submissions
WHERE status = 'approved'
GROUP BY category
ORDER BY cnt DESC
"#;

/// Khối lượng quy đổi 7 ngày — tổng ABS(amount) của exchange_in/out theo ngày.
const SQL_EXCHANGE_7D: &str = r#"
SELECT to_char(d.day, 'DD/MM') AS date_str,
       COALESCE(SUM(ABS(bt.amount)), 0) AS user_count
FROM generate_series(
    (CURRENT_DATE - INTERVAL '6 days')::timestamptz,
    CURRENT_DATE::timestamptz,
    INTERVAL '1 day'
) AS d(day)
LEFT JOIN balance_transactions bt
  ON date_trunc('day', bt.created_at) = d.day
  AND bt.tx_type IN ('exchange_in', 'exchange_out')
GROUP BY d.day
ORDER BY d.day ASC
"#;

// ─── Fetch helpers ────────────────────────────────────────────────────────

async fn fetch_dau_30(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, DateCountRow>(SQL_DAU_30)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(|r| (r.date_str, r.user_count)).collect())
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: DAU 30d query fail: {e}");
            vec![]
        })
}

async fn fetch_signups_30(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, DateCountRow>(SQL_SIGNUPS_30)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(|r| (r.date_str, r.user_count)).collect())
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: Signups 30d query fail: {e}");
            vec![]
        })
}

async fn fetch_top_tracks(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, NameCountRow>(SQL_TOP_TRACKS)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(|r| (r.name, r.cnt)).collect())
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: Top tracks query fail: {e}");
            vec![]
        })
}

async fn fetch_top_groups(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, NameCountRow>(SQL_TOP_GROUPS)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(|r| (r.name, r.cnt)).collect())
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: Top groups query fail: {e}");
            vec![]
        })
}

async fn fetch_music_categories(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, NameCountRow>(SQL_MUSIC_CATEGORIES)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|r| (category_label(&r.name).to_string(), r.cnt))
                .collect()
        })
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: Music categories query fail: {e}");
            vec![]
        })
}

async fn fetch_exchange_7d(pool: &sqlx::PgPool) -> Vec<(String, i64)> {
    sqlx::query_as::<_, DateCountRow>(SQL_EXCHANGE_7D)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(|r| (r.date_str, r.user_count)).collect())
        .unwrap_or_else(|e| {
            log::error!("❌ thong_ke: Exchange 7d query fail: {e}");
            vec![]
        })
}

/// Map category code → Vietnamese label hiển thị.
fn category_label(code: &str) -> &'static str {
    match code {
        "niem" => "📿 Nhạc Niệm",
        "thien" => "🧘 Nhạc Thiền",
        "dao" => "🛕 Nhạc Đạo",
        "khong_loi" => "🎵 Không Lời",
        _ => "Khác",
    }
}

/// Format i64 với dấu phẩy hàng nghìn (vd: 12345 → "12,345").
fn fmt_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let len = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

// ─── Trait adapters cho askama method calls ───────────────────────────────
//
// Askama generate method call code với auto-reference (vd: `(&self).fmt(&count)`)
// → tham số có thể là `i64`, `&i64`, hoặc `&&i64`. Trait `I64Like` / `UsizeLike`
// thống nhất tất cả các biến thể này về `i64` / `usize` để method signature
// chấp nhận mọi cách askama truyền tham số.

/// Adapter: chấp nhận `i64`, `&i64`, `&&i64`, ... → `i64`.
pub trait I64Like {
    fn to_i64(self) -> i64;
}
impl I64Like for i64 {
    fn to_i64(self) -> i64 {
        self
    }
}
impl I64Like for &i64 {
    fn to_i64(self) -> i64 {
        *self
    }
}
impl I64Like for &&i64 {
    fn to_i64(self) -> i64 {
        **self
    }
}
impl I64Like for &&&i64 {
    fn to_i64(self) -> i64 {
        ***self
    }
}

/// Adapter: chấp nhận `usize`, `&usize`, `&&usize`, ... → `usize`.
pub trait UsizeLike {
    fn to_usize(self) -> usize;
}
impl UsizeLike for usize {
    fn to_usize(self) -> usize {
        self
    }
}
impl UsizeLike for &usize {
    fn to_usize(self) -> usize {
        *self
    }
}
impl UsizeLike for &&usize {
    fn to_usize(self) -> usize {
        **self
    }
}

// ─── Template ─────────────────────────────────────────────────────────────

/// Template cho trang `/admin/thong-ke`.
#[derive(Template)]
#[template(path = "admin/thong-ke.html")]
pub struct ThongKeTemplate {
    pub user: Option<User>,
    pub active_page: String,
    /// DAU 30 ngày — (date "DD/MM", count users active that day).
    pub dau_30_days: Vec<(String, i64)>,
    /// Signups 30 ngày — (date "DD/MM", count new users that day).
    pub signups_30_days: Vec<(String, i64)>,
    /// Top 10 tracks — (title, play_count).
    pub top_tracks: Vec<(String, i64)>,
    /// Top 10 groups — (group name, topic_count).
    pub top_groups: Vec<(String, i64)>,
    /// Music categories — (label, count) từ user_music_submissions approved.
    pub music_categories: Vec<(String, i64)>,
    /// Exchange volume 7 ngày — (date "DD/MM", sum abs amount).
    pub exchange_volume_7d: Vec<(String, i64)>,
    /// Khi Server render fail → set true để hiện notice "không có dữ liệu".
    pub has_error: bool,
}

impl ThongKeTemplate {
    /// Max count trong DAU series — dùng để scale vertical bar height (0-100%).
    pub fn dau_max(&self) -> i64 {
        self.dau_30_days.iter().map(|(_, c)| *c).max().unwrap_or(0)
    }

    /// Max count trong Signups series.
    pub fn signups_max(&self) -> i64 {
        self.signups_30_days
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0)
    }

    /// Max play_count trong top tracks — dùng để scale horizontal bar width.
    pub fn top_tracks_max(&self) -> i64 {
        self.top_tracks.iter().map(|(_, c)| *c).max().unwrap_or(0)
    }

    /// Max topic_count trong top groups.
    pub fn top_groups_max(&self) -> i64 {
        self.top_groups.iter().map(|(_, c)| *c).max().unwrap_or(0)
    }

    /// Max volume trong exchange series.
    pub fn exchange_max(&self) -> i64 {
        self.exchange_volume_7d
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0)
    }

    /// Tổng count của tất cả categories — dùng để tính % cho pie chart.
    pub fn music_categories_total(&self) -> i64 {
        self.music_categories.iter().map(|(_, c)| *c).sum()
    }

    /// Phần trăm (0-100) của category theo index — cho pie chart display.
    ///
    /// Generic over `UsizeLike` để chấp nhận `usize`, `&usize`, `&&usize`
    /// (askama truyền `loop.index0` bằng reference đôi khi qua match ergonomics).
    pub fn category_percent<T: UsizeLike>(&self, idx: T) -> u64 {
        let idx = idx.to_usize();
        let total = self.music_categories_total();
        if total <= 0 {
            return 0;
        }
        let count = self
            .music_categories
            .get(idx)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        ((count as f64 / total as f64) * 100.0).round() as u64
    }

    /// Màu cho category theo index — cycle qua palette purple/indigo/violet.
    pub fn category_color<T: UsizeLike>(&self, idx: T) -> &'static str {
        const PALETTE: [&str; 6] = [
            "#7c3aed", // violet-600
            "#6366f1", // indigo-500
            "#8b5cf6", // violet-500
            "#a855f7", // purple-500
            "#9333ea", // purple-600
            "#5b21b6", // violet-900
        ];
        let idx = idx.to_usize();
        PALETTE[idx % PALETTE.len()]
    }

    /// Format count với dấu phẩy — dùng trong tooltip / label của bar chart.
    ///
    /// Generic over `I64Like` để chấp nhận cả `i64` (khi gọi `fmt(dau_max())`)
    /// lẫn `&i64` / `&&i64` (khi askama iterate tuple và truyền `count` bằng
    /// reference qua match ergonomics).
    pub fn fmt<T: I64Like>(&self, n: T) -> String {
        fmt_thousands(n.to_i64())
    }

    /// Tính % chiều cao (1-100) cho bar DAU — tránh 0% làm thanh biến mất hoàn toàn.
    pub fn dau_bar_height<T: I64Like>(&self, count: T) -> u64 {
        let (count, max) = (count.to_i64(), self.dau_max());
        if max <= 0 {
            return 0;
        }
        let pct = ((count as f64 / max as f64) * 100.0).round() as u64;
        if pct == 0 && count > 0 {
            1
        } else {
            pct
        }
    }

    /// Tính % chiều cao cho bar Signups.
    pub fn signups_bar_height<T: I64Like>(&self, count: T) -> u64 {
        let (count, max) = (count.to_i64(), self.signups_max());
        if max <= 0 {
            return 0;
        }
        let pct = ((count as f64 / max as f64) * 100.0).round() as u64;
        if pct == 0 && count > 0 {
            1
        } else {
            pct
        }
    }

    /// Tính % chiều cao cho bar Exchange.
    pub fn exchange_bar_height<T: I64Like>(&self, count: T) -> u64 {
        let (count, max) = (count.to_i64(), self.exchange_max());
        if max <= 0 {
            return 0;
        }
        let pct = ((count as f64 / max as f64) * 100.0).round() as u64;
        if pct == 0 && count > 0 {
            1
        } else {
            pct
        }
    }

    /// Tính % chiều rộng cho horizontal bar (top tracks).
    pub fn top_tracks_bar_width<T: I64Like>(&self, count: T) -> u64 {
        let (count, max) = (count.to_i64(), self.top_tracks_max());
        if max <= 0 {
            return 0;
        }
        let pct = ((count as f64 / max as f64) * 100.0).round() as u64;
        if pct == 0 && count > 0 {
            2
        } else {
            pct.max(2)
        }
    }

    /// Tính % chiều rộng cho horizontal bar (top groups).
    pub fn top_groups_bar_width<T: I64Like>(&self, count: T) -> u64 {
        let (count, max) = (count.to_i64(), self.top_groups_max());
        if max <= 0 {
            return 0;
        }
        let pct = ((count as f64 / max as f64) * 100.0).round() as u64;
        if pct == 0 && count > 0 {
            2
        } else {
            pct.max(2)
        }
    }
}

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /admin/thong-ke — Admin Analytics Dashboard (HTML).
///
/// Auth: phải là admin (`user.is_admin()`). Non-admin → redirect `/`.
pub async fn admin_thong_ke_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    // Fetch 6 datasets song song (tokio::join! để chạy concurrent).
    let (dau, signups, top_tracks, top_groups, music_cats, exchange) = tokio::join!(
        fetch_dau_30(&state.pool),
        fetch_signups_30(&state.pool),
        fetch_top_tracks(&state.pool),
        fetch_top_groups(&state.pool),
        fetch_music_categories(&state.pool),
        fetch_exchange_7d(&state.pool),
    );

    let has_error = dau.is_empty()
        && signups.is_empty()
        && top_tracks.is_empty()
        && top_groups.is_empty()
        && music_cats.is_empty()
        && exchange.is_empty();

    let tpl = ThongKeTemplate {
        user: Some(user),
        active_page: "admin".into(),
        dau_30_days: dau,
        signups_30_days: signups,
        top_tracks,
        top_groups,
        music_categories: music_cats,
        exchange_volume_7d: exchange,
        has_error,
    };

    let html = tpl.render().unwrap_or_else(|e| {
        log::error!("Template render error (admin thong-ke): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /admin/thong-ke/csv/{metric} — Tải CSV của metric tương ứng.
///
/// Auth: admin only. `metric` ∈ {dau, signups, top_tracks, top_groups,
/// music_categories, exchange}. Unknown → 404 plain text.
pub async fn admin_thong_ke_csv(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(metric): Path<String>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }
    // Suppress unused warning — `user` chỉ cần thiết để check quyền.
    let _ = &user;

    let metric_norm = metric.trim().to_lowercase();
    let (csv_body, filename): (String, String) = match metric_norm.as_str() {
        "dau" => {
            let rows = fetch_dau_30(&state.pool).await;
            (
                build_date_csv("Ngày (DD/MM)", "Số user active", &rows),
                "dau_stats.csv".into(),
            )
        }
        "signups" => {
            let rows = fetch_signups_30(&state.pool).await;
            (
                build_date_csv("Ngày (DD/MM)", "Số user đăng ký mới", &rows),
                "signups_stats.csv".into(),
            )
        }
        "top_tracks" => {
            let rows = fetch_top_tracks(&state.pool).await;
            (
                build_name_csv("Tên track", "Lượt phát (play_count)", &rows),
                "top_tracks_stats.csv".into(),
            )
        }
        "top_groups" => {
            let rows = fetch_top_groups(&state.pool).await;
            (
                build_name_csv("Tên nhóm", "Số chủ đề (topic_count)", &rows),
                "top_groups_stats.csv".into(),
            )
        }
        "music_categories" => {
            let rows = fetch_music_categories(&state.pool).await;
            (
                build_name_csv("Danh mục", "Số bài approved", &rows),
                "music_categories_stats.csv".into(),
            )
        }
        "exchange" => {
            let rows = fetch_exchange_7d(&state.pool).await;
            (
                build_date_csv(
                    "Ngày (DD/MM)",
                    "Khối lượng quy đổi (tổng abs amount)",
                    &rows,
                ),
                "exchange_stats.csv".into(),
            )
        }
        _ => {
            return Html(format!(
                r#"<html><body><h1>404 — Metric không hợp lệ</h1>
                <p>Metric '{metric_norm}' không được hỗ trợ.</p>
                <p>Các metric hợp lệ: dau, signups, top_tracks, top_groups, music_categories, exchange</p>
                <p><a href="/admin/thong-ke">← Về trang Thống Kê</a></p>
                </body></html>"#
            ))
            .into_response();
        }
    };

    let disposition = format!("attachment; filename=\"{filename}\"");
    let content_type = HeaderValue::from_static("text/csv; charset=utf-8");
    let content_disp = HeaderValue::from_str(&disposition).unwrap_or_else(|_| {
        HeaderValue::from_static("attachment; filename=\"stats.csv\"")
    });

    // ([(HeaderName, HeaderValue); N], body) → axum IntoResponse.
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, content_disp),
        ],
        csv_body,
    )
        .into_response()
}

// ─── CSV builder helpers ──────────────────────────────────────────────────

/// Escape một cell CSV: nếu chứa comma/newline/quote → wrap trong `"..."` và
/// escape `"` thành `""`. Tuân thủ RFC 4180.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('\n') || s.contains('\r') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Build CSV cho date-series data: header `date_col,count_col` + N rows.
fn build_date_csv(date_col: &str, count_col: &str, rows: &[(String, i64)]) -> String {
    let mut out = String::with_capacity(rows.len() * 24 + 64);
    out.push_str(&format!("{},{},\n", csv_escape(date_col), csv_escape(count_col)));
    for (date, count) in rows {
        out.push_str(&format!("{},{},\n", csv_escape(date), count));
    }
    out
}

/// Build CSV cho name-series data: header `name_col,count_col` + N rows.
fn build_name_csv(name_col: &str, count_col: &str, rows: &[(String, i64)]) -> String {
    let mut out = String::with_capacity(rows.len() * 48 + 64);
    out.push_str(&format!("{},{},\n", csv_escape(name_col), csv_escape(count_col)));
    for (name, count) in rows {
        out.push_str(&format!("{},{},\n", csv_escape(name), count));
    }
    out
}

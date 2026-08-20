//! Handlers cho Giai đoạn 71 (v0.9.47) — Nhật Ký Tu Học (Practice Diary).
//!
//! Theo tài liệu HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx mục I.2 (Nhà Nhật Ký):
//!   - Nhật Ký Tu Học: Tương tự trang cá nhân trên Facebook, là nơi thành viên
//!     ghi lại bút ký và cảm ngộ trong quá trình tu học.
//!   - Có thể cài công khai hoặc riêng tư.
//!   - Nếu cho phép bình luận thì người khác có thể bình luận (mặc định chỉ
//!     bạn bè được bình luận — hiện thực hóa ở giai đoạn này: bật/tắt comment).
//!
//! Routes (xem src/main.rs):
//!   GET  /nhat-ky-tu-hoc              — Trang list diaries (của user + public feed)
//!   POST /nhat-ky-tu-hoc/tao          — Tạo diary mới
//!   GET  /nhat-ky-tu-hoc/{id}         — Xem chi tiết diary + comments
//!   POST /nhat-ky-tu-hoc/{id}/xoa     — Xóa diary của chính mình
//!   POST /nhat-ky-tu-hoc/{id}/binh-luan — Thêm bình luận vào diary
//!   POST /nhat-ky-tu-hoc/{id}/an-binh-luan/{cmt_id} — Admin/Mod ẩn bình luận

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::{get_user_from_session, html_escape};

// ════════════════════════════════════════════════════════════════════════
// MODELS
// ════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PracticeDiary {
    pub id: i64,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub mood: String,
    pub is_public: bool,
    pub allow_comments: bool,
    pub view_count: i32,
    pub comment_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields (display only):
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_phap_danh: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DiaryComment {
    pub id: i64,
    pub diary_id: i64,
    pub user_id: Uuid,
    pub content: String,
    pub is_hidden: bool,
    pub created_at: DateTime<Utc>,
    // Joined:
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════
// HELPERS
// ════════════════════════════════════════════════════════════════════════

fn mood_label(mood: &str) -> &'static str {
    match mood {
        "peace" => "😌 Bình an",
        "joy" => "😊 Hoan hỷ",
        "gratitude" => "🙏 Tri ân",
        "repentance" => "🪷 Sám hối",
        "dedication" => "🪷 Hồi hướng",
        "reflection" => "🧘 Chiêm nghiệm",
        _ => "🪷",
    }
}

fn time_ago(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let dur = now.signed_duration_since(dt);
    let mins = dur.num_minutes();
    if mins < 1 {
        return "vừa xong".into();
    }
    if mins < 60 {
        return format!("{mins} phút trước");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours} giờ trước");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days} ngày trước");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months} tháng trước");
    }
    format!("{} năm trước", months / 12)
}

fn excerpt(s: &str, max_chars: usize) -> String {
    let truncated: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn author_display(name: &str, phap_danh: &Option<String>) -> String {
    if let Some(pd) = phap_danh {
        if !pd.trim().is_empty() {
            return pd.clone();
        }
    }
    name.to_string()
}

fn require_login(jar: &CookieJar, state: &AppState, next: &str) -> Response {
    let _ = (jar, state, next);
    Redirect::to(&format!("/dang-nhap?next={next}")).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// HANDLERS
// ════════════════════════════════════════════════════════════════════════

/// GET /nhat-ky-tu-hoc — Trang list diaries của user + public feed.
pub async fn nhat_ky_tu_hoc_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let is_logged_in = user.is_some();

    // Lấy 20 diary public gần nhất
    let public_diaries: Vec<PracticeDiary> = sqlx::query_as::<_, PracticeDiary>(
        "SELECT d.id, d.user_id, d.title, d.content, d.mood, d.is_public, d.allow_comments,
                d.view_count, d.comment_count, d.created_at, d.updated_at,
                u.display_name AS author_name,
                u.avatar_url   AS author_avatar,
                u.phap_danh    AS author_phap_danh
         FROM practice_diaries d
         JOIN users u ON u.id = d.user_id
         WHERE d.is_public = true AND u.is_active = true
         ORDER BY d.created_at DESC
         LIMIT 20"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Lấy 10 diary của user hiện tại (nếu login)
    let my_diaries: Vec<PracticeDiary> = if let Some(ref u) = user {
        sqlx::query_as::<_, PracticeDiary>(
            "SELECT d.id, d.user_id, d.title, d.content, d.mood, d.is_public, d.allow_comments,
                    d.view_count, d.comment_count, d.created_at, d.updated_at,
                    u.display_name AS author_name,
                    u.avatar_url   AS author_avatar,
                    u.phap_danh    AS author_phap_danh
             FROM practice_diaries d
             JOIN users u ON u.id = d.user_id
             WHERE d.user_id = $1
             ORDER BY d.created_at DESC
             LIMIT 10"
        )
        .bind(u.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let public_feed_html = if public_diaries.is_empty() {
        r#"<div class="text-center py-12 bg-white rounded-2xl border border-dashed border-gray-200">
            <span class="text-5xl">📖</span>
            <p class="text-gray-500 mt-3">Chưa có bút ký nào được chia sẻ công khai.</p>
            <p class="text-gray-400 text-sm mt-1">Hãy là người đầu tiên viết bút ký tu học!</p>
        </div>"#.to_string()
    } else {
        let mut html = String::from(r#"<div class="space-y-4">"#);
        for d in &public_diaries {
            let author = author_display(d.author_name.as_deref().unwrap_or("Ẩn danh"), &d.author_phap_danh);
            let avatar = if let Some(ref av) = d.author_avatar {
                format!(r#"<img src="{av}" alt="avatar" class="w-10 h-10 rounded-full object-cover" referrerpolicy="no-referrer">"#)
            } else {
                let initial = author.chars().next().unwrap_or('🪷');
                format!(r#"<div class="w-10 h-10 rounded-full bg-gradient-to-br from-tubi-500 to-tubi-700 flex items-center justify-center text-white font-bold">{initial}</div>"#)
            };
            let mood = mood_label(&d.mood);
            let excerpt_str = excerpt(&d.content, 220);
            let time = time_ago(d.created_at);
            let esc_title = html_escape(&d.title);
            let esc_excerpt = html_escape(&excerpt_str);
            let esc_author = html_escape(&author);
            let privacy_badge = if d.is_public {
                r#"<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-emerald-50 text-emerald-700">🌍 Công khai</span>"#
            } else {
                r#"<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-gray-100 text-gray-600">🔒 Riêng tư</span>"#
            };
            html.push_str(&format!(
                r##"<a href="/nhat-ky-tu-hoc/{id}" class="block bg-white rounded-2xl border border-gray-200 hover:border-tubi-400 hover:shadow-md transition-all p-5 max-w-full overflow-hidden">
                    <div class="flex items-start gap-3">
                        <div class="shrink-0">{avatar}</div>
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center gap-2 mb-1 flex-wrap">
                                <span class="font-semibold text-gray-900 break-words">{esc_author}</span>
                                <span class="text-xs text-gray-400">·</span>
                                <span class="text-xs text-gray-500">{time}</span>
                                <span class="text-xs text-gray-400">·</span>
                                <span class="text-xs">{mood}</span>
                                <span class="text-xs text-gray-400">·</span>
                                {privacy_badge}
                            </div>
                            <h3 class="font-bold text-gray-900 mb-1 break-words">{esc_title}</h3>
                            <p class="text-sm text-gray-600 break-words line-clamp-3">{esc_excerpt}</p>
                            <div class="flex items-center gap-4 mt-3 text-xs text-gray-400">
                                <span>👁️ {view_count} lượt xem</span>
                                <span>💬 {comment_count} bình luận</span>
                            </div>
                        </div>
                    </div>
                </a>"##,
                id = d.id,
                avatar = avatar,
                time = time,
                mood = mood,
                privacy_badge = privacy_badge,
                esc_author = esc_author,
                esc_title = esc_title,
                esc_excerpt = esc_excerpt,
                view_count = d.view_count,
                comment_count = d.comment_count,
            ));
        }
        html.push_str("</div>");
        html
    };

    let my_diaries_html = if is_logged_in {
        if my_diaries.is_empty() {
            r#"<div class="text-center py-8 bg-white rounded-2xl border border-dashed border-gray-200">
                <p class="text-gray-500 text-sm">Bạn chưa viết bút ký nào. Hãy viết entry đầu tiên!</p>
            </div>"#.to_string()
        } else {
            let mut html = String::from(r#"<div class="space-y-2">"#);
            for d in &my_diaries {
                let mood = mood_label(&d.mood);
                let time = time_ago(d.created_at);
                let esc_title = html_escape(&d.title);
                let privacy_badge = if d.is_public {
                    r#"🌍"# 
                } else { 
                    r#"🔒"#
                };
                html.push_str(&format!(
                    r##"<a href="/nhat-ky-tu-hoc/{id}" class="block bg-white rounded-xl border border-gray-200 hover:border-tubi-400 hover:shadow-sm transition p-3">
                        <div class="flex items-center justify-between gap-2">
                            <div class="min-w-0 flex-1">
                                <div class="font-semibold text-gray-900 text-sm truncate">{privacy} {title}</div>
                                <div class="text-xs text-gray-400 mt-0.5">{mood} · {time} · 💬 {cmt} · 👁 {views}</div>
                            </div>
                            <span class="text-gray-300">›</span>
                        </div>
                    </a>"##,
                    id = d.id, privacy = privacy_badge, title = esc_title, mood = mood, time = time,
                    cmt = d.comment_count, views = d.view_count,
                ));
            }
            html.push_str("</div>");
            html
        }
    } else {
        String::new()
    };

    // Write form (only for logged-in users)
    let write_form_html = if is_logged_in {
        r##"<div class="bg-white rounded-2xl border border-gray-200 shadow-sm p-5 sm:p-6 mb-8">
            <h2 class="text-lg font-bold text-gray-900 mb-2">✍️ Viết bút ký tu học</h2>
            <p class="text-sm text-gray-500 mb-4">Ghi lại cảm ngộ, bút ký và hành trình tu học của bạn. Mọi người có thể đọc và bình luận (nếu bạn cho phép).</p>
            <form action="/nhat-ky-tu-hoc/tao" method="POST" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Tiêu đề <span class="text-red-500">*</span></label>
                    <input type="text" name="title" required maxlength="200" placeholder="VD: Cảm ngộ sau một tuần niệm Phật"
                           class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-tubi-400 focus:border-transparent text-sm">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Nội dung <span class="text-red-500">*</span></label>
                    <textarea name="content" required rows="6" maxlength="10000" placeholder="Viết bút ký, cảm ngộ, hoặc nhật ký tu học của bạn..."
                              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-tubi-400 focus:border-transparent text-sm"></textarea>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Tâm trạng</label>
                        <select name="mood" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                            <option value="peace">😌 Bình an</option>
                            <option value="joy">😊 Hoan hỷ</option>
                            <option value="gratitude">🙏 Tri ân</option>
                            <option value="repentance">🪷 Sám hối</option>
                            <option value="dedication">🪷 Hồi hướng</option>
                            <option value="reflection">🧘 Chiêm nghiệm</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Hiển thị</label>
                        <select name="is_public" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                            <option value="true">🌍 Công khai</option>
                            <option value="false">🔒 Riêng tư</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Bình luận</label>
                        <select name="allow_comments" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
                            <option value="true">✅ Cho phép</option>
                            <option value="false">🚫 Tắt</option>
                        </select>
                    </div>
                </div>
                <div class="flex flex-col sm:flex-row gap-2">
                    <button type="submit" class="bg-tubi-700 hover:bg-tubi-800 text-white font-semibold px-5 py-2.5 rounded-xl transition text-sm" style="background-color:#2E7D32">
                        📝 Đăng bút ký
                    </button>
                    <a href="/nhat-ky-tu-hoc" class="text-center text-sm text-gray-500 hover:text-gray-700 px-5 py-2.5">Huỷ</a>
                </div>
            </form>
        </div>"##.to_string()
    } else {
        r##"<div class="bg-gradient-to-br from-tubi-50 to-amber-50 rounded-2xl border border-tubi-200 p-6 mb-8 text-center">
            <span class="text-4xl">📖</span>
            <h2 class="text-lg font-bold text-gray-900 mt-2">Đăng nhập để viết bút ký tu học</h2>
            <p class="text-sm text-gray-600 mt-1 mb-4">Ghi lại hành trình tu học, chia sẻ cảm ngộ với cộng đồng Từ Bi.</p>
            <a href="/auth/google" class="inline-block bg-tubi-700 hover:bg-tubi-800 text-white font-semibold px-5 py-2.5 rounded-xl transition text-sm" style="background-color:#2E7D32">🪷 Đăng Nhập</a>
        </div>"##.to_string()
    };

    let my_section_html = if is_logged_in {
        format!(
            r##"<div class="mb-10">
                <h2 class="text-xl font-bold text-gray-800 mb-3">📚 Bút ký của tôi</h2>
                {my_diaries_html}
            </div>"##
        )
    } else {
        String::new()
    };

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>📖 Nhật Ký Tu Học — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-gray-50 min-h-screen">
<section class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 overflow-x-hidden">
    <nav class="text-sm text-gray-500 mb-4">
        <a href="/khong-gian" class="hover:text-tubi-700">🌍 Không Gian</a>
        <span class="mx-2">›</span>
        <span class="text-gray-700">📖 Nhật Ký Tu Học</span>
    </nav>

    <div class="text-white rounded-2xl p-5 sm:p-8 shadow-lg mb-8 max-w-full overflow-hidden" style="background:linear-gradient(135deg,#2E7D32,#1B5E20)">
        <h1 class="text-2xl sm:text-3xl font-bold mb-2 break-words">📖 Nhật Ký Tu Học</h1>
        <p class="opacity-90 text-sm sm:text-base break-words">
            Ghi lại bút ký, cảm ngộ và hành trình tu học — chia sẻ với cộng đồng Từ Bi.<br>
            "Mỗi dòng viết là một bước chánh niệm. Mỗi lượt đọc là một duyên lành."
        </p>
    </div>

    {write_form_html}

    {my_section_html}

    <div>
        <h2 class="text-xl font-bold text-gray-800 mb-3">🌐 Bút ký cộng đồng</h2>
        {public_feed_html}
    </div>

    <p class="text-center text-xs text-gray-400 mt-8">
        📖 Nhật Ký Tu Học — v0.9.47 · Giai đoạn 71
    </p>
</section>
</body></html>"##,
        write_form_html = write_form_html,
        my_section_html = my_section_html,
        public_feed_html = public_feed_html,
    );

    Html(html).into_response()
}

/// POST /nhat-ky-tu-hoc/tao — Tạo diary mới.
#[derive(Debug, serde::Deserialize)]
pub struct CreateDiaryForm {
    pub title: String,
    pub content: String,
    pub mood: String,
    pub is_public: String,
    pub allow_comments: String,
}

pub async fn nhat_ky_tu_hoc_create(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CreateDiaryForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return require_login(&jar, &state, "/nhat-ky-tu-hoc");
    };

    let title = form.title.trim();
    let content = form.content.trim();
    if title.len() < 3 {
        return error_response("Tiêu đề quá ngắn (tối thiểu 3 ký tự).", "/nhat-ky-tu-hoc");
    }
    if content.len() < 10 {
        return error_response("Nội dung quá ngắn (tối thiểu 10 ký tự).", "/nhat-ky-tu-hoc");
    }
    let mood = match form.mood.as_str() {
        "peace" | "joy" | "gratitude" | "repentance" | "dedication" | "reflection" => form.mood.clone(),
        _ => "peace".to_string(),
    };
    let is_public = form.is_public == "true";
    let allow_comments = form.allow_comments == "true";

    let result = sqlx::query(
        "INSERT INTO practice_diaries (user_id, title, content, mood, is_public, allow_comments)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user.id)
    .bind(title)
    .bind(content)
    .bind(&mood)
    .bind(is_public)
    .bind(allow_comments)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Redirect::to("/nhat-ky-tu-hoc").into_response(),
        Err(e) => {
            log::error!("❌ nhat_ky_tu_hoc_create: {e}");
            error_response("Không thể tạo bút ký. Vui lòng thử lại.", "/nhat-ky-tu-hoc")
        }
    }
}

/// GET /nhat-ky-tu-hoc/{id} — Xem chi tiết diary + comments.
pub async fn nhat_ky_tu_hoc_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let diary: Option<PracticeDiary> = sqlx::query_as::<_, PracticeDiary>(
        "SELECT d.id, d.user_id, d.title, d.content, d.mood, d.is_public, d.allow_comments,
                d.view_count, d.comment_count, d.created_at, d.updated_at,
                u.display_name AS author_name,
                u.avatar_url   AS author_avatar,
                u.phap_danh    AS author_phap_danh
         FROM practice_diaries d
         JOIN users u ON u.id = d.user_id
         WHERE d.id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(diary) = diary else {
        return not_found_response();
    };

    // Privacy check: if not public, only author can view
    let is_author = user.as_ref().is_some_and(|u| u.id == diary.user_id);
    let is_staff = user.as_ref().is_some_and(|u| u.is_staff());
    if !diary.is_public && !is_author && !is_staff {
        return not_found_response();
    }

    // Increment view count (only if not author viewing own diary)
    if !is_author {
        let _ = sqlx::query("UPDATE practice_diaries SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await;
    }

    // Load comments
    let comments: Vec<DiaryComment> = sqlx::query_as::<_, DiaryComment>(
        "SELECT c.id, c.diary_id, c.user_id, c.content, c.is_hidden, c.created_at,
                u.display_name AS author_name,
                u.avatar_url   AS author_avatar
         FROM diary_comments c
         JOIN users u ON u.id = c.user_id
         WHERE c.diary_id = $1
         ORDER BY c.created_at ASC
         LIMIT 100"
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let comments_html = if diary.allow_comments {
        if comments.is_empty() {
            r#"<p class="text-center text-gray-400 text-sm py-6">Chưa có bình luận nào. Hãy là người đầu tiên bình luận!</p>"#.to_string()
        } else {
            let mut html = String::from(r#"<div class="space-y-3">"#);
            for c in &comments {
                let author = c.author_name.as_deref().unwrap_or("Ẩn danh");
                let avatar = if let Some(ref av) = c.author_avatar {
                    format!(r#"<img src="{av}" alt="avatar" class="w-8 h-8 rounded-full object-cover" referrerpolicy="no-referrer">"#)
                } else {
                    let initial = author.chars().next().unwrap_or('🪷');
                    format!(r#"<div class="w-8 h-8 rounded-full bg-gradient-to-br from-tubi-500 to-tubi-700 flex items-center justify-center text-white text-xs font-bold">{initial}</div>"#)
                };
                let esc_author = html_escape(author);
                let esc_content = html_escape(&c.content);
                let time = time_ago(c.created_at);
                let hidden_badge = if c.is_hidden {
                    r#"<span class="text-[10px] px-1.5 py-0.5 rounded-full bg-red-50 text-red-600 ml-2">đã ẩn</span>"#
                } else {
                    ""
                };
                html.push_str(&format!(
                    r##"<div class="flex items-start gap-3 bg-white rounded-xl border border-gray-100 p-3">
                        <div class="shrink-0">{avatar}</div>
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center gap-2 mb-1 flex-wrap">
                                <span class="font-semibold text-sm text-gray-900">{esc_author}</span>
                                <span class="text-xs text-gray-400">·</span>
                                <span class="text-xs text-gray-500">{time}</span>
                                {hidden_badge}
                            </div>
                            <p class="text-sm text-gray-700 break-words">{esc_content}</p>
                        </div>
                    </div>"##,
                    avatar = avatar, esc_author = esc_author, time = time, hidden_badge = hidden_badge,
                    esc_content = esc_content,
                ));
            }
            html.push_str("</div>");
            html
        }
    } else {
        r#"<p class="text-center text-gray-400 text-sm py-6">Tác giả đã tắt bình luận cho bút ký này.</p>"#.to_string()
    };

    // Comment form (only if logged in + allow_comments + not own diary)
    let comment_form_html = if let Some(ref _u) = user {
        if diary.allow_comments {
            r##"<form action="/nhat-ky-tu-hoc/{id}/binh-luan" method="POST" class="mt-6 bg-white rounded-xl border border-gray-200 p-4">
                <label class="block text-sm font-medium text-gray-700 mb-2">💬 Viết bình luận</label>
                <textarea name="content" required rows="3" maxlength="2000" placeholder="Chia sẻ suy nghĩ của bạn..."
                          class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-tubi-400 focus:border-transparent"></textarea>
                <button type="submit" class="mt-2 bg-tubi-700 hover:bg-tubi-800 text-white font-semibold px-4 py-2 rounded-lg text-sm transition" style="background-color:#2E7D32">
                    Gửi bình luận
                </button>
            </form>"##.replace("{id}", &id.to_string())
        } else {
            String::new()
        }
    } else {
        r##"<div class="mt-6 bg-tubi-50 rounded-xl border border-tubi-200 p-4 text-center">
            <p class="text-sm text-gray-600 mb-2">Đăng nhập để bình luận</p>
            <a href="/auth/google" class="inline-block bg-tubi-700 hover:bg-tubi-800 text-white font-semibold px-4 py-2 rounded-lg text-sm transition" style="background-color:#2E7D32">🪷 Đăng Nhập</a>
        </div>"##.to_string()
    };

    // Author info
    let author_name = author_display(
        diary.author_name.as_deref().unwrap_or("Ẩn danh"),
        &diary.author_phap_danh,
    );
    let avatar = if let Some(ref av) = diary.author_avatar {
        format!(r#"<img src="{av}" alt="avatar" class="w-12 h-12 rounded-full object-cover" referrerpolicy="no-referrer">"#)
    } else {
        let initial = author_name.chars().next().unwrap_or('🪷');
        format!(r#"<div class="w-12 h-12 rounded-full bg-gradient-to-br from-tubi-500 to-tubi-700 flex items-center justify-center text-white font-bold text-lg">{initial}</div>"#)
    };

    let mood = mood_label(&diary.mood);
    let time = time_ago(diary.created_at);
    let esc_title = html_escape(&diary.title);
    let esc_content = html_escape(&diary.content);
    let esc_author = html_escape(&author_name);

    // Author actions (delete own diary)
    let author_actions = if is_author {
        format!(
            r##"<form action="/nhat-ky-tu-hoc/{id}/xoa" method="POST" class="inline" onsubmit="return confirm('Xoá bút ký này? Hành động không thể hoàn tác.');">
                <button type="submit" class="text-xs text-red-600 hover:text-red-800 px-3 py-1.5 rounded-lg hover:bg-red-50 transition">🗑️ Xoá bút ký</button>
            </form>"##,
            id = id
        )
    } else {
        String::new()
    };

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>{esc_title} — Nhật Ký Tu Học — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-gray-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8 overflow-x-hidden">
    <nav class="text-sm text-gray-500 mb-4">
        <a href="/nhat-ky-tu-hoc" class="hover:text-tubi-700">📖 Nhật Ký Tu Học</a>
        <span class="mx-2">›</span>
        <span class="text-gray-700 truncate">{esc_title}</span>
    </nav>

    <article class="bg-white rounded-2xl border border-gray-200 shadow-sm p-5 sm:p-8 max-w-full overflow-hidden">
        <div class="flex items-start gap-3 mb-4">
            <div class="shrink-0">{avatar}</div>
            <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-semibold text-gray-900 break-words">{esc_author}</span>
                    <span class="text-xs text-gray-400">·</span>
                    <span class="text-xs text-gray-500">{time}</span>
                    <span class="text-xs text-gray-400">·</span>
                    <span class="text-xs">{mood}</span>
                </div>
                <div class="text-xs text-gray-400 mt-1">👁️ {view_count} lượt xem · 💬 {comment_count} bình luận</div>
            </div>
        </div>

        <h1 class="text-2xl sm:text-3xl font-bold text-gray-900 mb-4 break-words">{esc_title}</h1>

        <div class="prose prose-sm max-w-none text-gray-700 whitespace-pre-wrap break-words leading-relaxed">{esc_content}</div>

        <div class="mt-6 pt-4 border-t border-gray-100 flex items-center justify-between flex-wrap gap-2">
            <a href="/nhat-ky-tu-hoc" class="text-sm text-tubi-700 hover:text-tubi-900 font-medium">← Quay lại danh sách</a>
            {author_actions}
        </div>
    </article>

    <div class="mt-8">
        <h2 class="text-lg font-bold text-gray-800 mb-4">💬 Bình luận ({comment_count})</h2>
        {comments_html}
        {comment_form_html}
    </div>
</section>
</body></html>"##,
        esc_title = esc_title,
        avatar = avatar,
        esc_author = esc_author,
        time = time,
        mood = mood,
        view_count = diary.view_count,
        comment_count = diary.comment_count,
        esc_content = esc_content,
        author_actions = author_actions,
        comments_html = comments_html,
        comment_form_html = comment_form_html,
    );

    Html(html).into_response()
}

/// POST /nhat-ky-tu-hoc/{id}/xoa — Xóa diary của chính mình.
pub async fn nhat_ky_tu_hoc_delete(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return require_login(&jar, &state, &format!("/nhat-ky-tu-hoc/{id}"));
    };

    let result = sqlx::query("DELETE FROM practice_diaries WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Redirect::to("/nhat-ky-tu-hoc").into_response(),
        Ok(_) => error_response("Không tìm thấy bút ký hoặc bạn không có quyền xoá.", "/nhat-ky-tu-hoc"),
        Err(e) => {
            log::error!("❌ nhat_ky_tu_hoc_delete: {e}");
            error_response("Lỗi khi xoá bút ký.", "/nhat-ky-tu-hoc")
        }
    }
}

/// POST /nhat-ky-tu-hoc/{id}/binh-luan — Thêm bình luận.
#[derive(Debug, serde::Deserialize)]
pub struct CreateCommentForm {
    pub content: String,
}

pub async fn nhat_ky_tu_hoc_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<i64>,
    Form(form): Form<CreateCommentForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return require_login(&jar, &state, &format!("/nhat-ky-tu-hoc/{id}"));
    };

    let content = form.content.trim();
    if content.len() < 2 {
        return error_response("Bình luận quá ngắn (tối thiểu 2 ký tự).", &format!("/nhat-ky-tu-hoc/{id}"));
    }
    if content.len() > 2000 {
        return error_response("Bình luận quá dài (tối đa 2000 ký tự).", &format!("/nhat-ky-tu-hoc/{id}"));
    }

    // Check diary exists + allows comments
    let allow: Option<(bool, bool)> = sqlx::query_as::<_, (bool, bool)>(
        "SELECT allow_comments, is_public FROM practice_diaries WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((allow_comments, _is_public)) = allow else {
        return not_found_response();
    };
    if !allow_comments {
        return error_response("Bút ký này đã tắt bình luận.", &format!("/nhat-ky-tu-hoc/{id}"));
    }

    let result = sqlx::query(
        "INSERT INTO diary_comments (diary_id, user_id, content) VALUES ($1, $2, $3)"
    )
    .bind(id)
    .bind(user.id)
    .bind(content)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            // Update comment_count
            let _ = sqlx::query("UPDATE practice_diaries SET comment_count = comment_count + 1, updated_at = NOW() WHERE id = $1")
                .bind(id)
                .execute(&state.pool)
                .await;
            Redirect::to(&format!("/nhat-ky-tu-hoc/{id}")).into_response()
        }
        Err(e) => {
            log::error!("❌ nhat_ky_tu_hoc_comment: {e}");
            error_response("Không thể đăng bình luận.", &format!("/nhat-ky-tu-hoc/{id}"))
        }
    }
}

// ════════════════════════════════════════════════════════════════════════
// ERROR HELPERS
// ════════════════════════════════════════════════════════════════════════

fn error_response(msg: &str, back_url: &str) -> Response {
    let esc_msg = html_escape(msg);
    let esc_back = html_escape(back_url);
    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Lỗi — Ứng Dụng Từ Bi</title>
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-gray-50 min-h-screen flex items-center justify-center px-4">
<div class="max-w-md w-full bg-white rounded-2xl p-8 shadow-lg text-center">
    <div class="text-4xl mb-3">⚠️</div>
    <h1 class="text-xl font-bold text-red-700 mb-2">Không thể xử lý</h1>
    <p class="text-gray-600 text-sm mb-6">{esc_msg}</p>
    <a href="{esc_back}" class="inline-block bg-tubi-700 hover:bg-tubi-800 text-white px-5 py-2.5 rounded-xl transition text-sm" style="background-color:#2E7D32">← Quay lại</a>
</div>
</body></html>"##);
    Html(html).into_response()
}

fn not_found_response() -> Response {
    let html = r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Không tìm thấy — Ứng Dụng Từ Bi</title>
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-gray-50 min-h-screen flex items-center justify-center px-4">
<div class="max-w-md w-full bg-white rounded-2xl p-8 shadow-lg text-center">
    <div class="text-4xl mb-3">🔍</div>
    <h1 class="text-xl font-bold text-gray-700 mb-2">Không tìm thấy bút ký</h1>
    <p class="text-gray-500 text-sm mb-6">Bút ký có thể đã bị xoá hoặc bạn không có quyền xem.</p>
    <a href="/nhat-ky-tu-hoc" class="inline-block bg-tubi-700 hover:bg-tubi-800 text-white px-5 py-2.5 rounded-xl transition text-sm" style="background-color:#2E7D32">← Về Nhật Ký Tu Học</a>
</div>
</body></html>"##;
    Html(html).into_response()
}

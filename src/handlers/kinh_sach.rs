//! Handlers cho chuyên mục Kinh Sách (Giai đoạn 10 — v0.9.6).
//!
//! Bao gồm:
//!   * GET  /kinh-sach                              — Trang chính: lướt sách theo thư viện
//!   * GET  /kinh-sach/tim-kiem?q=...              — Tìm kiếm sách
//!   * GET  /kinh-sach/thu-vien/{category_slug}    — Lọc theo thư viện (Phật/Đạo/Kinh Văn/Sách Quý/Quan Trọng)
//!   * GET  /kinh-sach/{slug}                      — Xem thông tin sách + danh sách chương
//!   * GET  /kinh-sach/{slug}/chuong/{chapter_slug} — Đọc chương
//!   * POST /kinh-sach/{slug}/cam-ngo               — Gửi cảm ngộ (auth, min 100 chữ)
//!   * POST /kinh-sach/{slug}/tang-hoa              — Tặng hoa (auth, 1 user/sách)
//!
//! Theo HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx, mục IV. Kinh Sách:
//!   * 5 thư viện: Phật Gia, Đạo Gia, Kinh Văn, Sách Quý, Quan Trọng
//!   * Đọc online + tải offline (download_url)
//!   * Cảm ngộ phải có tối thiểu 100 chữ và qua xét duyệt
//!   * Có thể tặng hoa, kính (donate K — sẽ impl khi có currency system)

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use serde::Deserialize;
use std::collections::HashMap;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::kinh_sach::{
    BookCategory, BookChapter, BookChapterSummary, BookReviewForm, BookReviewWithAuthor,
    BookWithCategory, UserSearchHistoryItem,
};
use crate::models::user::User;

// --- Danh sách cột (đồng bộ với model) ---

const BOOK_LIST_COLUMNS: &str = "b.id, b.slug, b.title, b.author, b.translator, b.description, \
    b.category_id, bc.name AS category_name, bc.icon AS category_icon, \
    b.language, b.cover_url, b.download_url, b.book_type, \
    b.chapter_count, b.view_count, b.review_count, b.flower_count, b.donation_total_k, \
    b.is_featured, b.created_at, b.updated_at";

const REVIEW_LIST_COLUMNS: &str = "r.id, r.book_id, r.user_id, r.body, r.flower_count, \
    r.status, r.is_active, r.created_at, r.updated_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, u.rank AS author_rank";

// --- Template structs ---

#[derive(Template)]
#[template(path = "kinh-sach/index.html")]
pub struct KinhSachTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub categories: Vec<BookCategory>,
    pub featured_books: Vec<BookWithCategory>,
    pub recent_books: Vec<BookWithCategory>,
    pub popular_books: Vec<BookWithCategory>,
}

#[derive(Template)]
#[template(path = "kinh-sach/category.html")]
pub struct KinhSachCategoryTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub category: BookCategory,
    pub books: Vec<BookWithCategory>,
}

#[derive(Template)]
#[template(path = "kinh-sach/search.html")]
pub struct KinhSachSearchTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub query: String,
    pub books: Vec<BookWithCategory>,
    pub search_history: Vec<UserSearchHistoryItem>,
    /// Library slug filter hiện tại (rỗng nếu không lọc).
    pub library: String,
    /// Sort mode hiện tại: 'relevance' | 'popular' | 'recent'.
    pub sort: String,
    /// True nếu FTS trả về 0 kết quả và đã fallback sang ILIKE.
    pub used_fallback: bool,
}

#[derive(Template)]
#[template(path = "kinh-sach/book.html")]
pub struct KinhSachBookTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub book: BookWithCategory,
    pub chapters: Vec<BookChapterSummary>,
    pub reviews: Vec<BookReviewWithAuthor>,
    pub has_flowered: bool,
    pub user_review: Option<BookReviewWithAuthor>,
}

#[derive(Template)]
#[template(path = "kinh-sach/chapter.html")]
pub struct KinhSachChapterTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub book: BookWithCategory,
    pub chapter: BookChapter,
    pub chapters: Vec<BookChapterSummary>,
    pub prev_chapter: Option<BookChapterSummary>,
    pub next_chapter: Option<BookChapterSummary>,
}

// --- Handlers ---

/// GET /kinh-sach — Trang chính Kinh Sách.
pub async fn kinh_sach_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    // Lấy danh sách categories
    let categories = sqlx::query_as::<_, BookCategory>(
        "SELECT id, slug, name, description, icon, sort_order, created_at
         FROM book_categories ORDER BY sort_order ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Featured books (top 6)
    let featured_books = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.is_active = true AND b.status = 'published' AND b.is_featured = true
         ORDER BY b.created_at DESC LIMIT 6"
    ))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Recent books (top 12)
    let recent_books = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.is_active = true AND b.status = 'published'
         ORDER BY b.created_at DESC LIMIT 12"
    ))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Popular books (top 6 by view_count)
    let popular_books = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.is_active = true AND b.status = 'published'
         ORDER BY b.view_count DESC LIMIT 6"
    ))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = KinhSachTemplate {
        user,
        active_page: "books".into(),
        categories,
        featured_books,
        recent_books,
        popular_books,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kinh-sach index): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /kinh-sach/tim-kiem?q=...&library=...&sort=... — Tìm kiếm sách.
///
/// v0.9.44 — Giai đoạn 51: PostgreSQL Full-Text Search.
///   * Dùng `ts_rank_cd(search_tsv, plainto_tsquery('simple', $1))` để rank kết quả.
///   * Fallback sang ILIKE nếu FTS trả về 0 kết quả (giữ backward compat với
///     các query tiếng Việt có dấu mà tokenizer 'simple' không match tốt).
///   * Ghi lịch sử tìm kiếm vào `user_search_history` (nếu user đã đăng nhập).
///   * Hiển thị 10 chip lịch sử tìm kiếm gần nhất của user.
///   * Lọc theo thư viện (`library`) và sắp xếp theo `sort` (relevance/popular/recent).
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub library: Option<String>,
    pub sort: Option<String>,
}

pub async fn kinh_sach_search(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let q = query.q.unwrap_or_default().trim().to_string();
    let library = query.library.unwrap_or_default().trim().to_string();
    // Chuẩn hoá sort: chỉ chấp nhận 3 giá trị hợp lệ, mặc định 'relevance'.
    let sort = match query.sort.as_deref().unwrap_or("relevance") {
        "popular" => "popular".to_string(),
        "recent" => "recent".to_string(),
        _ => "relevance".to_string(),
    };

    // Ghi lịch sử tìm kiếm (chỉ khi user đã đăng nhập và query không rỗng).
    if let Some(ref u) = user {
        if !q.is_empty() {
            let _ = sqlx::query(
                "INSERT INTO user_search_history (user_id, query) VALUES ($1, $2)"
            )
            .bind(u.id)
            .bind(&q)
            .execute(&state.pool)
            .await;
        }
    }

    // Fetch 10 mục lịch sử tìm kiếm gần nhất của user.
    let search_history: Vec<UserSearchHistoryItem> = if let Some(ref u) = user {
        sqlx::query_as::<_, UserSearchHistoryItem>(
            "SELECT id, user_id, query, searched_at
             FROM user_search_history
             WHERE user_id = $1
             ORDER BY searched_at DESC
             LIMIT 10"
        )
        .bind(u.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut used_fallback = false;
    let books = if q.is_empty() {
        Vec::new()
    } else {
        // Xây ORDER BY clause theo sort mode.
        // Với FTS path: 'relevance' = ts_rank_cd DESC (mặc định).
        //               'popular'/'recent' = dùng view_count/created_at.
        let order_clause_fts = match sort.as_str() {
            "popular" => "ORDER BY b.view_count DESC",
            "recent" => "ORDER BY b.created_at DESC",
            // default = 'relevance': rank theo FTS.
            _ => "ORDER BY ts_rank_cd(b.search_tsv, plainto_tsquery('simple', $1)) DESC",
        };

        // FTS path: dùng GIN index → rất nhanh.
        // Library filter: nếu library rỗng thì không lọc (AND $2 = '' OR bc.slug = $2).
        let fts_sql = format!(
            "SELECT {BOOK_LIST_COLUMNS}
             FROM books b
             LEFT JOIN book_categories bc ON bc.id = b.category_id
             WHERE b.is_active = true AND b.status = 'published'
               AND b.search_tsv @@ plainto_tsquery('simple', $1)
               AND ($2::text = '' OR bc.slug = $2)
             {order_clause_fts}
             LIMIT 20"
        );

        let mut fts_books: Vec<BookWithCategory> =
            sqlx::query_as::<_, BookWithCategory>(&fts_sql)
                .bind(&q)
                .bind(&library)
                .fetch_all(&state.pool)
                .await
                .unwrap_or_default();

        if fts_books.is_empty() {
            // Fallback sang ILIKE (giữ backward compat với các query có dấu mà
            // tokenizer 'simple' không match tốt — ví dụ: "Từ Bi" vs "từ bi").
            used_fallback = true;

            // ILIKE path: 'relevance' = ưu tiên title match trước, rồi view_count.
            let order_clause_ilike = match sort.as_str() {
                "popular" => "ORDER BY b.view_count DESC",
                "recent" => "ORDER BY b.created_at DESC",
                _ => "ORDER BY \
                    CASE WHEN b.title ILIKE $1 ESCAPE '\\' THEN 0 ELSE 1 END, \
                    b.view_count DESC",
            };

            let escaped = q
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let pattern = format!("%{escaped}%");

            let ilike_sql = format!(
                "SELECT {BOOK_LIST_COLUMNS}
                 FROM books b
                 LEFT JOIN book_categories bc ON bc.id = b.category_id
                 WHERE b.is_active = true AND b.status = 'published'
                   AND (b.title ILIKE $1 ESCAPE '\\'
                        OR b.author ILIKE $1 ESCAPE '\\'
                        OR b.description ILIKE $1 ESCAPE '\\')
                   AND ($2::text = '' OR bc.slug = $2)
                 {order_clause_ilike}
                 LIMIT 50"
            );

            fts_books = sqlx::query_as::<_, BookWithCategory>(&ilike_sql)
                .bind(&pattern)
                .bind(&library)
                .fetch_all(&state.pool)
                .await
                .unwrap_or_default();
        }

        fts_books
    };

    let html = KinhSachSearchTemplate {
        user,
        active_page: "books".into(),
        query: q,
        books,
        search_history,
        library,
        sort,
        used_fallback,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kinh-sach search): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /kinh-sach/thu-vien/{category_slug} — Lọc theo thư viện.
pub async fn kinh_sach_category(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(category_slug): Path<String>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let category = sqlx::query_as::<_, BookCategory>(
        "SELECT id, slug, name, description, icon, sort_order, created_at
         FROM book_categories WHERE slug = $1"
    )
    .bind(&category_slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(category) = category else {
        return Redirect::to("/kinh-sach").into_response();
    };

    let books = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.is_active = true AND b.status = 'published' AND b.category_id = $1
         ORDER BY b.created_at DESC"
    ))
    .bind(category.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = KinhSachCategoryTemplate {
        user,
        active_page: "books".into(),
        category,
        books,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kinh-sach category): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /kinh-sach/{slug} — Trang thông tin sách.
pub async fn kinh_sach_book(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let book = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.slug = $1 AND b.is_active = true AND b.status = 'published'"
    ))
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(book) = book else {
        return Redirect::to("/kinh-sach").into_response();
    };

    // Tăng view_count
    let _ = sqlx::query("UPDATE books SET view_count = view_count + 1 WHERE id = $1")
        .bind(book.id)
        .execute(&state.pool)
        .await;

    // Lấy danh sách chương (chỉ summary, không include content)
    let chapters = sqlx::query_as::<_, BookChapterSummary>(
        "SELECT id, book_id, slug, title, sort_order, view_count
         FROM book_chapters
         WHERE book_id = $1 AND is_active = true
         ORDER BY sort_order ASC"
    )
    .bind(book.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Lấy danh sách cảm ngộ đã duyệt
    let reviews = sqlx::query_as::<_, BookReviewWithAuthor>(&format!(
        "SELECT {REVIEW_LIST_COLUMNS}
         FROM book_reviews r
         JOIN users u ON u.id = r.user_id
         WHERE r.book_id = $1 AND r.is_active = true AND r.status = 'approved'
         ORDER BY r.created_at DESC LIMIT 50"
    ))
    .bind(book.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Kiểm tra user đã tặng hoa chưa
    let has_flowered = if let Some(ref u) = user {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM book_flowers WHERE book_id = $1 AND user_id = $2)"
        )
        .bind(book.id)
        .bind(u.id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false)
    } else {
        false
    };

    // Lấy cảm ngộ của user (nếu có, kể cả pending)
    let user_review = if let Some(ref u) = user {
        sqlx::query_as::<_, BookReviewWithAuthor>(&format!(
            "SELECT {REVIEW_LIST_COLUMNS}
             FROM book_reviews r
             JOIN users u ON u.id = r.user_id
             WHERE r.book_id = $1 AND r.user_id = $2 AND r.is_active = true"
        ))
        .bind(book.id)
        .bind(u.id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let html = KinhSachBookTemplate {
        user,
        active_page: "books".into(),
        book,
        chapters,
        reviews,
        has_flowered,
        user_review,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kinh-sach book): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /kinh-sach/{slug}/chuong/{chapter_slug} — Đọc chương.
pub async fn kinh_sach_chapter(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((slug, chapter_slug)): Path<(String, String)>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let book = sqlx::query_as::<_, BookWithCategory>(&format!(
        "SELECT {BOOK_LIST_COLUMNS}
         FROM books b
         LEFT JOIN book_categories bc ON bc.id = b.category_id
         WHERE b.slug = $1 AND b.is_active = true AND b.status = 'published'"
    ))
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(book) = book else {
        return Redirect::to("/kinh-sach").into_response();
    };

    let chapter = sqlx::query_as::<_, BookChapter>(
        "SELECT id, book_id, slug, title, content, sort_order, view_count, is_active, created_at, updated_at
         FROM book_chapters
         WHERE book_id = $1 AND slug = $2 AND is_active = true"
    )
    .bind(book.id)
    .bind(&chapter_slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(chapter) = chapter else {
        // Nếu sách có 1 chương duy nhất → thử redirect về chương đó
        return Redirect::to(&format!("/kinh-sach/{slug}")).into_response();
    };

    // Tăng view_count chương
    let _ = sqlx::query("UPDATE book_chapters SET view_count = view_count + 1 WHERE id = $1")
        .bind(chapter.id)
        .execute(&state.pool)
        .await;
    // Cũng tăng view_count sách
    let _ = sqlx::query("UPDATE books SET view_count = view_count + 1 WHERE id = $1")
        .bind(book.id)
        .execute(&state.pool)
        .await;

    // Lấy tất cả chương để hiển thị sidebar
    let chapters = sqlx::query_as::<_, BookChapterSummary>(
        "SELECT id, book_id, slug, title, sort_order, view_count
         FROM book_chapters
         WHERE book_id = $1 AND is_active = true
         ORDER BY sort_order ASC"
    )
    .bind(book.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Tìm chương trước/sau
    let current_idx = chapters.iter().position(|c| c.id == chapter.id);
    let (prev_chapter, next_chapter) = if let Some(idx) = current_idx {
        let prev = if idx > 0 {
            chapters.get(idx - 1).cloned()
        } else {
            None
        };
        let next = chapters.get(idx + 1).cloned();
        (prev, next)
    } else {
        (None, None)
    };

    let html = KinhSachChapterTemplate {
        user,
        active_page: "books".into(),
        book,
        chapter,
        chapters,
        prev_chapter,
        next_chapter,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (kinh-sach chapter): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /kinh-sach/{slug}/cam-ngo — Gửi cảm ngộ (auth, min 100 chữ).
///
/// Theo HieuLouis/: "Cảm ngộ phải có tối thiểu 100 chữ và qua xét duyệt thì mới được hiển thị."
pub async fn kinh_sach_submit_review(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    Form(form): Form<BookReviewForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate body: không rỗng, tối thiểu 100 chữ (theo word count), tối đa 10000 ký tự
    let body = form.body.trim().to_string();
    let word_count = body.split_whitespace().count();
    if body.is_empty() {
        return Redirect::to(&format!("/kinh-sach/{slug}?error=empty")).into_response();
    }
    if body.chars().count() > 10000 {
        return Redirect::to(&format!("/kinh-sach/{slug}?error=too_long")).into_response();
    }
    if word_count < 100 {
        return Redirect::to(&format!(
            "/kinh-sach/{slug}?error=too_short&words={word_count}"
        ))
        .into_response();
    }

    // Lấy book_id
    let book_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM books WHERE slug = $1 AND is_active = true AND status = 'published'"
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(book_id) = book_id else {
        return Redirect::to("/kinh-sach").into_response();
    };

    // Insert hoặc update cảm ngộ (status = 'pending' — chờ duyệt)
    // ON CONFLICT nhờ unique index uq_book_reviews_book_user
    match sqlx::query(
        "INSERT INTO book_reviews (book_id, user_id, body, status)
         VALUES ($1, $2, $3, 'pending')
         ON CONFLICT (book_id, user_id) WHERE is_active = true
         DO UPDATE SET body = EXCLUDED.body, status = 'pending', updated_at = NOW()"
    )
    .bind(book_id)
    .bind(user.id)
    .bind(&body)
    .execute(&state.pool)
    .await
    {
        Ok(_) => {
            log::info!(
                "📖 Cảm ngộ mới từ user {} cho sách {} ({} chữ) — chờ duyệt",
                user.id,
                slug,
                word_count
            );
            Redirect::to(&format!("/kinh-sach/{slug}?review=submitted"))
        }
        Err(e) => {
            log::error!("❌ Lỗi lưu cảm ngộ: {e}");
            Redirect::to(&format!("/kinh-sach/{slug}?error=db"))
        }
    }
    .into_response()
}

/// POST /kinh-sach/{slug}/tang-hoa — Tặng hoa (auth, 1 user/sách).
pub async fn kinh_sach_give_flower(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let book_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM books WHERE slug = $1 AND is_active = true AND status = 'published'"
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(book_id) = book_id else {
        return Redirect::to("/kinh-sach").into_response();
    };

    // Insert ignore conflict (1 user chỉ tặng 1 lần)
    match sqlx::query(
        "INSERT INTO book_flowers (book_id, user_id)
         VALUES ($1, $2)
         ON CONFLICT (book_id, user_id) DO NOTHING"
    )
    .bind(book_id)
    .bind(user.id)
    .execute(&state.pool)
    .await
    {
        Ok(res) => {
            if res.rows_affected() > 0 {
                log::info!("🌸 User {} tặng hoa cho sách {}", user.id, slug);
                Redirect::to(&format!("/kinh-sach/{slug}?flower=given"))
            } else {
                Redirect::to(&format!("/kinh-sach/{slug}?flower=already"))
            }
        }
        Err(e) => {
            log::error!("❌ Lỗi tặng hoa: {e}");
            Redirect::to(&format!("/kinh-sach/{slug}?error=db"))
        }
    }
    .into_response()
}

/// API: Đếm số sách, chương, cảm ngộ — dùng cho health check hoặc dashboard.
pub async fn kinh_sach_stats(state: &sqlx::PgPool) -> HashMap<String, i64> {
    let mut stats = HashMap::new();
    stats.insert(
        "books".to_string(),
        sqlx::query_scalar("SELECT COUNT(*) FROM books WHERE is_active = true AND status = 'published'")
            .fetch_one(state)
            .await
            .unwrap_or(0),
    );
    stats.insert(
        "chapters".to_string(),
        sqlx::query_scalar("SELECT COUNT(*) FROM book_chapters WHERE is_active = true")
            .fetch_one(state)
            .await
            .unwrap_or(0),
    );
    stats.insert(
        "reviews".to_string(),
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM book_reviews WHERE is_active = true AND status = 'approved'",
        )
        .fetch_one(state)
        .await
        .unwrap_or(0),
    );
    stats.insert(
        "total_views".to_string(),
        sqlx::query_scalar("SELECT COALESCE(SUM(view_count), 0) FROM books WHERE is_active = true")
            .fetch_one(state)
            .await
            .unwrap_or(0),
    );
    stats
}

//! Handlers cho chuyên mục Cộng Đồng (v0.6+).
//!
//! Bao gồm:
//!   * GET  /cong-dong                       — Trang chính: lướt nhóm + lướt chủ đề
//!   * GET  /cong-dong/tao-nhom              — Form tạo nhóm (auth)
//!   * POST /cong-dong/tao-nhom              — Tạo nhóm mới (auth)
//!   * GET  /cong-dong/nhom/{slug}           — Xem nhóm + danh sách chủ đề
//!   * POST /cong-dong/nhom/{slug}/tham-gia  — Tham gia nhóm (auth)
//!   * POST /cong-dong/nhom/{slug}/roi-khoi  — Rời nhóm (auth)
//!   * GET  /cong-dong/nhom/{slug}/tao-chu-de — Form tạo chủ đề (auth + member)
//!   * POST /cong-dong/nhom/{slug}/tao-chu-de — Tạo chủ đề mới (auth + member)
//!   * GET  /cong-dong/chu-de/{id}           — Xem chủ đề + bình luận
//!   * POST /cong-dong/chu-de/{id}/binh-luan — Bình luận (auth)

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::community::{
    CommentCreateForm, CommentWithAuthor, GroupCategory, GroupCreateForm, GroupMember,
    GroupWithCategory, TopicCreateForm, TopicWithAuthor,
};
use crate::models::user::User;

// --- Danh sách cột (đồng bộ với model) ---

const GROUP_LIST_COLUMNS: &str = "g.id, g.slug, g.name, g.description, g.category_id, \
    gc.name AS category_name, gc.icon AS category_icon, \
    g.owner_id, g.visibility, g.require_approval, \
    g.member_count, g.topic_count, g.is_active, g.created_at, g.updated_at";

const TOPIC_LIST_COLUMNS: &str = "t.id, t.group_id, t.author_id, t.title, t.body, \
    t.is_pinned, t.is_locked, t.comment_count, t.view_count, t.is_active, \
    t.created_at, t.updated_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, u.rank AS author_rank";

const COMMENT_LIST_COLUMNS: &str = "c.id, c.topic_id, c.author_id, c.parent_id, c.body, \
    c.is_active, c.created_at, c.updated_at, \
    u.display_name AS author_display_name, u.avatar_url AS author_avatar_url, u.rank AS author_rank";

// --- Template structs ---

#[derive(Template)]
#[template(path = "community/index.html")]
pub struct CommunityTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub groups: Vec<GroupWithCategory>,
    pub hot_topics: Vec<TopicWithAuthor>,
}

#[derive(Template)]
#[template(path = "community/group.html")]
pub struct GroupTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub group: GroupWithCategory,
    pub topics: Vec<TopicWithAuthor>,
    pub membership: Option<GroupMember>,
}

#[derive(Template)]
#[template(path = "community/topic.html")]
pub struct TopicTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub topic: TopicWithAuthor,
    pub group_slug: String,
    pub group_name: String,
    pub comments: Vec<CommentWithAuthor>,
}

#[derive(Template)]
#[template(path = "community/create_group.html")]
pub struct CreateGroupTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub categories: Vec<GroupCategory>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "community/create_topic.html")]
pub struct CreateTopicTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub group_slug: String,
    pub group_name: String,
    pub error: Option<String>,
}

// --- Helpers ---

/// Lấy danh sách categories theo thứ tự sort_order.
async fn fetch_categories(pool: &PgPool) -> Vec<GroupCategory> {
    sqlx::query_as::<_, GroupCategory>(
        "SELECT id, slug, name, icon, sort_order, created_at
         FROM group_categories ORDER BY sort_order ASC, name ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Sinh slug từ tên (loại bỏ dấu, thay khoảng trắng bằng dấu gạch).
fn slugify(s: &str) -> String {
    let normalized: String = s
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() || c == '-' || c == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in normalized.chars() {
        if ch == '-' {
            if !prev_dash {
                out.push('-');
            }
            prev_dash = true;
        } else {
            out.push(ch);
            prev_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        format!("group-{}", Uuid::new_v4().simple())
    } else {
        out
    }
}

/// Đảm bảo slug là duy nhất — nếu trùng, thêm hậu tố ngắn.
async fn ensure_unique_slug(pool: &PgPool, base_slug: &str) -> String {
    let sql = "SELECT slug FROM groups WHERE slug = $1";
    let exists = sqlx::query_scalar::<_, String>(sql)
        .bind(base_slug)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    if exists.is_none() {
        return base_slug.to_string();
    }

    let suffix: String = Uuid::new_v4().simple().to_string().chars().take(6).collect();
    format!("{base_slug}-{suffix}")
}

/// Lấy membership của user trong group (nếu có).
async fn get_membership(pool: &PgPool, group_id: Uuid, user_id: Uuid) -> Option<GroupMember> {
    sqlx::query_as::<_, GroupMember>(
        "SELECT id, group_id, user_id, role, status, joined_at
         FROM group_members WHERE group_id = $1 AND user_id = $2",
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

// --- Page Handlers ---

/// GET /cong-dong — Trang chính Cộng Đồng.
pub async fn cong_dong_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let groups = sqlx::query_as::<_, GroupWithCategory>(&format!(
        "SELECT {GROUP_LIST_COLUMNS}
         FROM groups g
         LEFT JOIN group_categories gc ON gc.id = g.category_id
         WHERE g.is_active = true AND g.visibility = 'public'
         ORDER BY g.created_at DESC
         LIMIT 20"
    ))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let hot_topics = sqlx::query_as::<_, TopicWithAuthor>(&format!(
        "SELECT {TOPIC_LIST_COLUMNS}
         FROM topics t
         JOIN groups g ON g.id = t.group_id
         JOIN users u ON u.id = t.author_id
         WHERE t.is_active = true AND g.visibility = 'public' AND g.is_active = true
         ORDER BY t.is_pinned DESC, t.created_at DESC
         LIMIT 10"
    ))
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let categories = fetch_categories(&state.pool).await;
    let _ = categories;

    let html = CommunityTemplate {
        user,
        active_page: "community".into(),
        groups,
        hot_topics,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (community): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /cong-dong/tao-nhom — Form tạo nhóm mới.
pub async fn create_group_form(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let categories = fetch_categories(&state.pool).await;

    let html = CreateGroupTemplate {
        user: Some(user),
        active_page: "community".into(),
        categories,
        error: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (create_group): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /cong-dong/tao-nhom — Tạo nhóm mới.
pub async fn create_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<GroupCreateForm>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    // Validate name
    let name = form.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 100 {
        let categories = fetch_categories(&state.pool).await;
        let html = CreateGroupTemplate {
            user: Some(user),
            active_page: "community".into(),
            categories,
            error: Some("Tên nhóm không được để trống và tối đa 100 ký tự.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_group err): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }

    // Validate visibility
    let visibility = form.visibility.trim().to_string();
    if !matches!(visibility.as_str(), "public" | "private" | "hidden") {
        let categories = fetch_categories(&state.pool).await;
        let html = CreateGroupTemplate {
            user: Some(user),
            active_page: "community".into(),
            categories,
            error: Some("Visibility không hợp lệ.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_group vis): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }

    let description = form
        .description
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let category_id = form.category_id.filter(|&id| id > 0);
    let require_approval = form.require_approval.is_some();

    let base_slug = slugify(&name);
    let slug = ensure_unique_slug(&state.pool, &base_slug).await;

    // Tạo nhóm + thêm owner vào group_members với role='owner'
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log::error!("❌ Lỗi bắt đầu transaction tạo nhóm: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let group_id: Uuid = match sqlx::query_scalar(
        "INSERT INTO groups (slug, name, description, category_id, owner_id, visibility, require_approval)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(&slug)
    .bind(&name)
    .bind(&description)
    .bind(category_id)
    .bind(user.id)
    .bind(&visibility)
    .bind(require_approval)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("❌ Lỗi tạo nhóm: {e}");
            let _ = tx.rollback().await;
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi tạo nhóm",
            )
                .into_response();
        }
    };

    if let Err(e) = sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role, status)
         VALUES ($1, $2, 'owner', 'active')",
    )
    .bind(group_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await
    {
        log::error!("❌ Lỗi thêm owner vào group_members: {e}");
        let _ = tx.rollback().await;
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Lỗi tạo nhóm",
        )
            .into_response();
    }

    if let Err(e) = tx.commit().await {
        log::error!("❌ Lỗi commit tạo nhóm: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Lỗi tạo nhóm",
        )
            .into_response();
    }

    log::info!("✅ Nhóm mới được tạo: {slug} bởi user {}", user.id);

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// GET /cong-dong/nhom/{slug} — Trang nhóm + danh sách chủ đề.
pub async fn view_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let group = match sqlx::query_as::<_, GroupWithCategory>(&format!(
        "SELECT {GROUP_LIST_COLUMNS}
         FROM groups g
         LEFT JOIN group_categories gc ON gc.id = g.category_id
         WHERE g.slug = $1 AND g.is_active = true"
    ))
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(g)) => g,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(e) => {
            log::error!("❌ Lỗi truy vấn nhóm: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let topics = sqlx::query_as::<_, TopicWithAuthor>(&format!(
        "SELECT {TOPIC_LIST_COLUMNS}
         FROM topics t
         JOIN users u ON u.id = t.author_id
         WHERE t.group_id = $1 AND t.is_active = true
         ORDER BY t.is_pinned DESC, t.created_at DESC
         LIMIT 20"
    ))
    .bind(group.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let membership = if let Some(ref u) = user {
        get_membership(&state.pool, group.id, u.id).await
    } else {
        None
    };

    let html = GroupTemplate {
        user,
        active_page: "community".into(),
        group,
        topics,
        membership,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (group): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /cong-dong/nhom/{slug}/tham-gia — Tham gia nhóm.
pub async fn join_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(e) => {
            log::error!("❌ Lỗi truy vấn nhóm: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let status = if sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role, status)
         VALUES ($1, $2, 'member', 'active')
         ON CONFLICT (group_id, user_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(user.id)
    .execute(&state.pool)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0)
        > 0
    {
        "active"
    } else {
        "already_member"
    };

    log::info!("👥 User {} tham gia nhóm {slug} — status={status}", user.id);

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// POST /cong-dong/nhom/{slug}/roi-khoi — Rời nhóm.
pub async fn leave_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2",
    )
    .bind(group_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if role.as_deref() == Some("owner") {
        return Redirect::to(&format!(
            "/cong-dong/nhom/{slug}?err=owner_cannot_leave"
        ))
        .into_response();
    }

    let _ = sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(user.id)
        .execute(&state.pool)
        .await;

    log::info!("👋 User {} rời nhóm {slug}", user.id);

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// GET /cong-dong/nhom/{slug}/tao-chu-de — Form tạo chủ đề.
pub async fn create_topic_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let (group_id, group_name): (Uuid, String) = match sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let membership = get_membership(&state.pool, group_id, user.id).await;
    if membership.is_none() {
        return Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response();
    }

    let html = CreateTopicTemplate {
        user: Some(user),
        active_page: "community".into(),
        group_slug: slug,
        group_name,
        error: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (create_topic): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /cong-dong/nhom/{slug}/tao-chu-de — Tạo chủ đề mới.
pub async fn create_topic(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    Form(form): Form<TopicCreateForm>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };

    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
        }
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống",
            )
                .into_response();
        }
    };

    let membership = get_membership(&state.pool, group_id, user.id).await;
    if membership.is_none() {
        return Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response();
    }

    // Validate title + body
    let title = form.title.trim().to_string();
    let body = form.body.trim().to_string();
    if title.is_empty() || title.chars().count() > 200 {
        let html = CreateTopicTemplate {
            user: Some(user),
            active_page: "community".into(),
            group_slug: slug,
            group_name: String::new(),
            error: Some("Tiêu đề không được để trống và tối đa 200 ký tự.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_topic err): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }
    if body.is_empty() {
        let html = CreateTopicTemplate {
            user: Some(user),
            active_page: "community".into(),
            group_slug: slug,
            group_name: String::new(),
            error: Some("Nội dung không được để trống.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_topic body): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }

    let topic_id: Uuid = match sqlx::query_scalar(
        "INSERT INTO topics (group_id, author_id, title, body)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(group_id)
    .bind(user.id)
    .bind(&title)
    .bind(&body)
    .fetch_one(&state.pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("❌ Lỗi tạo chủ đề: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi tạo chủ đề",
            )
                .into_response();
        }
    };

    log::info!("📝 Chủ đề mới: {topic_id} trong nhóm {slug}");

    Redirect::to(&format!("/cong-dong/chu-de/{topic_id}")).into_response()
}

/// GET /cong-dong/chu-de/{id} — Trang chủ đề + bình luận.
pub async fn view_topic(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id_str): Path<String>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let topic_id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(_) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                "Chủ đề không tồn tại.",
            )
                .into_response();
        }
    };

    let (topic, group_slug, group_name) = match fetch_topic_with_group(&state.pool, topic_id).await {
        Some(x) => x,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                "Chủ đề không tồn tại.",
            )
                .into_response();
        }
    };

    // Tăng view_count (best-effort)
    let _ = sqlx::query("UPDATE topics SET view_count = view_count + 1 WHERE id = $1")
        .bind(topic_id)
        .execute(&state.pool)
        .await;

    let comments = sqlx::query_as::<_, CommentWithAuthor>(&format!(
        "SELECT {COMMENT_LIST_COLUMNS}
         FROM comments c
         JOIN users u ON u.id = c.author_id
         WHERE c.topic_id = $1 AND c.is_active = true
         ORDER BY c.created_at ASC
         LIMIT 100"
    ))
    .bind(topic_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = TopicTemplate {
        user,
        active_page: "community".into(),
        topic,
        group_slug,
        group_name,
        comments,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (topic): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Helper: lấy topic + group info.
async fn fetch_topic_with_group(
    pool: &PgPool,
    topic_id: Uuid,
) -> Option<(TopicWithAuthor, String, String)> {
    let topic = sqlx::query_as::<_, TopicWithAuthor>(&format!(
        "SELECT {TOPIC_LIST_COLUMNS}
         FROM topics t
         JOIN users u ON u.id = t.author_id
         WHERE t.id = $1 AND t.is_active = true"
    ))
    .bind(topic_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (group_slug, group_name): (String, String) = sqlx::query_as(
        "SELECT slug, name FROM groups WHERE id = $1",
    )
    .bind(topic.group_id)
    .fetch_one(pool)
    .await
    .ok()?;

    Some((topic, group_slug, group_name))
}

/// POST /cong-dong/chu-de/{id}/binh-luan — Đăng bình luận.
pub async fn create_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id_str): Path<String>,
    Form(form): Form<CommentCreateForm>,
) -> Response {
    let user = match get_user_from_session(&state.pool, &jar).await {
        Some(u) => u,
        None => return Redirect::to("/dang-nhap").into_response(),
    };
    let topic_id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(_) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                "Chủ đề không tồn tại.",
            )
                .into_response();
        }
    };

    let body = form.body.trim().to_string();
    if body.is_empty() || body.chars().count() > 5000 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Bình luận không hợp lệ.",
        )
            .into_response();
    }

    let parent_id = form
        .parent_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    let locked: Option<bool> = sqlx::query_scalar(
        "SELECT is_locked FROM topics WHERE id = $1 AND is_active = true",
    )
    .bind(topic_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if locked.is_none() {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "Chủ đề không tồn tại.",
        )
            .into_response();
    }
    if locked == Some(true) {
        return (axum::http::StatusCode::FORBIDDEN, "Chủ đề đã bị khoá.").into_response();
    }

    if let Err(e) = sqlx::query(
        "INSERT INTO comments (topic_id, author_id, parent_id, body)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(topic_id)
    .bind(user.id)
    .bind(parent_id)
    .bind(&body)
    .execute(&state.pool)
    .await
    {
        log::error!("❌ Lỗi đăng bình luận: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Lỗi đăng bình luận.",
        )
            .into_response();
    }

    log::info!("💬 Bình luận mới trên topic {topic_id} bởi user {}", user.id);

    Redirect::to(&format!("/cong-dong/chu-de/{topic_id}")).into_response()
}

/// Helper: format thời gian tương đối (vd "5 phút trước").
pub fn time_ago_display(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let dur = now.signed_duration_since(*dt);
    if dur.num_seconds() < 60 {
        return "vừa xong".to_string();
    }
    if dur.num_minutes() < 60 {
        return format!("{} phút trước", dur.num_minutes());
    }
    if dur.num_hours() < 24 {
        return format!("{} giờ trước", dur.num_hours());
    }
    if dur.num_days() < 30 {
        return format!("{} ngày trước", dur.num_days());
    }
    if dur.num_days() < 365 {
        return format!("{} tháng trước", dur.num_days() / 30);
    }
    format!("{} năm trước", dur.num_days() / 365)
}

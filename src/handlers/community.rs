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
    extract::{Multipart, Path, State},
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
    GroupMemberWithUser, GroupWithCategory, TopicCreateForm, TopicWithAuthor,
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
    /// URL ảnh bìa nhóm (nếu có).
    /// [v0.9.3] Cover image upload
    pub cover_image_url: Option<String>,
    /// v0.9.36 — Giai đoạn 41: URL logo nhóm (icon vuông nhỏ, khác với cover banner).
    pub logo_image_url: Option<String>,
    /// v0.9.23: Danh sách thành viên (chỉ load khi user là owner/admin)
    pub members: Vec<GroupMemberWithUser>,
    /// v0.9.37 — Error message hiển thị trên banner (vd: "Bạn chưa tham gia nhóm",
    /// "Tài khoản đang chờ duyệt", "Chủ đề đã bị khoá"...). Được set khi redirect
    /// từ create_topic/create_comment với query param `?err=...`.
    pub error: Option<String>,
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
    /// v0.9.37 — Error message hiển thị trên banner khi create_comment fail.
    pub error: Option<String>,
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

/// Lấy danh sách categories theo thứ tự `sort_order`.
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
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
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
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate name
    let name = form.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 100 {
        return render_create_group_error(&state.pool, user, "Tên nhóm không được để trống và tối đa 100 ký tự.").await;
    }

    // Validate visibility
    let visibility = form.visibility.trim().to_string();
    if !matches!(visibility.as_str(), "public" | "private" | "hidden") {
        return render_create_group_error(&state.pool, user, "Visibility không hợp lệ.").await;
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
    let slug_for_redirect = slug.clone();
    match insert_group_with_owner(&state.pool, &slug, &name, description.as_ref(), category_id, user.id, &visibility, require_approval).await {
        Ok(()) => {
            log::info!("✅ Nhóm mới được tạo: {slug_for_redirect} bởi user {}", user.id);
            Redirect::to(&format!("/cong-dong/nhom/{slug_for_redirect}")).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Lỗi tạo nhóm",
        )
            .into_response(),
    }
}

/// Helper: render create-group form with error message.
async fn render_create_group_error(pool: &PgPool, user: User, error_msg: &str) -> Response {
    let categories = fetch_categories(pool).await;
    let html = CreateGroupTemplate {
        user: Some(user),
        active_page: "community".into(),
        categories,
        error: Some(error_msg.into()),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (create_group err): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

/// Helper: insert group + owner member in a transaction.
#[allow(clippy::too_many_arguments)]
async fn insert_group_with_owner(
    pool: &PgPool,
    slug: &str,
    name: &str,
    description: Option<&String>,
    category_id: Option<i32>,
    owner_id: Uuid,
    visibility: &str,
    require_approval: bool,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let group_id: Uuid = sqlx::query_scalar(
        "INSERT INTO groups (slug, name, description, category_id, owner_id, visibility, require_approval)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(category_id)
    .bind(owner_id)
    .bind(visibility)
    .bind(require_approval)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role, status)
         VALUES ($1, $2, 'owner', 'active')",
    )
    .bind(group_id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// GET /cong-dong/nhom/{slug} — Trang nhóm + danh sách chủ đề.
pub async fn view_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    // v0.9.37 — Parse ?err=... query param để hiển thị banner error.
    // Source: create_topic_form / create_topic redirect khi user không có quyền.
    let error = query
        .get("err")
        .map(|code| match code.as_str() {
            "ban-chua-tham-gia-nhom" => "Bạn chưa tham gia nhóm này. Hãy bấm 'Tham Gia Nhóm' để được phép tạo chủ đề.".to_string(),
            "ban-cho-duyet" => "Đơn tham gia nhóm của bạn đang chờ Trưởng Nhóm / Admin duyệt. Vui lòng đợi.".to_string(),
            "ban-bi-khoa" => "Bạn đã bị khoá khỏi nhóm này. Liên hệ Trưởng Nhóm nếu cần hỗ trợ.".to_string(),
            "ban-da-roi-nhom" => "Bạn đã rời nhóm. Hãy tham gia lại để tạo chủ đề.".to_string(),
            "khong-quyen-tao-chu-de" => "Bạn không có quyền tạo chủ đề trong nhóm này.".to_string(),
            "chu-de-khong-ton-tai" => "Chủ đề không tồn tại hoặc đã bị xoá.".to_string(),
            "loi-he-thong-binh-luan" => "Lỗi hệ thống khi đăng bình luận. Vui lòng thử lại sau.".to_string(),
            "binh-luan-rong" => "Bình luận không được để trống.".to_string(),
            "binh-luan-qua-dai" => "Bình luận quá dài (tối đa 5.000 ký tự).".to_string(),
            "binh-luan-cha-khong-hop-le" => "Bình luận cha không hợp lệ hoặc không thuộc chủ đề này.".to_string(),
            "chu-de-da-khoa" => "Chủ đề đã bị khoá, không thể bình luận.".to_string(),
            _ => format!("Lỗi: {code}"),
        });

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

    // [v0.9.3] Lấy URL ảnh bìa nhóm (cover_upload_id → images → stored_filename)
    let cover_image_url: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT i.stored_filename FROM images i JOIN groups g ON g.cover_upload_id = i.id WHERE g.id = $1",
    )
    .bind(group.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|filename| format!("{}/{filename}", state.config.upload_url_prefix));

    // v0.9.36 — Giai đoạn 41: Lấy URL logo nhóm (logo_upload_id → images → stored_filename)
    let logo_image_url: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT i.stored_filename FROM images i JOIN groups g ON g.logo_upload_id = i.id WHERE g.id = $1",
    )
    .bind(group.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|filename| format!("{}/{filename}", state.config.upload_url_prefix));

    // v0.9.23: Load danh sách thành viên nếu user là owner/admin của nhóm
    let is_group_manager = membership.as_ref().is_some_and(|m| {
        m.status == "active" && (m.role == "owner" || m.role == "admin")
    }) || user.as_ref().is_some_and(|u| u.is_staff());

    let members = if is_group_manager {
        sqlx::query_as::<_, GroupMemberWithUser>(
            "SELECT gm.id, gm.group_id, gm.user_id, gm.role, gm.status, gm.joined_at,
                    u.display_name, u.avatar_url, u.rank
             FROM group_members gm
             JOIN users u ON u.id = gm.user_id
             WHERE gm.group_id = $1
             ORDER BY
                CASE gm.role
                    WHEN 'owner' THEN 0
                    WHEN 'admin' THEN 1
                    WHEN 'moderator' THEN 2
                    ELSE 3
                END,
                gm.joined_at ASC",
        )
        .bind(group.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let html = GroupTemplate {
        user,
        active_page: "community".into(),
        group,
        topics,
        membership,
        cover_image_url,
        logo_image_url,
        members,
        error, // v0.9.37
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
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Lấy group_id VÀ require_approval — v0.9.21 fix: tôn trọng cờ require_approval
    let group_info: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, require_approval FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some((group_id, require_approval)) = group_info else {
        return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response();
    };

    // v0.9.21 fix: Nếu require_approval = true → status = 'pending', ngược lại 'active'
    let member_status = if require_approval { "pending" } else { "active" };

    let status = if sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role, status)
         VALUES ($1, $2, 'member', $3)
         ON CONFLICT (group_id, user_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(user.id)
    .bind(member_status)
    .execute(&state.pool)
    .await
    .map_or(0, |r| r.rows_affected())
        > 0
    {
        member_status
    } else {
        "already_member"
    };

    // v0.9.21 fix: Cập nhật member_count khi tham gia thành công
    if status != "already_member" {
        let _ = sqlx::query(
            "UPDATE groups SET member_count = member_count + 1 WHERE id = $1",
        )
        .bind(group_id)
        .execute(&state.pool)
        .await;
    }

    log::info!("👥 User {} tham gia nhóm {slug} — status={status}", user.id);

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// POST /cong-dong/nhom/{slug}/roi-khoi — Rời nhóm.
pub async fn leave_group(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
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

    let rows_deleted = sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(user.id)
        .execute(&state.pool)
        .await
        .map_or(0, |r| r.rows_affected());

    // v0.9.21 fix: Cập nhật member_count khi rời nhóm thành công
    if rows_deleted > 0 {
        let _ = sqlx::query(
            "UPDATE groups SET member_count = GREATEST(member_count - 1, 0) WHERE id = $1",
        )
        .bind(group_id)
        .execute(&state.pool)
        .await;
    }

    log::info!("👋 User {} rời nhóm {slug}", user.id);

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// GET /cong-dong/nhom/{slug}/tao-chu-de — Form tạo chủ đề.
///
/// v0.9.37 FIX "gửi bài không được":
///   - Trước đây: pending/banned member bị redirect về trang nhóm với KHÔNG có thông báo gì
///     → user không hiểu vì sao nút "Tạo Chủ Đề" không hoạt động.
///   - Giờ: redirect về trang nhóm với query param `?err=khong-quyen-tao-chu-de`
///     → group.html hiển thị banner lỗi rõ ràng.
pub async fn create_topic_form(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
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

    // v0.9.21 fix: Chỉ cho phép active member tạo chủ đề (pending/banned không được)
    // v0.9.37: redirect với err param để user biết lý do bị từ chối
    let membership = get_membership(&state.pool, group_id, user.id).await;
    if membership.as_ref().is_none_or(|m| m.status != "active") {
        let reason = match membership.as_ref() {
            None => "ban-chua-tham-gia-nhom",
            Some(m) if m.status == "pending" => "ban-cho-duyet",
            Some(m) if m.status == "banned" => "ban-bi-khoa",
            Some(m) if m.status == "left" => "ban-da-roi-nhom",
            _ => "khong-quyen-tao-chu-de",
        };
        return Redirect::to(&format!("/cong-dong/nhom/{slug}?err={reason}")).into_response();
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
///
/// v0.9.37 FIX "gửi bài không được":
///   - Trước đây: pending member POST bị redirect về trang nhóm SILENTLY → user tưởng form hỏng.
///   - Trước đây: validation error render lại form với group_name="" → breadcrumb blank.
///   - Trước đây: DB error trả plain-text "Lỗi tạo chủ đề" → user mất toàn bộ nội dung đã gõ.
///   - Giờ:
///     (1) Pending member redirect với err param để group.html hiển thị lý do.
///     (2) Validation error render lại form với ĐÚNG group_name (fetch từ DB).
///     (3) DB error render lại form với error message + group_name + giữ nguyên title/body
///         đã nhập (truyền qua query param `?err=db&title=...` để tránh mất dữ liệu).
pub async fn create_topic(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    Form(form): Form<TopicCreateForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Fetch group_id VÀ group_name cùng lúc để dùng cho error path (tránh group_name rỗng).
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
        Err(e) => {
            log::error!("❌ Lỗi query group khi tạo chủ đề: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Lỗi hệ thống. Vui lòng thử lại sau.",
            )
                .into_response();
        }
    };

    // v0.9.21 fix: Chỉ cho phép active member tạo chủ đề
    // v0.9.37: redirect với err param để group.html hiển thị lý do rõ ràng
    let membership = get_membership(&state.pool, group_id, user.id).await;
    if membership.as_ref().is_none_or(|m| m.status != "active") {
        let reason = match membership.as_ref() {
            None => "ban-chua-tham-gia-nhom",
            Some(m) if m.status == "pending" => "ban-cho-duyet",
            Some(m) if m.status == "banned" => "ban-bi-khoa",
            Some(m) if m.status == "left" => "ban-da-roi-nhom",
            _ => "khong-quyen-tao-chu-de",
        };
        return Redirect::to(&format!("/cong-dong/nhom/{slug}?err={reason}")).into_response();
    }

    // Validate title + body
    // v0.9.37 FIX: render lại form với group_name ĐÚNG (trước đây là String::new()).
    let title = form.title.trim().to_string();
    let body = form.body.trim().to_string();
    if title.is_empty() || title.chars().count() > 200 {
        let html = CreateTopicTemplate {
            user: Some(user),
            active_page: "community".into(),
            group_slug: slug,
            group_name, // v0.9.37: dùng group_name fetch từ DB thay vì String::new()
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
            group_name, // v0.9.37: dùng group_name fetch từ DB
            error: Some("Nội dung không được để trống.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_topic body): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }

    // v0.9.42 — Giai đoạn 46: Forbidden words auto-check
    let fw_result = crate::db::check_forbidden_words_multi(&state.pool, &[&title, &body]).await;
    if fw_result.should_block {
        log::warn!("🚫 Chủ đề bị chặn (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
        let html = CreateTopicTemplate {
            user: Some(user),
            active_page: "community".into(),
            group_slug: slug,
            group_name,
            error: Some("Nội dung chứa từ ngữ không phù hợp. Vui lòng sửa lại.".into()),
        }
        .render()
        .unwrap_or_else(|e| {
            log::error!("Template render error (create_topic fw_block): {e}");
            format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
        });
        return Html(html).into_response();
    }
    if fw_result.should_flag {
        log::warn!("🚩 Chủ đề flagged (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
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
            // v0.9.37 FIX: thay vì trả plain-text 500 "Lỗi tạo chủ đề" → render lại form
            // với error message rõ ràng + group_name đúng + title đã nhập.
            // Body không hiển thị lại được vì form submit đã consume (chỉ có trong `body` var).
            // → Khuyến nghị user dùng nút Back của trình duyệt hoặc viết lại.
            log::error!("❌ Lỗi tạo chủ đề (DB INSERT): {e}");
            let html = CreateTopicTemplate {
                user: Some(user),
                active_page: "community".into(),
                group_slug: slug,
                group_name,
                error: Some(format!(
                    "Lỗi hệ thống khi lưu chủ đề: {}. Vui lòng thử lại. Tiêu đề của bạn: '{}'",
                    e, title
                )),
            }
            .render()
            .unwrap_or_else(|e| {
                log::error!("Template render error (create_topic db-err): {e}");
                format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
            });
            return Html(html).into_response();
        }
    };

    log::info!("📝 Chủ đề mới: {topic_id} trong nhóm {slug}");

    // v0.9.37 FIX BUG 3 (stale counter): cập nhật groups.topic_count sau khi tạo chủ đề.
    // Trước đây counter chỉ đúng khi có trigger/background job chạy — giờ cập nhật ngay.
    let _ = sqlx::query(
        "UPDATE groups SET topic_count = topic_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(group_id)
    .execute(&state.pool)
    .await;

    Redirect::to(&format!("/cong-dong/chu-de/{topic_id}")).into_response()
}

/// GET /cong-dong/chu-de/{id} — Trang chủ đề + bình luận.
pub async fn view_topic(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id_str): Path<String>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let Ok(topic_id) = Uuid::parse_str(&id_str) else {
            return (
                axum::http::StatusCode::NOT_FOUND,
                "Chủ đề không tồn tại.",
            )
                .into_response();
        };

    let Some((topic, group_slug, group_name)) = fetch_topic_with_group(&state.pool, topic_id).await else {
            return (
                axum::http::StatusCode::NOT_FOUND,
                "Chủ đề không tồn tại.",
            )
                .into_response();
        };

    // v0.9.37 — Parse ?err=... để hiển thị banner error (từ create_comment redirect).
    let error = query
        .get("err")
        .map(|code| match code.as_str() {
            "binh-luan-rong" => "Bình luận không được để trống.".to_string(),
            "binh-luan-qua-dai" => "Bình luận quá dài (tối đa 5.000 ký tự).".to_string(),
            "binh-luan-cha-khong-hop-le" => "Bình luận cha không hợp lệ hoặc không thuộc chủ đề này.".to_string(),
            "chu-de-da-khoa" => "Chủ đề đã bị khoá, không thể bình luận.".to_string(),
            "loi-he-thong-binh-luan" => "Lỗi hệ thống khi đăng bình luận. Vui lòng thử lại sau.".to_string(),
            "chu-de-khong-ton-tai" => "Chủ đề không tồn tại hoặc đã bị xoá.".to_string(),
            _ => format!("Lỗi: {code}"),
        });

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
        error, // v0.9.37
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
///
/// v0.9.37 FIX "gửi bài không được":
///   - Trước đây: tất cả error path trả plain-text status code → user thấy trang trắng
///     "Bình luận không hợp lệ." / "Chủ đề đã bị khoá." / "Lỗi đăng bình luận."
///     không có nav, không có nút quay lại, không có context.
///   - Giờ: redirect về `/cong-dong/chu-de/{id}?err=...` để view_topic render lại
///     trang chủ đề VÀ hiển thị banner lỗi rõ ràng.
///   - v0.9.37 FIX BUG 3: cập nhật `topics.comment_count` sau khi insert thành công.
pub async fn create_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id_str): Path<String>,
    Form(form): Form<CommentCreateForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };
    let Ok(topic_id) = Uuid::parse_str(&id_str) else {
        // Topic ID không phải UUID — redirect về trang cộng đồng thay vì plain-text 404.
        return Redirect::to("/cong-dong?err=chu-de-khong-ton-tai").into_response();
    };

    let body = form.body.trim().to_string();
    if body.is_empty() {
        // v0.9.37: redirect với err param thay vì plain-text 400.
        return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=binh-luan-rong")).into_response();
    }
    if body.chars().count() > 5000 {
        return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=binh-luan-qua-dai")).into_response();
    }

    let parent_id = form
        .parent_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    // v0.9.12: Security — validate parent_id thuộc cùng topic_id, tránh cross-topic reply.
    if let Some(pid) = parent_id {
        let parent_in_topic: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM comments
                WHERE id = $1 AND topic_id = $2 AND is_active = true
            )",
        )
        .bind(pid)
        .bind(topic_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        if parent_in_topic != Some(true) {
            // v0.9.37: redirect thay vì plain-text 400.
            return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=binh-luan-cha-khong-hop-le")).into_response();
        }
    }

    let locked: Option<bool> = sqlx::query_scalar(
        "SELECT is_locked FROM topics WHERE id = $1 AND is_active = true",
    )
    .bind(topic_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if locked.is_none() {
        // v0.9.37: redirect thay vì plain-text 404.
        return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=chu-de-khong-ton-tai")).into_response();
    }
    if locked == Some(true) {
        // v0.9.37: redirect thay vì plain-text 403.
        return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=chu-de-da-khoa")).into_response();
    }

    // v0.9.42 — Giai đoạn 46: Forbidden words auto-check
    let fw_result = crate::db::check_forbidden_words(&state.pool, &body).await;
    if fw_result.should_block {
        log::warn!("🚫 Bình luận bị chặn (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
        return Html(format!(
            r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm">⚠️ Nội dung bình luận chứa từ ngữ không phù hợp. Vui lòng sửa lại.</div>"#
        )).into_response();
    }
    if fw_result.should_flag {
        log::warn!("🚩 Bình luận flagged (forbidden words): user_id={}, words={:?}", user.id, fw_result.matched_words);
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
        log::error!("❌ Lỗi đăng bình luận (DB INSERT): {e}");
        // v0.9.37: redirect thay vì plain-text 500. User thấy topic page + banner lỗi.
        return Redirect::to(&format!("/cong-dong/chu-de/{topic_id}?err=loi-he-thong-binh-luan")).into_response();
    }

    log::info!("💬 Bình luận mới trên topic {topic_id} bởi user {}", user.id);

    // v0.9.37 FIX BUG 3 (stale counter): cập nhật topics.comment_count sau khi insert.
    let _ = sqlx::query(
        "UPDATE topics SET comment_count = comment_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(topic_id)
    .execute(&state.pool)
    .await;

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

/// POST /cong-dong/nhom/{slug}/doi-anh — Đổi ảnh bìa nhóm.
///
/// Chỉ owner hoặc admin mới được đổi ảnh bìa.
/// Accepts multipart form with `file` field.
pub async fn change_group_cover(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    mut multipart: Multipart,
) -> Response {
    // 1. Auth
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // 2. Resolve group
    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response(),
        Err(e) => {
            log::error!("❌ Lỗi truy vấn nhóm: {e}");
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi hệ thống.").into_response();
        }
    };

    // 3. Permission check — owner or admin
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(group_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let is_allowed = role.as_deref() == Some("owner") || role.as_deref() == Some("admin");
    if !is_allowed {
        return (axum::http::StatusCode::FORBIDDEN, "Bạn không có quyền đổi ảnh bìa.").into_response();
    }

    // 4. Read file from multipart
    let (file_bytes, _original_name, detected_mime) =
        match crate::handlers::uploads::read_multipart_file(&mut multipart, state.config.max_upload_bytes).await {
            Ok(result) => result,
            Err(resp) => return resp,
        };

    if file_bytes.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Không nhận được dữ liệu ảnh.").into_response();
    }

    // 5. Validate MIME
    let mime = detected_mime.as_deref().map_or_else(String::new, |m| crate::handlers::uploads::parse_mime(m).unwrap_or_default());
    let Some(ext) = crate::handlers::uploads::mime_to_ext(&mime) else {
        return (axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Định dạng ảnh không được hỗ trợ.").into_response();
    };

    // 6. Save file via upload helpers
    let file_id = Uuid::new_v4();
    let stored_filename = format!("{file_id}.{ext}");
    let sha256_str = crate::handlers::uploads::compute_sha256(&file_bytes);

    // Ensure upload_dir exists
    if let Err(e) = std::fs::create_dir_all(&state.config.upload_dir) {
        log::error!("❌ Không tạo được upload_dir: {e}");
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi server.").into_response();
    }

    let (width, height) = crate::handlers::uploads::parse_image_dimensions(&file_bytes, &mime);

    let Ok(image_id) = crate::handlers::uploads::insert_image_metadata(
        &state.pool, file_id, user.id, None, &stored_filename,
        &mime, &file_bytes, &sha256_str, width, height, ext,
    ).await else {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Không ghi được metadata ảnh.").into_response();
    };

    // Write file
    let file_path = state.config.upload_dir.join(&stored_filename);
    if let Err(e) = std::fs::write(&file_path, &file_bytes) {
        log::error!("❌ Lỗi ghi file ảnh: {e}");
        let _ = sqlx::query("DELETE FROM images WHERE id = $1").bind(image_id).execute(&state.pool).await;
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Không lưu được file ảnh.").into_response();
    }

    // 7. Update group's cover_upload_id
    let cover_url = format!("{}/{stored_filename}", state.config.upload_url_prefix);
    if let Err(e) = sqlx::query(
        "UPDATE groups SET cover_upload_id = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(image_id)
    .bind(group_id)
    .execute(&state.pool)
    .await
    {
        log::error!("❌ Lỗi cập nhật cover_upload_id: {e}");
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi cập nhật ảnh bìa.").into_response();
    }

    log::info!("🖼️ User {} updated cover image for group {slug}: {cover_url}", user.id);
    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// POST /cong-dong/nhom/{slug}/doi-logo — Đổi logo nhóm (icon đại diện).
///
/// v0.9.36 — Giai đoạn 41: Logo riêng (ảnh vuông nhỏ, khác với ảnh bìa banner).
/// Chỉ owner hoặc admin mới được đổi logo.
/// Accepts multipart form with `file` field (image/jpeg, image/png, image/webp, image/gif).
pub async fn change_group_logo(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
    mut multipart: Multipart,
) -> Response {
    // 1. Auth
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // 2. Resolve group
    let group_id: Uuid = match sqlx::query_scalar(
        "SELECT id FROM groups WHERE slug = $1 AND is_active = true",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return (axum::http::StatusCode::NOT_FOUND, "Nhóm không tồn tại.").into_response(),
        Err(e) => {
            log::error!("❌ Lỗi truy vấn nhóm (logo): {e}");
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi hệ thống.").into_response();
        }
    };

    // 3. Permission check — owner or admin
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(group_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let is_allowed = role.as_deref() == Some("owner") || role.as_deref() == Some("admin");
    if !is_allowed {
        return (axum::http::StatusCode::FORBIDDEN, "Bạn không có quyền đổi logo nhóm.").into_response();
    }

    // 4. Read file from multipart
    let (file_bytes, _original_name, detected_mime) =
        match crate::handlers::uploads::read_multipart_file(&mut multipart, state.config.max_upload_bytes).await {
            Ok(result) => result,
            Err(resp) => return resp,
        };

    if file_bytes.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Không nhận được dữ liệu ảnh logo.").into_response();
    }

    // 5. Validate MIME
    let mime = detected_mime.as_deref().map_or_else(String::new, |m| crate::handlers::uploads::parse_mime(m).unwrap_or_default());
    let Some(ext) = crate::handlers::uploads::mime_to_ext(&mime) else {
        return (axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Định dạng logo không được hỗ trợ. Chỉ chấp nhận JPEG, PNG, WebP, GIF.").into_response();
    };

    // 6. Save file via upload helpers
    let file_id = Uuid::new_v4();
    let stored_filename = format!("{file_id}.{ext}");
    let sha256_str = crate::handlers::uploads::compute_sha256(&file_bytes);

    // Ensure upload_dir exists
    if let Err(e) = std::fs::create_dir_all(&state.config.upload_dir) {
        log::error!("❌ Không tạo được upload_dir (logo): {e}");
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Lỗi server.").into_response();
    }

    let (width, height) = crate::handlers::uploads::parse_image_dimensions(&file_bytes, &mime);

    let Ok(image_id) = crate::handlers::uploads::insert_image_metadata(
        &state.pool, file_id, user.id, None, &stored_filename,
        &mime, &file_bytes, &sha256_str, width, height, ext,
    ).await else {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Không ghi được metadata ảnh logo.").into_response();
    };

    // Write file
    let file_path = state.config.upload_dir.join(&stored_filename);
    if let Err(e) = std::fs::write(&file_path, &file_bytes) {
        log::error!("❌ Lỗi ghi file logo: {e}");
        let _ = sqlx::query("DELETE FROM images WHERE id = $1").bind(image_id).execute(&state.pool).await;
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Không lưu được file logo.").into_response();
    }

    // 7. Update group's logo_upload_id
    // v0.9.38 — Giai đoạn 42: Nếu UPDATE fail (column không tồn tại do migration
    // 026 chưa được apply, hoặc DB error khác), redirect về trang nhóm với
    // ?err=... thay vì trả plain-text 500 (UX tệ). Group page đã có error banner
    // (v0.9.37 — fix lỗi "gửi bài không được"). Schema safety check ở db/mod.rs
    // đảm bảo cột logo_upload_id luôn tồn tại, nhưng vẫn wrap cho an toàn.
    let logo_url = format!("{}/{stored_filename}", state.config.upload_url_prefix);
    if let Err(e) = sqlx::query(
        "UPDATE groups SET logo_upload_id = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(image_id)
    .bind(group_id)
    .execute(&state.pool)
    .await
    {
        log::error!("❌ Lỗi cập nhật logo_upload_id (group {slug}, image {image_id}): {e}");
        // Cleanup file + image row đã tạo (tránh orphan rows)
        let _ = std::fs::remove_file(&file_path);
        let _ = sqlx::query("DELETE FROM images WHERE id = $1")
            .bind(image_id)
            .execute(&state.pool)
            .await;
        let err_msg = urlencoding::encode("Không cập nhật được logo nhóm. Vui lòng thử lại hoặc liên hệ admin kỹ thuật.");
        return Redirect::to(&format!("/cong-dong/nhom/{slug}?err={err_msg}")).into_response();
    }

    log::info!("🎨 User {} updated logo for group {slug}: {logo_url}", user.id);
    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

// ====================================================================
// Member Management — v0.9.23 Giai đoạn 28
// ====================================================================

/// POST /cong-dong/nhom/{slug}/duyet-thanh-vien/{member_id} — Duyệt thành viên đang chờ.
pub async fn approve_member(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((slug, member_id)): Path<(String, i64)>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Verify user is owner/admin of group OR staff
    let membership = get_membership_by_slug(&state.pool, &slug, user.id).await;
    let can_manage = membership.as_ref().is_some_and(|m| {
        m.status == "active" && (m.role == "owner" || m.role == "admin")
    }) || user.is_staff();

    if !can_manage {
        return (axum::http::StatusCode::FORBIDDEN, "Bạn không có quyền duyệt thành viên.").into_response();
    }

    // Approve the member
    let _ = sqlx::query(
        "UPDATE group_members SET status = 'active' WHERE id = $1 AND status = 'pending'",
    )
    .bind(member_id)
    .execute(&state.pool)
    .await;

    // Update member_count
    let _ = sqlx::query(
        "UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = (SELECT group_id FROM group_members WHERE id = $1) AND status = 'active') WHERE id = (SELECT group_id FROM group_members WHERE id = $1)",
    )
    .bind(member_id)
    .execute(&state.pool)
    .await;

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// POST /cong-dong/nhom/{slug}/xoa-thanh-vien/{member_id} — Xóa thành viên khỏi nhóm.
pub async fn remove_member(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((slug, member_id)): Path<(String, i64)>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Verify user is owner/admin of group OR staff
    let membership = get_membership_by_slug(&state.pool, &slug, user.id).await;
    let can_manage = membership.as_ref().is_some_and(|m| {
        m.status == "active" && (m.role == "owner" || m.role == "admin")
    }) || user.is_staff();

    if !can_manage {
        return (axum::http::StatusCode::FORBIDDEN, "Bạn không có quyền xóa thành viên.").into_response();
    }

    // Cannot remove owner
    let is_owner: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM group_members WHERE id = $1 AND role = 'owner')",
    )
    .bind(member_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if is_owner {
        return (axum::http::StatusCode::FORBIDDEN, "Không thể xóa chủ nhóm.").into_response();
    }

    // Delete the member
    let group_id: Option<Uuid> = sqlx::query_scalar(
        "DELETE FROM group_members WHERE id = $1 RETURNING group_id",
    )
    .bind(member_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Update member_count
    if let Some(gid) = group_id {
        let _ = sqlx::query(
            "UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = $1 AND status = 'active') WHERE id = $1",
        )
        .bind(gid)
        .execute(&state.pool)
        .await;
    }

    Redirect::to(&format!("/cong-dong/nhom/{slug}")).into_response()
}

/// Helper: Get membership by group slug instead of id.
async fn get_membership_by_slug(pool: &PgPool, slug: &str, user_id: Uuid) -> Option<GroupMember> {
    sqlx::query_as::<_, GroupMember>(
        "SELECT gm.id, gm.group_id, gm.user_id, gm.role, gm.status, gm.joined_at
         FROM group_members gm
         JOIN groups g ON g.id = gm.group_id
         WHERE g.slug = $1 AND gm.user_id = $2",
    )
    .bind(slug)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

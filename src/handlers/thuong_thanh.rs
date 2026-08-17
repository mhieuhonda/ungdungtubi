//! Handlers cho trang Thương Thành — Giai đoạn 40 (v0.9.35).
//! v0.9.40 — Giai đoạn 44: Rename "Chợ PvP" → "Chợ Đạo Hữu" + flexible categories
//!           + payment method K hoặc ngân hàng.
//!
//! Thương Thành là chợ/marketplace của Ứng Dụng Từ Bi — nơi đạo hữu
//! có thể mua bán, trao đổi vật phẩm Phật giáo và dịch vụ.
//!
//! Routes:
//!   - GET /thuong-thanh — Trang chính Thương Thành (2 cửa hàng)
//!   - GET /thuong-thanh/cua-hang-app — Cửa Hàng Ứng Dụng
//!   - GET /thuong-thanh/pvp — (back-compat) redirect → /thuong-thanh/cho-dao-huu
//!   - GET /thuong-thanh/cho-dao-huu — Chợ Đạo Hữu (rename từ PvP)
//!   - GET /thuong-thanh/vat-pham/{id} — Chi tiết vật phẩm
//!   - GET /thuong-thanh/vat-pham/tao — Form đăng bán (kèm categories)
//!   - POST /thuong-thanh/vat-pham/tao — Tạo vật phẩm (K hoặc bank)
//!   - POST /thuong-thanh/vat-pham/{id}/xoa — Xoá vật phẩm
//!   - GET /thuong-thanh/gio-hang — Giỏ hàng
//!   - POST /thuong-thanh/gio-hang/them — Thêm vào giỏ
//!   - POST /thuong-thanh/gio-hang/xoa/{cart_id} — Xoá khỏi giỏ
//!   - POST /thuong-thanh/gio-hang/thanh-toan — Thanh toán (giao dịch K hoặc bank)
//!   - GET /thuong-thanh/giao-dich — Lịch sử giao dịch
//!   - GET /api/thuong-thanh/stats — Thống kê
//!
//! v0.9.35: Remove Game store — only App + PvP.
//! v0.9.40: Rename PvP → Đạo Hữu. Cho phép user tạo danh mục mới và chọn
//!          nhận tiền qua K hoặc chuyển khoản ngân hàng.

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Json, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use serde_json::json;
use sqlx::PgPool;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;
use crate::models::thuong_thanh::{
    CartAddForm, CheckoutForm, ItemCreateForm, ShopCategory, ShopItem, ShopItemWithSeller,
    ThuongThanhStats,
};

// ─── Column list for shop_items ──────────────────────────────────────
// v0.9.40: thêm category_id, payment_method, price_vnd, bank_info,
// is_featured, moderation_status.

const ITEM_COLUMNS: &str = "id, store, category, name, description, price_k, icon, color, \
    seller_id, stock, sold_count, status, image_url, effects, sort_order, expires_at, \
    is_active, created_at, updated_at, \
    category_id, payment_method, price_vnd, bank_info, is_featured, moderation_status";

// ─── Template structs ────────────────────────────────────────────────

/// Template cho /thuong-thanh (trang chính).
#[derive(Template)]
#[template(path = "thuong-thanh/index.html")]
pub struct ThuongThanhTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub app_items: Vec<ShopItem>,
    pub pvp_items: Vec<ShopItemWithSeller>,
    pub stats: ThuongThanhStats,
}

/// Template cho /thuong-thanh/cua-hang-app.
#[derive(Template)]
#[template(path = "thuong-thanh/store.html")]
pub struct StoreTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub store_type: String,
    pub store_label: String,
    pub store_icon: String,
    pub store_color: String,
    pub items: Vec<ShopItem>,
    pub stats: ThuongThanhStats,
}

/// Template cho /thuong-thanh/cho-dao-huu (rename từ pvp.html).
#[derive(Template)]
#[template(path = "thuong-thanh/pvp.html")]
pub struct PvpTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub items: Vec<ShopItemWithSeller>,
    pub stats: ThuongThanhStats,
    /// v0.9.40: danh mục để user filter.
    pub categories: Vec<ShopCategory>,
}

/// Template cho /thuong-thanh/vat-pham/{id}.
#[derive(Template)]
#[template(path = "thuong-thanh/item.html")]
pub struct ItemTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub item: ShopItemWithSeller,
}

/// Template cho /thuong-thanh/gio-hang.
#[derive(Template)]
#[template(path = "thuong-thanh/cart.html")]
pub struct CartTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub cart_items: Vec<crate::models::thuong_thanh::CartItemWithItem>,
    pub total_k: i64,
    pub total_items: i64,
}

/// Template cho /thuong-thanh/vat-pham/tao.
#[derive(Template)]
#[template(path = "thuong-thanh/create.html")]
pub struct CreateItemTemplate {
    pub user: Option<User>,
    pub active_page: String,
    /// v0.9.40: danh sách danh mục để user chọn.
    pub categories: Vec<ShopCategory>,
    /// v0.9.40: error message nếu validate fail.
    pub error: Option<String>,
}

/// Template cho lịch sử giao dịch.
#[derive(Template)]
#[template(path = "thuong-thanh/transactions.html")]
pub struct TransactionsTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub transactions: Vec<crate::models::thuong_thanh::TransactionWithUsers>,
}

// ─── Helper ──────────────────────────────────────────────────────────

/// Lấy thống kê Thương Thành.
async fn get_stats(pool: &PgPool) -> ThuongThanhStats {
    let total_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shop_items WHERE is_active = true")
        .fetch_one(pool).await.unwrap_or(0);
    let app_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shop_items WHERE store = 'app' AND is_active = true")
        .fetch_one(pool).await.unwrap_or(0);
    // v0.9.40: đếm cả 'pvp' (cũ) và 'dao_huu' (mới) — cùng là Chợ Đạo Hữu.
    let pvp_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shop_items WHERE store IN ('pvp', 'dao_huu') AND is_active = true"
    ).fetch_one(pool).await.unwrap_or(0);
    let total_transactions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
        .fetch_one(pool).await.unwrap_or(0);
    let total_k_volume: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount_k), 0) FROM transactions WHERE status = 'completed'")
        .fetch_one(pool).await.unwrap_or(0);
    let total_fees: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(fee_k), 0) FROM transactions WHERE status = 'completed'")
        .fetch_one(pool).await.unwrap_or(0);
    let active_pvp_listings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shop_items WHERE store IN ('pvp', 'dao_huu') AND is_active = true AND status = 'active' AND (expires_at IS NULL OR expires_at > NOW())"
    ).fetch_one(pool).await.unwrap_or(0);

    ThuongThanhStats {
        total_items, app_items, pvp_items,
        total_transactions, total_k_volume, total_fees, active_pvp_listings,
    }
}

/// Fetch all active, approved shop_categories (cho dropdown ở form đăng bán
/// và cho filter ở trang Chợ Đạo Hữu).
async fn fetch_categories(pool: &PgPool) -> Vec<ShopCategory> {
    sqlx::query_as::<_, ShopCategory>(
        "SELECT id, slug, name_vi, description, icon, color, parent_id, sort_order, \
                is_system, is_approved, is_active, created_by, created_at, updated_at \
         FROM shop_categories \
         WHERE is_active = true AND is_approved = true \
         ORDER BY sort_order, name_vi"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Tạo slug từ tên tiếng Việt (bỏ dấu, thay khoảng trắng = '-').
fn slugify_vi(s: &str) -> String {
    // Bỏ dấu tiếng Việt
    let normalized: String = s.chars().filter_map(|c| {
        match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => Some('a'),
            'À' | 'Á' | 'Ạ' | 'Ả' | 'Ã' | 'Â' | 'Ầ' | 'Ấ' | 'Ậ' | 'Ẩ' | 'Ẫ' | 'Ă' | 'Ằ' | 'Ắ' | 'Ặ' | 'Ẳ' | 'Ẵ' => Some('a'),
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => Some('e'),
            'È' | 'É' | 'Ẹ' | 'Ẻ' | 'Ẽ' | 'Ê' | 'Ề' | 'Ế' | 'Ệ' | 'Ể' | 'Ễ' => Some('e'),
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => Some('i'),
            'Ì' | 'Í' | 'Ị' | 'Ỉ' | 'Ĩ' => Some('i'),
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ' | 'ợ' | 'ở' | 'ỡ' => Some('o'),
            'Ò' | 'Ó' | 'Ọ' | 'Ỏ' | 'Õ' | 'Ô' | 'Ồ' | 'Ố' | 'Ộ' | 'Ổ' | 'Ỗ' | 'Ơ' | 'Ờ' | 'Ớ' | 'Ợ' | 'Ở' | 'Ỡ' => Some('o'),
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => Some('u'),
            'Ù' | 'Ú' | 'Ụ' | 'Ủ' | 'Ũ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ự' | 'Ử' | 'Ữ' => Some('u'),
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => Some('y'),
            'Ỳ' | 'Ý' | 'Ỵ' | 'Ỷ' | 'Ỹ' => Some('y'),
            'đ' => Some('d'),
            'Đ' => Some('d'),
            _ => Some(c),
        }
    }).collect();
    normalized
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

// ─── Handlers ────────────────────────────────────────────────────────

/// GET /thuong-thanh — Trang Thương Thành chính.
pub async fn thuong_thanh_index(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    // Lấy items cho mỗi store (limit 8 cho trang chính)
    let app_items = sqlx::query_as::<_, ShopItem>(
        &format!("SELECT {ITEM_COLUMNS} FROM shop_items WHERE store = 'app' AND is_active = true ORDER BY sort_order, id LIMIT 8")
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    let pvp_items = sqlx::query_as::<_, ShopItemWithSeller>(
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar, \
                sc.name_vi AS category_name, sc.slug AS category_slug, sc.icon AS category_icon \
         FROM shop_items si \
         LEFT JOIN users u ON si.seller_id = u.id \
         LEFT JOIN shop_categories sc ON si.category_id = sc.id \
         WHERE si.store IN ('pvp', 'dao_huu') AND si.is_active = true \
           AND si.moderation_status = 'approved' \
           AND (si.expires_at IS NULL OR si.expires_at > NOW()) \
         ORDER BY si.is_featured DESC, si.created_at DESC LIMIT 8")
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    let stats = get_stats(&state.pool).await;

    let html = ThuongThanhTemplate {
        user,
        active_page: "thuong_thanh".into(),
        app_items, pvp_items, stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (thuong-thanh): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /thuong-thanh/cua-hang-app — Cửa Hàng Ứng Dụng.
pub async fn store_app(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let items = sqlx::query_as::<_, ShopItem>(
        &format!("SELECT {ITEM_COLUMNS} FROM shop_items WHERE store = 'app' AND is_active = true ORDER BY sort_order, id")
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    let stats = get_stats(&state.pool).await;

    let html = StoreTemplate {
        user,
        active_page: "thuong_thanh".into(),
        store_type: "app".into(),
        store_label: "Cửa Hàng Ứng Dụng".into(),
        store_icon: "🛒".into(),
        store_color: "#0F766E".into(),
        items, stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (store-app): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /thuong-thanh/pvp — Back-compat redirect → /thuong-thanh/cho-dao-huu.
/// v0.9.40: Rename "Chợ PvP" → "Chợ Đạo Hữu". Route cũ redirect 301.
pub async fn store_pvp_redirect(
    State(_state): State<AppState>,
    _jar: CookieJar,
) -> Response {
    Redirect::permanent("/thuong-thanh/cho-dao-huu").into_response()
}

/// GET /thuong-thanh/cho-dao-huu — Chợ Đạo Hữu (rename từ PvP).
pub async fn store_dao_huu(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let items = sqlx::query_as::<_, ShopItemWithSeller>(
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar, \
                sc.name_vi AS category_name, sc.slug AS category_slug, sc.icon AS category_icon \
         FROM shop_items si \
         LEFT JOIN users u ON si.seller_id = u.id \
         LEFT JOIN shop_categories sc ON si.category_id = sc.id \
         WHERE si.store IN ('pvp', 'dao_huu') AND si.is_active = true \
           AND si.moderation_status = 'approved' \
           AND (si.expires_at IS NULL OR si.expires_at > NOW()) \
         ORDER BY si.is_featured DESC, si.created_at DESC")
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    let stats = get_stats(&state.pool).await;
    let categories = fetch_categories(&state.pool).await;

    let html = PvpTemplate {
        user,
        active_page: "thuong_thanh".into(),
        items, stats, categories,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (dao-huu): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /thuong-thanh/vat-pham/{id} — Chi tiết vật phẩm.
pub async fn item_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let item = sqlx::query_as::<_, ShopItemWithSeller>(
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar, \
                sc.name_vi AS category_name, sc.slug AS category_slug, sc.icon AS category_icon \
         FROM shop_items si \
         LEFT JOIN users u ON si.seller_id = u.id \
         LEFT JOIN shop_categories sc ON si.category_id = sc.id \
         WHERE si.id = $1")
    )
    .bind(id)
    .fetch_optional(&state.pool).await.unwrap_or(None);

    let Some(item) = item else {
        return Html("<html><body><h1>Vật phẩm không tồn tại</h1><a href='/thuong-thanh'>← Quay lại</a></body></html>").into_response();
    };

    let html = ItemTemplate {
        user,
        active_page: "thuong_thanh".into(),
        item,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (item-detail): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /thuong-thanh/vat-pham/tao — Form đăng bán (Chợ Đạo Hữu).
/// v0.9.40: kèm danh sách categories để user chọn hoặc tạo mới.
pub async fn create_item_form(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    if user.is_none() {
        return Redirect::to("/dang-nhap").into_response();
    }

    let categories = fetch_categories(&state.pool).await;

    let html = CreateItemTemplate {
        user,
        active_page: "thuong_thanh".into(),
        categories,
        error: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (create-item): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /thuong-thanh/vat-pham/tao — Tạo vật phẩm (Chợ Đạo Hữu).
/// v0.9.40: hỗ trợ chọn category có sẵn HOẶC tạo mới, payment K HOẶC bank.
pub async fn create_item(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ItemCreateForm>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate form (returns category_id, payment_method, bank_info, price_vnd)
    let validated = match form.validate() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("⚠️ Tạo vật phẩm thất bại: {e}");
            // Re-render form với error message
            let categories = fetch_categories(&state.pool).await;
            let html = CreateItemTemplate {
                user: Some(u.clone()),
                active_page: "thuong_thanh".into(),
                categories,
                error: Some(e),
            }
            .render()
            .unwrap_or_else(|err| {
                log::error!("Template render error (create-item re-render): {err}");
                format!("<html><body><h1>Lỗi render template</h1><pre>{err}</pre></body></html>")
            });
            return Html(html).into_response();
        }
    };

    let icon = form.icon.as_deref().unwrap_or("📦");
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    // Xử lý category: nếu user nhập new_category_name → INSERT mới, lấy id.
    // Nếu không, dùng validated.category_id (đã có sẵn) hoặc fallback 'khac'.
    let (final_category_id, final_category_text) = if let Some(new_name) = &form.new_category_name {
        if !new_name.trim().is_empty() {
            let slug = slugify_vi(new_name);
            let cat_icon = form.new_category_icon.as_deref().unwrap_or("📦");
            // INSERT user-submitted category (is_system = false, is_approved = false → cần admin duyệt)
            let row: Result<(i64,), _> = sqlx::query_as(
                "INSERT INTO shop_categories (slug, name_vi, icon, color, is_system, is_approved, is_active, created_by) \
                 VALUES ($1, $2, $3, '#C62828', false, false, true, $4) \
                 ON CONFLICT (slug) DO UPDATE SET name_vi = EXCLUDED.name_vi \
                 RETURNING id"
            )
            .bind(&slug)
            .bind(new_name.trim())
            .bind(cat_icon)
            .bind(u.id)
            .fetch_one(&state.pool).await;

            match row {
                Ok((cid,)) => (Some(cid), slug),
                Err(e) => {
                    log::error!("❌ Lỗi tạo danh mục mới '{new_name}': {e}");
                    // Fallback: dùng 'khac' nếu tạo category fail
                    (None, "khac".to_string())
                }
            }
        } else {
            (validated.category_id, validated.category_text.clone())
        }
    } else {
        (validated.category_id, validated.category_text.clone())
    };

    // Build bank_info JSON nếu payment_method = 'bank'
    let bank_info_json: serde_json::Value = validated.bank_info
        .as_ref()
        .map(|bi| bi.to_json())
        .unwrap_or(serde_json::json!({}));

    // Insert item — store = 'dao_huu' (mới). moderation_status = 'approved' (auto-approve
    // cho user-created items — admin có thể review sau qua /admin/thuong-thanh).
    let result = sqlx::query(
        "INSERT INTO shop_items (store, category, category_id, name, description, price_k, price_vnd, \
            payment_method, bank_info, icon, color, seller_id, stock, sort_order, expires_at, \
            is_active, moderation_status) \
         VALUES ('dao_huu', $1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9, '#C62828', $10, $11, 0, $12, true, 'approved')"
    )
    .bind(&final_category_text)
    .bind(final_category_id)
    .bind(&form.name)
    .bind(&form.description)
    .bind(form.price_k)
    .bind(validated.price_vnd)
    .bind(&validated.payment_method)
    .bind(&bank_info_json)
    .bind(icon)
    .bind(u.id)
    .bind(form.stock)
    .bind(expires_at)
    .execute(&state.pool).await;

    match result {
        Ok(_) => log::info!("✅ Vật phẩm '{}' tạo thành công bởi {} (payment: {})", form.name, u.display_name, validated.payment_method),
        Err(e) => log::error!("❌ Lỗi tạo vật phẩm: {e}"),
    }

    Redirect::to("/thuong-thanh/cho-dao-huu").into_response()
}

/// POST /thuong-thanh/vat-pham/{id}/xoa — Xoá vật phẩm (chỉ seller hoặc admin).
pub async fn delete_item(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<i64>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Chỉ seller hoặc admin mới xoá được
    let result = sqlx::query(
        "UPDATE shop_items SET is_active = false, status = 'inactive', updated_at = NOW() \
         WHERE id = $1 AND (seller_id = $2 OR $3 = true)"
    )
    .bind(id)
    .bind(u.id)
    .bind(u.is_admin())
    .execute(&state.pool).await;

    match result {
        Ok(r) if r.rows_affected() > 0 => log::info!("✅ Vật phẩm #{id} đã xoá bởi {}", u.display_name),
        Ok(_) => log::warn!("⚠️ Không có quyền xoá vật phẩm #{id}"),
        Err(e) => log::error!("❌ Lỗi xoá vật phẩm #{id}: {e}"),
    }

    Redirect::to("/thuong-thanh/cho-dao-huu").into_response()
}

/// GET /thuong-thanh/gio-hang — Giỏ hàng.
pub async fn cart_view(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let cart_items = sqlx::query_as::<_, crate::models::thuong_thanh::CartItemWithItem>(
        "SELECT ci.id AS cart_id, ci.user_id, ci.item_id, ci.quantity, ci.added_at, \
                si.name AS item_name, si.icon AS item_icon, si.color AS item_color, \
                si.price_k AS item_price_k, si.store AS item_store, si.description AS item_description \
         FROM cart_items ci JOIN shop_items si ON ci.item_id = si.id \
         WHERE ci.user_id = $1 ORDER BY ci.added_at DESC"
    )
    .bind(u.id)
    .fetch_all(&state.pool).await.unwrap_or_default();

    let total_k: i64 = cart_items.iter().map(|c| c.total_k() as i64).sum();
    let total_items: i64 = cart_items.iter().map(|c| c.quantity as i64).sum();

    let html = CartTemplate {
        user,
        active_page: "thuong_thanh".into(),
        cart_items, total_k, total_items,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cart): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /thuong-thanh/gio-hang/them — Thêm vào giỏ hàng.
/// v0.9.40: nếu item là 'bank' payment, KHÔNG thêm vào giỏ — chuyển hướng
/// buyer tới trang chi tiết vật phẩm (bank transfer cần liên hệ seller).
pub async fn cart_add(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<CartAddForm>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let quantity = form.quantity.unwrap_or(1).max(1);

    // Check payment_method của item
    let item_payment: Option<String> = sqlx::query_scalar(
        "SELECT payment_method FROM shop_items WHERE id = $1 AND is_active = true"
    )
    .bind(form.item_id)
    .fetch_optional(&state.pool).await.unwrap_or(None);

    match item_payment.as_deref() {
        Some("bank") => {
            // Bank items: KHÔNG cho vào giỏ. Redirect tới trang chi tiết để buyer
            // xem bank info và liên hệ seller trực tiếp.
            return Redirect::to(&format!("/thuong-thanh/vat-pham/{}?bank=1", form.item_id)).into_response();
        }
        _ => {} // 'k' hoặc NULL → thêm vào giỏ bình thường
    }

    // Upsert: nếu item đã có trong giỏ thì tăng quantity
    let result = sqlx::query(
        "INSERT INTO cart_items (user_id, item_id, quantity) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, item_id) DO UPDATE SET quantity = cart_items.quantity + $3"
    )
    .bind(u.id)
    .bind(form.item_id)
    .bind(quantity)
    .execute(&state.pool).await;

    match result {
        Ok(_) => log::info!("✅ Thêm vật phẩm #{} vào giỏ hàng cho {}", form.item_id, u.display_name),
        Err(e) => log::error!("❌ Lỗi thêm vào giỏ: {e}"),
    }

    Redirect::to("/thuong-thanh/gio-hang").into_response()
}

/// POST /thuong-thanh/gio-hang/xoa/{cart_id} — Xoá khỏi giỏ hàng.
pub async fn cart_remove(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(cart_id): Path<i64>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let _ = sqlx::query("DELETE FROM cart_items WHERE id = $1 AND user_id = $2")
        .bind(cart_id)
        .bind(u.id)
        .execute(&state.pool).await;

    Redirect::to("/thuong-thanh/gio-hang").into_response()
}

/// POST /thuong-thanh/gio-hang/thanh-toan — Thanh toán giỏ hàng (giao dịch K).
/// v0.9.40: chỉ áp dụng cho K-payment items. Bank-payment items không qua giỏ.
pub async fn cart_checkout(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(_form): Form<CheckoutForm>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Lấy giỏ hàng
    let cart_items = sqlx::query_as::<_, crate::models::thuong_thanh::CartItemWithItem>(
        "SELECT ci.id AS cart_id, ci.user_id, ci.item_id, ci.quantity, ci.added_at, \
                si.name AS item_name, si.icon AS item_icon, si.color AS item_color, \
                si.price_k AS item_price_k, si.store AS item_store, si.description AS item_description \
         FROM cart_items ci JOIN shop_items si ON ci.item_id = si.id \
         WHERE ci.user_id = $1"
    )
    .bind(u.id)
    .fetch_all(&state.pool).await.unwrap_or_default();

    if cart_items.is_empty() {
        return Redirect::to("/thuong-thanh/gio-hang").into_response();
    }

    let total_k: i64 = cart_items.iter().map(|c| c.total_k() as i64).sum();

    // Kiểm tra số dư K
    let k_balance: i64 = sqlx::query_scalar("SELECT k_balance FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&state.pool).await.unwrap_or(0);

    if k_balance < total_k {
        log::warn!("⚠️ Không đủ K: cần {} K, có {} K", total_k, k_balance);
        return Redirect::to("/thuong-thanh/gio-hang").into_response();
    }

    // Bắt đầu giao dịch (trừ K, tạo transaction, xoá cart)
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ Lỗi bắt đầu DB transaction: {e}");
            return Redirect::to("/thuong-thanh/gio-hang").into_response();
        }
    };

    // Trừ K từ buyer
    if let Err(e) = sqlx::query("UPDATE users SET k_balance = k_balance - $1, updated_at = NOW() WHERE id = $2")
        .bind(total_k)
        .bind(u.id)
        .execute(&mut *tx).await
    {
        log::error!("❌ Lỗi trừ K: {e}");
        let _ = tx.rollback().await;
        return Redirect::to("/thuong-thanh/gio-hang").into_response();
    }

    // Process each cart item
    for ci in &cart_items {
        let is_dao_huu = ci.item_store == "pvp" || ci.item_store == "dao_huu";
        // v0.9.40: giảm fee từ 20% → 10% cho Chợ Đạo Hữu (PvP cũ vẫn 20%).
        let fee_k = if is_dao_huu {
            if ci.item_store == "dao_huu" {
                (ci.total_k() as f64 * 0.10).round() as i32
            } else {
                (ci.total_k() as f64 * 0.20).round() as i32
            }
        } else { 0 };
        let seller_gets = ci.total_k() - fee_k;

        // Fetch seller_id cho item này
        let seller_id: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT seller_id FROM shop_items WHERE id = $1"
        )
        .bind(ci.item_id)
        .fetch_optional(&mut *tx).await.unwrap_or(None);

        // Tạo transaction — v0.9.40: thêm payment_method = 'k'
        let _ = sqlx::query(
            "INSERT INTO transactions (tx_type, buyer_id, seller_id, item_id, quantity, amount_k, fee_k, status, payment_method) \
             VALUES ('purchase', $1, $2, $3, $4, $5, $6, 'completed', 'k')"
        )
        .bind(u.id)
        .bind(seller_id)
        .bind(ci.item_id)
        .bind(ci.quantity)
        .bind(ci.total_k())
        .bind(fee_k)
        .execute(&mut *tx).await;

        // Cộng K cho seller (Đạo Hữu/PvP)
        if is_dao_huu && seller_gets > 0 {
            if let Some(sid) = seller_id {
                let _ = sqlx::query("UPDATE users SET k_balance = k_balance + $1, updated_at = NOW() WHERE id = $2")
                    .bind(seller_gets)
                    .bind(sid)
                    .execute(&mut *tx).await;
            }
        }

        // Cập nhật sold_count
        let _ = sqlx::query("UPDATE shop_items SET sold_count = sold_count + $1, updated_at = NOW() WHERE id = $2")
            .bind(ci.quantity)
            .bind(ci.item_id)
            .execute(&mut *tx).await;
    }

    // Xoá cart
    let _ = sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(u.id)
        .execute(&mut *tx).await;

    // Commit
    match tx.commit().await {
        Ok(_) => log::info!("✅ Thanh toán thành công: {} K cho {} items bởi {}", total_k, cart_items.len(), u.display_name),
        Err(e) => log::error!("❌ Lỗi commit thanh toán: {e}"),
    }

    Redirect::to("/thuong-thanh").into_response()
}

/// GET /thuong-thanh/giao-dich — Lịch sử giao dịch.
pub async fn transactions_view(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    let transactions = sqlx::query_as::<_, crate::models::thuong_thanh::TransactionWithUsers>(
        "SELECT t.id, t.tx_type, t.buyer_id, t.seller_id, t.item_id, t.quantity, t.amount_k, t.fee_k, t.status, t.note, t.created_at, \
                b.display_name AS buyer_name, s.display_name AS seller_name, \
                si.name AS item_name, si.icon AS item_icon \
         FROM transactions t \
         JOIN users b ON t.buyer_id = b.id \
         LEFT JOIN users s ON t.seller_id = s.id \
         LEFT JOIN shop_items si ON t.item_id = si.id \
         WHERE t.buyer_id = $1 OR t.seller_id = $1 \
         ORDER BY t.created_at DESC LIMIT 50"
    )
    .bind(u.id)
    .fetch_all(&state.pool).await.unwrap_or_default();

    let html = TransactionsTemplate {
        user,
        active_page: "thuong_thanh".into(),
        transactions,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (transactions): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// GET /api/thuong-thanh/stats — Thống kê API.
pub async fn thuong_thanh_stats_api(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    if user.is_none() {
        return Json(json!({"error": "Unauthorized"})).into_response();
    }

    let stats = get_stats(&state.pool).await;
    Json(json!(stats)).into_response()
}

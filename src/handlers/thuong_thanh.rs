//! Handlers cho trang Thương Thành — Giai đoạn 40 (v0.9.35).
//!
//! Thương Thành là chợ/marketplace của Ứng Dụng Từ Bi — nơi đạo hữu
//! có thể mua bán, trao đổi vật phẩm Phật giáo và dịch vụ.
//!
//! Routes:
//!   - GET /thuong-thanh — Trang chính Thương Thành (2 cửa hàng)
//!   - GET /thuong-thanh/cua-hang-app — Cửa Hàng Ứng Dụng
//!   - GET /thuong-thanh/pvp — Chợ PvP
//!   - GET /thuong-thanh/vat-pham/{id} — Chi tiết vật phẩm
//!   - POST /thuong-thanh/vat-pham/tao — Tạo vật phẩm (PvP)
//!   - POST /thuong-thanh/vat-pham/{id}/xoa — Xoá vật phẩm
//!   - GET /thuong-thanh/gio-hang — Giỏ hàng
//!   - POST /thuong-thanh/gio-hang/them — Thêm vào giỏ
//!   - POST /thuong-thanh/gio-hang/xoa/{cart_id} — Xoá khỏi giỏ
//!   - POST /thuong-thanh/gio-hang/thanh-toan — Thanh toán (giao dịch K)
//!   - GET /api/thuong-thanh/stats — Thống kê
//!
//! v0.9.35: Remove Game store — only App + PvP.

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
    CartAddForm, CheckoutForm, ItemCreateForm, ShopItem, ShopItemWithSeller,
    ThuongThanhStats,
};

// ─── Column list for shop_items ──────────────────────────────────────

const ITEM_COLUMNS: &str = "id, store, category, name, description, price_k, icon, color, \
    seller_id, stock, sold_count, status, image_url, effects, sort_order, expires_at, \
    is_active, created_at, updated_at";

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

/// Template cho /thuong-thanh/pvp.
#[derive(Template)]
#[template(path = "thuong-thanh/pvp.html")]
pub struct PvpTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub items: Vec<ShopItemWithSeller>,
    pub stats: ThuongThanhStats,
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
    let pvp_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shop_items WHERE store = 'pvp' AND is_active = true")
        .fetch_one(pool).await.unwrap_or(0);
    let total_transactions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
        .fetch_one(pool).await.unwrap_or(0);
    let total_k_volume: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount_k), 0) FROM transactions WHERE status = 'completed'")
        .fetch_one(pool).await.unwrap_or(0);
    let total_fees: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(fee_k), 0) FROM transactions WHERE status = 'completed'")
        .fetch_one(pool).await.unwrap_or(0);
    let active_pvp_listings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shop_items WHERE store = 'pvp' AND is_active = true AND status = 'active' AND (expires_at IS NULL OR expires_at > NOW())"
    ).fetch_one(pool).await.unwrap_or(0);

    ThuongThanhStats {
        total_items, app_items, pvp_items,
        total_transactions, total_k_volume, total_fees, active_pvp_listings,
    }
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
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar \
         FROM shop_items si LEFT JOIN users u ON si.seller_id = u.id \
         WHERE si.store = 'pvp' AND si.is_active = true AND (si.expires_at IS NULL OR si.expires_at > NOW()) \
         ORDER BY si.created_at DESC LIMIT 8")
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

/// GET /thuong-thanh/pvp — Chợ PvP.
pub async fn store_pvp(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let items = sqlx::query_as::<_, ShopItemWithSeller>(
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar \
         FROM shop_items si LEFT JOIN users u ON si.seller_id = u.id \
         WHERE si.store = 'pvp' AND si.is_active = true AND (si.expires_at IS NULL OR si.expires_at > NOW()) \
         ORDER BY si.created_at DESC")
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    let stats = get_stats(&state.pool).await;

    let html = PvpTemplate {
        user,
        active_page: "thuong_thanh".into(),
        items, stats,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (pvp): {e}");
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
        &format!("SELECT si.{ITEM_COLUMNS}, u.display_name AS seller_name, u.avatar_url AS seller_avatar \
         FROM shop_items si LEFT JOIN users u ON si.seller_id = u.id \
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

/// GET /thuong-thanh/vat-pham/tao — Form tạo vật phẩm PvP.
pub async fn create_item_form(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    if user.is_none() {
        return Redirect::to("/dang-nhap").into_response();
    }

    let html = CreateItemTemplate {
        user,
        active_page: "thuong_thanh".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (create-item): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /thuong-thanh/vat-pham/tao — Tạo vật phẩm PvP.
pub async fn create_item(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ItemCreateForm>,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let Some(ref u) = user else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate
    if let Err(e) = form.validate() {
        log::warn!("⚠️ Tạo vật phẩm thất bại: {e}");
        return Redirect::to("/thuong-thanh/pvp").into_response();
    }

    let icon = form.icon.as_deref().unwrap_or("📦");
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    // Insert PvP listing
    let result = sqlx::query(
        "INSERT INTO shop_items (store, category, name, description, price_k, icon, color, seller_id, stock, sort_order, expires_at, is_active) \
         VALUES ('pvp', $1, $2, $3, $4, $5, '#C62828', $6, $7, 0, $8, true)"
    )
    .bind(&form.category)
    .bind(&form.name)
    .bind(&form.description)
    .bind(form.price_k)
    .bind(icon)
    .bind(u.id)
    .bind(form.stock)
    .bind(expires_at)
    .execute(&state.pool).await;

    match result {
        Ok(_) => log::info!("✅ Vật phẩm PvP '{}' tạo thành công bởi {}", form.name, u.display_name),
        Err(e) => log::error!("❌ Lỗi tạo vật phẩm PvP: {e}"),
    }

    Redirect::to("/thuong-thanh/pvp").into_response()
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

    Redirect::to("/thuong-thanh/pvp").into_response()
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
    // Dùng transaction DB để đảm bảo atomic
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
        let fee_k = if ci.item_store == "pvp" { (ci.total_k() as f64 * 0.2).round() as i32 } else { 0 };
        let seller_gets = ci.total_k() - fee_k;

        // Tạo transaction
        let _ = sqlx::query(
            "INSERT INTO transactions (tx_type, buyer_id, seller_id, item_id, quantity, amount_k, fee_k, status) \
             VALUES ('purchase', $1, NULL, $2, $3, $4, $5, 'completed')"
        )
        .bind(u.id)
        .bind(ci.item_id)
        .bind(ci.quantity)
        .bind(ci.total_k())
        .bind(fee_k)
        .execute(&mut *tx).await;

        // Cộng K cho seller (PvP)
        if ci.item_store == "pvp" && seller_gets > 0 {
            let _ = sqlx::query("UPDATE users SET k_balance = k_balance + $1, updated_at = NOW() WHERE id = (SELECT seller_id FROM shop_items WHERE id = $2)")
                .bind(seller_gets)
                .bind(ci.item_id)
                .execute(&mut *tx).await;
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

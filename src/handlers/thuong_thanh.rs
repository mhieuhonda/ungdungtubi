//! Handlers cho trang Thương Thành — Giai đoạn 28 (v0.9.23).
//!
//! Thương Thành là chợ/marketplace của Ứng Dụng Từ Bi — nơi đạo hữu
//! có thể mua bán, trao đổi vật phẩm Phật giáo và dịch vụ.
//!
//! Routes:
//!   - GET /thuong-thanh — Trang chính Thương Thành (danh sách vật phẩm)
//!
//! v0.9.23: Phiên bản đầu tiên — UI hoàn chỉnh với danh mục vật phẩm,
//! hệ thống phân loại, và liên kết đến các tính năng sẽ phát triển.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Data ─────────────────────────────────────────────────────────────

/// Một danh mục vật phẩm trong Thương Thành.
#[allow(dead_code)]
pub struct ProductCategory {
    pub slug: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
    pub color: &'static str,
    pub item_count: u32,
}

/// Danh sách danh mục — hardcoded cho v0.9.23.
#[allow(dead_code)]
pub const PRODUCT_CATEGORIES: &[ProductCategory] = &[
    ProductCategory {
        slug: "phat-tu",
        name: "Phật Tử",
        icon: "🪷",
        description: "Tượng Phật, chuỗi hạt, lư hương, mộc thư",
        color: "#2E7D32",
        item_count: 0,
    },
    ProductCategory {
        slug: "kinh-sach",
        name: "Kinh Sách",
        icon: "📚",
        description: "Kinh điển, sách tu học, tài liệu Phật giáo",
        color: "#1565C0",
        item_count: 0,
    },
    ProductCategory {
        slug: "do-cung-tu",
        name: "Đồ Cúng Tụ",
        icon: "🕯️",
        description: "Nhang, đèn, hoa tươi, trái cây cúng dường",
        color: "#FF6F00",
        item_count: 0,
    },
    ProductCategory {
        slug: "trang-phuc",
        name: "Trang Phục",
        icon: "👘",
        description: "Áo lam, áo tràng, khăn tu, giày đi lễ",
        color: "#6A1B9A",
        item_count: 0,
    },
    ProductCategory {
        slug: "dich-vu",
        name: "Dịch Vụ",
        icon: "🤝",
        description: "Thiết kế chùa, in kinh, tổ chức lễ",
        color: "#0F766E",
        item_count: 0,
    },
    ProductCategory {
        slug: "khac",
        name: "Khác",
        icon: "📦",
        description: "Vật phẩm và dịch vụ khác",
        color: "#9E9E9E",
        item_count: 0,
    },
];

// ─── Template ─────────────────────────────────────────────────────────

/// Template cho trang /thuong-thanh.
#[derive(Template)]
#[template(path = "thuong-thanh/index.html")]
pub struct ThuongThanhTemplate {
    pub user: Option<User>,
    pub active_page: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────

/// GET /thuong-thanh — Trang Thương Thành.
pub async fn thuong_thanh_index(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let html = ThuongThanhTemplate {
        user,
        active_page: "thuong_thanh".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (thuong-thanh): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

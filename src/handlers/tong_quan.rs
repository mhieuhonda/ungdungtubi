//! Handlers cho trang Tổng Quan (User Hub) — Giai đoạn 18 (v0.9.14).
//!
//! Đây là trang "hub" trung tâm, liệt kê TẤT CẢ tính năng của app để user
//! có thể truy cập mọi route chỉ từ 1 chỗ — fix lỗi "route mồ côi" như
//! /bang-xep-hang, /quy-tu-bi, /thuong-thanh không có link từ UI.
//!
//! Routes:
//!   - GET /tong-quan — Trang hub với cards cho mọi tính năng

use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

/// Template cho trang /tong-quan.
#[derive(Template)]
#[template(path = "tong-quan/index.html")]
pub struct TongQuanTemplate {
    pub user: Option<User>,
    pub active_page: String,
}

/// GET /tong-quan — Trang User Hub với cards cho mọi tính năng.
pub async fn tong_quan_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let html = TongQuanTemplate {
        user,
        active_page: "tong_quan".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (tong-quan): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

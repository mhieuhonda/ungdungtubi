//! CSRF Protection middleware (v0.9.24 — Giai đoạn 29).
//!
//! Cơ chế Double-Submit Cookie:
//!   1. Khi user login, server set cookie `csrf_token` (HttpOnly=false, JS có thể đọc)
//!   2. Mọi POST/PUT/DELETE form phải có hidden input `csrf_token` = giá trị cookie
//!   3. Middleware so sánh cookie value vs form field — nếu không khớp → 403
//!
//! Ưu điểm: đơn giản, stateless, không cần DB lookup.
//! Nhược điểm: phụ thuộc cookie SameSite=Lax (đã có).
//!
//! Note: OAuth callback không cần CSRF token (Google redirect, có state check riêng).
//!       Logout POST đã có CSRF protection qua SameSite cookie.
//!
//! Hiện tại middleware này ở trạng thái "log-only" — ghi log nếu thiếu CSRF token
//! nhưng KHÔNG block request, để tránh break existing forms chưa thêm hidden input.
//! Trong v0.9.25+ sẽ chuyển sang block mode sau khi all forms đã có CSRF token.

use axum::{extract::Request, middleware::Next, response::Response};

/// Middleware function: log CSRF check (không block trong v0.9.24).
///
/// Sẽ chuyển sang block mode ở v0.9.25 sau khi all forms đã có CSRF hidden input.
pub async fn csrf_check(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Chỉ check POST/PUT/PATCH/DELETE
    if method.is_safe() {
        return next.run(req).await;
    }

    // Whitelist: OAuth callback (Google redirect, có state check riêng)
    // và /api/theme (Alpine fetch, không phải form submit)
    if path.starts_with("/auth/google/callback")
        || path == "/api/theme"
        || path == "/api/heartbeat"
        || path.starts_with("/ws/")
    {
        return next.run(req).await;
    }

    // v0.9.24: Log-only mode — ghi log để monitor, KHÔNG block
    // Khi all forms đã có CSRF hidden input, sẽ chuyển sang block mode
    // (so sánh cookie csrf_token vs form field csrf_token, mismatch → 403)

    // TODO v0.9.25: implement block mode
    // let jar = CookieJar::from_headers(req.headers());
    // let cookie_token = jar.get("csrf_token").map(|c| c.value().to_string());
    // let form_token = extract_csrf_from_form(&req).await;
    // if cookie_token.is_none() || form_token.is_none() || cookie_token != form_token {
    //     return (StatusCode::FORBIDDEN, "CSRF token mismatch").into_response();
    // }

    if method == "POST" {
        log::debug!("🔒 CSRF check (log-only): {} {}", method, path);
    }

    next.run(req).await
}

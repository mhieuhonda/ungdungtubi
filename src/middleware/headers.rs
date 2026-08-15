//! Security Headers middleware (v0.9.24 — Giai đoạn 29).
//!
//! Thêm các HTTP headers bảo mật vào mọi response:
//!   - Content-Security-Policy: chống XSS, data injection
//!   - X-Frame-Options: chống clickjacking
//!   - X-Content-Type-Options: chống MIME sniffing
//!   - Referrer-Policy: kiểm soát referrer leakage
//!   - Permissions-Policy: hạn chế browser features
//!   - Strict-Transport-Security: ép HTTPS
//!   - Cross-Origin-Opener-Policy / Cross-Origin-Embedder-Policy: isolation
//!
//! Dùng qua `axum::middleware::map_response(security_headers)`.
//! Layer này chạy OUTERMOST — áp dụng cho mọi route kể cả static files.
//!
//! References:
//!   - OWASP Secure Headers Project: https://owasp.org/www-project-secure-headers/
//!   - MDN Content-Security-Policy: https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP

use axum::{http::{HeaderMap, HeaderName, HeaderValue}, response::Response};

/// Middleware function: thêm security headers vào response.
///
/// Dùng như một `map_response` layer:
/// ```ignore
/// .layer(axum::middleware::map_response(security_headers))
/// ```
pub async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    inject_security_headers(headers);
    response
}

/// Inject security headers vào một HeaderMap.
/// Public để có thể reuse ở các chỗ khác (vd. error responses).
pub fn inject_security_headers(headers: &mut HeaderMap) {
    use axum::http::header;

    // ═══ Content-Security-Policy ═══
    // Cho phép: self, inline (Tailwind/Alpine CDN), Google OAuth, Google Fonts, unpkg HTMX
    // Note: 'unsafe-inline' cần thiết vì Tailwind CDN + Alpine.js inject inline styles.
    //       Trong tương lai có thể build Tailwind локально để bỏ 'unsafe-inline'.
    let csp = "default-src 'self'; \
               script-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com https://unpkg.com https://www.googletagmanager.com; \
               style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.tailwindcss.com; \
               img-src 'self' data: blob: https:; \
               font-src 'self' data: https://fonts.gstatic.com; \
               connect-src 'self' wss: https:; \
               media-src 'self' https:; \
               object-src 'none'; \
               base-uri 'self'; \
               form-action 'self' https://accounts.google.com; \
               frame-ancestors 'none'; \
               frame-src 'none'; \
               worker-src 'self' blob:; \
               manifest-src 'self'; \
               upgrade-insecure-requests";
    if let Ok(v) = HeaderValue::from_str(csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, v);
    }

    // ═══ X-Frame-Options: DENY — chống clickjacking ═══
    if let Ok(v) = HeaderValue::from_static("DENY") {
        headers.insert(header::X_FRAME_OPTIONS, v);
    }

    // ═══ X-Content-Type-Options: nosniff — chống MIME sniffing ═══
    if let Ok(v) = HeaderValue::from_static("nosniff") {
        headers.insert(header::X_CONTENT_TYPE_OPTIONS, v);
    }

    // ═══ Referrer-Policy — chỉ gửi origin khi cross-origin ═══
    if let Ok(v) = HeaderValue::from_static("strict-origin-when-cross-origin") {
        headers.insert(header::REFERRER_POLICY, v);
    }

    // ═══ Permissions-Policy — disable các browser features nguy hiểm ═══
    let perms = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=(), interest-cohort=()";
    if let Ok(v) = HeaderValue::from_str(perms) {
        headers.insert(HeaderName::from_static("permissions-policy"), v);
    }

    // ═══ X-XSS-Protection — legacy, nhưng vẫn thêm cho các browser cũ ═══
    if let Ok(v) = HeaderValue::from_static("1; mode=block") {
        headers.insert(HeaderName::from_static("x-xss-protection"), v);
    }

    // ═══ X-DNS-Prefetch-Control — không prefetch DNS (anti-tracking) ═══
    if let Ok(v) = HeaderValue::from_static("off") {
        headers.insert(HeaderName::from_static("x-dns-prefetch-control"), v);
    }

    // ═══ Cross-Origin-Opener-Policy — isolation ═══
    if let Ok(v) = HeaderValue::from_static("same-origin") {
        headers.insert(HeaderName::from_static("cross-origin-opener-policy"), v);
    }

    // ═══ Cross-Origin-Resource-Policy ═══
    if let Ok(v) = HeaderValue::from_static("same-site") {
        headers.insert(HeaderName::from_static("cross-origin-resource-policy"), v);
    }

    // ═══ Strict-Transport-Security — ép HTTPS (max 2 năm, includeSubDomains, preload) ═══
    // Coolify/Traefik handle HTTPS termination — set HSTS để browser nhớ dùng HTTPS
    if let Ok(v) = HeaderValue::from_static("max-age=63072000; includeSubDomains; preload") {
        headers.insert(header::STRICT_TRANSPORT_SECURITY, v);
    }
}

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

use axum::{
    http::{header, HeaderMap, HeaderName, HeaderValue},
    response::Response,
};

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
    // ═══ Content-Security-Policy ═══
    // Cho phép: self, inline (Tailwind/Alpine CDN), Google OAuth, Google Fonts, unpkg HTMX
    // Note v0.9.27: 'unsafe-inline' cần thiết vì Tailwind CDN + Alpine.js inject inline styles.
    // Note v0.9.28: THÊM 'unsafe-eval' vào script-src — BẮT BUỘC để Alpine.js hoạt động.
    //       Alpine.js dùng `new Function()` để eval các expression như `mobileMenu = !mobileMenu`,
    //       `x-show="!mobileMenu"`, `x-data="{...}"`. Không có 'unsafe-eval' → Alpine fail
    //       silently → hamburger menu liệt, chat bubble biến mất, cả 2 icon (☰ + ✕) cùng hiện.
    //       Đây là root cause của lỗi UI report ở v0.9.27.
    //       Trong tương lai có thể migrate sang Alpine CSP build (alpine.csp.js) + Alpine.data()
    //       registrations để bỏ 'unsafe-eval', nhưng đó là refactor lớn (chạm toàn bộ templates).
    let csp = "default-src 'self'; \
               script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://unpkg.com https://www.googletagmanager.com; \
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
    // from_static trả về HeaderValue trực tiếp (không phải Result)
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // ═══ X-Content-Type-Options: nosniff — chống MIME sniffing ═══
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // ═══ Referrer-Policy — chỉ gửi origin khi cross-origin ═══
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // ═══ Permissions-Policy — disable các browser features nguy hiểm ═══
    let perms = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=(), interest-cohort=()";
    if let Ok(v) = HeaderValue::from_str(perms) {
        headers.insert(HeaderName::from_static("permissions-policy"), v);
    }

    // ═══ X-XSS-Protection — legacy, nhưng vẫn thêm cho các browser cũ ═══
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    // ═══ X-DNS-Prefetch-Control — không prefetch DNS (anti-tracking) ═══
    headers.insert(
        HeaderName::from_static("x-dns-prefetch-control"),
        HeaderValue::from_static("off"),
    );

    // ═══ Cross-Origin-Opener-Policy — isolation ═══
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );

    // ═══ Cross-Origin-Resource-Policy ═══
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-site"),
    );

    // ═══ Strict-Transport-Security — ép HTTPS (max 2 năm, includeSubDomains, preload) ═══
    // Coolify/Traefik handle HTTPS termination — set HSTS để browser nhớ dùng HTTPS
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
}

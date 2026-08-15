//! Middleware module — security hardening (v0.9.24 — Giai đoạn 29).
//!
//! Các middleware bảo mật:
//!   - `headers::security_headers`: thêm HTTP headers bảo mật (CSP, X-Frame-Options, etc.)
//!   - `csrf::csrf_check`: kiểm tra CSRF token cho POST/PUT/DELETE requests
//!   - `rate_limit::rate_limit`: giới hạn request rate per IP (chống brute-force, DoS)
//!
//! References:
//!   - OWASP Secure Headers Project: https://owasp.org/www-project-secure-headers/
//!   - MDN Content-Security-Policy: https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP

pub mod csrf;
pub mod headers;
pub mod rate_limit;

pub use rate_limit::RateLimitState;

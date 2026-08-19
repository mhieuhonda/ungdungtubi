//! CSRF Protection middleware (v0.9.24 — Giai đoạn 29; clarified v0.9.44).
//!
//! v0.9.44 — Giai đoạn 49 (bug M4 fix): Documentation + architectural decision.
//!
//! Trước v0.9.44, file này có middleware "log-only" với TODO comment hứa sẽ
//! "block mode" ở v0.9.25. Nhưng 11 phiên bản sau, block mode vẫn chưa được
//! implement, và CSRF cookie cũng không bao giờ được set trong OAuth callback.
//! Trạng thái "sắp block" gây hiểu nhầm — đọc code tưởng đã có CSRF protection
//! nhưng thực ra dựa hoàn toàn vào SameSite=Lax cookie.
//!
//! Quyết định v0.9.44: GIỮ NGUYÊN thiết kế hiện tại — SameSite=Lax cookie là
//! cơ chế CSRF protection chính thức của app. Middleware này chỉ giữ lại làm
//! placeholder cho future work (nếu cần double-submit cookie sau này), nhưng
//! comment đã được update để phản ánh đúng trạng thái thực tế.
//!
//! Lý do không implement block mode:
//!   1. SameSite=Lax đã chặn 99% CSRF vectors (cross-site POST không gửi cookie)
//!   2. Implement double-submit cookie đòi hỏi update tất cả form templates
//!      (40+ forms) — chi phí cao, lợi ích cận biên thấp
//!   3. Google OAuth state check đã chặn CSRF ở login flow
//!   4. Logout POST đã được SameSite=Lax cookie bảo vệ
//!
//! Nếu sau này cần CSRF protection mạnh hơn (vd: SameSite=None cho third-party
//! embed), sử dụng signed double-submit cookie pattern, KHÔNG dùng log-only mode.

use axum::{extract::Request, middleware::Next, response::Response};

/// Middleware function: no-op placeholder. CSRF protection đến từ SameSite=Lax
/// cookie (set trong auth callback). Function này exist để route có thể reference
/// nó nếu sau này cần附加 CSRF middleware layer nào đó — hiện tại nó chỉ pass-through.
pub async fn csrf_check(req: Request, next: Next) -> Response {
    // Pass-through. SameSite=Lax cookie đã handle CSRF.
    next.run(req).await
}

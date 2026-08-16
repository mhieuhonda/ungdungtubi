use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;

use config::Config;
use handlers::chat::{DmChatHub, GlobalChatHub};
use middleware::RateLimitState;

/// Shared application state — replaces actix-web's `web::Data<T>`.
///
/// v0.9.3: thêm `global_chat_hub` cho Chat Chung toàn platform.
/// v0.9.5: thêm `dm_chat_hub` cho Direct Messages 1-1 (Giai đoạn 9).
/// v0.9.21: xoá `chat_hub` (group live chat) — chỉ giữ Chat Chung.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<Config>,
    pub global_chat_hub: GlobalChatHub,
    pub dm_chat_hub: DmChatHub,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Init logger
    env_logger::init();

    // Load config
    let config = Config::from_env();
    let bind_addr = format!("{}:{}", config.host, config.port);

    log::info!("🪷 Ứng Dụng Từ Bi v0.9.32 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("🌍 App base URL: {}", config.app_base_url);
    log::info!("📡 Server: {bind_addr}");
    log::info!("🔑 Google OAuth redirect_uri: {}", config.google_redirect_uri);
    log::info!("🖼️  Upload dir: {} (max {} bytes)", config.upload_dir.display(), config.max_upload_bytes);
    log::info!("📦 DB pool max: {}", config.db_max_connections);
    log::info!("📦 Phiên bản: v0.9.32 — Giai đoạn 37: Admin Phát Triển Dashboard + Logo Emoji 🪷 + Version Sync");

    // Database connection pool (lazy - connects when first query runs)
    let db_pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect_lazy(&config.database_url)
        .expect("Không thể tạo PostgreSQL pool");

    log::info!("✅ PostgreSQL pool đã cấu hình");

    // v0.9.10/v0.9.11: Safety schema check — ensure critical columns/tables exist
    // BEFORE sqlx migrations run. This fixes the "column i_balance does not exist"
    // login error caused by migration checksum mismatch or partial deploy.
    // Runs idempotent DDL directly, no dependency on _sqlx_migrations table.
    {
        match sqlx::query_scalar::<_, String>("SELECT version()")
            .fetch_one(&db_pool)
            .await
        {
            Ok(_) => {
                db::ensure_schema_safety(&db_pool).await;
            }
            Err(e) => {
                log::warn!("⚠️ DB chưa sẵn sàng cho safety check: {e}");
            }
        }
    }

    // Auto-run migrations on startup
    // (chỉ chạy khi APP_ENV=production hoặc RUN_MIGRATIONS=true)
    let should_migrate = config.is_production
        || std::env::var("RUN_MIGRATIONS").is_ok_and(|v| v == "true");
    if should_migrate {
        log::info!("🔄 Đang chạy migrations...");
        match sqlx::migrate!("./migrations").run(&db_pool).await {
            Ok(()) => log::info!("✅ Migrations đã chạy xong"),
            Err(e) => {
                log::error!("❌ Lỗi chạy migrations: {e}");
                log::error!("   Server vẫn khởi động. Safety schema đã chạy ở trên nên các cột quan trọng đã tồn tại.");
            }
        }
    } else {
        log::info!("ℹ️  Auto-migration bỏ qua (set RUN_MIGRATIONS=true hoặc APP_ENV=production để bật).");
    }

    // Test DB connectivity
    match sqlx::query_scalar::<_, String>("SELECT version()")
        .fetch_one(&db_pool)
        .await
    {
        Ok(version) => {
            log::info!("✅ PostgreSQL đã kết nối: {}", version.split('(').next().unwrap_or("unknown"));
        }
        Err(e) => {
            log::warn!("⚠️ Không kết nối được PostgreSQL: {e}");
            log::warn!("   Server vẫn khởi động. Một số route sẽ không hoạt động.");
        }
    }

    // Start background task: clean up expired sessions every hour
    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_hours(1));
        loop {
            interval.tick().await;
            match db::cleanup_expired_sessions(&cleanup_pool).await {
                Ok(count) if count > 0 => {
                    log::info!("🧹 Đã xoá {count} phiên hết hạn");
                }
                Ok(_) => {}
                Err(e) => {
                    log::warn!("⚠️ Lỗi xoá phiên hết hạn: {e}");
                }
            }
        }
    });

    // Ensure upload directory exists
    if let Err(e) = std::fs::create_dir_all(&config.upload_dir) {
        log::warn!("⚠️ Không tạo được upload_dir {}: {e}", config.upload_dir.display());
    }

    // v0.9.25 FIX (bug B4): Trước v0.9.25, spawn_cleanup_task nhận `RateLimitState::new()`
    // (instance MỚI với empty map) trong khi middleware thực tế dùng `RateLimitState::get_global()`
    // (OnceLock singleton — instance KHÁC). Cleanup task làm trống map rỗng thay vì global map
    // → memory leak theo thời gian.
    // Fix: spawn cleanup task dùng đúng global instance.
    middleware::rate_limit::spawn_cleanup_task(RateLimitState::get_global().clone());

    // Build shared state (v0.9.3: + global_chat_hub; v0.9.5: + dm_chat_hub; v0.9.21: - chat_hub)
    let state = AppState {
        pool: db_pool,
        config: Arc::new(config.clone()),
        global_chat_hub: GlobalChatHub::default(),
        dm_chat_hub: DmChatHub::default(),
    };

    // Build router
    let static_dir = config.static_dir.clone();
    let app = build_router(state, static_dir);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    log::info!("🚀 Server đang chạy tại http://{bind_addr}");
    log::info!("🪷 Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    log::info!("👋 Server đã dừng hẳn. Nguyện công đức vô lượng.");
    Ok(())
}

/// Build the axum Router with all routes.
///
/// v0.9.24: Thêm security middleware layers (rate limit + CSRF + headers).
fn build_router(state: AppState, static_dir: std::path::PathBuf) -> Router {
    use axum::middleware as axum_mw;

    Router::new()
        // Routes — Trang chủ
        .route("/", get(handlers::home))
        // Routes — Auth (Google OAuth là phương thức đăng nhập duy nhất)
        .route("/dang-nhap", get(handlers::login_page).post(handlers::auth::google_login))
        // POST-only logout để chống CSRF (bỏ GET /dang-xuat)
        .route("/dang-xuat", post(handlers::auth::logout))
        // Google OAuth endpoints
        .route("/auth/google", get(handlers::auth::google_login))
        .route("/auth/google/callback", get(handlers::auth::google_callback))
        // Routes — 4 Chuyên Mục Chính
        .route("/khong-gian", get(handlers::khong_gian::khong_gian_index))
        .route("/cong-dong", get(handlers::cong_dong))
        .route("/ban-be", get(handlers::ban_be))
        .route("/kinh-sach", get(handlers::kinh_sach))
        // Routes — Không Gian (v0.9.9 — Giai đoạn 13: Niệm Phật + Tượng Phật + Nhật ký)
        .route("/api/niem-phat", post(handlers::khong_gian::niem_phat))
        .route("/tuong-phat/cau-nguyen", post(handlers::khong_gian::tuong_phat_cau_nguyen))
        .route("/tuong-phat/sam-hoi", post(handlers::khong_gian::tuong_phat_sam_hoi))
        .route("/tuong-phat/hoi-huong", post(handlers::khong_gian::tuong_phat_hoi_huong))
        .route("/api/khong-gian/stats", get(handlers::khong_gian::khong_gian_stats_api))
        // Routes — Kinh Sách (v0.9.6 — Giai đoạn 10)
        .route("/kinh-sach/tim-kiem", get(handlers::kinh_sach::kinh_sach_search))
        .route("/kinh-sach/thu-vien/{category_slug}", get(handlers::kinh_sach::kinh_sach_category))
        .route("/kinh-sach/{slug}/cam-ngo", post(handlers::kinh_sach::kinh_sach_submit_review))
        .route("/kinh-sach/{slug}/tang-hoa", post(handlers::kinh_sach::kinh_sach_give_flower))
        .route("/kinh-sach/{slug}/chuong/{chapter_slug}", get(handlers::kinh_sach::kinh_sach_chapter))
        .route("/kinh-sach/{slug}", get(handlers::kinh_sach::kinh_sach_book))
        // Routes — Cộng Đồng (v0.6+)
        .route(
            "/cong-dong/tao-nhom",
            get(handlers::community::create_group_form).post(handlers::community::create_group),
        )
        .route(
            "/cong-dong/nhom/{slug}",
            get(handlers::community::view_group),
        )
        .route(
            "/cong-dong/nhom/{slug}/tham-gia",
            post(handlers::community::join_group),
        )
        .route(
            "/cong-dong/nhom/{slug}/roi-khoi",
            post(handlers::community::leave_group),
        )
        // v0.9.23: Member management (owner/admin only)
        .route(
            "/cong-dong/nhom/{slug}/duyet-thanh-vien/{member_id}",
            post(handlers::community::approve_member),
        )
        .route(
            "/cong-dong/nhom/{slug}/xoa-thanh-vien/{member_id}",
            post(handlers::community::remove_member),
        )
        .route(
            "/cong-dong/nhom/{slug}/tao-chu-de",
            get(handlers::community::create_topic_form).post(handlers::community::create_topic),
        )
        .route(
            "/cong-dong/chu-de/{id}",
            get(handlers::community::view_topic),
        )
        .route(
            "/cong-dong/chu-de/{id}/binh-luan",
            post(handlers::community::create_comment),
        )
        // Routes — Chat Chung toàn platform (v0.9.3)
        // v0.9.21: Đã xoá group live chat routes (chỉ giữ Chat Chung)
        .route(
            "/ws/chat-chung",
            get(handlers::chat::global_chat_ws_upgrade),
        )
        .route(
            "/api/chat-chung/history",
            get(handlers::chat::global_chat_history),
        )
        // v0.9.31: REST fallback cho global chat — gửi tin nhắn qua HTTP khi WS không khả dụng
        .route(
            "/api/chat-chung/gui",
            post(handlers::chat::global_chat_send_rest),
        )
        // Routes — Hệ Thống
        .route("/quy-tu-bi", get(handlers::quy_tu_bi))
        .route("/thuong-thanh", get(handlers::thuong_thanh::thuong_thanh_index))
        .route("/doi-ngu-quan-li", get(handlers::doi_ngu::doi_ngu_quan_li))
        .route("/bang-xep-hang", get(handlers::bang_xep_hang::bang_xep_hang_index))
        // Routes — Tổng Quan (User Hub) [v0.9.14 — Giai đoạn 18]
        .route("/tong-quan", get(handlers::tong_quan::tong_quan_index))
        // Routes — Cài Đặt [v0.9.14 — Giai đoạn 18]
        .route("/cai-dat", get(handlers::cai_dat::cai_dat_index))
        .route("/cai-dat/cap-nhat", post(handlers::cai_dat::cai_dat_cap_nhat))
        // Routes — Thành Tích [v0.9.14 — Giai đoạn 19]
        .route("/thanh-tich", get(handlers::thanh_tich::thanh_tich_index))
        .route("/api/thanh-tich/stats", get(handlers::thanh_tich::thanh_tich_stats_api))
        // Routes — Tìm Kiếm toàn cục [v0.9.14 — Giai đoạn 19]
        .route("/tim-kiem", get(handlers::tim_kiem::tim_kiem_index))
        // Routes — Bảng Xếp Hạng (v0.9.10 — Giai đoạn 14)
        .route("/api/bang-xep-hang/stats", get(handlers::bang_xep_hang::bang_xep_hang_stats_api))
        // Routes — Quỹ Từ Bi (v0.9.11 — Giai đoạn 15)
        .route("/quy-tu-bi/dong-gop", post(handlers::quy_tu_bi::quy_tu_bi_dong_gop))
        .route("/api/quy-tu-bi/stats", get(handlers::quy_tu_bi::quy_tu_bi_stats_api))
        // Routes — Hồ sơ cá nhân
        .route("/ca-nhan", get(handlers::ca_nhan))
        .route("/ca-nhan/cap-nhat", post(handlers::cap_nhat_ho_so))
        .route("/ca-nhan/doi-anh-dai-dien", post(handlers::uploads::change_avatar))
        // Routes — Admin (v0.9.8 — Giai đoạn 12: 3 giao diện admin riêng biệt)
        .route("/admin", get(handlers::admin))
        .route("/admin/ky-thuat", get(handlers::admin::admin_ky_thuat_dashboard))
        .route("/admin/ky-thuat/users", get(handlers::admin::admin_ky_thuat_users_redirect))
        .route("/admin/cong-dong", get(handlers::admin::admin_cong_dong_dashboard))
        .route("/admin/quan-li", get(handlers::admin::admin_quan_li_dashboard))
        // v0.9.32: Dashboard riêng cho Admin Phát Triển (indigo, vision, roadmap, CI/CD)
        .route("/admin/phat-trien", get(handlers::admin::admin_phat_trien_dashboard))
        .route("/admin/thanh-vien", get(handlers::admin::admin_users_list))
        .route("/admin/thanh-vien/{user_id}/role", post(handlers::admin::admin_change_role))
        .route("/admin/thanh-vien/{user_id}/ban", post(handlers::admin::admin_ban_user))
        .route("/admin/thanh-vien/{user_id}/kich-hoat", post(handlers::admin::admin_activate_user))
        .route("/admin/ky-thuat/nhat-ky", get(handlers::admin::admin_audit_log_page))
        .route("/admin/cong-dong/cam-ngo", get(handlers::admin::admin_cam_ngo_list))
        .route("/admin/cong-dong/cam-ngo/{review_id}/duyet", post(handlers::admin::admin_cam_ngo_duyet))
        .route("/admin/cong-dong/cam-ngo/{review_id}/tu-choi", post(handlers::admin::admin_cam_ngo_tu_choi))
        // Routes — Admin placeholder pages (v0.9.17 — Giai đoạn 22: fix admin nav bug)
        // Trước đây các nav tile trong admin dashboard trỏ tới user pages (/cong-dong, /kinh-sach, ...)
        // khiến user click vào rồi bị redirect ra khỏi admin context.
        // Giờ tạo các route admin riêng cho các module chưa có UI quản trị đầy đủ.
        .route("/admin/cong-dong/nhom", get(handlers::admin::admin_groups_placeholder))
        .route("/admin/kinh-sach", get(handlers::admin::admin_kinh_sach_placeholder))
        .route("/admin/binh-luan", get(handlers::admin::admin_binh_luan_placeholder))
        .route("/admin/quy-tu-bi", get(handlers::admin::admin_quy_tu_bi_placeholder))
        // Routes — Theme toggle API (v0.9.17 — Giai đoạn 22)
        .route("/api/theme", post(handlers::cai_dat::api_theme_toggle))
        // Group cover image change (v0.9.3)
        .route(
            "/cong-dong/nhom/{slug}/doi-anh",
            post(handlers::community::change_group_cover),
        )
        // Routes — Bạn Bè (v0.9.5 — Giai đoạn 9)
        // Note: /ban-be route đã có ở trên (delegate sang handlers::friends::ban_be_index)
        .route("/ban-be/keu-ban/{user_id}", post(handlers::friends::send_friend_request))
        .route("/ban-be/chap-nhan/{friendship_id}", post(handlers::friends::accept_friend_request))
        .route("/ban-be/tu-choi/{friendship_id}", post(handlers::friends::decline_friend_request))
        .route("/ban-be/huy-ket-ban/{user_id}", post(handlers::friends::remove_friend))
        .route("/ban-be/tao-conversation", post(handlers::friends::create_conversation))
        .route("/ban-be/tin-nhan", get(handlers::friends::dm_inbox))
        .route("/ban-be/tin-nhan/{conversation_id}", get(handlers::friends::dm_view))
        .route(
            "/ws/ban-be/tin-nhan/{conversation_id}",
            get(handlers::friends::dm_ws_upgrade),
        )
        .route(
            "/api/ban-be/tin-nhan/{conversation_id}/history",
            get(handlers::friends::dm_history),
        )
        // v0.9.30: REST fallback gửi DM — đảm bảo tin nhắn LUÔN gửi được
        // ngay cả khi WebSocket fail (fix lỗi "không thể gửi tin nhắn cho bạn bè")
        .route(
            "/api/ban-be/tin-nhan/{conversation_id}/gui",
            post(handlers::friends::dm_send_message),
        )
        .route("/ban-be/thu", get(handlers::friends::mail_inbox))
        .route("/ban-be/thu/gui", get(handlers::friends::mail_compose_form).post(handlers::friends::mail_send))
        .route("/ban-be/thu/{mail_id}", get(handlers::friends::mail_view))
        .route("/ban-be/thong-bao", get(handlers::friends::notifications_list))
        .route("/api/ban-be/thong-bao/chua-doc", get(handlers::friends::notifications_unread_count))
        .route("/api/ban-be/thong-bao/{notification_id}/da-doc", post(handlers::friends::mark_notification_read))
        .route("/ban-be/tim-kiem", get(handlers::friends::search_users))
        // API
        // v0.9.23: /api/health yêu cầu auth + admin role — không công khai cho user thường
        .route("/api/health", get(health_check_secure))
        // v0.9.24: /api/ping — public health endpoint cho Docker healthcheck + monitoring
        // (không cần DB, không cần auth, luôn trả 200 "pong")
        .route("/api/ping", get(handlers::ping))
        .route("/api/heartbeat", post(handlers::heartbeat))
        // API — Upload ảnh (v0.5+)
        .route("/api/upload-info", get(handlers::uploads::upload_info))
        .route("/api/upload-image", post(handlers::uploads::upload_image))
        // Static files (CSS/JS/uploads) — tower-http ServeDir
        .nest_service("/static", ServeDir::new(static_dir))
        // Shared state
        .with_state(state)
        // Middleware (order matters: outermost last)
        // v0.9.24: Security headers — map_response (chỉ sửa response, không đọc request)
        .layer(axum_mw::map_response(middleware::headers::security_headers))
        // v0.9.24: CSRF check (log-only mode trong v0.9.24) — from_fn (đọc request, gọi next)
        .layer(axum_mw::from_fn(middleware::csrf::csrf_check))
        // v0.9.24: Rate limit (per-IP + per-endpoint) — from_fn, dùng global static state
        .layer(axum_mw::from_fn(middleware::rate_limit::rate_limit))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// GET /api/health — Health check JSON + DB status.
///
/// v0.9.20: Tách `features` array ra thành constant riêng để tránh
/// `serde_json::json!` recursion limit (array quá dài sau v0.9.20).
const HEALTH_FEATURES: &[&str] = &[
    "google-oauth",
    "profile-ranks",
    "image-upload",
    "community-groups-topics-comments",
    "live-chat-websocket",
    "global-chat-websocket",
    "avatar-upload",
    "group-cover-upload",
    "friends-system",
    "direct-messaging",
    "mail-inbox",
    "notifications",
    "scripture-library",
    "book-chapters",
    "book-reviews",
    "book-flowers",
    "admin-roles",
    "admin-panel",
    "role-based-permissions",
    "granular-permissions-150",
    "admin-ky-thuat-dashboard",
    "admin-cong-dong-dashboard",
    "admin-quan-li-dashboard",
    "admin-ky-thuat-mobile-ux-redesign",
    "admin-ky-thuat-users-redirect",
    "audit-log-activity-log",
    "admin-content-moderation-cam-ngo",
    "admin-user-ban-activate",
    "mobile-chat-keyboard-fix",
    "mobile-viewport-interactive-widget",
    "khong-gian-personal-space",
    "niem-phat-counter",
    "tuong-phat-vows",
    "practice-diary",
    "i-balance-nguyen-luc",
    "bang-xep-hang",
    "leaderboard-rankings",
    "practice-stats",
    "quy-tu-bi",
    "fund-donations",
    "fund-campaigns",
    "fund-expenses",
    "fund-summary-view",
    "user-hub-tong-quan",
    "user-settings-cai-dat",
    "achievements-system",
    "global-search-tim-kiem",
    "mega-menu-navigation",
    "mobile-drawer-navigation",
    "permissions-150-expanded",
    "ui-redesign-compact",
    "route-hub-tong-quan-v2",
    "kinh-sach-5-thu-vien-links",
    "bang-xep-hang-5-tabs-links",
    "admin-dashboard-quick-links",
    "admin-nav-fix-v0.9.17",
    "dark-mode-toggle",
    "theme-cookie-persistence",
    "admin-groups-placeholder",
    "admin-kinh-sach-placeholder",
    "admin-binh-luan-placeholder",
    "admin-quy-tu-bi-placeholder",
    "mobile-first-touch-targets",
    "mobile-ui-overhaul-v0.9.18",
    "admin-placeholder-back-role-aware-v0.9.18",
    "admin-quan-li-tabs-fix-v0.9.18",
    "mobile-drawer-auth-state-fix-v0.9.18",
    "admin-dashboards-responsive-v0.9.18",
    "users-page-back-role-aware-v0.9.18",
    "mod-role-v0.9.19",
    "admin-mod-chat-bypass-membership-v0.9.19",
    "admin-mod-message-effects-v0.9.19",
    "admin-ky-thuat-coder-effect-v0.9.19",
    "admin-quan-li-gold-frame-v0.9.19",
    "admin-cong-dong-shield-frame-v0.9.19",
    "mod-teal-frame-v0.9.19",
    "live-chat-community-fix-v0.9.19",
    "mod-can-view-admin-pages-v0.9.19",
    "mod-can-moderate-reviews-v0.9.19",
    "author-role-in-chat-messages-v0.9.19",
    "ws-ping-pong-keepalive-v0.9.20",
    "ws-idle-timeout-v0.9.20",
    "ws-max-message-size-v0.9.20",
    "app-level-ping-v0.9.20",
    "ws-health-check-v0.9.20",
    "optimistic-ui-v0.9.20",
    "message-queue-offline-v0.9.20",
    "send-timeout-retry-v0.9.20",
    "session-heartbeat-fix-v0.9.20",
    "removed-sound-effects-v0.9.21",
    "msg-slide-in-animation-v0.9.20",
    "send-btn-pulse-animation-v0.9.20",
    "conn-indicator-pulse-v0.9.20",
    "removed-group-live-chat-v0.9.21",
    "enlarged-global-chat-popup-v0.9.20",
    "debounced-scroll-rAF-v0.9.20",
    "messages-array-capped-200-v0.9.20",
    "dom-refs-cached-v0.9.20",
    "reduced-motion-support-v0.9.20",
    "removed-sound-toggle-v0.9.21",
    "security-health-check-admin-only-v0.9.23",
    "doi-ngu-quan-li-page-v0.9.22",
    "css-js-split-modules-v0.9.20",
    "body-data-logged-in-v0.9.20",
    "ws-close-code-1008-handling-v0.9.20",
    "reconnect-backoff-improved-v0.9.20",
    "member-management-v0.9.23",
    "thuong-thanh-marketplace-v0.9.23",
    "dm-ctrlmessage-fix-v0.9.23",
    "friends-list-mobile-ui-fix-v0.9.23",
    "doi-ngu-btn-in-tong-quan-v0.9.23",
    "admin-equal-permissions-v0.9.24",
    "permission-scope-per-domain-v0.9.24",
    "svg-lotus-redesign-v0.9.24",
    "favicon-svg-redraw-v0.9.24",
    "logo-svg-inline-v0.9.24",
    "csp-security-headers-v0.9.24",
    "rate-limit-middleware-v0.9.24",
    "csrf-check-log-only-v0.9.24",
    "audit-log-ip-tracking-v0.9.24",
    "login-attempts-table-v0.9.24",
    "rate-limit-log-table-v0.9.24",
    "hsts-strict-transport-security-v0.9.24",
    "permissions-policy-v0.9.24",
    "cross-origin-isolation-v0.9.24",
    "deploy-fix-v0.9.24",
    // v0.9.25 — Giai đoạn 30: Stability Fix + Critical Bug Fixes
    "login-session-csrf-token-fix-v0.9.25",
    "pgcrypto-extension-migration-fix-v0.9.25",
    "ensure-schema-safety-column-fix-v0.9.25",
    "rate-limit-cleanup-global-state-fix-v0.9.25",
    "search-cover-column-fix-v0.9.25",
    "admin-ky-thuat-change-role-fix-v0.9.25",
    "buddha-vow-char-count-fix-v0.9.25",
    "notifications-toctou-fix-v0.9.25",
    "version-drift-footer-fix-v0.9.25",
    // v0.9.26 features
    "hamburger-menu-click-outside-fix-v0.9.26",
    "hamburger-menu-escape-key-fix-v0.9.26",
    "hamburger-menu-icon-toggle-fix-v0.9.26",
    "hamburger-menu-link-click-closes-fix-v0.9.26",
    "chat-popup-backdrop-overlay-v0.9.26",
    "chat-popup-mobile-height-reduced-v0.9.26",
    "chat-popup-body-scroll-lock-v0.9.26",
    "chat-popup-escape-key-close-v0.9.26",
    "chat-popup-close-button-bigger-v0.9.26",
    "deploy-pipeline-single-commit-fix-v0.9.26",
    "dockerfile-coolify-auto-update-sha-v0.9.26",
    "duplicate-env-vars-cleanup-v0.9.26",
    // v0.9.28 — Giai đoạn 33: CSP Fix (Alpine.js) + XSS Hardening + Memory Leak Fix
    "csp-unsafe-eval-fix-alpine-js-v0.9.28",
    "xss-fix-auth-error-page-v0.9.28",
    "xss-fix-friends-htmx-responses-v0.9.28",
    "html-escape-utility-v0.9.28",
    "dm-chat-hub-memory-leak-fix-v0.9.28",
    "mobile-menu-click-working-v0.9.28",
    "chat-bubble-visible-when-logged-in-v0.9.28",
    "hamburger-icon-no-longer-duplicate-v0.9.28",
    // v0.9.29 — Giai đoạn 34: Admin Equal Rebalance + Live Chat Optimize + DM Fix + Performance
    "admin-equal-permissions-rebalance-v0.9.29",
    "doi-ngu-page-sync-with-code-v0.9.29",
    "live-chat-popup-taller-v0.9.29",
    "live-chat-backdrop-removed-v0.9.29",
    "admin-mod-msg-effects-removed-v0.9.29",
    "dm-send-queue-enabled-v0.9.29",
    "dm-autoreconnect-fast-v0.9.29",
    "css-animations-reduced-v0.9.29",
    "chat-scroll-performance-v0.9.29",
    "permission-count-sync-migration-021-v0.9.29",
    // v0.9.27 — Giai đoạn 32: Critical UI Fix (FOUC + Chat + Menu) + Chat History Robustness
    "fouc-fix-style-display-none-fallback-v0.9.27",
    "fouc-fix-x-cloak-class-specificity-v0.9.27",
    "chat-popup-never-auto-open-v0.9.27",
    "chat-popup-mobile-height-45dvh-v0.9.27",
    "chat-popup-close-button-bigger-v0.9.27",
    "chat-popup-togglechat-boolean-guard-v0.9.27",
    "chat-history-retry-with-backoff-v0.9.27",
    "mobile-menu-drawer-fouc-fix-v0.9.27",
    "mobile-menu-never-auto-open-v0.9.27",
    // v0.9.32 — Giai đoạn 37: Admin Phát Triển Dashboard + Logo Emoji 🪷 + Version Sync
    "admin-phat-trien-dashboard-v0.9.32",
    "admin-phat-trien-indigo-vision-theme-v0.9.32",
    "admin-4-dashboards-separate-v0.9.32",
    "logo-emoji-lotus-v0.9.32",
    "favicon-emoji-lotus-v0.9.32",
    "version-sync-v0.9.32",
];

/// GET /api/health — Health check (public minimal + admin full).
/// v0.9.24: Coolify/Docker health check cần endpoint trả 200 cho unauthenticated.
/// Trước đây v0.9.23 yêu cầu admin auth → Coolify health check fail → container bị mark unhealthy.
/// Giờ: unauthenticated nhận 200 `{"status":"ok","version":"0.9.28"}` (không lộ data nhạy cảm).
/// Admin/mod auth nhận full response (DB version, features, role hierarchy, user counts).
async fn health_check_secure(State(state): State<AppState>, jar: CookieJar) -> Response {
    use crate::handlers::get_user_from_session;

    // Check if user is authenticated staff
    let user = get_user_from_session(&state.pool, &jar).await;
    let is_staff = user.as_ref().is_some_and(|u| u.is_staff());

    if !is_staff {
        // Public minimal response — chỉ trả version + status, không lộ data nhạy cảm
        return Json(serde_json::json!({
            "status": "ok",
            "version": "0.9.32",
            "app": "Ứng Dụng Từ Bi"
        }))
        .into_response();
    }

    // Full response for authenticated staff
    health_check_inner(&state).await
}

/// Inner health check logic (extracted for reuse).
async fn health_check_inner(state: &AppState) -> Response {
    // DB ping
    let db_ok: Result<String, _> = sqlx::query_scalar("SELECT version()")
        .fetch_one(&state.pool)
        .await;

    let (db_status, db_version): (&str, String) = db_ok.map_or_else(|_| ("error", String::new()), |v| ("ok", v));

    // Kinh Sách stats (v0.9.6 — Giai đoạn 10)
    let kinh_sach_stats = handlers::kinh_sach::kinh_sach_stats(&state.pool).await;

    // Admin stats (v0.9.7 — Giai đoạn 11)
    let admin_stats = fetch_admin_stats_summary(&state.pool).await;

    // v0.9.20: Build features array từ const (tránh json! recursion limit)
    let features: Vec<&str> = HEALTH_FEATURES.to_vec();

    Json(serde_json::json!({
        "app": "Ứng Dụng Từ Bi",
        "version": "0.9.32",
        "domain": "tubi.louis.vangioitutien.com",
        "auth": "google-oauth-only",
        "phase": 37,
        "phase_name": "Giai đoạn 37 — Admin Phát Triển Dashboard + Logo Emoji 🪷 + Version Sync",
        "framework": "axum 0.8 + tower-http + ws",
        "status": "running",
        "features": features,
        "roles": {
            "hierarchy": ["admin_ky_thuat", "admin_quan_li", "admin_cong_dong", "admin_phat_trien", "mod", "member"],
            "default": "member",
            "permission_counts": {"admin_ky_thuat": 41, "admin_quan_li": 40, "admin_cong_dong": 45, "admin_phat_trien": 39, "mod": 15, "member": 0},
            "system_permission_counts": {"admin_ky_thuat": 41, "admin_quan_li": 40, "admin_cong_dong": 45, "admin_phat_trien": 39, "mod": 15, "member": 0},
            "admin_panel_access": ["admin_ky_thuat", "admin_quan_li", "admin_cong_dong", "admin_phat_trien", "mod"],
            "admin_ky_thuat_dashboard": "/admin/ky-thuat",
            "admin_cong_dong_dashboard": "/admin/cong-dong",
            "admin_quan_li_dashboard": "/admin/quan-li",
            "admin_phat_trien_dashboard": "/admin/phat-trien",
            "mod_dashboard": "/admin/thanh-vien",
            "v0_9_32_note": "Admin Phat Trien Dashboard rieng (/admin/phat-trien) + Logo emoji 🪷 + Version sync",
            "v0_9_30_note": "Them role admin_phat_trien (Admin Phat Trien) - 4 admin ngang hang cap 3",
            "v0_9_24_note": "Tất cả admin NGANG HÀNH (level 3) — mỗi admin có scope quyền riêng theo phần phụ trách"
        },
        "database": {
            "status": db_status,
            "version": db_version,
        },
        "khong_gian": {
            "status": "ok",
            "features": ["niem-phat", "tuong-phat-vows", "practice-diary", "i-balance"],
            "vow_types": ["prayer", "repentance", "dedication"],
            "i_rewards": {"prayer": 1, "repentance": 2, "dedication": 3}
        },
        "kinh_sach": kinh_sach_stats,
        "admin": admin_stats,
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
    .into_response()
}

/// Helper: fetch admin stats summary for /api/health.
/// Bất kỳ lỗi nào → trả về zeros (không fail health check).
async fn fetch_admin_stats_summary(pool: &sqlx::PgPool) -> serde_json::Value {
    let row: Result<(i64, i64, i64), _> = sqlx::query_as(
        "SELECT
            COUNT(*)::BIGINT,
            COUNT(*) FILTER (WHERE is_active)::BIGINT,
            COUNT(*) FILTER (WHERE role != 'member')::BIGINT
         FROM users",
    )
    .fetch_one(pool)
    .await;

    match row {
        Ok((total, active, admins)) => serde_json::json!({
            "total_users": total,
            "active_users": active,
            "admins": admins,
            "status": "ok"
        }),
        Err(e) => {
            log::warn!("⚠️ Health check: admin stats query failed: {e}");
            serde_json::json!({
                "status": "error",
                "error": e.to_string()
            })
        }
    }
}

/// Graceful shutdown signal handler — listens for Ctrl+C / SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    log::info!("🛑 Tín hiệu dừng nhận được — đang graceful shutdown (timeout 30s)...");
}

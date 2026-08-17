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

    log::info!("🪷 Ứng Dụng Từ Bi v0.9.39 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("🌍 App base URL: {}", config.app_base_url);
    log::info!("📡 Server: {bind_addr}");
    log::info!("🔑 Google OAuth redirect_uri: {}", config.google_redirect_uri);
    log::info!("🖼️  Upload dir: {} (max {} bytes)", config.upload_dir.display(), config.max_upload_bytes);
    log::info!("📦 DB pool max: {}", config.db_max_connections);
    log::info!("📦 Phiên bản: v0.9.40 — Giai đoạn 44: Chợ Đạo Hữu + Admin Thương Thành Hoàn Thiện + Payment K/Bank 🪷");

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
        // v0.9.37 — Trang giới thiệu chi tiết Ứng Dụng Từ Bi (công khai, không yêu cầu login)
        .route("/gioi-thieu", get(handlers::gioi_thieu))
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
        // Routes — Nhà Nhạc (v0.9.33 — Giai đoạn 38: Music House — KG-03)
        // Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx":
        //   5 thư mục nhạc (Niem/Thien/Dao/KhongLoi/CaNhan) + 5 chế độ phát + hẹn giờ tắt.
        .route("/khong-gian/nha-nhac", get(handlers::nha_nhac::nha_nhac_index))
        .route("/khong-gian/nha-nhac/{category}", get(handlers::nha_nhac::nha_nhac_category))
        .route("/api/nha-nhac/tracks", get(handlers::nha_nhac::nha_nhac_tracks_api))
        .route("/api/nha-nhac/tracks/{category}", get(handlers::nha_nhac::nha_nhac_tracks_by_category_api))
        .route("/api/nha-nhac/preferences", get(handlers::nha_nhac::nha_nhac_prefs_api).post(handlers::nha_nhac::nha_nhac_prefs_update))
        .route("/api/nha-nhac/ca-nhan/them", post(handlers::nha_nhac::nha_nhac_ca_nhan_add))
        .route("/api/nha-nhac/ca-nhan/xoa/{track_id}", post(handlers::nha_nhac::nha_nhac_ca_nhan_remove))
        .route("/api/nha-nhac/track/{track_id}/play", post(handlers::nha_nhac::nha_nhac_track_play))
        .route("/api/nha-nhac/stats", get(handlers::nha_nhac::nha_nhac_stats_api))
        // Routes — Nhạc Cộng Đồng (v0.9.35 — Giai đoạn 40: User music submissions)
        .route("/api/nha-nhac/dang-nhac", post(handlers::nha_nhac::nha_nhac_submit_music))
        .route("/admin/nha-nhac/dang-cho-duyet", get(handlers::nha_nhac::admin_music_pending))
        .route("/admin/nha-nhac/dang-cho-duyet/{id}", post(handlers::nha_nhac::admin_music_review))
        .route("/api/nha-nhac/submissions", get(handlers::nha_nhac::nha_nhac_my_submissions_api))
        .route("/api/nha-nhac/submissions/approved", get(handlers::nha_nhac::nha_nhac_community_music_api))
        .route("/api/nha-nhac/submission/{id}/play", post(handlers::nha_nhac::nha_nhac_submission_play))
        // v0.9.36 — Giai đoạn 41: Audio file upload (MP3/M4A/OGG/WAV/FLAC)
        .route("/api/nha-nhac/dang-nhac-file", post(handlers::nha_nhac::nha_nhac_submit_music_file))
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
        // Routes — Thương Thành (v0.9.35 — Giai đoạn 40: App + PvP, Game removed)
        // v0.9.40 — Giai đoạn 44: Rename "Chợ PvP" → "Chợ Đạo Hữu" + flexible categories + K/bank payment
        // CRUD vật phẩm · Giỏ hàng · Giao dịch K · Bank transfer
        .route("/thuong-thanh", get(handlers::thuong_thanh::thuong_thanh_index))
        .route("/thuong-thanh/cua-hang-app", get(handlers::thuong_thanh::store_app))
        .route("/thuong-thanh/pvp", get(handlers::thuong_thanh::store_pvp_redirect))
        .route("/thuong-thanh/cho-dao-huu", get(handlers::thuong_thanh::store_dao_huu))
        .route("/thuong-thanh/vat-pham/tao", get(handlers::thuong_thanh::create_item_form).post(handlers::thuong_thanh::create_item))
        .route("/thuong-thanh/vat-pham/{id}", get(handlers::thuong_thanh::item_detail))
        .route("/thuong-thanh/vat-pham/{id}/xoa", post(handlers::thuong_thanh::delete_item))
        .route("/thuong-thanh/gio-hang", get(handlers::thuong_thanh::cart_view))
        .route("/thuong-thanh/gio-hang/them", post(handlers::thuong_thanh::cart_add))
        .route("/thuong-thanh/gio-hang/xoa/{cart_id}", post(handlers::thuong_thanh::cart_remove))
        .route("/thuong-thanh/gio-hang/thanh-toan", post(handlers::thuong_thanh::cart_checkout))
        .route("/thuong-thanh/giao-dich", get(handlers::thuong_thanh::transactions_view))
        .route("/api/thuong-thanh/stats", get(handlers::thuong_thanh::thuong_thanh_stats_api))
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
        // v0.9.40 — Giai đoạn 44: Admin quản lý Thương Thành hoàn thiện
        .route("/admin/thuong-thanh", get(handlers::admin::admin_thuong_thanh_list))
        .route("/admin/thuong-thanh/danh-muc", get(handlers::admin::admin_thuong_thanh_categories))
        .route("/admin/thuong-thanh/{item_id}/xoa", post(handlers::admin::admin_thuong_thanh_delete))
        .route("/admin/thuong-thanh/{item_id}/noi-bat", post(handlers::admin::admin_thuong_thanh_toggle_featured))
        .route("/admin/thuong-thanh/{item_id}/duyet", post(handlers::admin::admin_thuong_thanh_approve))
        .route("/admin/thuong-thanh/{item_id}/tu-choi", post(handlers::admin::admin_thuong_thanh_reject))
        .route("/admin/thuong-thanh/danh-muc/tao", post(handlers::admin::admin_category_create))
        .route("/admin/thuong-thanh/danh-muc/{cat_id}/xoa", post(handlers::admin::admin_category_delete))
        .route("/admin/thuong-thanh/danh-muc/{cat_id}/duyet", post(handlers::admin::admin_category_approve))
        // Routes — Theme toggle API (v0.9.17 — Giai đoạn 22)
        .route("/api/theme", post(handlers::cai_dat::api_theme_toggle))
        // Group cover image change (v0.9.3)
        .route(
            "/cong-dong/nhom/{slug}/doi-anh",
            post(handlers::community::change_group_cover),
        )
        // v0.9.36 — Giai đoạn 41: Group logo change (icon đại diện, khác với ảnh bìa)
        .route(
            "/cong-dong/nhom/{slug}/doi-logo",
            post(handlers::community::change_group_logo),
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
        // v0.9.37 — Mark ALL notifications as read (bulk update)
        .route("/api/ban-be/thong-bao/da-doc-tat-ca", post(handlers::friends::mark_all_notifications_read))
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
    // v0.9.33 — Giai đoạn 38: Nhà Nhạc (Music House — KG-03) + Logo Emoji Sharpened
    "nha-nhac-music-house-v0.9.33",
    "music-player-5-categories-v0.9.33",
    "music-playback-modes-4-v0.9.33",
    "music-sleep-timer-v0.9.33",
    "music-personal-playlist-v0.9.33",
    "music-preferences-persisted-v0.9.33",
    "music-stats-play-count-v0.9.33",
    "logo-emoji-sharpened-geometric-precision-v0.9.33",
    "favicon-svg-256-viewbox-v0.9.33",
    "emoji-font-family-fallback-v0.9.33",
    // v0.9.34 — Giai đoạn 39: Thương Thành MVP (CRUD vật phẩm + Giỏ hàng + Giao dịch K)
    "thuong-thanh-mvp-v0.9.34",
    "thuong-thanh-crud-vat-pham-v0.9.34",
    "thuong-thanh-2-stores-v0.9.34",
    "thuong-thanh-gio-hang-v0.9.34",
    "thuong-thanh-giao-dich-k-v0.9.34",
    "thuong-thanh-pvp-listing-v0.9.34",
    "thuong-thanh-20-percent-fee-v0.9.34",
    "thuong-thanh-transactions-history-v0.9.34",
    "thuong-thanh-stats-api-v0.9.34",
    "open-graph-meta-tags-v0.9.34",
    "admin-dashboard-centering-fix-v0.9.34",
    // v0.9.35 — Giai đoạn 40: Nhạc Cộng Đồng (YouTube submissions, admin approval, Game cleanup)
    "user-music-submissions-v0.9.35",
    "music-youtube-embed-inline-v0.9.35",
    "music-admin-approval-v0.9.35",
    "music-rate-limit-5-per-day-v0.9.35",
    "music-duplicate-check-v0.9.35",
    "game-store-removed-v0.9.35",
    // v0.9.36 — Giai đoạn 41: Community Group Logo + Audio File Uploads
    "community-group-logo-upload-v0.9.36",
    "group-logo-change-endpoint-v0.9.36",
    "audio-files-table-v0.9.36",
    "music-audio-file-upload-mp3-v0.9.36",
    "music-audio-file-upload-m4a-v0.9.36",
    "music-audio-file-upload-ogg-v0.9.36",
    "music-audio-file-upload-wav-v0.9.36",
    "music-audio-file-upload-flac-v0.9.36",
    "music-audio-20mb-limit-v0.9.36",
    "music-audio-duration-estimate-v0.9.36",
    "music-audio-sha256-dedup-v0.9.36",
    "music-source-type-youtube-or-audio-v0.9.36",
    "admin-music-pending-shows-source-type-v0.9.36",
    // v0.9.37 — Giai đoạn 41 (phần 2): About Page + Orphan-Link Fix + Post-Submit Fix + Notification Mark-All + 429 Hardening
    "about-page-gioi-thieu-v0.9.37",
    "orphan-link-mobile-drawer-expanded-v0.9.37",
    "orphan-link-admin-music-pending-tile-v0.9.37",
    "orphan-link-nha-nhac-mega-menu-v0.9.37",
    "post-submit-silent-reject-fix-v0.9.37",
    "post-submit-group-name-empty-fix-v0.9.37",
    "post-submit-db-error-renders-form-v0.9.37",
    "post-submit-flash-error-banner-v0.9.37",
    "post-submit-stale-counter-fix-v0.9.37",
    "comment-submit-redirect-with-err-v0.9.37",
    "comment-submit-stale-counter-fix-v0.9.37",
    "notification-mark-all-as-read-endpoint-v0.9.37",
    "notification-mark-all-as-read-button-v0.9.37",
    "notification-per-item-mark-as-read-button-v0.9.37",
    "notification-badge-realtime-sync-v0.9.37",
    "rate-limit-api-60-to-180-v0.9.37",
    "rate-limit-social-60-to-180-v0.9.37",
    "rate-limit-general-120-to-300-v0.9.37",
    "rate-limit-post-30-to-60-v0.9.37",
    "rate-limit-block-60s-to-30s-v0.9.37",
    "rate-limit-classify-api-ban-be-to-social-v0.9.37",
    "rate-limit-classify-music-upload-correct-v0.9.37",
    "rate-limit-classify-kinh-sach-to-post-v0.9.37",
    "rate-limit-429-html-page-with-countdown-v0.9.37",
    "rate-limit-429-json-response-for-fetch-v0.9.37",
    "client-429-aware-fetch-wrapper-v0.9.37",
    "client-toast-notification-system-v0.9.37",
    "client-pause-polling-when-tab-hidden-v0.9.37",
    "client-resume-polling-on-tab-visible-v0.9.37",
    "client-notification-badge-pause-on-429-v0.9.37",
    // ─── v0.9.38 — Giai đoạn 42: Logo PNG + Group Logo Bug Fix + Music Submit Bug Fix + About Page Team Update
    "logo-png-replace-emoji-favicon-v0.9.38",
    "logo-png-replace-emoji-header-v0.9.38",
    "logo-png-replace-emoji-bottom-nav-v0.9.38",
    "logo-png-replace-emoji-footer-v0.9.38",
    "logo-png-replace-emoji-home-hero-v0.9.38",
    "logo-png-replace-emoji-login-page-v0.9.38",
    "logo-png-replace-emoji-error-pages-v0.9.38",
    "logo-png-og-image-twitter-card-v0.9.38",
    "logo-png-apple-touch-icon-v0.9.38",
    "group-logo-safety-schema-fix-v0.9.38",
    "music-submit-safety-schema-fix-v0.9.38",
    "audio-files-table-safety-schema-v0.9.38",
    "music-submissions-source-type-safety-schema-v0.9.38",
    "group-logo-error-redirect-with-err-v0.9.38",
    "music-submit-error-message-improved-v0.9.38",
    "about-page-team-update-cuong-hieu-v0.9.38",
    // ─── v0.9.39 — Giai đoạn 43: Active User Sync + Settings Fix + Stats Fix + Mobile Menu Accordion
    "active-user-sync-heartbeat-update-last_seen_at-v0.9.39",
    "active-users-count-real-online-v0.9.39",
    "user-list-last_seen_at-instead-of-session-created-v0.9.39",
    "user-settings-table-safety-schema-v0.9.39",
    "user-settings-relation-does-not-exist-fix-v0.9.39",
    "timezone-streak-fix-tz-asia-ho_chi_minh-v0.9.39",
    "timezone-today-niem-fix-local-tz-v0.9.39",
    "mobile-menu-accordion-compact-v0.9.39",
    "mobile-menu-alpine-collapse-plugin-v0.9.39",
    "mobile-menu-section-toggle-state-v0.9.39",
    "mobile-menu-quick-access-grid-v0.9.39",
    "heartbeat-update-last_seen_at-v0.9.39",
    "dockerfile-tz-asia-ho_chi_minh-v0.9.39",
    "dockerfile-tzdata-package-v0.9.39",
    "migration-027-user-last_seen_at-v0.9.39",
    "migration-027-seed-last_seen_at-from-sessions-v0.9.39",
    // ─── v0.9.40 — Giai đoạn 44: Chợ Đạo Hữu + Admin Thương Thành Hoàn Thiện + Payment K/Bank
    "cho-dao-huu-rename-from-pvp-v0.9.40",
    "pvp-route-redirect-to-cho-dao-huu-v0.9.40",
    "shop-categories-table-v0.9.40",
    "shop-categories-system-seed-12-v0.9.40",
    "shop-categories-user-create-v0.9.40",
    "shop-categories-user-needs-approval-v0.9.40",
    "shop-items-payment-method-k-or-bank-v0.9.40",
    "shop-items-price-vnd-column-v0.9.40",
    "shop-items-bank-info-jsonb-v0.9.40",
    "shop-items-is-featured-column-v0.9.40",
    "shop-items-moderation-status-v0.9.40",
    "shop-items-category-id-link-v0.9.40",
    "create-item-form-categories-dropdown-v0.9.40",
    "create-item-form-new-category-toggle-v0.9.40",
    "create-item-form-payment-method-toggle-v0.9.40",
    "create-item-form-bank-info-fields-v0.9.40",
    "create-item-form-qr-image-url-v0.9.40",
    "create-item-validation-bank-info-v0.9.40",
    "item-detail-show-bank-info-v0.9.40",
    "item-detail-show-qr-image-v0.9.40",
    "cart-block-bank-payment-items-v0.9.40",
    "cart-redirect-bank-to-item-detail-v0.9.40",
    "dao-huu-fee-reduced-20-to-10-percent-v0.9.40",
    "slugify-vi-for-categories-v0.9.40",
    "admin-thuong-thanh-list-page-v0.9.40",
    "admin-thuong-thanh-delete-item-v0.9.40",
    "admin-thuong-thanh-toggle-featured-v0.9.40",
    "admin-thuong-thanh-approve-item-v0.9.40",
    "admin-thuong-thanh-reject-item-v0.9.40",
    "admin-thuong-thanh-categories-page-v0.9.40",
    "admin-thuong-thanh-category-create-v0.9.40",
    "admin-thuong-thanh-category-approve-v0.9.40",
    "admin-thuong-thanh-category-delete-v0.9.40",
    "admin-thuong-thanh-audit-log-v0.9.40",
    "transactions-payment-method-column-v0.9.40",
    "transactions-price-vnd-column-v0.9.40",
    "transactions-bank-info-snapshot-v0.9.40",
    "transactions-buyer-contact-column-v0.9.40",
    "migration-028-cho-dao-huu-marketplace-v0.9.40",
    "safety-schema-shop-categories-v0.9.40",
    "safety-schema-shop-items-new-columns-v0.9.40",
    "safety-schema-transactions-new-columns-v0.9.40",
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
            "version": "0.9.40",
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
        "version": "0.9.40",
        "domain": "tubi.louis.vangioitutien.com",
        "auth": "google-oauth-only",
        "phase": 44,
        "phase_name": "Giai đoạn 44 — Chợ Đạo Hữu + Admin Thương Thành Hoàn Thiện + Payment K/Bank 🪷",
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
            "v0_9_36_note": "Giai đoạn 41 — Community group logo upload + audio file uploads (MP3/M4A/OGG/WAV/FLAC) for music submissions",
            "v0_9_37_note": "Giai đoạn 41 (phần 2) — Trang /gioi-thieu + fix orphan links (mobile drawer + admin music pending tile) + fix lỗi gửi bài (silent reject + empty group_name + stale counter) + nút đánh dấu đã đọc (per-item + mark-all) + fix 429 (tăng limit, sửa classify, HTML page, fetch wrapper)",
            "v0_9_38_note": "Giai đoạn 42 — Replace all web logos (favicon/header/footer/bottom-nav/home/login) với tubi.png (PNG thật, không còn emoji 🪷); fix bug 'Lỗi cập nhật logo nhóm' (safety schema cho groups.logo_upload_id); fix bug 'lỗi gửi bài' khi đăng nhạc (safety schema cho audio_files + user_music_submissions.source_type/audio_file_upload_id/audio_duration_seconds); cập nhật /gioi-thieu team info (Đỗ Văn Cường rút về hỗ trợ, Nguyễn Đình Minh Hiếu chuyển sang Admin Kỹ Thuật).",
            "v0_9_39_note": "Giai đoạn 43 — Fix 5 bug sync/stats/UI: (1) Active user sync — heartbeat handler giờ update users.last_seen_at mỗi 10 phút, admin stats active_users đếm WHERE last_seen_at > NOW()-5min (trước đây đếm is_active = 'không bị ban' → 5 active nhưng vào quản lý không thấy ai). (2) user_settings table safety schema — fix 'relation user_settings does not exist' khi lưu cài đặt. (3) Timezone streak/today_niem — set TZ=Asia/Ho_Chi_Minh trong Dockerfile, dùng chrono::Local thay Utc (trước đây user niệm phật 01:00 Saigon Aug 17 = 18:00 UTC Aug 16 → log_date = Aug 16 sai). (4) Mobile menu accordion — chia 27 items thành 6 section collapse/expand (trước đây tràn màn hình). (5) Migration 027 — users.last_seen_at column + seed từ MAX(sessions.created_at).",
            "v0_9_40_note": "Giai đoạn 44 — Rename 'Chợ PvP' → 'Chợ Đạo Hữu' (game đã bị xóa hoàn toàn). User đăng bán có thể chọn danh mục có sẵn HOẶC tạo mới (cần admin duyệt). Chọn nhận tiền K (10% phí, giảm từ 20%) HOẶC chuyển khoản ngân hàng (tự điền bank_name/account_number/account_holder/QR URL). Migration 028: shop_categories table + cột mới shop_items (payment_method, price_vnd, bank_info, category_id, is_featured, moderation_status) + cột mới transactions (payment_method, price_vnd, bank_info, buyer_contact). Admin Thương Thành hoàn thiện: /admin/thuong-thanh (list + duyệt + xóa + featured), /admin/thuong-thanh/danh-muc (CRUD categories). Bank-payment items không qua giỏ hàng — buyer xem bank info trên trang chi tiết + liên hệ seller trực tiếp.",
            "v0_9_33_note": "Nha Nhac (Music House KG-03) — 5 categories, 4 playback modes, sleep timer, personal playlist + Logo emoji sharpened",
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
            "features": ["niem-phat", "tuong-phat-vows", "practice-diary", "i-balance", "nha-nhac-music-house", "nha-nhac-audio-file-upload"],
            "vow_types": ["prayer", "repentance", "dedication"],
            "i_rewards": {"prayer": 1, "repentance": 2, "dedication": 3},
            "nha_nhac": {
                "status": "ok",
                "route": "/khong-gian/nha-nhac",
                "categories": ["niem", "thien", "dao", "khong_loi", "ca_nhan"],
                "playback_modes": ["single_repeat", "shuffle", "repeat_all", "loop"],
                "sleep_timer": true,
                "personal_playlist": true,
                "submission_sources": ["youtube", "audio_file"],
                "audio_formats": ["mp3", "m4a", "ogg", "wav", "flac"],
                "audio_max_bytes": 20971520
            }
        },
        "cong_dong": {
            "status": "ok",
            "features": ["groups", "topics", "comments", "group-cover-upload", "group-logo-upload-v0.9.36"],
            "group_logo_route": "POST /cong-dong/nhom/{slug}/doi-logo"
        },
        "kinh_sach": kinh_sach_stats,
        "admin": admin_stats,
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
    .into_response()
}

/// Helper: fetch admin stats summary for /api/health.
/// Bất kỳ lỗi nào → trả về zeros (không fail health check).
///
/// v0.9.39 — Giai đoạn 43 FIX (active user sync):
///   active_users đếm user có `last_seen_at > NOW() - INTERVAL '5 min'`
///   (đang online thật) thay vì `is_active` (không bị ban).
///   Fallback: nếu last_seen_at không tồn tại, dùng is_active.
async fn fetch_admin_stats_summary(pool: &sqlx::PgPool) -> serde_json::Value {
    let row: Result<(i64, i64, i64), _> = sqlx::query_as(
        "SELECT
            COUNT(*)::BIGINT,
            COUNT(*) FILTER (WHERE last_seen_at IS NOT NULL AND last_seen_at > NOW() - INTERVAL '5 minutes')::BIGINT,
            COUNT(*) FILTER (WHERE role != 'member')::BIGINT
         FROM users",
    )
    .fetch_one(pool)
    .await;

    let (total, active, admins) = match row {
        Ok((t, a, m)) => (t, a, m),
        Err(e) => {
            // Fallback: nếu last_seen_at không tồn tại, dùng is_active (cũ).
            log::warn!("⚠️ fetch_admin_stats_summary: last_seen_at query fail, fallback to is_active: {e}");
            sqlx::query_as(
                "SELECT
                    COUNT(*)::BIGINT,
                    COUNT(*) FILTER (WHERE is_active)::BIGINT,
                    COUNT(*) FILTER (WHERE role != 'member')::BIGINT
                 FROM users",
            )
            .fetch_one(pool)
            .await
            .map(|(t, a, m)| (t, a, m))
            .unwrap_or((0, 0, 0))
        }
    };

    serde_json::json!({
        "total_users": total,
        "active_users": active,
        "admins": admins,
        "status": "ok"
    })
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

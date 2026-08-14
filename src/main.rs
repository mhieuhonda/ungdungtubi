use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

mod config;
mod db;
mod errors;
mod handlers;
mod models;

use config::Config;
use handlers::chat::{ChatHub, DmChatHub, GlobalChatHub};

/// Shared application state — replaces actix-web's `web::Data<T>`.
///
/// v0.9.3: thêm `global_chat_hub` cho Chat Chung toàn platform.
/// v0.9.5: thêm `dm_chat_hub` cho Direct Messages 1-1 (Giai đoạn 9).
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<Config>,
    pub chat_hub: ChatHub,
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

    log::info!("🪷 Ứng Dụng Từ Bi v0.9.5 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("🌍 App base URL: {}", config.app_base_url);
    log::info!("📡 Server: {bind_addr}");
    log::info!("🔑 Google OAuth redirect_uri: {}", config.google_redirect_uri);
    log::info!("🖼️  Upload dir: {} (max {} bytes)", config.upload_dir.display(), config.max_upload_bytes);
    log::info!("📦 DB pool max: {}", config.db_max_connections);
    log::info!("📦 Phiên bản: v0.9.5 — Giai đoạn 9: Module Bạn Bè + Fix live chat bugs");

    // Database connection pool (lazy - connects when first query runs)
    let db_pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect_lazy(&config.database_url)
        .expect("Không thể tạo PostgreSQL pool");

    log::info!("✅ PostgreSQL pool đã cấu hình");

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
                log::error!("   Server vẫn khởi động để bạn có thể debug. Tắt RUN_MIGRATIONS để bỏ qua.");
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

    // Build shared state (v0.9.3: + chat_hub + global_chat_hub; v0.9.5: + dm_chat_hub)
    let state = AppState {
        pool: db_pool,
        config: Arc::new(config.clone()),
        chat_hub: ChatHub::default(),
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
fn build_router(state: AppState, static_dir: std::path::PathBuf) -> Router {
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
        .route("/khong-gian", get(handlers::khong_gian))
        .route("/cong-dong", get(handlers::cong_dong))
        .route("/ban-be", get(handlers::ban_be))
        .route("/kinh-sach", get(handlers::kinh_sach))
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
        // Routes — Live Chat (v0.9.2 — Giai đoạn 7)
        .route(
            "/ws/cong-dong/nhom/{slug}",
            get(handlers::chat::chat_ws_upgrade),
        )
        .route(
            "/api/cong-dong/nhom/{slug}/chat-history",
            get(handlers::chat::chat_history),
        )
        // Routes — Chat Chung toàn platform (v0.9.3)
        .route(
            "/ws/chat-chung",
            get(handlers::chat::global_chat_ws_upgrade),
        )
        .route(
            "/api/chat-chung/history",
            get(handlers::chat::global_chat_history),
        )
        // Routes — Hệ Thống
        .route("/quy-tu-bi", get(handlers::quy_tu_bi))
        .route("/thuong-thanh", get(handlers::thuong_thanh))
        .route("/bang-xep-hang", get(handlers::bang_xep_hang))
        // Routes — Hồ sơ cá nhân
        .route("/ca-nhan", get(handlers::ca_nhan))
        .route("/ca-nhan/cap-nhat", post(handlers::cap_nhat_ho_so))
        .route("/ca-nhan/doi-anh-dai-dien", post(handlers::uploads::change_avatar))
        // Group cover image change (v0.9.3)
        .route(
            "/cong-dong/nhom/{slug}/doi-anh",
            post(handlers::community::change_group_cover),
        )
        // Routes — Bạn Bè (v0.9.5 — Giai đoạn 9)
        .route("/ban-be", get(handlers::friends::ban_be_index))
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
        .route("/ban-be/thu", get(handlers::friends::mail_inbox))
        .route("/ban-be/thu/gui", get(handlers::friends::mail_compose_form).post(handlers::friends::mail_send))
        .route("/ban-be/thu/{mail_id}", get(handlers::friends::mail_view))
        .route("/ban-be/thong-bao", get(handlers::friends::notifications_list))
        .route("/api/ban-be/thong-bao/chua-doc", get(handlers::friends::notifications_unread_count))
        .route("/api/ban-be/thong-bao/{notification_id}/da-doc", post(handlers::friends::mark_notification_read))
        .route("/ban-be/tim-kiem", get(handlers::friends::search_users))
        // API
        .route("/api/health", get(health_check))
        .route("/api/heartbeat", post(handlers::heartbeat))
        // API — Upload ảnh (v0.5+)
        .route("/api/upload-info", get(handlers::uploads::upload_info))
        .route("/api/upload-image", post(handlers::uploads::upload_image))
        // Static files (CSS/JS/uploads) — tower-http ServeDir
        .nest_service("/static", ServeDir::new(static_dir))
        // Shared state
        .with_state(state)
        // Middleware (order matters: outermost last)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// GET /api/health — Health check JSON + DB status.
async fn health_check(State(state): State<AppState>) -> Response {
    // DB ping
    let db_ok: Result<String, _> = sqlx::query_scalar("SELECT version()")
        .fetch_one(&state.pool)
        .await;

    let (db_status, db_version): (&str, String) = db_ok.map_or_else(|_| ("error", String::new()), |v| ("ok", v));

    Json(serde_json::json!({
        "app": "Ứng Dụng Từ Bi",
        "version": "0.9.5",
        "domain": "tubi.louis.vangioitutien.com",
        "auth": "google-oauth-only",
        "phase": 9,
        "phase_name": "Giai đoạn 9 — Module Bạn Bè (Friends + DM + Mail + Notifications) + Fix live chat bugs",
        "framework": "axum 0.8 + tower-http + ws",
        "status": "running",
        "features": [
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
            "notifications"
        ],
        "database": {
            "status": db_status,
            "version": db_version,
        },
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
    .into_response()
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

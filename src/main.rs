use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;

mod config;
mod db;
mod errors;
mod handlers;
mod models;

use config::Config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Init logger
    env_logger::init();

    // Load config
    let config = Config::from_env();
    let bind_addr = format!("{}:{}", config.host, config.port);

    log::info!("🪷 Ứng Dụng Từ Bi v0.6 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("🌍 App base URL: {}", config.app_base_url);
    log::info!("📡 Server: {}", bind_addr);
    log::info!("🔑 Google OAuth redirect_uri: {}", config.google_redirect_uri);
    log::info!("🖼️  Upload dir: {:?} (max {} bytes)", config.upload_dir, config.max_upload_bytes);
    log::info!("📦 DB pool max: {}", config.db_max_connections);
    log::info!("📦 Phiên bản: v0.6 — Cộng Đồng Foundation (Nhóm + Chủ Đề + Bình luận)");

    // Database connection pool (lazy - connects when first query runs)
    let db_pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect_lazy(&config.database_url)
        .expect("Không thể tạo PostgreSQL pool");

    log::info!("✅ PostgreSQL pool đã cấu hình");

    // Auto-run migrations on startup
    // (chỉ chạy khi APP_ENV=production hoặc RUN_MIGRATIONS=true)
    let should_migrate = config.is_production
        || std::env::var("RUN_MIGRATIONS").map(|v| v == "true").unwrap_or(false);
    if should_migrate {
        log::info!("🔄 Đang chạy migrations...");
        match sqlx::migrate!("./migrations").run(&db_pool).await {
            Ok(_) => log::info!("✅ Migrations đã chạy xong"),
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
    actix_web::rt::spawn(async move {
        let mut interval =
            actix_web::rt::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match db::cleanup_expired_sessions(&cleanup_pool).await {
                Ok(count) if count > 0 => {
                    log::info!("🧹 Đã xoá {} phiên hết hạn", count);
                }
                Ok(_) => {}
                Err(e) => {
                    log::warn!("⚠️ Lỗi xoá phiên hết hạn: {}", e);
                }
            }
        }
    });

    // Ensure upload directory exists
    if let Err(e) = std::fs::create_dir_all(&config.upload_dir) {
        log::warn!("⚠️ Không tạo được upload_dir {:?}: {e}", config.upload_dir);
    }

    // Start server
    log::info!("🚀 Server đang chạy tại http://{}", bind_addr);
    log::info!("🪷 Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.");

    let static_dir = config.static_dir.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Static files (actix-files mặc định không list thư mục)
            .service(fs::Files::new("/static", &static_dir))
            // Routes — Trang chủ
            .route("/", web::get().to(handlers::home))
            // Routes — Auth (Google OAuth là phương thức đăng nhập duy nhất)
            .route("/dang-nhap", web::get().to(handlers::login_page))
            // /dang-nhap cũng nhận POST để tương thích với các form cũ (chuyển hướng sang Google)
            .route("/dang-nhap", web::post().to(handlers::auth::google_login))
            // POST-only logout để chống CSRF (bỏ GET /dang-xuat)
            .route("/dang-xuat", web::post().to(handlers::auth::logout))
            // Google OAuth endpoints
            .route("/auth/google", web::get().to(handlers::auth::google_login))
            .route(
                "/auth/google/callback",
                web::get().to(handlers::auth::google_callback),
            )
            // Routes — 4 Chuyên Mục Chính
            .route("/khong-gian", web::get().to(handlers::khong_gian))
            .route("/cong-dong", web::get().to(handlers::cong_dong))
            .route("/ban-be", web::get().to(handlers::ban_be))
            .route("/kinh-sach", web::get().to(handlers::kinh_sach))
            // Routes — Cộng Đồng (v0.6)
            .route("/cong-dong/tao-nhom", web::get().to(handlers::community::create_group_form))
            .route("/cong-dong/tao-nhom", web::post().to(handlers::community::create_group))
            .route("/cong-dong/nhom/{slug}", web::get().to(handlers::community::view_group))
            .route("/cong-dong/nhom/{slug}/tham-gia", web::post().to(handlers::community::join_group))
            .route("/cong-dong/nhom/{slug}/roi-khoi", web::post().to(handlers::community::leave_group))
            .route("/cong-dong/nhom/{slug}/tao-chu-de", web::get().to(handlers::community::create_topic_form))
            .route("/cong-dong/nhom/{slug}/tao-chu-de", web::post().to(handlers::community::create_topic))
            .route("/cong-dong/chu-de/{id}", web::get().to(handlers::community::view_topic))
            .route("/cong-dong/chu-de/{id}/binh-luan", web::post().to(handlers::community::create_comment))
            // Routes — Hệ Thống
            .route("/quy-tu-bi", web::get().to(handlers::quy_tu_bi))
            .route("/thuong-thanh", web::get().to(handlers::thuong_thanh))
            .route("/bang-xep-hang", web::get().to(handlers::bang_xep_hang))
            // Routes — Hồ sơ cá nhân
            .route("/ca-nhan", web::get().to(handlers::ca_nhan))
            .route("/ca-nhan/cap-nhat", web::post().to(handlers::cap_nhat_ho_so))
            // API
            .route("/api/health", web::get().to(health_check))
            .route("/api/heartbeat", web::post().to(handlers::heartbeat))
            // API — Upload ảnh (v0.5)
            .route("/api/upload-info", web::get().to(handlers::uploads::upload_info))
            .route(
                "/api/upload-image",
                web::post().to(handlers::uploads::upload_image),
            )
    })
    .bind(&bind_addr)?
    .workers(4)
    .shutdown_timeout(30) // graceful shutdown 30s
    .run()
    .await
}

async fn health_check(
    pool: web::Data<sqlx::PgPool>,
) -> actix_web::HttpResponse {
    // DB ping
    let db_ok: Result<String, _> = sqlx::query_scalar("SELECT version()")
        .fetch_one(pool.get_ref())
        .await;

    let (db_status, db_version): (&str, String) = match db_ok {
        Ok(v) => ("ok", v),
        Err(_) => ("error", String::new()),
    };

    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "app": "Ứng Dụng Từ Bi",
        "version": "0.6.0",
        "domain": "tubi.louis.vangioitutien.com",
        "auth": "google-oauth-only",
        "phase": 6,
        "phase_name": "Cộng Đồng Foundation — Nhóm + Chủ Đề + Bình luận",
        "status": "running",
        "database": {
            "status": db_status,
            "version": db_version,
        },
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
}

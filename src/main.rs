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

    log::info!("🪷 Ứng Dụng Từ Bi v0.3 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("📡 Server: {}", bind_addr);
    log::info!("🔑 Google OAuth redirect_uri: {}", config.google_redirect_uri);

    // Database connection pool (lazy - connects when first query runs)
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&config.database_url)
        .expect("Không thể tạo PostgreSQL pool");

    log::info!("✅ PostgreSQL pool đã cấu hình");

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

    // Start server
    log::info!("🚀 Server đang chạy tại http://{}", bind_addr);
    log::info!("🪷 Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Static files (no directory listing for security)
            .service(fs::Files::new("/static", "src/static"))
            // Routes — Trang chủ
            .route("/", web::get().to(handlers::home))
            // Routes — Auth (Google OAuth)
            .route("/dang-nhap", web::get().to(handlers::login_page))
            // /dang-nhap cũng nhận POST để tương thích với các form cũ (chuyển hướng sang Google)
            .route("/dang-nhap", web::post().to(handlers::auth::google_login))
            .route("/dang-xuat", web::post().to(handlers::auth::logout))
            .route("/dang-xuat", web::get().to(handlers::auth::logout))
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
            // Routes — Hệ Thống
            .route("/quy-tu-bi", web::get().to(handlers::quy_tu_bi))
            .route("/thuong-thanh", web::get().to(handlers::thuong_thanh))
            .route("/bang-xep-hang", web::get().to(handlers::bang_xep_hang))
            .route("/ca-nhan", web::get().to(handlers::ca_nhan))
            // API
            .route("/api/health", web::get().to(health_check))
            .route("/api/heartbeat", web::post().to(handlers::heartbeat))
    })
    .bind(&bind_addr)?
    .run()
    .await
}

async fn health_check() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "app": "Ứng Dụng Từ Bi",
        "version": "0.3.0",
        "domain": "tubi.louis.vangioitutien.com",
        "auth": "google-oauth-only",
        "status": "running",
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
}

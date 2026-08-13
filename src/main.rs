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
    
    log::info!("🪷 Ứng Dụng Từ Bi v0.2 — Khởi động...");
    log::info!("🌍 Domain: {}", config.domain);
    log::info!("📡 Server: {}", bind_addr);
    
    // Database connection pool (lazy - connects when first query runs)
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&config.database_url)
        .expect("Không thể tạo PostgreSQL pool");
    
    log::info!("✅ PostgreSQL pool đã cấu hình");
    
    // Start background task: clean up expired sessions every hour
    let cleanup_pool = db_pool.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(std::time::Duration::from_secs(3600));
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
            // Routes — Trang chủ & Auth
            .route("/", web::get().to(handlers::home))
            .route("/dang-nhap", web::get().to(handlers::login_page))
            .route("/dang-ky", web::get().to(handlers::register_page))
            .route("/dang-nhap", web::post().to(handlers::auth::login))
            .route("/dang-ky", web::post().to(handlers::auth::register))
            .route("/dang-xuat", web::post().to(handlers::auth::logout))
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
        "version": "0.2.0",
        "domain": "tubi.louis.vangioitutien.com",
        "status": "running",
        "message": "Nguyện công đức vô lượng. Nam Mô A Di Đà Phật."
    }))
}

#![allow(dead_code)]

use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub secret_key: String,
    pub is_production: bool,
    /// Google OAuth client ID
    pub google_client_id: String,
    /// Google OAuth client secret
    pub google_client_secret: String,
    /// OAuth redirect URI (e.g. https://tubi.louis.vangioitutien.com/auth/google/callback)
    pub google_redirect_uri: String,
    /// Base URL of the app for building absolute redirect URLs
    pub app_base_url: String,
    /// Directory containing static assets (CSS/JS/uploads).
    /// In Docker this is `/app/static`; in dev it falls back to `src/static`.
    pub static_dir: PathBuf,
    /// Directory for user-uploaded files (avatars, images).
    /// Defaults to `<static_dir>/uploads`.
    pub upload_dir: PathBuf,
    /// Maximum upload size per file in bytes (default 5 MB).
    pub max_upload_bytes: usize,
    /// Maximum DB pool connections (default 10).
    pub db_max_connections: u32,
    /// Public URL prefix for uploaded assets.
    pub upload_url_prefix: String,
}

impl Config {
    pub fn from_env() -> Self {
        let is_production = env::var("APP_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        let secret_key = env::var("SECRET_KEY").unwrap_or_else(|_| {
            if is_production {
                panic!("SECRET_KEY must be set in production environment");
            }
            log::warn!("⚠️ Using default SECRET_KEY — NOT for production!");
            "ung-dung-tu-bi-dev-secret-key".into()
        });

        let app_base_url = env::var("APP_BASE_URL").unwrap_or_else(|_| {
            if is_production {
                "https://tubi.louis.vangioitutien.com".into()
            } else {
                "http://localhost:8080".into()
            }
        });

        let google_client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
            if is_production {
                panic!("GOOGLE_CLIENT_ID must be set in production environment");
            }
            log::warn!("⚠️ GOOGLE_CLIENT_ID is not set — Google login will not work!");
            String::new()
        });

        let google_client_secret = env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| {
            if is_production {
                panic!("GOOGLE_CLIENT_SECRET must be set in production environment");
            }
            log::warn!("⚠️ GOOGLE_CLIENT_SECRET is not set — Google login will not work!");
            String::new()
        });

        // Default redirect URI follows the production domain + /auth/google/callback
        let default_redirect = format!("{app_base_url}/auth/google/callback");
        let google_redirect_uri =
            env::var("GOOGLE_REDIRECT_URI").unwrap_or(default_redirect);

        // Static directory: prefer STATIC_DIR env (set in Docker), else dev path.
        let static_dir = env::var("STATIC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Dev fallback: <project_root>/src/static
                let mut p = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                p.push("src");
                p.push("static");
                p
            });

        // Upload directory: prefer UPLOAD_DIR env, else <static_dir>/uploads
        let upload_dir = env::var("UPLOAD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = static_dir.clone();
                p.push("uploads");
                p
            });

        // Max upload size: default 5 MB
        let max_upload_bytes = env::var("MAX_UPLOAD_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5 * 1024 * 1024);

        // DB pool size: default 10
        let db_max_connections = env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let upload_url_prefix =
            env::var("UPLOAD_URL_PREFIX").unwrap_or_else(|_| "/static/uploads".into());

        Config {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://tubi:tubi_password@localhost:5432/ungdungtubi".into()
            }),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .unwrap_or(8080),
            domain: env::var("DOMAIN")
                .unwrap_or_else(|_| "tubi.louis.vangioitutien.com".into()),
            secret_key,
            is_production,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
            app_base_url,
            static_dir,
            upload_dir,
            max_upload_bytes,
            db_max_connections,
            upload_url_prefix,
        }
    }
}

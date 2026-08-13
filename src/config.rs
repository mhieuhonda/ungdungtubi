#![allow(dead_code)]

use std::env;

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
        }
    }
}

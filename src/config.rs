use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub secret_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://tubi:tubi_password@localhost:5432/ungdungtubi".into()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .unwrap_or(8080),
            domain: env::var("DOMAIN")
                .unwrap_or_else(|_| "tubi.louis.vangioitutien.com".into()),
            secret_key: env::var("SECRET_KEY")
                .unwrap_or_else(|_| "ung-dung-tu-bi-secret-key".into()),
        }
    }
}

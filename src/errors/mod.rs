use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Database(String),
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(msg) => write!(f, "Lỗi cơ sở dữ liệu: {msg}"),
            AppError::NotFound(msg) => write!(f, "Không tìm thấy: {msg}"),
            AppError::Unauthorized(msg) => write!(f, "Chưa xác thực: {msg}"),
            AppError::BadRequest(msg) => write!(f, "Yêu cầu không hợp lệ: {msg}"),
            AppError::Internal(msg) => write!(f, "Lỗi hệ thống: {msg}"),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Database(_) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": self.to_string()
            })),
            AppError::NotFound(_) => HttpResponse::NotFound().json(serde_json::json!({
                "error": self.to_string()
            })),
            AppError::Unauthorized(_) => HttpResponse::Unauthorized().json(serde_json::json!({
                "error": self.to_string()
            })),
            AppError::BadRequest(_) => HttpResponse::BadRequest().json(serde_json::json!({
                "error": self.to_string()
            })),
            AppError::Internal(_) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": self.to_string()
            })),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

#![allow(dead_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::fmt;

// AppError enum reserved for future phases (currently using Response directly)

#[derive(Debug)]
pub enum AppError {
    Database(String),
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Forbidden(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "Lỗi cơ sở dữ liệu: {msg}"),
            Self::NotFound(msg) => write!(f, "Không tìm thấy: {msg}"),
            Self::Unauthorized(msg) => write!(f, "Chưa xác thực: {msg}"),
            Self::BadRequest(msg) => write!(f, "Yêu cầu không hợp lệ: {msg}"),
            Self::Forbidden(msg) => write!(f, "Bị từ chối: {msg}"),
            Self::Internal(msg) => write!(f, "Lỗi hệ thống: {msg}"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            Self::Database(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

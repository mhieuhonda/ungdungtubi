use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub rank: String,
    pub a_balance: i64,
    pub k_balance: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl User {
    pub fn rank_display(&self) -> &str {
        match self.rank.as_str() {
            "new" => "Người Mới",
            "normal" => "Người Thường",
            "common" => "Người Bình Thường",
            "good" => "Người Tốt",
            "very_good" => "Người Khá Tốt",
            "great" => "Người Rất Tốt",
            "excellent" => "Người Cực Kỳ Tốt",
            "benevolent" => "Thiện Nhân",
            "tycoon" => "Đại Gia",
            _ => "Người Mới",
        }
    }
}

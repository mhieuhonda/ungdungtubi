//! Models cho Không Gian Cá Nhân — Giai đoạn 13 (v0.9.9).
//!
//! Bao gồm:
//!   - `PracticeLog` — nhật ký niệm Phật theo ngày (1 row/user/day)
//!   - `BuddhaVow` — lời Cầu Nguyện / Sám Hối / Hồi Hướng trước Tượng Phật
//!   - `BuddhaVowForm` — form payload từ UI Tượng Phật
//!   - `PublicVow` — vow hiển thị công khai trên bảng Kính Nguyện

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Nhật ký niệm Phật theo ngày — 1 row/user/day.
#[allow(dead_code)] // sqlx::query_as dùng qua FromRow nhưng Rust không thấy được.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PracticeLog {
    pub id: i64,
    pub user_id: Uuid,
    pub log_date: NaiveDate,
    pub niem_count: i64,
    pub last_niem_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Loại vow trước Tượng Phật.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VowType {
    Prayer,
    Repentance,
    Dedication,
}

impl VowType {
    /// Parse từ chuỗi request (form field `vow_type`).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "prayer" => Some(Self::Prayer),
            "repentance" => Some(Self::Repentance),
            "dedication" => Some(Self::Dedication),
            _ => None,
        }
    }

    /// Tên tiếng Việt hiển thị.
    pub fn display(&self) -> &'static str {
        match self {
            Self::Prayer => "Cầu Nguyện",
            Self::Repentance => "Sám Hối",
            Self::Dedication => "Hồi Hướng",
        }
    }

    /// Emoji đại diện.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Prayer => "🙏",
            Self::Repentance => "🙇",
            Self::Dedication => "🌸",
        }
    }

    /// Màu sắc (hex) cho badge.
    #[allow(dead_code)]
    pub fn color(&self) -> &'static str {
        match self {
            Self::Prayer => "#FFB300",
            Self::Repentance => "#795548",
            Self::Dedication => "#FF6F00",
        }
    }

    /// Phần thưởng I (Nguyên lực) khi thực hiện vow.
    pub fn i_reward(&self) -> i64 {
        match self {
            Self::Prayer => 1,
            Self::Repentance => 2,
            Self::Dedication => 3,
        }
    }

    /// Database string value.
    pub fn db_value(&self) -> &'static str {
        match self {
            Self::Prayer => "prayer",
            Self::Repentance => "repentance",
            Self::Dedication => "dedication",
        }
    }
}

/// Một vow trong DB.
#[allow(dead_code)] // sqlx::query_as dùng qua FromRow nhưng Rust không thấy được.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BuddhaVow {
    pub id: i64,
    pub user_id: Uuid,
    pub vow_type: String,
    pub content: String,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

/// Vow công khai hiển thị trên bảng Kính Nguyện — kèm thông tin tác giả.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PublicVow {
    pub id: i64,
    pub vow_type: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub author_name: String,
    pub author_avatar: Option<String>,
}

impl PublicVow {
    /// Emoji cho loại vow.
    pub fn icon(&self) -> &'static str {
        match self.vow_type.as_str() {
            "prayer" => "🙏",
            "repentance" => "🙇",
            "dedication" => "🌸",
            _ => "🪷",
        }
    }

    /// Tên hiển thị tiếng Việt cho loại vow.
    pub fn label(&self) -> &'static str {
        match self.vow_type.as_str() {
            "prayer" => "Cầu Nguyện",
            "repentance" => "Sám Hối",
            "dedication" => "Hồi Hướng",
            _ => "Vow",
        }
    }

    /// Màu sắc (hex) cho badge.
    pub fn color(&self) -> &'static str {
        match self.vow_type.as_str() {
            "prayer" => "#FFB300",
            "repentance" => "#795548",
            "dedication" => "#FF6F00",
            _ => "#2E7D32",
        }
    }
}

/// Form payload từ UI Tượng Phật.
#[derive(Debug, Deserialize)]
pub struct BuddhaVowForm {
    pub vow_type: String,
    pub content: String,
    pub is_public: Option<String>, // "on" nếu checkbox được tick
}

impl BuddhaVowForm {
    /// Validate form: vow_type hợp lệ, content 10–2000 ký tự.
    /// Trả về (VowType, content đã trim) hoặc None nếu không hợp lệ.
    pub fn validate(&self) -> Option<(VowType, String)> {
        let vt = VowType::from_str(self.vow_type.trim())?;
        let content = self.content.trim();
        if content.len() < 10 || content.len() > 2000 {
            return None;
        }
        Some((vt, content.to_string()))
    }

    /// `is_public` checkbox từ HTML form trả về "on" hoặc None.
    pub fn is_public_bool(&self) -> bool {
        self.is_public.as_deref() == Some("on")
    }
}

/// Stats cho trang Không Gian.
#[derive(Debug, Clone, Default, Serialize)]
pub struct KhongGianStats {
    /// Số lần niệm hôm nay.
    pub today_niem: i64,
    /// Tổng số lần niệm (all-time).
    pub total_niem: i64,
    /// Số ngày tu học liên tiếp (streak).
    pub streak_days: i32,
    /// Số vow đã thực hiện (all-time).
    pub total_vows: i64,
}

/// Dữ liệu 7 ngày gần nhất cho biểu đồ tu học.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DailyNiem {
    pub log_date: NaiveDate,
    pub niem_count: i64,
}

impl DailyNiem {
    /// Tính chiều cao cột bar chart (5–100%) dựa trên max_count.
    /// Trả về i64 để Askama render dễ dàng (tránh method chain phức tạp trong template).
    /// v0.9.9: nhận `&i64` vì Askama truyền field bằng reference.
    pub fn height_pct(&self, max_count: &i64) -> i64 {
        if *max_count <= 0 {
            return 5;
        }
        let pct = (self.niem_count * 100) / *max_count;
        pct.clamp(5, 100)
    }
}

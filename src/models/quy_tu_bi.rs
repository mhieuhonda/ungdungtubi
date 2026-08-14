//! Models cho Quỹ Từ Bi — Giai đoạn 15 (v0.9.11).
//!
//! Theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục VI:
//!   * Quỹ Từ Bi là quỹ chung của toàn bộ cộng đồng
//!   * Nguồn: đóng góp thành viên, mạnh thường quân, lợi nhuận dự án
//!   * Nguyên tắc: Công khai · Minh bạch · Cùng quản lý · Cùng phát triển

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Loại quỹ / loại đóng góp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DonationType {
    /// Quỹ chung — không phân dòng
    General,
    /// Quỹ Sách Từ Bi — in ấn kinh sách
    Sach,
    /// Quỹ Tu Từ Bi — hỗ trợ tu học
    Tu,
    /// Quỹ Quà Từ Bi — tặng quà cho thành viên
    Qua,
    /// Quỹ Thiện Nguyện — từ thiện xã hội
    ThienNguyen,
}

impl DonationType {
    /// Tên hiển thị tiếng Việt.
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "Quỹ Chung",
            Self::Sach => "Quỹ Sách Từ Bi",
            Self::Tu => "Quỹ Tu Từ Bi",
            Self::Qua => "Quỹ Quà Từ Bi",
            Self::ThienNguyen => "Quỹ Thiện Nguyện",
        }
    }

    /// Emoji đại diện.
    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "🪷",
            Self::Sach => "📚",
            Self::Tu => "🕉️",
            Self::Qua => "🎁",
            Self::ThienNguyen => "🤝",
        }
    }

    /// Màu sắc (hex) cho badge.
    pub fn color(self) -> &'static str {
        match self {
            Self::General => "#2E7D32", // tubi-800
            Self::Sach => "#1565C0",    // blue-800
            Self::Tu => "#6A1B9A",      // purple-800
            Self::Qua => "#FF6F00",     // amber-900
            Self::ThienNguyen => "#C62828", // red-800
        }
    }

    /// Parse từ chuỗi DB.
    pub fn from_str(s: &str) -> Self {
        match s {
            "sach" => Self::Sach,
            "tu" => Self::Tu,
            "qua" => Self::Qua,
            "thien_nguyen" => Self::ThienNguyen,
            _ => Self::General,
        }
    }

    /// Chuỗi cho DB.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Sach => "sach",
            Self::Tu => "tu",
            Self::Qua => "qua",
            Self::ThienNguyen => "thien_nguyen",
        }
    }
}

/// Một đóng góp vào Quỹ Từ Bi.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FundDonation {
    pub id: i64,
    pub user_id: Uuid,
    pub amount_k: i64,
    pub donation_type: String,
    pub message: Option<String>,
    pub is_anonymous: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Đóng góp join với users để hiển thị trên bảng.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FundDonationWithUser {
    pub id: i64,
    pub user_id: Uuid,
    pub amount_k: i64,
    pub donation_type: String,
    pub message: Option<String>,
    pub is_anonymous: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    /// display_name từ users — NULL nếu ẩn danh thì hiển thị "Đạo hữu ẩn danh"
    pub display_name: Option<String>,
    /// avatar_url từ users
    pub avatar_url: Option<String>,
}

impl FundDonationWithUser {
    /// Tên hiển thị — "Đạo hữu ẩn danh" nếu is_anonymous=true.
    pub fn display_label(&self) -> String {
        if self.is_anonymous {
            "Đạo hữu ẩn danh".to_string()
        } else {
            self.display_name.clone().unwrap_or_else(|| "Đạo hữu".to_string())
        }
    }

    /// Loại đóng góp (enum).
    pub fn donation_type_enum(&self) -> DonationType {
        DonationType::from_str(&self.donation_type)
    }

    /// Thời gian tương đối ("X phút trước", "X giờ trước", v.v.).
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let dur = now.signed_duration_since(self.created_at);
        let mins = dur.num_minutes();
        if mins < 1 {
            "vừa xong".to_string()
        } else if mins < 60 {
            format!("{mins} phút trước")
        } else if mins < 60 * 24 {
            format!("{} giờ trước", mins / 60)
        } else if mins < 60 * 24 * 7 {
            format!("{} ngày trước", mins / (60 * 24))
        } else {
            self.created_at.format("%d/%m/%Y").to_string()
        }
    }
}

/// Form nhập đóng góp.
#[derive(Debug, Deserialize)]
pub struct DonationForm {
    pub amount_k: i64,
    pub donation_type: String,
    pub message: Option<String>,
    /// "on" nếu checkbox được check
    pub is_anonymous: Option<String>,
}

impl DonationForm {
    /// Validate form. Trả về (Ok, error_message).
    pub fn validate(&self) -> Result<(), String> {
        if self.amount_k <= 0 {
            return Err("Số K đóng góp phải lớn hơn 0.".into());
        }
        if self.amount_k > 1_000_000 {
            return Err("Số K đóng góp tối đa 1.000.000 K / lần.".into());
        }
        if !matches!(self.donation_type.as_str(),
            "general" | "sach" | "tu" | "qua" | "thien_nguyen"
        ) {
            return Err("Loại quỹ không hợp lệ.".into());
        }
        if let Some(msg) = &self.message {
            if msg.chars().count() > 500 {
                return Err("Lời nhắn tối đa 500 ký tự.".into());
            }
        }
        Ok(())
    }

    /// is_anonymous = true nếu checkbox được check (= "on").
    pub fn anonymous(&self) -> bool {
        self.is_anonymous.as_deref() == Some("on")
    }
}

/// Tổng quan Quỹ Từ Bi (từ view v_fund_summary).
#[derive(Debug, Clone, Default, FromRow, Serialize)]
pub struct FundSummary {
    pub total_income_k: i64,
    pub total_expense_k: i64,
    pub balance_k: i64,
    pub total_donations_count: i64,
    pub unique_donors: i64,
    pub fund_general: i64,
    pub fund_sach: i64,
    pub fund_tu: i64,
    pub fund_qua: i64,
    pub fund_thien_nguyen: i64,
}

impl FundSummary {
    /// Tổng K trong toàn hệ thống (sum của users.k_balance).
    /// Tính riêng vì không thuộc view v_fund_summary.
    pub fn total_k_in_system_label() -> &'static str {
        "Tổng K trong hệ thống"
    }
}

/// Top nhà hảo tâm.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TopDonor {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub total_k: i64,
    pub donation_count: i64,
}

/// Một khoản chi tiêu từ quỹ.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FundExpense {
    pub id: i64,
    pub amount_k: i64,
    pub expense_type: String,
    pub description: String,
    pub receipt_url: Option<String>,
    pub spent_at: chrono::NaiveDate,
    pub approved_by: Option<Uuid>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

impl FundExpense {
    /// Loại chi tiêu (enum).
    pub fn expense_type_enum(&self) -> DonationType {
        DonationType::from_str(&self.expense_type)
    }
}

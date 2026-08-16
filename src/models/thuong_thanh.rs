//! Models cho Thương Thành — Giai đoạn 39 (v0.9.34).
//!
//! Theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục V:
//!   * 3 cửa hàng: Cửa Hàng Ứng Dụng (app), Cửa Hàng Game (game), PvP
//!   * CRUD vật phẩm — tạo/xem/sửa/xoá
//!   * Giỏ hàng — thêm/xoá/thanh toán
//!   * Giao dịch K — mua/bán/chuyển/refund

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Enums ────────────────────────────────────────────────────────────

/// Loại cửa hàng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopStore {
    /// Cửa Hàng Ứng Dụng — vật phẩm hệ thống (thẻ, phiếu, danh hiệu)
    App,
    /// Cửa Hàng Game — vật phẩm game (thuốc, tinh thạch, bẫy)
    Game,
    /// PvP — người dùng tự đăng bán (20% fee, max 7 ngày)
    Pvp,
}

impl ShopStore {
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "Cửa Hàng Ứng Dụng",
            Self::Game => "Cửa Hàng Game",
            Self::Pvp => "Chợ PvP",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::App => "🛒",
            Self::Game => "🎮",
            Self::Pvp => "⚔️",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::App => "#0F766E",
            Self::Game => "#6A1B9A",
            Self::Pvp => "#C62828",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "app" => Self::App,
            "game" => Self::Game,
            "pvp" => Self::Pvp,
            _ => Self::App,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Game => "game",
            Self::Pvp => "pvp",
        }
    }
}

/// Loại giao dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    Purchase,
    Sale,
    Transfer,
    Refund,
}

impl TxType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Purchase => "Mua",
            Self::Sale => "Bán",
            Self::Transfer => "Chuyển K",
            Self::Refund => "Hoàn lại",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Purchase => "🛒",
            Self::Sale => "💰",
            Self::Transfer => "💸",
            Self::Refund => "↩️",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "sale" => Self::Sale,
            "transfer" => Self::Transfer,
            "refund" => Self::Refund,
            _ => Self::Purchase,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::Sale => "sale",
            Self::Transfer => "transfer",
            Self::Refund => "refund",
        }
    }
}

// ─── DB Models ────────────────────────────────────────────────────────

/// Một vật phẩm trong Thương Thành.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ShopItem {
    pub id: i64,
    pub store: String,
    pub category: String,
    pub name: String,
    pub description: Option<String>,
    pub price_k: i32,
    pub icon: String,
    pub color: String,
    pub seller_id: Option<Uuid>,
    pub stock: Option<i32>,
    pub sold_count: i32,
    pub status: String,
    pub image_url: Option<String>,
    pub effects: Option<serde_json::Value>,
    pub sort_order: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ShopItem {
    /// Store enum.
    pub fn store_enum(&self) -> ShopStore {
        ShopStore::from_str(&self.store)
    }

    /// Hiển thị giá K.
    pub fn price_display(&self) -> String {
        format!("{} K", self.price_k)
    }

    /// Tình trạng kho.
    pub fn stock_label(&self) -> String {
        match self.stock {
            None => "Vô hạn".to_string(),
            Some(0) => "Hết hàng".to_string(),
            Some(n) => format!("Còn {n}"),
        }
    }

    /// Thời gian tương đối.
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

    /// Category hiển thị.
    pub fn category_label(&self) -> String {
        match self.category.as_str() {
            "the_tu_hoc" => "Thẻ Tu Học",
            "the_doi_ten" => "Thẻ Đổi Tên",
            "the_ho_tro" => "Thẻ Hỗ Trợ",
            "the_nhom" => "Thẻ Nhóm",
            "the_bau_chon" => "Thẻ Bầu Chọn",
            "vat_pham" => "Vật Phẩm",
            "cao_cap" => "Cao Cấp",
            "thuoc" => "Thuốc",
            "tinh_thach" => "Tinh Thạch",
            "thuoc_dac_biet" => "Thuốc Đặc Biệt",
            "di_chuyen" => "Di Chuyển",
            "bay" => "Bẫy",
            "da_nang_cap" => "Đá Nâng Cấp",
            _ => &self.category,
        }
        .to_string()
    }
}

/// Vật phẩm kèm tên người bán (PvP).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ShopItemWithSeller {
    pub id: i64,
    pub store: String,
    pub category: String,
    pub name: String,
    pub description: Option<String>,
    pub price_k: i32,
    pub icon: String,
    pub color: String,
    pub seller_id: Option<Uuid>,
    pub stock: Option<i32>,
    pub sold_count: i32,
    pub status: String,
    pub image_url: Option<String>,
    pub effects: Option<serde_json::Value>,
    pub sort_order: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// display_name từ users (người bán PvP)
    pub seller_name: Option<String>,
    /// avatar_url từ users
    pub seller_avatar: Option<String>,
}

impl ShopItemWithSeller {
    pub fn store_enum(&self) -> ShopStore {
        ShopStore::from_str(&self.store)
    }

    pub fn price_display(&self) -> String {
        format!("{} K", self.price_k)
    }

    pub fn category_label(&self) -> String {
        ShopItem {
            category: self.category.clone(),
            ..Default::default()
        }
        .category_label()
    }
}

impl Default for ShopItem {
    fn default() -> Self {
        Self {
            id: 0,
            store: "app".into(),
            category: String::new(),
            name: String::new(),
            description: None,
            price_k: 0,
            icon: "📦".into(),
            color: "#0F766E".into(),
            seller_id: None,
            stock: None,
            sold_count: 0,
            status: "active".into(),
            image_url: None,
            effects: None,
            sort_order: 0,
            expires_at: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Một item trong giỏ hàng.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CartItem {
    pub id: i64,
    pub user_id: Uuid,
    pub item_id: i64,
    pub quantity: i32,
    pub added_at: DateTime<Utc>,
}

/// Cart item kèm thông tin vật phẩm.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CartItemWithItem {
    pub cart_id: i64,
    pub user_id: Uuid,
    pub item_id: i64,
    pub quantity: i32,
    pub added_at: DateTime<Utc>,
    // ShopItem fields
    pub item_name: String,
    pub item_icon: String,
    pub item_color: String,
    pub item_price_k: i32,
    pub item_store: String,
    pub item_description: Option<String>,
}

impl CartItemWithItem {
    /// Tổng K cho item này (price × quantity).
    pub fn total_k(&self) -> i32 {
        self.item_price_k * self.quantity
    }
}

/// Một giao dịch K.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Transaction {
    pub id: i64,
    pub tx_type: String,
    pub buyer_id: Uuid,
    pub seller_id: Option<Uuid>,
    pub item_id: Option<i64>,
    pub quantity: i32,
    pub amount_k: i32,
    pub fee_k: i32,
    pub status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Transaction {
    pub fn tx_type_enum(&self) -> TxType {
        TxType::from_str(&self.tx_type)
    }

    /// Thời gian tương đối.
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

/// Transaction kèm tên buyer/seller.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TransactionWithUsers {
    pub id: i64,
    pub tx_type: String,
    pub buyer_id: Uuid,
    pub seller_id: Option<Uuid>,
    pub item_id: Option<i64>,
    pub quantity: i32,
    pub amount_k: i32,
    pub fee_k: i32,
    pub status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub buyer_name: String,
    pub seller_name: Option<String>,
    pub item_name: Option<String>,
    pub item_icon: Option<String>,
}

// ─── Form Structs ─────────────────────────────────────────────────────

/// Form tạo vật phẩm (PvP listing).
#[derive(Debug, Deserialize)]
pub struct ItemCreateForm {
    pub name: String,
    pub description: Option<String>,
    pub price_k: i32,
    pub category: String,
    pub icon: Option<String>,
    pub stock: Option<i32>,
}

impl ItemCreateForm {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Tên vật phẩm không được để trống.".into());
        }
        if self.name.chars().count() > 100 {
            return Err("Tên vật phẩm tối đa 100 ký tự.".into());
        }
        if self.price_k <= 0 {
            return Err("Giá phải lớn hơn 0 K.".into());
        }
        if self.price_k > 1_000_000 {
            return Err("Giá tối đa 1.000.000 K.".into());
        }
        if let Some(desc) = &self.description {
            if desc.chars().count() > 500 {
                return Err("Mô tả tối đa 500 ký tự.".into());
            }
        }
        Ok(())
    }
}

/// Form thêm vào giỏ hàng.
#[derive(Debug, Deserialize)]
pub struct CartAddForm {
    pub item_id: i64,
    pub quantity: Option<i32>,
}

/// Form thanh toán giỏ hàng.
#[derive(Debug, Deserialize)]
pub struct CheckoutForm {
    /// Có thể thêm coupon sau này
    pub note: Option<String>,
}

/// Thống kê Thương Thành.
#[derive(Debug, Clone, Default, FromRow, Serialize)]
pub struct ThuongThanhStats {
    pub total_items: i64,
    pub app_items: i64,
    pub game_items: i64,
    pub pvp_items: i64,
    pub total_transactions: i64,
    pub total_k_volume: i64,
    pub total_fees: i64,
    pub active_pvp_listings: i64,
}

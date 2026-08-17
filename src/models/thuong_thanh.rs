//! Models cho Thương Thành — Giai đoạn 40 (v0.9.35).
//! v0.9.40 — Giai đoạn 44: Chợ Đạo Hữu (rename PvP → Đạo Hữu, thêm payment_method bank + category_id).
//!
//! Theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục V:
//!   * 2 cửa hàng: Cửa Hàng Ứng Dụng (app), Chợ Đạo Hữu (dao_huu — trước là pvp)
//!   * CRUD vật phẩm — tạo/xem/sửa/xoá
//!   * Giỏ hàng — thêm/xoá/thanh toán
//!   * Giao dịch K — mua/bán/chuyển/refund
//!   * v0.9.40: user có thể chọn nhận K hoặc chuyển khoản ngân hàng khi đăng bán
//!   * v0.9.40: user có thể chọn danh mục có sẵn hoặc tạo mới khi đăng bán

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
    /// Chợ Đạo Hữu — người dùng tự đăng bán (v0.9.40 rename từ PvP).
    /// Store value trong DB vẫn có thể là 'pvp' (data cũ) hoặc 'dao_huu' (mới).
    DaoHuu,
}

impl ShopStore {
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "Cửa Hàng Ứng Dụng",
            Self::DaoHuu => "Chợ Đạo Hữu",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::App => "🛒",
            Self::DaoHuu => "🤝",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::App => "#0F766E",
            Self::DaoHuu => "#C62828",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "app" => Self::App,
            // v0.9.40: 'pvp' (cũ) và 'dao_huu' (mới) đều map về DaoHuu — cùng 1 marketplace.
            "pvp" | "dao_huu" => Self::DaoHuu,
            _ => Self::App,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::DaoHuu => "dao_huu",
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

/// Một danh mục Thương Thành (v0.9.40 — Giai đoạn 44).
/// Admin tạo (is_system = true) hoặc user tự tạo khi đăng bán.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ShopCategory {
    pub id: i64,
    pub slug: String,
    pub name_vi: String,
    pub description: Option<String>,
    pub icon: String,
    pub color: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub is_system: bool,
    pub is_approved: bool,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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
    /// v0.9.40 — link tới shop_categories(id). NULL cho item cũ dùng `category` TEXT.
    #[sqlx(default)]
    pub category_id: Option<i64>,
    /// v0.9.40 — 'k' (nhận K) hoặc 'bank' (nhận chuyển khoản ngân hàng).
    #[sqlx(default)]
    pub payment_method: String,
    /// v0.9.40 — giá VNĐ khi payment_method = 'bank'.
    #[sqlx(default)]
    pub price_vnd: Option<i64>,
    /// v0.9.40 — JSONB {bank_name, account_number, account_holder, qr_image_url}.
    #[sqlx(default)]
    pub bank_info: Option<serde_json::Value>,
    /// v0.9.40 — admin có thể set nổi bật.
    #[sqlx(default)]
    pub is_featured: bool,
    /// v0.9.40 — 'pending' | 'approved' | 'rejected' | 'removed'.
    #[sqlx(default)]
    pub moderation_status: String,
}

impl ShopItem {
    /// Store enum.
    pub fn store_enum(&self) -> ShopStore {
        ShopStore::from_str(&self.store)
    }

    /// Hiển thị giá K (nếu payment_method = 'k') hoặc giá VNĐ (nếu 'bank').
    /// v0.9.40: hỗ trợ 2 loại payment.
    pub fn price_display(&self) -> String {
        if self.payment_method == "bank" {
            if let Some(vnd) = self.price_vnd {
                format_vnd(vnd)
            } else {
                "Liên hệ".to_string()
            }
        } else {
            format!("{} K", self.price_k)
        }
    }

    /// Label cho payment method.
    pub fn payment_label(&self) -> &'static str {
        if self.payment_method == "bank" { "Chuyển khoản" } else { "Tiền K" }
    }

    /// Trích bank_info thành struct (None nếu không có hoặc parse fail).
    pub fn bank_info_struct(&self) -> Option<BankInfo> {
        self.bank_info.as_ref().and_then(|v| serde_json::from_value::<BankInfo>(v.clone()).ok())
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

/// Vật phẩm kèm tên người bán (Chợ Đạo Hữu).
/// v0.9.40: rename từ PvP → Đạo Hữu, thêm các trường payment_method, bank_info.
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
    /// display_name từ users (người bán)
    pub seller_name: Option<String>,
    /// avatar_url từ users
    pub seller_avatar: Option<String>,
    /// v0.9.40 — link tới shop_categories(id).
    #[sqlx(default)]
    pub category_id: Option<i64>,
    /// v0.9.40 — 'k' hoặc 'bank'.
    #[sqlx(default)]
    pub payment_method: String,
    /// v0.9.40 — giá VNĐ khi payment_method = 'bank'.
    #[sqlx(default)]
    pub price_vnd: Option<i64>,
    /// v0.9.40 — JSONB {bank_name, account_number, account_holder, qr_image_url}.
    #[sqlx(default)]
    pub bank_info: Option<serde_json::Value>,
    /// v0.9.40 — nổi bật do admin set.
    #[sqlx(default)]
    pub is_featured: bool,
    /// v0.9.40 — 'pending' | 'approved' | 'rejected' | 'removed'.
    #[sqlx(default)]
    pub moderation_status: String,
    /// v0.9.40 — tên danh mục (JOIN shop_categories).
    #[sqlx(default)]
    pub category_name: Option<String>,
    /// v0.9.40 — slug danh mục (JOIN shop_categories).
    #[sqlx(default)]
    pub category_slug: Option<String>,
    /// v0.9.40 — icon danh mục (JOIN shop_categories).
    #[sqlx(default)]
    pub category_icon: Option<String>,
}

impl ShopItemWithSeller {
    pub fn store_enum(&self) -> ShopStore {
        ShopStore::from_str(&self.store)
    }

    /// Hiển thị giá (K hoặc VNĐ tuỳ payment_method).
    pub fn price_display(&self) -> String {
        if self.payment_method == "bank" {
            if let Some(vnd) = self.price_vnd {
                format_vnd(vnd)
            } else {
                "Liên hệ".to_string()
            }
        } else {
            format!("{} K", self.price_k)
        }
    }

    /// Label cho payment method.
    pub fn payment_label(&self) -> &'static str {
        if self.payment_method == "bank" { "Chuyển khoản" } else { "Tiền K" }
    }

    /// Trích bank_info thành struct (None nếu không có hoặc parse fail).
    pub fn bank_info_struct(&self) -> Option<BankInfo> {
        self.bank_info.as_ref().and_then(|v| serde_json::from_value::<BankInfo>(v.clone()).ok())
    }

    pub fn stock_label(&self) -> String {
        match self.stock {
            None => "Vô hạn".to_string(),
            Some(0) => "Hết hàng".to_string(),
            Some(n) => format!("Còn {n}"),
        }
    }

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

    pub fn category_label(&self) -> String {
        // v0.9.40: ưu tiên category_name (từ JOIN shop_categories), fallback về TEXT map.
        if let Some(name) = &self.category_name {
            return name.clone();
        }
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
            category_id: None,
            payment_method: "k".into(),
            price_vnd: None,
            bank_info: None,
            is_featured: false,
            moderation_status: "approved".into(),
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

impl TransactionWithUsers {
    pub fn tx_type_enum(&self) -> TxType {
        TxType::from_str(&self.tx_type)
    }

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

// ─── Form Structs ─────────────────────────────────────────────────────

/// Thông tin ngân hàng cho payment_method = 'bank' (v0.9.40 — Giai đoạn 44).
/// Lưu vào shop_items.bank_info dưới dạng JSONB.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BankInfo {
    /// Tên ngân hàng (VD: "Vietcombank", "Techcombank", "MB Bank").
    pub bank_name: Option<String>,
    /// Số tài khoản.
    pub account_number: Option<String>,
    /// Tên chủ tài khoản.
    pub account_holder: Option<String>,
    /// Chi nhánh (optional).
    pub branch: Option<String>,
    /// URL ảnh QR VietQR hoặc QR tự tạo (optional).
    pub qr_image_url: Option<String>,
}

impl BankInfo {
    /// Build JSON value for sqlx bind.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "bank_name": self.bank_name,
            "account_number": self.account_number,
            "account_holder": self.account_holder,
            "branch": self.branch,
            "qr_image_url": self.qr_image_url,
        })
    }

    /// Validate: bank_name + account_number + account_holder là bắt buộc.
    pub fn validate(&self) -> Result<(), String> {
        if self.bank_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err("Tên ngân hàng không được để trống.".into());
        }
        if self.account_number.as_deref().unwrap_or("").trim().is_empty() {
            return Err("Số tài khoản không được để trống.".into());
        }
        if self.account_holder.as_deref().unwrap_or("").trim().is_empty() {
            return Err("Tên chủ tài khoản không được để trống.".into());
        }
        if self.account_number.as_ref().unwrap().chars().count() > 30 {
            return Err("Số tài khoản tối đa 30 ký tự.".into());
        }
        if self.account_holder.as_ref().unwrap().chars().count() > 100 {
            return Err("Tên chủ tài khoản tối đa 100 ký tự.".into());
        }
        Ok(())
    }
}

/// Format số VNĐ: 1500000 → "1.500.000 ₫".
pub fn format_vnd(amount: i64) -> String {
    let s = amount.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let n = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push('.');
        }
        out.push(*b as char);
    }
    out.push_str(" ₫");
    out
}

/// Form tạo vật phẩm (Chợ Đạo Hữu — v0.9.40).
/// User có thể chọn category có sẵn (category_id) hoặc tạo mới (new_category_name).
/// User có thể chọn nhận K (payment_method = 'k') hoặc ngân hàng ('bank').
#[derive(Debug, Deserialize)]
pub struct ItemCreateForm {
    pub name: String,
    pub description: Option<String>,
    /// Giá K (bắt buộc nếu payment_method = 'k').
    #[serde(default)]
    pub price_k: i32,
    /// Danh mục có sẵn (chọn từ dropdown). Có thể = 0 nếu user chọn tạo mới.
    #[serde(default)]
    pub category_id: Option<i64>,
    /// Slug category TEXT (back-compat với code cũ). Sẽ được dùng nếu category_id = None.
    #[serde(default)]
    pub category: Option<String>,
    /// Tạo danh mục mới: tên hiển thị.
    #[serde(default)]
    pub new_category_name: Option<String>,
    /// Tạo danh mục mới: icon emoji.
    #[serde(default)]
    pub new_category_icon: Option<String>,
    pub icon: Option<String>,
    pub stock: Option<i32>,
    /// 'k' hoặc 'bank'. Mặc định 'k'.
    #[serde(default = "default_payment_method")]
    pub payment_method: String,
    /// Giá VNĐ (bắt buộc nếu payment_method = 'bank').
    #[serde(default)]
    pub price_vnd: Option<i64>,
    /// Bank info (bắt buộc nếu payment_method = 'bank').
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub account_holder: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub qr_image_url: Option<String>,
    /// Thông tin liên hệ người mua (khi bank transfer — VD: Zalo, SĐT).
    #[serde(default)]
    pub buyer_contact: Option<String>,
}

fn default_payment_method() -> String {
    "k".to_string()
}

impl ItemCreateForm {
    /// Validate form. Trả về (category_id, payment_method, bank_info, price_vnd).
    pub fn validate(&self) -> Result<ValidatedItem, String> {
        if self.name.trim().is_empty() {
            return Err("Tên vật phẩm không được để trống.".into());
        }
        if self.name.chars().count() > 100 {
            return Err("Tên vật phẩm tối đa 100 ký tự.".into());
        }
        if let Some(desc) = &self.description {
            if desc.chars().count() > 500 {
                return Err("Mô tả tối đa 500 ký tự.".into());
            }
        }

        // Validate payment method
        let payment_method = if self.payment_method == "bank" { "bank" } else { "k" };

        let mut price_vnd: Option<i64> = None;
        let mut bank_info: Option<BankInfo> = None;

        if payment_method == "bank" {
            // Bank: yêu cầu bank_name, account_number, account_holder, price_vnd
            let bi = BankInfo {
                bank_name: self.bank_name.clone(),
                account_number: self.account_number.clone(),
                account_holder: self.account_holder.clone(),
                branch: self.branch.clone(),
                qr_image_url: self.qr_image_url.clone(),
            };
            bi.validate()?;
            let vnd = self.price_vnd.unwrap_or(0);
            if vnd <= 0 {
                return Err("Giá VNĐ phải lớn hơn 0.".into());
            }
            if vnd > 1_000_000_000 {
                return Err("Giá VNĐ tối đa 1.000.000.000 ₫.".into());
            }
            price_vnd = Some(vnd);
            bank_info = Some(bi);
        } else {
            // K: yêu cầu price_k > 0
            if self.price_k <= 0 {
                return Err("Giá K phải lớn hơn 0.".into());
            }
            if self.price_k > 1_000_000 {
                return Err("Giá K tối đa 1.000.000.".into());
            }
        }

        // Validate category: phải có category_id HOẶC new_category_name
        let category_id = if let Some(name) = &self.new_category_name {
            if name.trim().is_empty() {
                // Fallback: dùng category_id nếu user không nhập tên mới
                self.category_id.filter(|&id| id > 0)
            } else {
                // Tạo mới — sẽ được INSERT trong handler
                None
            }
        } else {
            self.category_id.filter(|&id| id > 0)
        };

        // Fallback: nếu không có category_id và không tạo mới, dùng 'khac'
        let category_text = self.category.clone().unwrap_or_else(|| "khac".to_string());

        Ok(ValidatedItem {
            category_id,
            category_text,
            payment_method: payment_method.to_string(),
            price_vnd,
            bank_info,
        })
    }
}

/// Result trả về từ ItemCreateForm::validate().
pub struct ValidatedItem {
    pub category_id: Option<i64>,
    pub category_text: String,
    pub payment_method: String,
    pub price_vnd: Option<i64>,
    pub bank_info: Option<BankInfo>,
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
    pub pvp_items: i64,
    pub total_transactions: i64,
    pub total_k_volume: i64,
    pub total_fees: i64,
    pub active_pvp_listings: i64,
}

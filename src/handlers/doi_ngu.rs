//! Handlers cho trang Đội Ngũ Quản Lí — Giai đoạn 27 (v0.9.22).
//!
//! Trang này công khai — mọi người (kể cả chưa đăng nhập) đều có thể xem.
//! Hiển thị thông tin chi tiết về đội ngũ quản trị Ứng Dụng Từ Bi.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use askama::Template;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

// ─── Data ─────────────────────────────────────────────────────────────

/// Một thành viên đội ngũ quản lí.
#[allow(dead_code)]
pub struct TeamMember {
    /// Họ tên đầy đủ.
    pub full_name: &'static str,
    /// Pháp danh (nếu có).
    pub phap_danh: &'static str,
    /// Năm sinh.
    pub birth_year: u16,
    /// Quê quán.
    pub hometown: &'static str,
    /// Tôn giáo.
    pub religion: &'static str,
    /// Chức vụ.
    pub role_title: &'static str,
    /// Mô tả chi tiết về chức vụ.
    pub role_detail: &'static str,
    /// Link Facebook.
    pub facebook_url: &'static str,
    /// Icon đại diện.
    pub icon: &'static str,
    /// CSS accent color cho card.
    pub accent_color: &'static str,
}

/// Danh sách đội ngũ quản lí — hardcoded vì đây là thông tin cố định.
#[allow(dead_code)]
pub const TEAM_MEMBERS: &[TeamMember] = &[
    TeamMember {
        full_name: "Đỗ Minh Đức",
        phap_danh: "Không có",
        birth_year: 1991,
        hometown: "Thanh Ba, Phú Thọ",
        religion: "Không",
        role_title: "Admin Quản Lí",
        role_detail: "Quản lí chuyên mục hỏi đáp",
        facebook_url: "https://www.facebook.com/chieuhavang91?mibextid=ZbWKwL",
        icon: "👑",
        accent_color: "#f59e0b", // amber-500 — Premium Gold
    },
    TeamMember {
        full_name: "Võ Đăng Trọng Nghĩa",
        phap_danh: "Thích Giác Ti",
        birth_year: 1991,
        hometown: "Duy Vinh, Quảng Nam (nay Nam Phước, Đà Nẵng)",
        religion: "Phật giáo",
        // v0.9.29: Đổi từ "Admin Phát Triển" (role không tồn tại trong code)
        // sang "Admin Cộng Đồng" — vai trò phù hợp với phụ trách:
        // định hướng nội dung, cộng đồng, truyền thông, sự kiện.
        role_title: "Admin Cộng Đồng",
        role_detail: "Định hướng nội dung, cộng đồng, truyền thông và sự kiện",
        facebook_url: "https://www.facebook.com/likedliti?mibextid=ZbWKwL",
        icon: "🛡️",
        accent_color: "#1565C0", // blue-800 — Shield Blue
    },
    TeamMember {
        full_name: "Đỗ Văn Cường",
        phap_danh: "Không có",
        birth_year: 0, // Không công bố
        hometown: "Không công bố",
        religion: "Không",
        role_title: "Admin Kỹ Thuật",
        role_detail: "Hiện tại đã lui về hỗ trợ",
        facebook_url: "https://www.facebook.com/dvcuong.hust?mibextid=ZbWKwL",
        icon: "⚙️",
        accent_color: "#10b981", // emerald-500 — Coder
    },
    TeamMember {
        full_name: "Nguyễn Đình Minh Hiếu",
        phap_danh: "Không có",
        birth_year: 0, // Không công bố
        hometown: "Không công bố",
        religion: "Không",
        role_title: "Admin Kỹ Thuật",
        role_detail: "Hiện tại đang làm chính",
        facebook_url: "https://www.facebook.com/profile.php?id=61591104916229&mibextid=ZbWKwL",
        icon: "💻",
        accent_color: "#06b6d4", // cyan-500
    },
];

// ─── Template ─────────────────────────────────────────────────────────

/// Template cho trang /doi-ngu-quan-li.
#[derive(Template)]
#[template(path = "doi-ngu-quan-li/index.html")]
pub struct DoiNguTemplate {
    pub user: Option<User>,
    pub active_page: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────

/// GET /doi-ngu-quan-li — Trang Đội Ngũ Quản Lí.
/// Công khai — không yêu cầu đăng nhập.
pub async fn doi_ngu_quan_li(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let html = DoiNguTemplate {
        user,
        active_page: "doi_ngu".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (doi-ngu-quan-li): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

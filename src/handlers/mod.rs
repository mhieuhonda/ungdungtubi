pub mod auth;

use actix_web::{HttpRequest, Responder};
use askama::Template;
use sqlx::PgPool;

use crate::models::user::User;

/// Helper: Extract authenticated user from session cookie.
async fn get_user_from_session(pool: &PgPool, req: &HttpRequest) -> Option<User> {
    let cookie = req.cookie("session_id")?;
    let session_id = cookie.value();

    sqlx::query_as::<_, User>(
        "SELECT u.id, u.email, u.display_name, u.password_hash, u.rank, u.a_balance, u.k_balance, u.is_active, u.created_at, u.updated_at, u.google_sub, u.avatar_url, u.email_verified
         FROM users u
         JOIN sessions s ON s.user_id = u.id
         WHERE s.id = $1 AND s.expires_at > NOW() AND u.is_active = true",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

// --- Template Structs ---

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub user: Option<User>,
    pub active_page: String,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub error: Option<String>,
}

// --- Page Handlers ---

pub async fn home(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    let html = HomeTemplate {
        user,
        active_page: "home".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (home): {e}");
        format!(
            "<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>"
        )
    });
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn login_page(
    req: HttpRequest,
    pool: actix_web::web::Data<PgPool>,
) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    let html = LoginTemplate {
        user,
        active_page: "login".into(),
        error: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (login): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// --- Placeholder Section Handlers (Vietnamese) ---
//
// Các trang dưới đây dùng chung một helper `placeholder_page` để giữ
// giao diện nhất quán (header/footer từ layout, không phải HTML rời).

pub async fn khong_gian(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "home", "Không Gian", "🌍", "Không gian cá nhân, cộng tu, niệm Phật", "Giai đoạn 4")
}

pub async fn cong_dong(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "community", "Cộng Đồng", "👥", "Diễn đàn kết hợp mạng xã hội — Lướt nhóm, chủ đề, live chat", "Giai đoạn 12")
}

pub async fn ban_be(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "friends", "Bạn Bè", "👤", "Kết nối, nhắn tin, gửi thư — Kết bạn đạo hữu", "Giai đoạn 15")
}

pub async fn kinh_sach(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "books", "Kinh Sách", "📚", "Thư viện kinh sách Phật giáo, Đạo giáo và triết học", "Giai đoạn 17")
}

pub async fn quy_tu_bi(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "", "Quỹ Từ Bi", "🪷", "Quỹ chung cộng đồng — quyên góp, phát quà, hỗ trợ mạnh thường quân", "Giai đoạn 10")
}

pub async fn thuong_thanh(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "", "Thương Thành", "🏪", "Mua bán, trao đổi vật phẩm và dịch vụ trong cộng đồng", "Giai đoạn 10")
}

pub async fn bang_xep_hang(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "", "Bảng Xếp Hạng", "🏆", "Thành tích niệm Phật, tài Phú K, niệm lực A, phiếu Từ Bi", "Giai đoạn 19")
}

pub async fn ca_nhan(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    placeholder_page(user, "", "Hồ Sơ Cá Nhân", "👤", "Chỉnh sửa hồ sơ, pháp danh, giới tính và cấp bậc", "Giai đoạn 3")
}

/// API: Heartbeat — keeps session alive (called every 5 min by client JS).
pub async fn heartbeat() -> impl Responder {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok"
    }))
}

// --- Helper ---

/// Render trang "tính năng đang phát triển" dùng layout chính.
fn placeholder_page(
    user: Option<User>,
    active_page: &str,
    title: &str,
    icon: &str,
    desc: &str,
    phase: &str,
) -> impl Responder {
    // Build HTML inline bằng layout placeholder.html
    let body = format!(
        r#"<section class="max-w-4xl mx-auto px-4 py-20 text-center">
    <span class="text-6xl">{icon}</span>
    <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2" style="color:#2E7D32">{title}</h1>
    <p class="text-gray-500 mb-2">{desc}</p>
    <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — {phase}</p>
    <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition" style="background-color:#2E7D32">← Về trang chủ</a>
</section>"#
    );
    // Render layout với user/active_page/body — dùng askama inline không khả thi, nên
    // ta render bằng tay theo layout.html.
    let user_html = if let Some(u) = &user {
        format!(
            r#"<div class="flex items-center space-x-3">
                <span class="text-tubi-100 text-sm">🪷 {name}</span>
                <a href="/ca-nhan" class="text-tubi-200 hover:text-white text-sm transition-colors">Hồ sơ</a>
                <a href="/dang-xuat" class="bg-tubi-600 hover:bg-tubi-500 px-3 py-1.5 rounded-lg text-sm transition-colors">Thoát</a>
            </div>"#,
            name = u.display_name
        )
    } else {
        r#"<a href="/dang-nhap" class="bg-lotus hover:bg-golden text-tubi-900 px-4 py-2 rounded-lg text-sm font-semibold transition-colors">
                🪷 Đăng Nhập Bằng Google
            </a>"#
            .to_string()
    };

    let mobile_user_html = if user.is_some() {
        r#"<a href="/ca-nhan" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">Hồ sơ</a>
           <a href="/dang-xuat" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">Thoát</a>"#
            .to_string()
    } else {
        r#"<a href="/dang-nhap" class="block px-3 py-2 rounded-lg bg-lotus text-tubi-900 mt-1">🪷 Đăng Nhập Bằng Google</a>"#
            .to_string()
    };

    let nav_item = |href: &str, label: &str, emoji: &str, page: &str| -> String {
        let cls = if active_page == page {
            "bg-tubi-900 text-white"
        } else {
            "text-tubi-100 hover:bg-tubi-700"
        };
        format!(
            r#"<a href="{href}" class="nav-tab px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200 {cls}">{emoji} {label}</a>"#
        )
    };

    let bottom_nav_item = |href: &str, label: &str, emoji: &str, page: &str| -> String {
        let cls = if active_page == page {
            "text-tubi-700"
        } else {
            "text-gray-400"
        };
        format!(
            r#"<a href="{href}" class="flex flex-col items-center justify-center space-y-0.5 {cls}">
                <span class="text-xl">{emoji}</span>
                <span class="text-xs">{label}</span>
            </a>"#
        )
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Ứng Dụng Từ Bi - Sân chơi tu đạo, phát triển thư viện kinh sách">
    <title>{title} — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>
        tailwind.config = {{
            theme: {{
                extend: {{
                    colors: {{
                        'tubi': {{
                            50:'#E8F5E9', 100:'#C8E6C9', 200:'#A5D6A7', 300:'#81C784',
                            400:'#66BB6A', 500:'#4CAF50', 600:'#43A047', 700:'#388E3C',
                            800:'#2E7D32', 900:'#1B5E20'
                        }},
                        'lotus':'#FFB300', 'golden':'#FFA000'
                    }}
                }}
            }}
        }}
    </script>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <script defer src="https://unpkg.com/alpinejs@3.14.9/dist/cdn.min.js"></script>
    <link rel="stylesheet" href="/static/css/app.css">
</head>
<body class="bg-gray-50 text-gray-900 min-h-screen flex flex-col" x-data="{{ mobileMenu: false }}">
    <header class="bg-tubi-800 text-white shadow-lg sticky top-0 z-50" style="background-color:#2E7D32">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div class="flex items-center justify-between h-16">
                <a href="/" class="flex items-center space-x-3 group">
                    <div class="w-10 h-10 bg-lotus rounded-full flex items-center justify-center group-hover:scale-110 transition-transform duration-300 shadow-md">
                        <span class="text-tubi-900 text-xl" style="color:#1B5E20">🪷</span>
                    </div>
                    <div>
                        <h1 class="text-lg font-bold tracking-wide">ỨNG DỤNG TỪ BI</h1>
                        <p class="text-xs text-tubi-200 -mt-1">Giác Ngộ · Giải Thoát · Từ Bi</p>
                    </div>
                </a>
                <nav class="hidden md:flex items-center space-x-1">
                    {nav_kg}
                    {nav_cd}
                    {nav_bb}
                    {nav_ks}
                </nav>
                <div class="hidden md:flex items-center space-x-3">
                    {user_html}
                </div>
                <button @click="mobileMenu = !mobileMenu" class="md:hidden text-tubi-100 hover:text-white p-2">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                    </svg>
                </button>
            </div>
        </div>
        <div x-show="mobileMenu" x-transition class="md:hidden bg-tubi-900 border-t border-tubi-700" style="background-color:#1B5E20">
            <div class="px-4 py-3 space-y-2">
                <a href="/" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">🌍 Không Gian</a>
                <a href="/cong-dong" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">👥 Cộng Đồng</a>
                <a href="/ban-be" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">👤 Bạn Bè</a>
                <a href="/kinh-sach" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">📚 Kinh Sách</a>
                <div class="pt-2 border-t border-tubi-700">
                    {mobile_user_html}
                </div>
            </div>
        </div>
    </header>
    <main class="flex-1">
        {body}
    </main>
    <footer class="hidden md:block bg-tubi-900 text-tubi-200 mt-auto" style="background-color:#1B5E20">
        <div class="max-w-7xl mx-auto px-4 py-8">
            <div class="grid grid-cols-1 md:grid-cols-4 gap-8">
                <div>
                    <div class="flex items-center space-x-2 mb-4">
                        <span class="text-2xl">🪷</span>
                        <span class="font-bold text-white">Ứng Dụng Từ Bi</span>
                    </div>
                    <p class="text-sm text-tubi-300">
                        Siêu thoát không siêu thích.<br>
                        Giải thoát không giải thích.<br>
                        Buông bỏ mới có thể trở về.
                    </p>
                </div>
                <div>
                    <h3 class="font-semibold text-white mb-3">Chuyên Mục</h3>
                    <ul class="space-y-2 text-sm">
                        <li><a href="/" class="hover:text-white transition-colors">Không Gian</a></li>
                        <li><a href="/cong-dong" class="hover:text-white transition-colors">Cộng Đồng</a></li>
                        <li><a href="/ban-be" class="hover:text-white transition-colors">Bạn Bè</a></li>
                        <li><a href="/kinh-sach" class="hover:text-white transition-colors">Kinh Sách</a></li>
                    </ul>
                </div>
                <div>
                    <h3 class="font-semibold text-white mb-3">Hệ Thống</h3>
                    <ul class="space-y-2 text-sm">
                        <li><a href="/quy-tu-bi" class="hover:text-white transition-colors">Quỹ Từ Bi</a></li>
                        <li><a href="/thuong-thanh" class="hover:text-white transition-colors">Thương Thành</a></li>
                        <li><a href="/bang-xep-hang" class="hover:text-white transition-colors">Bảng Xếp Hạng</a></li>
                    </ul>
                </div>
                <div>
                    <h3 class="font-semibold text-white mb-3">Liên Hệ</h3>
                    <p class="text-sm text-tubi-300">tubi.louis.vangioitutien.com</p>
                </div>
            </div>
            <div class="mt-8 pt-4 border-t border-tubi-700 text-center text-sm text-tubi-400">
                <p>🪷 Ứng Dụng Từ Bi v0.3 · Nguyện công đức vô lượng · Nam Mô A Di Đà Phật</p>
            </div>
        </div>
    </footer>
    <nav class="md:hidden bg-white border-t border-gray-200 fixed bottom-0 left-0 right-0 z-50">
        <div class="flex items-center justify-around h-16">
            {bottom_kg}
            {bottom_cd}
            <a href="/" class="flex flex-col items-center justify-center -mt-4">
                <div class="w-12 h-12 bg-tubi-800 rounded-full flex items-center justify-center shadow-lg border-2 border-white" style="background-color:#2E7D32">
                    <span class="text-2xl">🪷</span>
                </div>
            </a>
            {bottom_bb}
            {bottom_ks}
        </div>
    </nav>
    <script src="/static/js/app.js"></script>
</body>
</html>"#,
        title = title,
        user_html = user_html,
        mobile_user_html = mobile_user_html,
        body = body,
        nav_kg = nav_item("/", "Không Gian", "🌍", "home"),
        nav_cd = nav_item("/cong-dong", "Cộng Đồng", "👥", "community"),
        nav_bb = nav_item("/ban-be", "Bạn Bè", "👤", "friends"),
        nav_ks = nav_item("/kinh-sach", "Kinh Sách", "📚", "books"),
        bottom_kg = bottom_nav_item("/", "Không Gian", "🌍", "home"),
        bottom_cd = bottom_nav_item("/cong-dong", "Cộng Đồng", "👥", "community"),
        bottom_bb = bottom_nav_item("/ban-be", "Bạn Bè", "👤", "friends"),
        bottom_ks = bottom_nav_item("/kinh-sach", "Kinh Sách", "📚", "books"),
    );
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

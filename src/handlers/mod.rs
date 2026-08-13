pub mod auth;

use actix_web::{HttpRequest, Responder};
use askama::Template;
use sqlx::PgPool;

use crate::models::user::User;

/// Helper: Extract authenticated user from session cookie
async fn get_user_from_session(pool: &PgPool, req: &HttpRequest) -> Option<User> {
    let cookie = req.cookie("session_id")?;
    let session_id = cookie.value();

    sqlx::query_as::<_, User>(
        "SELECT u.id, u.email, u.display_name, u.password_hash, u.rank, u.a_balance, u.k_balance, u.is_active, u.created_at, u.updated_at
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

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub user: Option<User>,
    pub active_page: String,
}

// --- Page Handlers ---

pub async fn home(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    let html = HomeTemplate {
        user,
        active_page: "home".into(),
    }
    .render()
    .unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn login_page(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    let html = LoginTemplate {
        user,
        active_page: "login".into(),
        error: None,
    }
    .render()
    .unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn register_page(req: HttpRequest, pool: actix_web::web::Data<PgPool>) -> impl Responder {
    let user = get_user_from_session(pool.get_ref(), &req).await;
    let html = RegisterTemplate {
        user,
        active_page: "register".into(),
    }
    .render()
    .unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

// --- Placeholder Section Handlers (Vietnamese) ---

pub async fn khong_gian() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Không Gian — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl5">🌍</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Không Gian</h1>
        <p class="text-gray-500 mb-2">Không gian cá nhân, cộng tu, niệm Phật</p>
        <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — Giai đoạn 4</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Về trang chủ</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn cong_dong() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Cộng Đồng — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl5">👥</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Cộng Đồng</h1>
        <p class="text-gray-500 mb-2">Diễn đàn kết hợp mạng xã hội — Lướt nhóm, chủ đề, live chat</p>
        <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — Giai đoạn 12</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Về trang chủ</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn ban_be() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Bạn Bè — �" "Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl5">👤</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Bạn Bè</h1>
        <p class="text-gray-500 mb-2">Kết nối, nhắn tin, gửi thư — Kết bạn đạo hữu</p>
        <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — Giai đoạn 15</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Về trang chủ</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn kinh_sach() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Kinh Sách — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl5">📚</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Kinh Sách</h1>
        <p class="text-gray-500 mb-2">Thư viện kinh sách Phật giáo, Đạo giáo và triết học</p>
        <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — Giai đoạn 17</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Về trang chủ</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn quy_tu_bi() -> impl Responder {
    placeholder_page("Quỹ Từ Bi", "🪷", "Quỹ chung cộng đồng — quyên góp, phát quà, hỗ trợ mạnh thường quân", "Giai đoạn 10")
}

pub async fn thuong_thanh() -> impl Responder {
    placeholder_page("Thương Thành", "🏪", "Mua bán, trao đổi vật phẩm và dịch vụ trong cộng đồng", "Giai đoạn 10")
}

pub async fn bang_xep_hang() -> impl Responder {
    placeholder_page("Bảng Xếp Hạng", "🏆", "Thành tích niệm Phật, tài Phú K, niệm lực A, phiếu Từ Bi", "Giai đoạn 19")
}

pub async fn ca_nhan() -> impl Responder {
    placeholder_page("Hồ Sơ Cá Nhân", "👤", "Chỉnh sửa hồ sơ, pháp danh, giới tính và cấp bậc", "Giai đoạn 3")
}

/// API: Heartbeat — keeps session alive (called every 5 min by client JS)
pub async fn heartbeat() -> impl Responder {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok"
    }))
}

// --- Helper ---

fn placeholder_page(title: &str, icon: &str, desc: &str, phase: &str) -> impl Responder {
    let html = format!(r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>{title} — Ứng Dụng Từ Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={{theme:{{extend:{{colors:{{'tubi':{{800:'#2E7D32',900:'#1B5E20'}},'lotus':'#FFB300'}}}}}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl">{icon}</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">{title}</h1>
        <p class="text-gray-500 mb-2">{desc}</p>
        <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — {phase}</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Về trang chủ</a>
    </div>
    </body></html>"#);
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

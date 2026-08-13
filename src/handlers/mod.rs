pub mod auth;

use actix_web::{HttpRequest, Responder};
use askama::Template;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate;

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate;

pub async fn home(_req: HttpRequest) -> impl Responder {
    let html = HomeTemplate.render().unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn login_page(_req: HttpRequest) -> impl Responder {
    let html = LoginTemplate { error: None }.render().unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn register_page(_req: HttpRequest) -> impl Responder {
    let html = RegisterTemplate.render().unwrap();
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn cong_dong() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Cong Dong — Ung Dung Tu Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl">&#x1F465;</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Cong Dong</h1>
        <p class="text-gray-500 mb-2">Dien dan ket hop mang xa hoi — Luot nhom, chu de, live chat</p>
        <p class="text-sm text-amber-600 mb-8">Tinh nang dang phat trien — Giai doan 12</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Ve trang chu</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn ban_be() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Ban Be — Ung Dung Tu Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl">&#x1F464;</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Ban Be</h1>
        <p class="text-gray-500 mb-2">Ket noi, nhan tin, gui thu — Ket ban dao huu</p>
        <p class="text-sm text-amber-600 mb-8">Tinh nang dang phat trien — Giai doan 15</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Ve trang chu</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

pub async fn kinh_sach() -> impl Responder {
    let html = r#"<!DOCTYPE html><html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Kinh Sach — Ung Dung Tu Bi</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script>tailwind.config={theme:{extend:{colors:{'tubi':{800:'#2E7D32',900:'#1B5E20'},'lotus':'#FFB300'}}}}</script>
    </head>
    <body class="bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto px-4 py-20 text-center">
        <span class="text-6xl">&#x1F4DA;</span>
        <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2">Kinh Sach</h1>
        <p class="text-gray-500 mb-2">Thu vien kinh sach Phat giao, Dao giao va triet hoc</p>
        <p class="text-sm text-amber-600 mb-8">Tính nang dang phat trien — Giai doan 17</p>
        <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition">&larr; Ve trang chu</a>
    </div>
    </body></html>"#;
    actix_web::HttpResponse::Ok().content_type("text/html").body(html)
}

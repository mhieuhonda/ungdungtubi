//! Handlers cho SEO — Giai đoạn 60 (v0.9.45).
//!
//! Bao gồm:
//!   * GET /sitemap.xml      — XML sitemap cho Google/Bing
//!   * GET /robots.txt       — robots.txt cho crawler
//!   * GET /manifest.json    — PWA manifest
//!   * GET /api/seo/structured-data  — JSON-LD structured data
//!
//! Theo tài liệu "ỨNG DỤNG TỪ BI.docx" mục III (Quốc tế hóa):
//!   Trang web phải có SEO tốt để lan tỏa từ bi tới cộng đồng.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;

use crate::AppState;

/// Base URL cho sitemap — đọc từ config (APP_BASE_URL env).
fn base_url(state: &AppState) -> String {
    state.config.app_base_url.trim_end_matches('/').to_string()
}

/// GET /robots.txt — cho phép crawl tất cả + chỉ sitemap.
pub async fn robots_txt(State(state): State<AppState>, _jar: CookieJar) -> Response {
    let base = base_url(&state);
    let body = format!(
        "# Ứng Dụng Từ Bi — robots.txt\n\
         # Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.\n\
         User-agent: *\n\
         Allow: /\n\
         Disallow: /admin/\n\
         Disallow: /api/\n\
         Disallow: /ban-be/tin-nhan/\n\
         Disallow: /ws/\n\
         \n\
         Sitemap: {base}/sitemap.xml\n"
    );
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

/// GET /sitemap.xml — XML sitemap với các route công khai.
pub async fn sitemap_xml(State(state): State<AppState>, _jar: CookieJar) -> Response {
    let base = base_url(&state);
    let now = chrono::Utc::now().format("%Y-%m-%d");

    // Static public routes
    let static_urls = [
        ("/", "daily", "1.0"),
        ("/gioi-thieu", "weekly", "0.8"),
        ("/kinh-sach", "weekly", "0.9"),
        ("/cong-dong", "hourly", "0.9"),
        ("/cong-dong/hoat-dong", "hourly", "0.7"),
        ("/cong-dong/kham-pha", "hourly", "0.8"),
        ("/bang-xep-hang", "daily", "0.6"),
        ("/quy-tu-bi", "weekly", "0.7"),
        ("/thuong-thanh", "daily", "0.7"),
        ("/thuong-thanh/cho-dao-huu", "daily", "0.7"),
        ("/tien-te", "weekly", "0.5"),
        ("/tu-si", "weekly", "0.6"),
        ("/doi-ngu-quan-li", "monthly", "0.4"),
        ("/dang-nhap", "monthly", "0.3"),
    ];

    let mut xml = String::with_capacity(8192);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    for (path, freq, prio) in static_urls.iter() {
        xml.push_str(&format!(
            "  <url>\n    <loc>{base}{path}</loc>\n    \
             <lastmod>{now}</lastmod>\n    \
             <changefreq>{freq}</changefreq>\n    \
             <priority>{prio}</priority>\n  </url>\n"
        ));
    }

    // Dynamic: kinhs sách books
    let books_result: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT slug FROM books WHERE slug IS NOT NULL ORDER BY view_count DESC LIMIT 200"
    )
    .fetch_all(&state.pool)
    .await;

    if let Ok(books) = books_result {
        for (slug,) in books {
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/kinh-sach/{slug}</loc>\n    \
                 <changefreq>weekly</changefreq>\n    <priority>0.7</priority>\n  </url>\n"
            ));
        }
    }

    // Dynamic: nhóm cộng đồng công khai
    let groups_result: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT slug FROM groups WHERE slug IS NOT NULL AND is_active = true ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&state.pool)
    .await;

    if let Ok(groups) = groups_result {
        for (slug,) in groups {
            xml.push_str(&format!(
                "  <url>\n    <loc>{base}/cong-dong/nhom/{slug}</loc>\n    \
                 <changefreq>daily</changefreq>\n    <priority>0.6</priority>\n  </url>\n"
            ));
        }
    }

    xml.push_str("</urlset>\n");

    (
        [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

/// GET /manifest.json — PWA manifest cơ bản.
pub async fn manifest_json(State(state): State<AppState>, _jar: CookieJar) -> Response {
    let base = base_url(&state);
    let body = serde_json::json!({
        "name": "Ứng Dụng Từ Bi",
        "short_name": "Từ Bi",
        "description": "Sân chơi tu đạo, thư viện kinh sách, niệm Phật, cộng tu.",
        "start_url": "/",
        "display": "standalone",
        "background_color": "#FFFFFF",
        "theme_color": "#0f766e",
        "lang": "vi",
        "icons": [
            {
                "src": format!("{}/static/tubi.png", base),
                "sizes": "192x192",
                "type": "image/png"
            },
            {
                "src": format!("{}/static/tubi.png", base),
                "sizes": "512x512",
                "type": "image/png"
            }
        ],
        "categories": ["lifestyle", "education", "social"]
    });

    axum::response::Json(body).into_response()
}

/// GET /api/seo/structured-data — JSON-LD cho trang chủ.
pub async fn structured_data(State(state): State<AppState>, _jar: CookieJar) -> Response {
    let base = base_url(&state);
    let body = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "WebApplication",
        "name": "Ứng Dụng Từ Bi",
        "url": base,
        "description": "Ứng dụng Phật giáo Việt Nam giúp thành viên ứng dụng từ bi vào cuộc sống, tu học và giải trí.",
        "applicationCategory": "LifestyleApplication",
        "operatingSystem": "Web",
        "inLanguage": "vi-VN",
        "offers": {
            "@type": "Offer",
            "price": "0",
            "priceCurrency": "VND"
        },
        "publisher": {
            "@type": "Organization",
            "name": "Từ Bi Team",
            "email": "ungdungtubi@gmail.com"
        },
        "featureList": [
            "Niệm Phật Counter",
            "Tượng Phật — Cầu nguyện, Sám hối, Hồi hướng",
            "Nhật Ký Tu Học",
            "Nhà Nhạc — 5 thư mục nhạc thiền",
            "Cộng Đồng — Nhóm, Chủ Đề, Live Chat",
            "Kinh Sách — Thư viện Phật giáo & Đạo giáo",
            "Thương Thành — Chợ Đạo Hữu",
            "Tiền Tệ — A/K/Bi",
            "Bảng Xếp Hạng",
            "Quỹ Từ Bi"
        ]
    });

    axum::response::Json(body).into_response()
}

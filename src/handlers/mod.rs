pub mod auth;
pub mod community;
pub mod uploads;

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use sqlx::PgPool;

use crate::AppState;
use crate::models::user::{MemberRank, ProfileUpdate, User};

/// Danh sách cột users đầy đủ (đồng bộ với model User).
/// Tránh drift khi SELECT * — luôn liệt kê rõ ràng các cột.
pub const USER_COLUMNS: &str = "u.id, u.email, u.display_name, u.password_hash, u.rank, \
    u.a_balance, u.k_balance, u.is_active, u.created_at, u.updated_at, \
    u.google_sub, u.avatar_url, u.email_verified, \
    u.phap_danh, u.phap_hieu, u.but_danh, u.gender, u.bio, \
    u.avatar_upload_id";

/// Helper: Extract authenticated user from session cookie.
pub async fn get_user_from_session(pool: &PgPool, jar: &CookieJar) -> Option<User> {
    let cookie = jar.get("session_id")?;
    let session_id = cookie.value();

    let sql = format!(
        "SELECT {USER_COLUMNS}
         FROM users u
         JOIN sessions s ON s.user_id = u.id
         WHERE s.id = $1 AND s.expires_at > NOW() AND u.is_active = true"
    );
    sqlx::query_as::<_, User>(&sql)
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
#[template(path = "profile.html")]
pub struct ProfileTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub ranks: Vec<MemberRank>,
    pub error: Option<String>,
    pub success: Option<String>,
}

// --- Page Handlers ---

pub async fn home(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    let html = HomeTemplate {
        user,
        active_page: "home".into(),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (home): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}

pub async fn login_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
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
    Html(html).into_response()
}

/// GET /ca-nhan — Trang hồ sơ cá nhân + form chỉnh sửa.
pub async fn ca_nhan(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    // Lấy danh sách cấp bậc để hiển thị tiến độ.
    let ranks = sqlx::query_as::<_, MemberRank>(
        "SELECT code, name, description, min_k_balance, color, icon, sort_order, created_at
         FROM member_ranks ORDER BY sort_order ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let html = ProfileTemplate {
        user,
        active_page: "profile".into(),
        ranks,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (profile): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /ca-nhan/cap-nhat — Cập nhật hồ sơ cá nhân.
///
/// Chỉ cho phép cập nhật các trường:
/// `display_name`, `phap_danh`, `phap_hieu`, `but_danh`, gender, bio.
/// Không cho phép chỉnh email, rank, số dư A/K, `is_active`.
pub async fn cap_nhat_ho_so(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ProfileUpdate>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate
    let display_name = form.display_name.trim().to_string();
    if display_name.is_empty() || display_name.chars().count() > 100 {
        return render_profile_error(
            &state.pool,
            Some(user),
            "Tên hiển thị không được để trống và tối đa 100 ký tự.",
        )
        .await;
    }

    let gender = form.gender.trim().to_string();
    if !matches!(gender.as_str(), "male" | "female" | "other") {
        return render_profile_error(
            &state.pool,
            Some(user),
            "Giới tính không hợp lệ.",
        )
        .await;
    }

    // Chuẩn hoá các trường tùy chọn (None nếu rỗng).
    let phap_danh = normalize_optional(form.phap_danh.as_ref());
    let phap_hieu = normalize_optional(form.phap_hieu.as_ref());
    let but_danh = normalize_optional(form.but_danh.as_ref());
    let bio = normalize_optional(form.bio.as_ref());

    // Cập nhật DB
    let update_sql = format!(
        "UPDATE users
         SET display_name = $1,
             phap_danh    = $2,
             phap_hieu    = $3,
             but_danh     = $4,
             gender       = $5,
             bio          = $6,
             updated_at   = NOW()
         WHERE id = $7
         RETURNING {USER_COLUMNS}",
        USER_COLUMNS = USER_COLUMNS.replace("u.", "")
    );

    match sqlx::query_as::<_, User>(&update_sql)
        .bind(&display_name)
        .bind(&phap_danh)
        .bind(&phap_hieu)
        .bind(&but_danh)
        .bind(&gender)
        .bind(&bio)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
    {
        Ok(updated_user) => {
            // Reload ranks để render trang hồ sơ
            let ranks = sqlx::query_as::<_, MemberRank>(
                "SELECT code, name, description, min_k_balance, color, icon, sort_order, created_at
                 FROM member_ranks ORDER BY sort_order ASC"
            )
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let html = ProfileTemplate {
                user: Some(updated_user),
                active_page: "profile".into(),
                ranks,
                error: None,
                success: Some("Hồ sơ đã được cập nhật. Nguyện công đức vô lượng.".into()),
            }
            .render()
            .unwrap_or_else(|e| {
                log::error!("Template render error (profile after update): {e}");
                format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
            });

            Html(html).into_response()
        }
        Err(e) => {
            log::error!("❌ Lỗi cập nhật hồ sơ: {e}");
            render_profile_error(
                &state.pool,
                Some(user),
                "Không thể cập nhật hồ sơ. Vui lòng thử lại.",
            )
            .await
        }
    }
}

/// Helper: Render trang profile với thông báo lỗi.
async fn render_profile_error(
    pool: &PgPool,
    user: Option<User>,
    error: &str,
) -> Response {
    let ranks = sqlx::query_as::<_, MemberRank>(
        "SELECT code, name, description, min_k_balance, color, icon, sort_order, created_at
         FROM member_ranks ORDER BY sort_order ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let html = ProfileTemplate {
        user,
        active_page: "profile".into(),
        ranks,
        error: Some(error.into()),
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (profile error): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// Helper: Chuẩn hoá chuỗi tuỳ chọn (None nếu rỗng hoặc chỉ whitespace).
fn normalize_optional(s: Option<&String>) -> Option<String> {
    s.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// --- Placeholder Section Handlers (Vietnamese) ---
//
// Các trang dưới đây dùng chung một helper `placeholder_page` để giữ
// giao diện nhất quán (header/footer từ layout, không phải HTML rời).

pub async fn khong_gian(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "home", "Không Gian", "🌍", "Không gian cá nhân, cộng tu, niệm Phật", "Giai đoạn 5")
}

pub async fn cong_dong(State(state): State<AppState>, jar: CookieJar) -> Response {
    // [v0.6+] Cộng Đồng đã có trang riêng — delegate cho community handler.
    community::cong_dong_index(State(state), jar).await
}

pub async fn ban_be(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "friends", "Bạn Bè", "👤", "Kết nối, nhắn tin, gửi thư — Kết bạn đạo hữu", "Giai đoạn 15")
}

pub async fn kinh_sach(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "books", "Kinh Sách", "📚", "Thư viện kinh sách Phật giáo, Đạo giáo và triết học", "Giai đoạn 17")
}

pub async fn quy_tu_bi(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "", "Quỹ Từ Bi", "🪷", "Quỹ chung cộng đồng — quyên góp, phát quà, hỗ trợ mạnh thường quân", "Giai đoạn 10")
}

pub async fn thuong_thanh(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "", "Thương Thành", "🏪", "Mua bán, trao đổi vật phẩm và dịch vụ trong cộng đồng", "Giai đoạn 10")
}

pub async fn bang_xep_hang(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;
    placeholder_page(user.as_ref(), "", "Bảng Xếp Hạng", "🏆", "Thành tích niệm Phật, tài Phú K, niệm lực A, phiếu Từ Bi", "Giai đoạn 19")
}

/// API: Heartbeat — keeps session alive (called every 5 min by client JS).
pub async fn heartbeat() -> Response {
    axum::Json(serde_json::json!({ "status": "ok" })).into_response()
}

// --- Helper ---

/// Render trang "tính năng đang phát triển" dùng layout chính.
#[allow(clippy::too_many_lines)]
fn placeholder_page(
    user: Option<&User>,
    active_page: &str,
    title: &str,
    icon: &str,
    desc: &str,
    phase: &str,
) -> Response {
    let body = format!(
        r#"<section class="max-w-4xl mx-auto px-4 py-20 text-center">
    <span class="text-6xl">{icon}</span>
    <h1 class="text-3xl font-bold text-tubi-800 mt-4 mb-2" style="color:#2E7D32">{title}</h1>
    <p class="text-gray-500 mb-2">{desc}</p>
    <p class="text-sm text-amber-600 mb-8">Tính năng đang phát triển — {phase}</p>
    <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition" style="background-color:#2E7D32">← Về trang chủ</a>
</section>"#
    );

    let user_html = render_user_menu_html(user);
    let mobile_user_html = render_mobile_user_menu_html(user);

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
                <p>🪷 Ứng Dụng Từ Bi v0.9 · Nguyện công đức vô lượng · Nam Mô A Di Đà Phật</p>
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
    Html(html).into_response()
}

/// Helper: render HTML cho menu user ở header desktop.
#[allow(clippy::option_if_let_else)]
fn render_user_menu_html(user: Option<&User>) -> String {
    if let Some(u) = user {
        let avatar_html = u.avatar_url.as_ref().map_or_else(
            || {
                let first_char = u.display_name.chars().next().unwrap_or('🪷');
                format!(
                    r#"<span class="w-8 h-8 rounded-full bg-lotus flex items-center justify-center text-tubi-900 font-bold" style="color:#1B5E20">{first_char}</span>"#
                )
            },
            |avatar| {
                format!(
                    r#"<img src="{avatar}" alt="avatar" class="w-8 h-8 rounded-full border-2 border-lotus" referrerpolicy="no-referrer">"#
                )
            },
        );
        let mut html = String::new();
        html.push_str("<div class=\"flex items-center space-x-3\">");
        html.push_str(&avatar_html);
        html.push_str("<span class=\"text-tubi-100 text-sm\" title=\"");
        html.push_str(u.rank_display());
        html.push_str("\">");
        html.push_str(u.rank_icon());
        html.push(' ');
        html.push_str(&u.display_name);
        html.push_str("</span>");
        html.push_str("<a href=\"/ca-nhan\" class=\"text-tubi-200 hover:text-white text-sm transition-colors\">Hồ sơ</a>");
        html.push_str("<a href=\"#\" onclick=\"event.preventDefault(); document.getElementById(&#39;logout-form-desktop&#39;).submit();\" \
                       class=\"bg-tubi-600 hover:bg-tubi-500 px-3 py-1.5 rounded-lg text-sm transition-colors cursor-pointer\">Thoát</a>");
        html.push_str("<form id=\"logout-form-desktop\" action=\"/dang-xuat\" method=\"POST\" class=\"hidden\"></form>");
        html.push_str("</div>");
        html
    } else {
        r#"<a href="/auth/google" class="bg-lotus hover:bg-golden text-tubi-900 px-4 py-2 rounded-lg text-sm font-semibold transition-colors">
                🪷 Đăng Nhập Bằng Google
            </a>"#
            .to_string()
    }
}

/// Helper: render HTML cho menu user ở mobile menu.
fn render_mobile_user_menu_html(user: Option<&User>) -> String {
    user.map_or_else(
        || r#"<a href="/auth/google" class="block px-3 py-2 rounded-lg bg-lotus text-tubi-900 mt-1">🪷 Đăng Nhập Bằng Google</a>"#
            .to_string(),
        |_| r#"<a href="/ca-nhan" class="block px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700">Hồ sơ cá nhân</a>
           <form action="/dang-xuat" method="POST" class="block">
               <button type="submit" class="w-full text-left px-3 py-2 rounded-lg text-tubi-100 hover:bg-tubi-700 cursor-pointer">Thoát</button>
           </form>"#
            .to_string(),
    )
}



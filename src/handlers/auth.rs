use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use rand::Rng;
use serde::Deserialize;

use crate::AppState;
use crate::config::Config;
use crate::models::user::{GoogleUserInfo, User};

/// Danh sách cột users đầy đủ (đồng bộ với `handlers::USER_COLUMNS` và model User).
/// v0.9.8: thêm `role` (fix bug CRITICAL — Google OAuth login bị hỏng vì thiếu cột role).
const USER_COLUMNS: &str = "id, email, display_name, password_hash, rank, \
    a_balance, k_balance, is_active, created_at, updated_at, \
    google_sub, avatar_url, email_verified, \
    phap_danh, phap_hieu, but_danh, gender, bio, \
    avatar_upload_id, role";

/// Tên cookie lưu OAuth state (chống CSRF).
const OAUTH_STATE_COOKIE: &str = "oauth_state";
/// Tên cookie lưu đường dẫn quay lại sau khi đăng nhập.
const OAUTH_RETURN_COOKIE: &str = "oauth_return";

/// Query params cho GET /dang-nhap hoặc GET /auth/google (chỉ có `?next=...`).
#[derive(Debug, Deserialize, Default)]
pub struct LoginQuery {
    pub next: Option<String>,
}

/// GET /dang-nhap hoặc POST /dang-nhap hoặc GET /auth/google —
/// chuyển hướng người dùng sang Google OAuth.
///
/// Tạo state ngẫu nhiên, lưu vào cookie (`HttpOnly`, `SameSite=Lax`),
/// sau đó redirect tới <https://accounts.google.com/o/oauth2/v2/auth>
/// với scope `openid email profile`.
pub async fn google_login(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
) -> Response {
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use time::Duration;

    let config = &state.config;

    // Nếu chưa cấu hình Google OAuth → báo lỗi thân thiện.
    if config.google_client_id.is_empty() || config.google_client_secret.is_empty() {
        return error_page(
            "Chưa cấu hình Google OAuth",
            "Ứng dụng chưa được cấp Client ID / Secret của Google. Vui lòng liên hệ quản trị viên.",
        );
    }

    // Sinh state ngẫu nhiên 32 bytes (hex 64 chars).
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    let state_value: String = bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = std::fmt::Write::write_fmt(&mut s, std::format_args!("{b:02x}"));
        s
    });

    // Lưu đường dẫn quay lại (nếu có query ?next=...) — chỉ cho phép path tương đối.
    let next = query
        .next
        .as_deref()
        .map(|s| urlencoding::decode(s).ok().map(std::borrow::Cow::into_owned).unwrap_or_default())
        .filter(|s| s.starts_with('/') && !s.starts_with("//"))
        .unwrap_or_else(|| "/".to_string());

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         response_type=code&\
         client_id={client_id}&\
         redirect_uri={redirect_uri}&\
         scope=openid%20email%20profile&\
         state={state}&\
         access_type=online&\
         prompt=select_account",
        client_id = urlencoding::encode(&config.google_client_id),
        redirect_uri = urlencoding::encode(&config.google_redirect_uri),
        state = state_value,
    );

    // Build cookies to set in response.
    let state_cookie = Cookie::build((OAUTH_STATE_COOKIE, state_value))
        .path("/")
        .max_age(Duration::seconds(600))
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    let return_cookie = Cookie::build((OAUTH_RETURN_COOKIE, next))
        .path("/")
        .max_age(Duration::seconds(600))
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    let jar = CookieJar::new().add(state_cookie).add(return_cookie);

    // Combine CookieJar + Redirect into a Response manually so we can attach Set-Cookie headers.
    let mut resp = Redirect::to(&auth_url).into_response();
    let headers = resp.headers_mut();
    for c in jar.iter() {
        if let Ok(s) = c.encoded().to_string().try_into() {
            headers.append(axum::http::header::SET_COOKIE, s);
        }
    }
    resp
}

/// Query string Google gửi về /auth/google/callback.
#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Google gửi kèm error khi người dùng từ chối.
    pub error: Option<String>,
}

/// GET /auth/google/callback — Google redirect người dùng về đây.
///
/// 1. Kiểm tra state từ cookie khớp query (chống CSRF).
/// 2. Đổi code lấy `access_token` qua POST /o/oauth2/token.
/// 3. Gọi /oauth2/v3/userinfo để lấy sub, email, name, picture.
/// 4. Tìm user theo `google_sub`; nếu không có thì tạo mới.
///    Nếu email đã tồn tại trong hệ thống (tài khoản cũ) thì link luôn.
/// 5. Tạo session, set cookie, redirect về `next` (mặc định "/").
pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GoogleCallbackQuery>,
) -> Response {
    let pool = &state.pool;
    let config = &state.config;

    // Trường hợp người dùng từ chối cấp quyền.
    if let Some(err) = &query.error {
        return error_page(
            "Chưa đăng nhập được bằng Google",
            &format!("Google báo lỗi: {err}. Vui lòng thử lại."),
        );
    }

    let code = match &query.code {
        Some(c) if !c.is_empty() => c.clone(),
        _ => return error_page("Thiếu mã xác thực", "Google không trả về mã code. Vui lòng thử lại."),
    };

    // Kiểm tra state từ cookie khớp query state (chống CSRF).
    let cookie_state = jar
        .get(OAUTH_STATE_COOKIE)
        .map(|c| c.value().to_string());
    let return_path = jar
        .get(OAUTH_RETURN_COOKIE)
        .map(|c| c.value().to_string())
        .filter(|s| s.starts_with('/') && !s.starts_with("//"))
        .unwrap_or_else(|| "/".to_string());

    let state_match = match (&query.state, &cookie_state) {
        (Some(s), Some(c)) => s == c && !s.is_empty(),
        _ => false,
    };

    if !state_match {
        return error_page(
            "Phiên đăng nhập không hợp lệ",
            "State không khớp — có thể do cookie hết hạn hoặc tấn công CSRF. Vui lòng đăng nhập lại.",
        );
    }

    // Đổi code lấy access_token.
    let token_res = exchange_code_for_token(config, &code).await;
    let access_token = match token_res {
        Ok(t) => t,
        Err(e) => {
            log::error!("❌ Lỗi đổi code Google OAuth: {e}");
            return error_page(
                "Không lấy được access token",
                "Đổi mã code sang access_token thất bại. Vui lòng thử lại sau.",
            );
        }
    };

    // Lấy userinfo.
    let user_info = match fetch_google_userinfo(&access_token).await {
        Ok(u) => u,
        Err(e) => {
            log::error!("❌ Lỗi lấy Google userinfo: {e}");
            return error_page(
                "Không lấy được thông tin Google",
                "Không đọc được email/định danh từ Google. Vui lòng thử lại sau.",
            );
        }
    };

    // Upsert user (theo google_sub, hoặc theo email nếu tài khoản cũ đã có sẵn).
    let user = match upsert_google_user(pool, &user_info).await {
        Ok(u) => u,
        Err(e) => {
            log::error!("❌ Lỗi upsert user Google: {e}");
            return error_page(
                "Không tạo được tài khoản",
                "Lỗi khi ghi nhận người dùng. Vui lòng thử lại sau.",
            );
        }
    };

    if !user.is_active {
        return error_page(
            "Tài khoản đã bị vô hiệu hóa",
            "Tài khoản của bạn đã bị quản trị viên vô hiệu. Vui lòng liên hệ hỗ trợ.",
        );
    }

    // Tạo session.
    let session_id = uuid::Uuid::new_v4().to_string();
    let insert_session = sqlx::query(
        "INSERT INTO sessions (id, user_id, created_at, expires_at)
         VALUES ($1, $2, NOW(), NOW() + INTERVAL '7 days')",
    )
    .bind(&session_id)
    .bind(user.id)
    .execute(pool)
    .await;

    if let Err(e) = insert_session {
        log::error!("❌ Lỗi tạo session: {e}");
        return error_page("Lỗi tạo phiên", "Không tạo được phiên đăng nhập. Vui lòng thử lại.");
    }

    // Xoá cookie OAuth tạm + set session cookie.
    build_session_redirect_response(
        jar, config.is_production, &session_id, &return_path,
    )
}

/// Build redirect response with session cookie set and OAuth cookies cleared.
fn build_session_redirect_response(
    jar: axum_extra::extract::CookieJar,
    is_production: bool,
    session_id: &str,
    return_path: &str,
) -> Response {
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use time::Duration;

    let cleared_state = Cookie::build((OAUTH_STATE_COOKIE, ""))
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    let cleared_return = Cookie::build((OAUTH_RETURN_COOKIE, ""))
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    let session_cookie = Cookie::build(("session_id", session_id.to_string()))
        .path("/")
        .max_age(Duration::days(7))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(is_production)
        .build();

    let new_jar = jar
        .remove(cleared_state)
        .remove(cleared_return)
        .add(session_cookie);

    let mut resp = Redirect::to(return_path).into_response();
    let headers = resp.headers_mut();
    for c in new_jar.iter() {
        if let Ok(s) = c.encoded().to_string().try_into() {
            headers.append(axum::http::header::SET_COOKIE, s);
        }
    }
    resp
}

/// POST /dang-xuat — xoá session khỏi DB và cookie phía client.
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    use axum_extra::extract::cookie::{Cookie, SameSite};
    use time::Duration;

    if let Some(cookie) = jar.get("session_id") {
        let session_id = cookie.value();
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(&state.pool)
            .await;
    }

    let cleared = Cookie::build(("session_id", ""))
        .path("/")
        .max_age(Duration::ZERO)
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    let new_jar = jar.remove(cleared);

    let mut resp = Redirect::to("/").into_response();
    let headers = resp.headers_mut();
    for c in new_jar.iter() {
        if let Ok(s) = c.encoded().to_string().try_into() {
            headers.append(axum::http::header::SET_COOKIE, s);
        }
    }
    resp
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserinfoRaw {
    sub: String,
    email: String,
    email_verified: Option<serde_json::Value>, // Google trả về bool hoặc chuỗi "true"
    name: Option<String>,
    picture: Option<String>,
}

/// Đổi authorization code lấy `access_token`.
async fn exchange_code_for_token(
    config: &Config,
    code: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;

    let params = [
        ("code", code.to_string()),
        ("client_id", config.google_client_id.clone()),
        ("client_secret", config.google_client_secret.clone()),
        ("redirect_uri", config.google_redirect_uri.clone()),
        ("grant_type", "authorization_code".to_string()),
    ];

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("token request: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("token endpoint returned {status}: {body}"));
    }

    let token: GoogleTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("decode token json: {e}"))?;

    Ok(token.access_token)
}

/// Lấy thông tin người dùng từ Google userinfo endpoint.
async fn fetch_google_userinfo(access_token: &str) -> Result<GoogleUserInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;

    let resp = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("userinfo request: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("userinfo returned {status}: {body}"));
    }

    let raw: GoogleUserinfoRaw = resp
        .json()
        .await
        .map_err(|e| format!("decode userinfo json: {e}"))?;

    // Google có thể trả email_verified là bool hoặc chuỗi "true"/"false".
    let email_verified = match raw.email_verified {
        Some(serde_json::Value::Bool(b)) => b,
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => false,
    };

    Ok(GoogleUserInfo {
        sub: raw.sub,
        email: raw.email,
        email_verified,
        name: raw.name.unwrap_or_else(|| "Đạo hữu".to_string()),
        picture: raw.picture,
    })
}

/// Tìm user theo `google_sub`. Nếu chưa có:
/// - Nếu email trùng với tài khoản cũ đã đăng ký bằng email/password → link `google_sub` vào.
/// - Nếu không → tạo user mới với rank "new", A=0, K=0.
async fn upsert_google_user(
    pool: &sqlx::PgPool,
    info: &GoogleUserInfo,
) -> Result<User, sqlx::Error> {
    let select_sql = format!("SELECT {USER_COLUMNS} FROM users WHERE google_sub = $1");

    // 1. Tìm theo google_sub.
    if let Some(u) = sqlx::query_as::<_, User>(&select_sql)
        .bind(&info.sub)
        .fetch_optional(pool)
        .await?
    {
        return Ok(u);
    }

    // 2. Nếu chưa có — tìm theo email (tài khoản cũ email/password).
    //    Nếu có, link google_sub + avatar_url vào.
    let link_sql = format!(
        "UPDATE users
         SET google_sub = $1,
             avatar_url = COALESCE($2, avatar_url),
             email_verified = $3,
             updated_at = NOW()
         WHERE email = $4 AND google_sub IS NULL
         RETURNING {USER_COLUMNS}"
    );
    let linked = sqlx::query_as::<_, User>(&link_sql)
        .bind(&info.sub)
        .bind(&info.picture)
        .bind(info.email_verified)
        .bind(&info.email)
        .fetch_optional(pool)
        .await?;
    if let Some(u) = linked {
        return Ok(u);
    }

    // 3. Tạo user mới.
    let insert_sql = format!(
        "INSERT INTO users (email, display_name, password_hash, rank, a_balance, k_balance, is_active, google_sub, avatar_url, email_verified, gender)
         VALUES ($1, $2, NULL, 'new', 0, 0, true, $3, $4, $5, 'other')
         RETURNING {USER_COLUMNS}"
    );
    let u = sqlx::query_as::<_, User>(&insert_sql)
        .bind(&info.email)
        .bind(&info.name)
        .bind(&info.sub)
        .bind(&info.picture)
        .bind(info.email_verified)
        .fetch_one(pool)
        .await?;
    Ok(u)
}

/// Trang lỗi đơn giản (dùng khi OAuth thất bại).
fn error_page(title: &str, msg: &str) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="vi"><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title} — Ứng Dụng Từ Bi</title>
<script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-50 min-h-screen flex items-center justify-center px-4">
<div class="max-w-md w-full bg-white rounded-2xl p-8 shadow-lg text-center">
  <div class="text-5xl mb-4">🪷</div>
  <h1 class="text-xl font-bold text-tubi-800 mb-2" style="color:#1B5E20">{title}</h1>
  <p class="text-gray-600 text-sm mb-6">{msg}</p>
  <a href="/" class="inline-block bg-tubi-800 text-white px-6 py-2 rounded-xl hover:bg-tubi-900 transition" style="background-color:#2E7D32">← Về trang chủ</a>
  <a href="/dang-nhap" class="inline-block ml-2 text-tubi-700 px-6 py-2 rounded-xl border border-tubi-300 hover:bg-tubi-50 transition" style="color:#388E3C;border-color:#A5D6A7">Thử lại →</a>
</div>
</body></html>"#
    );
    Html(html).into_response()
}

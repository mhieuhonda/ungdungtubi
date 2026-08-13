use actix_web::{web, HttpRequest, HttpResponse};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{LoginRequest, RegisterRequest, User};

pub async fn register(
    pool: web::Data<PgPool>,
    form: web::Form<RegisterRequest>,
) -> HttpResponse {
    // Validate
    if form.password != form.confirm_password {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Mật khẩu xác nhận không khớp"
        }));
    }

    if form.password.len() < 6 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Mật khẩu phải có ít nhất 6 ký tự"
        }));
    }

    // Hash password — handle error gracefully instead of .unwrap()
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = match argon2.hash_password(form.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Lỗi mã hóa mật khẩu. Vui lòng thử lại."
            }));
        }
    };

    // Insert user — use ON CONFLICT to handle race condition atomically
    let user_id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, rank, a_balance, k_balance, is_active) 
         VALUES ($1, $2, $3, $4, 'new', 0, 0, true)
         ON CONFLICT (email) DO NOTHING",
    )
    .bind(user_id)
    .bind(&form.email)
    .bind(&form.display_name)
    .bind(&password_hash)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() == 0 {
                // Email already exists (ON CONFLICT did nothing)
                HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Email đã được đăng ký"
                }))
            } else {
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": "Đăng ký thành công! Chào mừng bạn đến với Ứng Dụng Từ Bi."
                }))
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Lỗi đăng ký: {e}")
        })),
    }
}

pub async fn login(
    pool: web::Data<PgPool>,
    form: web::Form<LoginRequest>,
) -> HttpResponse {
    // Find user
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, display_name, password_hash, rank, a_balance, k_balance, is_active, created_at, updated_at 
         FROM users WHERE email = $1",
    )
    .bind(&form.email)
    .fetch_optional(pool.get_ref())
    .await;

    match user {
        Ok(Some(u)) => {
            // Check if account is active (banned/deactivated users cannot log in)
            if !u.is_active {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "Tài khoản đã bị vô hiệu hóa"
                }));
            }

            // Verify password — handle corrupted hash gracefully
            let parsed_hash = match PasswordHash::new(&u.password_hash) {
                Ok(h) => h,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Lỗi hệ thống xác thực. Vui lòng liên hệ quản trị viên."
                    }));
                }
            };

            if Argon2::default()
                .verify_password(form.password.as_bytes(), &parsed_hash)
                .is_ok()
            {
                // Create session
                let session_id = Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO sessions (id, user_id, created_at, expires_at) 
                     VALUES ($1, $2, NOW(), NOW() + INTERVAL '7 days')",
                )
                .bind(&session_id)
                .bind(u.id)
                .execute(pool.get_ref())
                .await;

                HttpResponse::Ok()
                    .cookie(
                        actix_web::cookie::Cookie::build("session_id", &session_id)
                            .path("/")
                            .max_age(actix_web::cookie::time::Duration::days(7))
                            .http_only(true)
                            .same_site(actix_web::cookie::SameSite::Lax)
                            .finish(),
                    )
                    .json(serde_json::json!({
                        "success": true,
                        "user": {
                            "id": u.id,
                            "display_name": u.display_name,
                            "rank": u.rank
                        }
                    }))
            } else {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Mật khẩu không đúng"
                }))
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Email không tồn tại"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Lỗi hệ thống: {e}")
        })),
    }
}

pub async fn logout(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    // Delete session from database if cookie exists
    if let Some(cookie) = req.cookie("session_id") {
        let session_id = cookie.value();
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool.get_ref())
            .await;
    }

    // Clear the client-side cookie
    HttpResponse::Ok()
        .cookie(
            actix_web::cookie::Cookie::build("session_id", "")
                .path("/")
                .max_age(actix_web::cookie::time::Duration::ZERO)
                .http_only(true)
                .finish(),
        )
        .json(serde_json::json!({
            "success": true,
            "message": "Đã đăng xuất"
        }))
}

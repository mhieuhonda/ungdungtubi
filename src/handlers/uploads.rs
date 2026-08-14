//! Handler upload ảnh (avatar, bài viết, bình luận).
//!
//! Giới hạn:
//!   * Kích thước tối đa: `config.max_upload_bytes` (mặc định 5 MB)
//!   * MIME type: chỉ cho phép JPEG / PNG / WebP / GIF
//!   * Lưu file vào `config.upload_dir` với tên = `<uuid>.<ext>`
//!   * Ghi metadata vào bảng `images`
//!   * Trả về JSON `{ id, url, size, mime_type }`

use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::CookieJar;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;

/// Danh sách MIME types được phép + phần mở rộng tương ứng.
const ALLOWED_MIME: &[(&str, &str)] = &[
    ("image/jpeg", "jpg"),
    ("image/png", "png"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
];

/// Tìm MIME type trong Content-Type header.
fn parse_mime(content_type: &str) -> Option<String> {
    let mime = content_type.split(';').next()?.trim().to_lowercase();
    if mime.is_empty() {
        None
    } else {
        Some(mime)
    }
}

/// Kiểm tra MIME type có được phép không, trả về phần mở rộng.
fn mime_to_ext(mime: &str) -> Option<&'static str> {
    ALLOWED_MIME
        .iter()
        .find(|(m, _)| *m == mime)
        .map(|(_, ext)| *ext)
}

/// GET /api/upload-info — trả về giới hạn upload.
pub async fn upload_info() -> Response {
    Json(serde_json::json!({
        "max_bytes": 5 * 1024 * 1024,
        "allowed_mime": ALLOWED_MIME.iter().map(|(m, _)| m).collect::<Vec<_>>(),
    }))
    .into_response()
}

/// POST /api/upload-image — upload một ảnh.
///
/// Form-data field name = `file`. Yêu cầu đăng nhập.
/// Trả về JSON `{ id, url, size, mime_type, width, height }`.
pub async fn upload_image(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response {
    let pool = &state.pool;
    let config = &state.config;

    // 1. Auth
    let user = match get_user_from_session(pool, &jar).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Vui lòng đăng nhập để upload ảnh."
                })),
            )
                .into_response();
        }
    };

    // 2. Đảm bảo upload_dir tồn tại
    if let Err(e) = std::fs::create_dir_all(&config.upload_dir) {
        log::error!("❌ Không tạo được upload_dir {:?}: {e}", config.upload_dir);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Lỗi server: không chuẩn bị được thư mục upload."
            })),
        )
            .into_response();
    }

    // 3. Đọc field `file` từ multipart (axum::extract::Multipart)
    let mut original_name: Option<String> = None;
    let mut detected_mime: Option<String> = None;
    let mut field_count = 0u32;
    let mut accumulated: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        field_count += 1;
        if field_count > 5 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Quá nhiều field trong form upload."
                })),
            )
                .into_response();
        }

        let field_name = field.name().unwrap_or("").to_string();
        let field_filename = field.file_name().map(|s| s.to_string());
        let content_type = field.content_type().map(|m| m.to_string());

        if field_name != "file" {
            // Skip các field khác (chỉ cho phép `file`)
            continue;
        }

        // Lưu filename + mime từ field này
        if let Some(fname) = field_filename {
            let safe: String = fname
                .chars()
                .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-'))
                .collect();
            if !safe.is_empty() {
                original_name = Some(safe);
            }
        }
        if let Some(ct) = content_type {
            detected_mime = Some(ct);
        }

        // Đọc bytes — axum::Field đọc toàn bộ nội dung field bằng `bytes()`
        match field.bytes().await {
            Ok(chunk) => {
                accumulated.extend_from_slice(&chunk);
                if accumulated.len() as u64 > config.max_upload_bytes as u64 {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({
                            "error": format!(
                                "Ảnh vượt quá giới hạn {} bytes.",
                                config.max_upload_bytes
                            )
                        })),
                    )
                        .into_response();
                }
            }
            Err(e) => {
                log::error!("❌ Lỗi đọc field multipart: {e}");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Không đọc được dữ liệu upload."
                    })),
                )
                    .into_response();
            }
        }
    }

    let file_bytes: Bytes = Bytes::from(accumulated);

    if file_bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Không nhận được dữ liệu ảnh."
            })),
        )
            .into_response();
    }

    // 4. Validate MIME type
    let mime = match detected_mime.as_deref() {
        Some(m) => parse_mime(m).unwrap_or_default(),
        None => String::new(),
    };
    let ext = match mime_to_ext(&mime) {
        Some(e) => e,
        None => {
            return (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Json(serde_json::json!({
                    "error": "Định dạng ảnh không được hỗ trợ. Chỉ chấp nhận JPEG, PNG, WebP, GIF."
                })),
            )
                .into_response();
        }
    };

    // 5. Tính SHA-256
    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let sha256_hex = hasher.finalize();
    let sha256_str = hex_encode(&sha256_hex);

    // 6. Kiểm tra ảnh trùng (theo sha256 của user)
    let duplicate_check = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM images WHERE sha256 = $1 AND uploader_id = $2 LIMIT 1",
    )
    .bind(&sha256_str)
    .bind(user.id)
    .fetch_optional(pool)
    .await;

    if let Ok(Some(existing_id)) = duplicate_check {
        let url = format!("{}/{existing_id}.{ext}", config.upload_url_prefix);
        return Json(serde_json::json!({
            "id": existing_id,
            "url": url,
            "duplicate": true,
            "size": file_bytes.len(),
            "mime_type": mime,
        }))
        .into_response();
    }

    // 7. Sinh file_id + stored_filename
    let file_id = Uuid::new_v4();
    let stored_filename = format!("{file_id}.{ext}");

    // 8. Đọc kích thước ảnh (đơn giản: chỉ JPEG/PNG/WebP/GIF header)
    let (width, height) = parse_image_dimensions(&file_bytes, &mime);

    // 9. Ghi vào DB trước khi ghi file (để rollback dễ hơn)
    let original_name = original_name.unwrap_or_else(|| format!("upload.{ext}"));
    let insert_result = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO images
            (id, uploader_id, original_name, stored_filename, mime_type,
             size_bytes, sha256, width, height, purpose, is_public)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'other', true)
         RETURNING id",
    )
    .bind(file_id)
    .bind(user.id)
    .bind(&original_name)
    .bind(&stored_filename)
    .bind(&mime)
    .bind(file_bytes.len() as i64)
    .bind(&sha256_str)
    .bind(width)
    .bind(height)
    .fetch_one(pool)
    .await;

    let image_id = match insert_result {
        Ok(id) => id,
        Err(e) => {
            log::error!("❌ Lỗi ghi metadata ảnh: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Không ghi được metadata ảnh."
                })),
            )
                .into_response();
        }
    };

    // 10. Ghi file ra filesystem
    let file_path = config.upload_dir.join(&stored_filename);
    if let Err(e) = std::fs::write(&file_path, &file_bytes) {
        log::error!("❌ Lỗi ghi file ảnh {:?}: {e}", file_path);
        let _ = sqlx::query("DELETE FROM images WHERE id = $1")
            .bind(image_id)
            .execute(pool)
            .await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Không lưu được file ảnh."
            })),
        )
            .into_response();
    }

    let url = format!("{}/{stored_filename}", config.upload_url_prefix);

    log::info!(
        "🖼️  User {} uploaded image {} ({} bytes, {})",
        user.id,
        image_id,
        file_bytes.len(),
        mime
    );

    Json(serde_json::json!({
        "id": image_id,
        "url": url,
        "size": file_bytes.len(),
        "mime_type": mime,
        "width": width,
        "height": height,
        "sha256": sha256_str,
    }))
    .into_response()
}

/// Helper: hex-encode 32 bytes thành 64-char string.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Đọc width/height từ header ảnh.
fn parse_image_dimensions(bytes: &[u8], mime: &str) -> (Option<i32>, Option<i32>) {
    match mime {
        "image/png" => parse_png_dimensions(bytes),
        "image/jpeg" => parse_jpeg_dimensions(bytes),
        "image/gif" => parse_gif_dimensions(bytes),
        "image/webp" => parse_webp_dimensions(bytes),
        _ => (None, None),
    }
}

fn parse_png_dimensions(bytes: &[u8]) -> (Option<i32>, Option<i32>) {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return (None, None);
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (Some(w as i32), Some(h as i32))
}

fn parse_gif_dimensions(bytes: &[u8]) -> (Option<i32>, Option<i32>) {
    if bytes.len() < 10 {
        return (None, None);
    }
    let w = u16::from_le_bytes([bytes[6], bytes[7]]);
    let h = u16::from_le_bytes([bytes[8], bytes[9]]);
    (Some(w as i32), Some(h as i32))
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> (Option<i32>, Option<i32>) {
    if bytes.len() < 4 || bytes[0..2] != [0xFF, 0xD8] {
        return (None, None);
    }
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let marker = bytes[i];
        i += 1;

        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if i + 7 >= bytes.len() {
                break;
            }
            let h = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]);
            let w = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]);
            return (Some(w as i32), Some(h as i32));
        }

        if i + 1 >= bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if seg_len < 2 {
            break;
        }
        i += seg_len;
    }
    (None, None)
}

fn parse_webp_dimensions(bytes: &[u8]) -> (Option<i32>, Option<i32>) {
    if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return (None, None);
    }
    let chunk_type = &bytes[12..16];
    match chunk_type {
        b"VP8 " => {
            if bytes.len() < 30 {
                return (None, None);
            }
            let w = u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF;
            let h = u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF;
            (Some(w as i32), Some(h as i32))
        }
        b"VP8L" => {
            if bytes.len() < 25 {
                return (None, None);
            }
            let b0 = bytes[21] as u32;
            let b1 = bytes[22] as u32;
            let b2 = bytes[23] as u32;
            let b3 = bytes[24] as u32;
            let w = 1 + (b0 | ((b1 & 0x3F) << 8));
            let h = 1 + (((b1 >> 6) & 0x03) | (b2 << 2) | ((b3 & 0x0F) << 10));
            (Some(w as i32), Some(h as i32))
        }
        b"VP8X" => {
            if bytes.len() < 30 {
                return (None, None);
            }
            let w = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
            let h = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
            (Some(w as i32), Some(h as i32))
        }
        _ => (None, None),
    }
}

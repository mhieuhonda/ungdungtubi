#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Một thư viện (category) trong Kinh Sách.
///
/// 5 thư viện chính theo HieuLouis/: Phật Gia, Đạo Gia, Kinh Văn, Sách Quý, Quan Trọng.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookCategory {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Một cuốn sách trong thư viện Kinh Sách.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Book {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub author: Option<String>,
    pub translator: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub language: String,
    pub cover_url: Option<String>,
    pub download_url: Option<String>,
    /// single | multi
    pub book_type: String,
    pub chapter_count: i32,
    pub view_count: i64,
    pub review_count: i32,
    pub flower_count: i32,
    pub donation_total_k: i64,
    pub is_featured: bool,
    pub status: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Book kèm thông tin category (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookWithCategory {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub author: Option<String>,
    pub translator: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub language: String,
    pub cover_url: Option<String>,
    pub download_url: Option<String>,
    /// single | multi
    pub book_type: String,
    pub chapter_count: i32,
    pub view_count: i64,
    pub review_count: i32,
    pub flower_count: i32,
    pub donation_total_k: i64,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Một chương sách.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookChapter {
    pub id: Uuid,
    pub book_id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub sort_order: i32,
    pub view_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// BookChapter dạng tóm tắt (không include content) — dùng cho list chapters.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookChapterSummary {
    pub id: Uuid,
    pub book_id: Uuid,
    pub slug: String,
    pub title: String,
    pub sort_order: i32,
    pub view_count: i64,
}

/// Cảm ngộ của thành viên về sách.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookReview {
    pub id: Uuid,
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub flower_count: i32,
    /// pending | approved | rejected
    pub status: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// BookReview kèm thông tin author (join query).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookReviewWithAuthor {
    pub id: Uuid,
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub flower_count: i32,
    pub status: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Từ users
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_rank: String,
}

/// Form tạo cảm ngộ.
#[derive(Debug, Deserialize)]
pub struct BookReviewForm {
    pub body: String,
}

/// Một mục trong lịch sử tìm kiếm Kinh Sách của user (v0.9.44 — Giai đoạn 51).
///
/// Đọc từ bảng `user_search_history` (migration 031). Hiển thị 10 chip gần nhất
/// trên trang `/kinh-sach/tim-kiem` — click chip sẽ tái chạy tìm kiếm với query đó.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSearchHistoryItem {
    pub id: i64,
    pub user_id: Uuid,
    pub query: String,
    pub searched_at: DateTime<Utc>,
}

impl UserSearchHistoryItem {
    /// Hiển thị thời gian tương đối (ví dụ: "3 phút trước").
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.searched_at)
    }

    /// URL-encoded query để chèn vào link `?q=...` (an toàn cho URL).
    pub fn url_encoded_query(&self) -> String {
        url_encode(&self.query)
    }
}

/// URL-encode đơn giản (đủ cho query param `q=...`).
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

impl BookWithCategory {
    /// Hiển thị category icon (fallback 📚).
    pub fn category_icon_or_book(&self) -> String {
        self.category_icon.clone().unwrap_or_else(|| "📚".into())
    }

    /// Hiển thị category name (fallback "Khác").
    pub fn category_name_or_other(&self) -> String {
        self.category_name.clone().unwrap_or_else(|| "Khác".into())
    }

    /// Hiển thị ngôn ngữ tiếng Việt.
    pub fn language_display(&self) -> &str {
        match self.language.as_str() {
            "en" => "Tiếng Anh",
            "zh" => "Tiếng Trung",
            _ => "Tiếng Việt",
        }
    }

    /// Hiển thị thời gian tương đối.
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    /// Trích đoạn description (max 200 ký tự).
    pub fn description_excerpt(&self) -> String {
        let desc = self.description.clone().unwrap_or_default();
        let s = desc.chars().take(200).collect::<String>();
        if desc.chars().count() > 200 {
            format!("{s}…")
        } else {
            s
        }
    }

    /// Highlight từ khoá `query` trong tiêu đề sách bằng thẻ `<mark>` (HTML-safe).
    /// Dùng cho trang tìm kiếm Kinh Sách (v0.9.44 — Giai đoạn 51).
    /// Trả về HTML đã escape — caller PHẢI dùng filter `| safe` trong Askama.
    pub fn highlighted_title(&self, query: &str) -> String {
        highlight_match(&self.title, query)
    }

    /// Highlight từ khoá `query` trong trích đoạn description bằng thẻ `<mark>`.
    /// Giới hạn 200 ký tự để tránh tràn layout (v0.9.44 — Giai đoạn 51).
    pub fn highlighted_description(&self, query: &str) -> String {
        let desc = self.description.clone().unwrap_or_default();
        let chars: Vec<char> = desc.chars().collect();
        let excerpt: String = if chars.len() > 200 {
            chars.iter().take(200).collect::<String>() + "…"
        } else {
            desc
        };
        highlight_match(&excerpt, query)
    }
}

/// Highlight từ khoá `query` trong `text` bằng thẻ `<mark>`, HTML-safe.
///
/// Phân biệt chữ hoa/thường khi match (lowercase cả hai), nhưng giữ nguyên
/// chữ hoa/thường của text gốc khi hiển thị. An toàn cho tiếng Việt có dấu
/// (so sánh theo byte offset, không theo char index).
///
/// Trả về chuỗi HTML đã escape phần text thường, KHÔNG escape thẻ `<mark>`
/// — caller dùng `| safe` trong Askama để render HTML.
fn highlight_match(text: &str, query: &str) -> String {
    if query.trim().is_empty() {
        return html_escape_encode(text);
    }
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let q_len = query.len();
    let mut result = String::with_capacity(text.len() + 32);
    let mut last = 0usize;
    let mut search_from = 0usize;
    while let Some(rel_pos) = text_lower[search_from..].find(&query_lower) {
        let pos = search_from + rel_pos;
        result.push_str(&html_escape_encode(&text[last..pos]));
        result.push_str("<mark>");
        result.push_str(&html_escape_encode(&text[pos..pos + q_len]));
        result.push_str("</mark>");
        last = pos + q_len;
        search_from = last;
    }
    result.push_str(&html_escape_encode(&text[last..]));
    result
}

/// HTML-escape chuỗi (an toàn cho Askama `| safe`).
fn html_escape_encode(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl BookReviewWithAuthor {
    /// Hiển thị thời gian tương đối.
    pub fn time_ago(&self) -> String {
        crate::handlers::community::time_ago_display(&self.created_at)
    }

    /// Chữ cái đầu tên author.
    pub fn author_initial(&self) -> char {
        self.author_display_name.chars().next().unwrap_or('🪷')
    }
}

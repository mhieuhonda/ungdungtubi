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

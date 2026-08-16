//! Models cho Nhà Nhạc — Giai đoạn 40 (v0.9.35).
//!
//! Nhà Nhạc (Music House) là 1 trong 8 phòng của Không Gian (KG-03).
//! Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx":
//!   - 5 thư mục nhạc: Niem · Thien · Dao · KhongLoi · CaNhan
//!   - 5 chế độ phát: SingleRepeat · Shuffle · RepeatAll · Loop · SleepTimer
//!   - Khi mở nhạc, thành viên trong Không Gian có thể nghe cùng
//!   - Cá Nhân = danh sách nhạc do user tải lên hoặc thêm từ kho hệ thống
//!   - Nhạc Cộng Đồng = user submit YouTube links, admin approve/reject
//!
//! Models:
//!   - `MusicCategory` — enum 4 category hệ thống (niem/thien/dao/khong_loi) + "ca_nhan"
//!   - `MusicTrack` — 1 bài nhạc trong kho hệ thống
//!   - `PlaybackMode` — chế độ phát nhạc
//!   - `UserMusicPrefs` — preferences phát nhạc per-user
//!   - `PersonalPlaylistItem` — entry trong playlist Cá Nhân của user
//!   - `UserMusicSubmission` — user-submitted YouTube music (pending/approved/rejected)
//!   - `SubmitMusicForm` — form cho POST /api/nha-nhac/dang-nhac
//!   - `ReviewSubmissionForm` — form cho admin review
//!   - `SubmissionWithUser` — submission kèm tên người đăng (cho admin view)
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Category ─────────────────────────────────────────────────────────────

/// Category nhạc — 4 category hệ thống + 1 "Cá Nhân" (lưu riêng trong user_personal_tracks).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MusicCategory {
    /// 📿 Nhạc Niệm — Phật hiệu, thần chú, danh xưng Phật.
    Niem,
    /// 🧘 Nhạc Thiền — thiền thanh tịnh, tĩnh tâm.
    Thien,
    /// 🛕 Nhạc Đạo — nhạc Phật giáo, ca khúc tu học.
    Dao,
    /// 🎵 Không Lời — instrumental, ambient.
    KhongLoi,
    /// ⭐ Cá Nhân — playlist riêng của user (lưu trong user_personal_tracks).
    CaNhan,
}

impl MusicCategory {
    /// Parse từ chuỗi (URL param / DB string).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "niem" => Some(Self::Niem),
            "thien" => Some(Self::Thien),
            "dao" => Some(Self::Dao),
            "khong_loi" => Some(Self::KhongLoi),
            "ca_nhan" => Some(Self::CaNhan),
            _ => None,
        }
    }

    /// DB string value.
    pub fn db_value(&self) -> &'static str {
        match self {
            Self::Niem => "niem",
            Self::Thien => "thien",
            Self::Dao => "dao",
            Self::KhongLoi => "khong_loi",
            Self::CaNhan => "ca_nhan",
        }
    }

    /// Tên hiển thị tiếng Việt.
    pub fn display(&self) -> &'static str {
        match self {
            Self::Niem => "Nhạc Niệm",
            Self::Thien => "Nhạc Thiền",
            Self::Dao => "Nhạc Đạo",
            Self::KhongLoi => "Không Lời",
            Self::CaNhan => "Cá Nhân",
        }
    }

    /// Emoji icon cho category.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Niem => "📿",
            Self::Thien => "🧘",
            Self::Dao => "🛕",
            Self::KhongLoi => "🎵",
            Self::CaNhan => "⭐",
        }
    }

    /// Màu sắc (hex) cho UI accent.
    pub fn color(&self) -> &'static str {
        match self {
            Self::Niem => "#D97706",     // amber-600
            Self::Thien => "#0EA5E9",    // sky-500
            Self::Dao => "#DC2626",      // red-600
            Self::KhongLoi => "#7C3AED", // violet-600
            Self::CaNhan => "#16A34A",   // green-600
        }
    }

    /// Mô tả dài cho UI.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Niem => "Phật hiệu, thần chú, danh xưng Phật — giúp tập trung niệm Phật.",
            Self::Thien => "Nhạc thiền thanh tịnh, tĩnh tâm — dành cho thiền định.",
            Self::Dao => "Ca khúc Phật giáo, nhạc đạo — tôn vinh giáo lý.",
            Self::KhongLoi => "Nhạc không lời, instrumental — thư giãn, đọc sách.",
            Self::CaNhan => "Danh sách nhạc bạn thêm từ kho hệ thống. Sau này: upload riêng.",
        }
    }

    /// Tất cả category hệ thống (không bao gồm CaNhan).
    pub fn all_system() -> &'static [MusicCategory] {
        &[
            MusicCategory::Niem,
            MusicCategory::Thien,
            MusicCategory::Dao,
            MusicCategory::KhongLoi,
        ]
    }
}

// ─── Playback Mode ────────────────────────────────────────────────────────

/// Chế độ phát nhạc — theo tài liệu Nhà Nhạc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    /// 🔂 Một bài liên tục — lặp lại 1 bài.
    SingleRepeat,
    /// 🔀 Ngẫu nhiên liên tục — shuffle + repeat.
    Shuffle,
    /// 🔁 Lặp lại liên tục — repeat all.
    RepeatAll,
    /// 🔄 Lặp lại một vòng — loop playlist.
    Loop,
}

impl PlaybackMode {
    /// Parse từ chuỗi.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "single_repeat" => Some(Self::SingleRepeat),
            "shuffle" => Some(Self::Shuffle),
            "repeat_all" => Some(Self::RepeatAll),
            "loop" => Some(Self::Loop),
            _ => None,
        }
    }

    /// DB string value.
    pub fn db_value(&self) -> &'static str {
        match self {
            Self::SingleRepeat => "single_repeat",
            Self::Shuffle => "shuffle",
            Self::RepeatAll => "repeat_all",
            Self::Loop => "loop",
        }
    }

    /// Tên hiển thị tiếng Việt.
    pub fn display(&self) -> &'static str {
        match self {
            Self::SingleRepeat => "Một bài liên tục",
            Self::Shuffle => "Ngẫu nhiên liên tục",
            Self::RepeatAll => "Lặp lại liên tục",
            Self::Loop => "Lặp lại một vòng",
        }
    }

    /// Emoji icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::SingleRepeat => "🔂",
            Self::Shuffle => "🔀",
            Self::RepeatAll => "🔁",
            Self::Loop => "🔄",
        }
    }

    /// Tất cả playback modes (cho UI selector).
    pub fn all() -> &'static [PlaybackMode] {
        &[
            PlaybackMode::SingleRepeat,
            PlaybackMode::Shuffle,
            PlaybackMode::RepeatAll,
            PlaybackMode::Loop,
        ]
    }
}

impl Default for PlaybackMode {
    fn default() -> Self {
        Self::RepeatAll
    }
}

// ─── Music Track ──────────────────────────────────────────────────────────

/// Một bài nhạc trong kho hệ thống.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MusicTrack {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub description: Option<String>,
    pub artist: Option<String>,
    pub audio_url: String,
    pub duration_seconds: i32,
    pub cover_url: Option<String>,
    pub is_public: bool,
    pub upload_user_id: Option<Uuid>,
    pub sort_order: i32,
    pub is_active: bool,
    pub play_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MusicTrack {
    /// Category enum (parse từ DB string).
    pub fn category_enum(&self) -> Option<MusicCategory> {
        MusicCategory::from_str(&self.category)
    }

    /// Format duration thành "MM:SS" hoặc "HH:MM:SS".
    pub fn duration_display(&self) -> String {
        let total = self.duration_seconds;
        if total <= 0 {
            return "—:—".to_string();
        }
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }

    /// Kiểm tra track có thể play không (cần audio_url hợp lệ).
    pub fn can_play(&self) -> bool {
        !self.audio_url.trim().is_empty()
    }

    /// Emoji cover fallback (nếu cover_url rỗng).
    pub fn cover_emoji(&self) -> &'static str {
        match self.category.as_str() {
            "niem" => "📿",
            "thien" => "🧘",
            "dao" => "🛕",
            "khong_loi" => "🎵",
            _ => "🎶",
        }
    }
}

// ─── User Music Preferences ───────────────────────────────────────────────

/// Preferences phát nhạc per-user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMusicPrefs {
    pub user_id: Uuid,
    pub playback_mode: String,
    pub volume: i32,
    pub sleep_timer_minutes: Option<i32>,
    pub last_track_id: Option<i64>,
    pub updated_at: DateTime<Utc>,
}

impl UserMusicPrefs {
    /// PlaybackMode enum.
    pub fn playback_mode_enum(&self) -> PlaybackMode {
        PlaybackMode::from_str(&self.playback_mode).unwrap_or_default()
    }
}

impl Default for UserMusicPrefs {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            playback_mode: PlaybackMode::default().db_value().to_string(),
            volume: 70,
            sleep_timer_minutes: None,
            last_track_id: None,
            updated_at: Utc::now(),
        }
    }
}

// ─── Form Payloads ────────────────────────────────────────────────────────

/// Form payload cho POST /api/nha-nhac/preferences — update preferences.
#[derive(Debug, Deserialize)]
pub struct MusicPrefsForm {
    pub playback_mode: Option<String>,
    pub volume: Option<i32>,
    pub sleep_timer_minutes: Option<Option<i32>>, // outer Option = field có trong form không; inner = value (None = tắt timer)
    pub last_track_id: Option<i64>,
}

impl MusicPrefsForm {
    /// Validate form — trả về (mode, volume, sleep_timer, last_track) đã sanitize.
    /// Trả về None nếu playback_mode không hợp lệ (chỉ khi field được gửi).
    pub fn validate(&self) -> Option<(Option<PlaybackMode>, Option<i32>, Option<Option<i32>>, Option<i64>)> {
        let mode = if let Some(m) = &self.playback_mode {
            Some(PlaybackMode::from_str(m)?)
        } else {
            None
        };
        let volume = self.volume.map(|v| v.clamp(0, 100));
        let sleep = self.sleep_timer_minutes.map(|inner| {
            inner.and_then(|mins| if mins > 0 { Some(mins) } else { None })
        });
        Some((mode, volume, sleep, self.last_track_id))
    }
}

/// Form payload cho POST /api/nha-nhac/ca-nhan/them — add track vào playlist cá nhân.
#[derive(Debug, Deserialize)]
pub struct AddPersonalTrackForm {
    pub track_id: i64,
}

// ─── Personal Playlist Item ───────────────────────────────────────────────

/// Một entry trong playlist Cá Nhân của user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PersonalPlaylistItem {
    pub id: i64,
    pub user_id: Uuid,
    pub track_id: i64,
    pub sort_order: i32,
    pub added_at: DateTime<Utc>,
}

// ─── Stats ────────────────────────────────────────────────────────────────

/// Stats cho Nhà Nhạc — hiển thị trên UI.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NhaNhacStats {
    /// Tổng số track trong kho hệ thống.
    pub total_tracks: i64,
    /// Số track theo category.
    pub tracks_by_category: Vec<(String, i64)>,
    /// Số track trong playlist Cá Nhân của user.
    pub personal_tracks: i64,
    /// Tổng lượt play.
    pub total_plays: i64,
}

/// Category tab cho UI — pre-computed để tránh Rust path expression trong Askama template.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryTab {
    /// DB value (vd: "niem", "thien", ...).
    pub db_value: String,
    /// Tên hiển thị tiếng Việt (vd: "Nhạc Niệm").
    pub display: String,
    /// Emoji icon (vd: "📿").
    pub icon: String,
    /// Mô tả dài.
    pub description: String,
    /// True nếu đây là category hiện tại đang được xem.
    pub is_current: bool,
}

impl CategoryTab {
    /// Tạo list 5 category tabs (Niem · Thien · Dao · KhongLoi · CaNhan).
    /// `current` là db_value của category đang active.
    pub fn all_tabs(current: &str) -> Vec<CategoryTab> {
        MusicCategory::all_system()
            .iter()
            .map(|c| CategoryTab {
                db_value: c.db_value().to_string(),
                display: c.display().to_string(),
                icon: c.icon().to_string(),
                description: c.description().to_string(),
                is_current: c.db_value() == current,
            })
            .chain(std::iter::once(CategoryTab {
                db_value: MusicCategory::CaNhan.db_value().to_string(),
                display: MusicCategory::CaNhan.display().to_string(),
                icon: MusicCategory::CaNhan.icon().to_string(),
                description: MusicCategory::CaNhan.description().to_string(),
                is_current: MusicCategory::CaNhan.db_value() == current,
            }))
            .collect()
    }
}

// ─── User Music Submission ────────────────────────────────────────────

/// Trạng thái submission.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubmissionStatus {
    Pending,
    Approved,
    Rejected,
}

impl SubmissionStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
    pub fn db_value(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
    pub fn display(&self) -> &'static str {
        match self {
            Self::Pending => "Chờ duyệt",
            Self::Approved => "Đã duyệt",
            Self::Rejected => "Từ chối",
        }
    }
    pub fn color(&self) -> &'static str {
        match self {
            Self::Pending => "#F59E0B",   // amber
            Self::Approved => "#16A34A",   // green
            Self::Rejected => "#DC2626",   // red
        }
    }
}

/// Một submission nhạc từ user.
///
/// v0.9.36 — Giai đoạn 41: thêm `source_type` ('youtube' | 'audio_file') +
/// `audio_file_upload_id` (link tới audio_files.id) + `audio_duration_seconds`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMusicSubmission {
    pub id: i64,
    pub user_id: Uuid,
    pub title: String,
    pub artist: String,
    pub category: String,
    pub youtube_url: String,
    pub youtube_id: String,
    pub description: String,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub play_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// v0.9.36 — Giai đoạn 41: 'youtube' hoặc 'audio_file'.
    #[sqlx(default)]
    pub source_type: String,
    /// v0.9.36 — Giai đoạn 41: NULL cho YouTube, link tới audio_files.id cho upload.
    #[sqlx(default)]
    pub audio_file_upload_id: Option<Uuid>,
    /// v0.9.36 — Giai đoạn 41: thời lượng file âm thanh (giây) nếu source_type=audio_file.
    #[sqlx(default)]
    pub audio_duration_seconds: Option<i32>,
}

impl UserMusicSubmission {
    pub fn status_enum(&self) -> SubmissionStatus {
        SubmissionStatus::from_str(&self.status).unwrap_or(SubmissionStatus::Pending)
    }
    pub fn category_display(&self) -> &str {
        match self.category.as_str() {
            "niem" => "Nhạc Niệm",
            "thien" => "Nhạc Thiền",
            "dao" => "Nhạc Đạo",
            "khong_loi" => "Không Lời",
            _ => "Khác",
        }
    }
    /// Generate YouTube embed URL for inline playback (only for source_type= youtube).
    pub fn youtube_embed_url(&self) -> String {
        format!("https://www.youtube.com/embed/{}?rel=0&modestbranding=1", self.youtube_id)
    }
    /// True nếu đây là submission dạng upload file âm thanh.
    pub fn is_audio_file(&self) -> bool {
        self.source_type == "audio_file"
    }
    /// Format duration thành "MM:SS" hoặc "HH:MM:SS" nếu có audio_duration_seconds.
    pub fn duration_display(&self) -> String {
        match self.audio_duration_seconds {
            Some(total) if total > 0 => {
                let h = total / 3600;
                let m = (total % 3600) / 60;
                let s = total % 60;
                if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
            }
            _ => "—:—".to_string(),
        }
    }
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let dur = now.signed_duration_since(self.created_at);
        let mins = dur.num_minutes();
        if mins < 1 { "vừa xong".to_string() }
        else if mins < 60 { format!("{mins} phút trước") }
        else if mins < 60 * 24 { format!("{} giờ trước", mins / 60) }
        else if mins < 60 * 24 * 7 { format!("{} ngày trước", mins / (60 * 24)) }
        else { self.created_at.format("%d/%m/%Y").to_string() }
    }
}

/// Form cho POST /api/nha-nhac/dang-nhac — user submit music.
#[derive(Debug, Deserialize)]
pub struct SubmitMusicForm {
    pub title: String,
    pub artist: String,
    pub category: String,
    pub youtube_url: String,
    pub description: Option<String>,
}

impl SubmitMusicForm {
    /// Validate form. Returns (sanitized_title, sanitized_artist, category, youtube_id, description) or error message.
    pub fn validate(&self) -> Result<(String, String, MusicCategory, String, String), String> {
        // Title
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err("Tiêu đề không được để trống.".into());
        }
        if title.chars().count() > 200 {
            return Err("Tiêu đề tối đa 200 ký tự.".into());
        }
        // Artist
        let artist = self.artist.trim().to_string();
        if artist.is_empty() {
            return Err("Nghệ sĩ không được để trống.".into());
        }
        if artist.chars().count() > 100 {
            return Err("Nghệ sĩ tối đa 100 ký tự.".into());
        }
        // Category
        let cat = MusicCategory::from_str(&self.category)
            .ok_or_else(|| "Thư mục nhạc không hợp lệ. Chọn: niem, thien, dao, khong_loi.".to_string())?;
        if matches!(cat, MusicCategory::CaNhan) {
            return Err("Không thể đăng vào thư mục Cá Nhân.".into());
        }
        // YouTube URL — extract video ID
        let youtube_id = extract_youtube_id(&self.youtube_url)
            .ok_or_else(|| "Link YouTube không hợp lệ. Ví dụ: https://www.youtube.com/watch?v=XXXXXXXXXXX".to_string())?;
        // Description
        let desc = self.description.as_deref().unwrap_or("").trim().to_string();
        if desc.chars().count() > 500 {
            return Err("Mô tả tối đa 500 ký tự.".into());
        }
        Ok((title, artist, cat, youtube_id, desc))
    }
}

/// Extract YouTube video ID from various URL formats.
/// Supports:
///   - https://www.youtube.com/watch?v=VIDEO_ID
///   - https://youtu.be/VIDEO_ID
///   - https://youtube.com/embed/VIDEO_ID
///   - https://m.youtube.com/watch?v=VIDEO_ID
///   - https://youtube.com/shorts/VIDEO_ID
///   - VIDEO_ID (11 chars directly)
pub fn extract_youtube_id(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() { return None; }
    // Direct 11-char ID
    if s.len() == 11 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Some(s.to_string());
    }
    // youtu.be/VIDEO_ID
    if let Some(pos) = s.find("youtu.be/") {
        let start = pos + 9;
        if start >= s.len() { return None; }
        let rest = &s[start..];
        let id: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(11).collect();
        if id.len() == 11 { return Some(id); }
    }
    // ?v=VIDEO_ID
    if let Some(pos) = s.find("v=") {
        let start = pos + 2;
        if start >= s.len() { return None; }
        let rest = &s[start..];
        let id: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(11).collect();
        if id.len() == 11 { return Some(id); }
    }
    // /embed/VIDEO_ID or /shorts/VIDEO_ID
    for prefix in &["/embed/", "/shorts/", "/v/"] {
        if let Some(pos) = s.find(prefix) {
            let start = pos + prefix.len();
            if start >= s.len() { continue; }
            let rest = &s[start..];
            let id: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(11).collect();
            if id.len() == 11 { return Some(id); }
        }
    }
    None
}

/// Form cho admin review.
#[derive(Debug, Deserialize)]
pub struct ReviewSubmissionForm {
    pub action: String,  // "approve" or "reject"
    pub note: Option<String>,
}

/// Submission kèm tên người đăng (cho admin view).
///
/// v0.9.36 — Giai đoạn 41: thêm `source_type`, `audio_file_upload_id`,
/// `audio_duration_seconds`, `audio_file_url` (URL playback file local).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SubmissionWithUser {
    pub id: i64,
    pub user_id: Uuid,
    pub title: String,
    pub artist: String,
    pub category: String,
    pub youtube_url: String,
    pub youtube_id: String,
    pub description: String,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub play_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitter_name: String,
    pub submitter_avatar: Option<String>,
    /// v0.9.36 — Giai đoạn 41: 'youtube' hoặc 'audio_file'.
    #[sqlx(default)]
    pub source_type: String,
    /// v0.9.36 — Giai đoạn 41: NULL cho YouTube, link tới audio_files.id cho upload.
    #[sqlx(default)]
    pub audio_file_upload_id: Option<Uuid>,
    /// v0.9.36 — Giai đoạn 41: thời lượng file âm thanh (giây) nếu source_type=audio_file.
    #[sqlx(default)]
    pub audio_duration_seconds: Option<i32>,
    /// v0.9.36 — Giai đoạn 41: stored_filename của audio_files (để build URL playback).
    /// JOIN từ audio_files khi query. NULL cho YouTube.
    #[sqlx(default)]
    pub audio_stored_filename: Option<String>,
}

impl SubmissionWithUser {
    /// True nếu đây là submission dạng upload file âm thanh.
    pub fn is_audio_file(&self) -> bool {
        self.source_type == "audio_file"
    }
    /// Generate YouTube embed URL for inline playback (only for source_type=youtube).
    pub fn youtube_embed_url(&self) -> String {
        format!("https://www.youtube.com/embed/{}?rel=0&modestbranding=1", self.youtube_id)
    }
    /// Format duration thành "MM:SS" hoặc "HH:MM:SS" nếu có audio_duration_seconds.
    pub fn duration_display(&self) -> String {
        match self.audio_duration_seconds {
            Some(total) if total > 0 => {
                let h = total / 3600;
                let m = (total % 3600) / 60;
                let s = total % 60;
                if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
            }
            _ => "—:—".to_string(),
        }
    }
    /// Icon cho loại nguồn (YouTube hoặc file âm thanh).
    pub fn source_icon(&self) -> &'static str {
        if self.is_audio_file() { "🎵" } else { "▶️" }
    }
    /// Nhãn cho loại nguồn.
    pub fn source_label(&self) -> &'static str {
        if self.is_audio_file() { "File âm thanh" } else { "YouTube" }
    }
}

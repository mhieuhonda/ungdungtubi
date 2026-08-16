-- =====================================================================
-- Migration 026 — Giai đoạn 41 (v0.9.36): Community Group Logo + Audio File Uploads
--
-- Mục tiêu:
--   1. Cho phép nhóm cộng đồng đổi LOGO (icon đại diện, khác với ảnh bìa).
--      Hiện tại `groups.cover_upload_id` chỉ là ảnh bìa banner. Logo là ảnh
--      vuông nhỏ (256×256) hiển thị ở header nhóm, danh sách nhóm, v.v.
--
--   2. Hỗ trợ user upload file âm thanh (MP3/M4A/OGG/WAV) khi đăng nhạc
--      cộng đồng — bổ sung cho source YouTube hiện có. Theo tài liệu
--      "Hệ Thống Và Chức Năng Chi Tiết.docx" mục 3 (Nhà Nhạc):
--      "Cá nhân là danh sách nhạc do thành viên tải lên từ điện thoại
--       hoặc thêm từ kho nhạc miễn phí của hệ thống."
--
-- Thiết kế:
--   * groups.logo_upload_id UUID REFERENCES images(id) — link tới ảnh logo
--     (lưu trong bảng images hiện có, purpose='other', mime image/*).
--   * audio_files — bảng mới cho metadata file âm thanh (mime audio/*,
--     tách biệt với images vì có các field riêng: duration_seconds,
--     bitrate, v.v.). stored_filename = <uuid>.<ext>.
--   * user_music_submissions:
--       - source_type: 'youtube' (mặc định) hoặc 'audio_file'
--       - audio_file_upload_id: NULL cho YouTube, link tới audio_files.id cho upload
--       - audio_duration_seconds: thời lượng ghi âm (giây)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS + ALTER TABLE ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại (nếu chưa có từ migration 025)
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Thêm cột logo_upload_id vào groups — logo riêng (khác cover_upload_id)
ALTER TABLE groups ADD COLUMN IF NOT EXISTS logo_upload_id UUID REFERENCES images(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_groups_logo_upload ON groups(logo_upload_id) WHERE logo_upload_id IS NOT NULL;
COMMENT ON COLUMN groups.logo_upload_id IS 'ID ảnh logo nhóm (vuông nhỏ, khác với cover_upload_id ảnh bìa banner)';

-- 2. Tạo bảng audio_files — metadata file âm thanh do user upload
CREATE TABLE IF NOT EXISTS audio_files (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    uploader_id      UUID         REFERENCES users(id) ON DELETE SET NULL,
    original_name    VARCHAR(255) NOT NULL,
    stored_filename  VARCHAR(255) NOT NULL UNIQUE,
    mime_type        VARCHAR(100) NOT NULL,
    size_bytes       BIGINT       NOT NULL,
    sha256           VARCHAR(64)  NOT NULL,
    duration_seconds INT,
    -- 'music_submission' | 'personal_track' | 'other'
    purpose          VARCHAR(50)  NOT NULL DEFAULT 'other',
    is_public        BOOLEAN      NOT NULL DEFAULT true,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audio_files_uploader  ON audio_files(uploader_id);
CREATE INDEX IF NOT EXISTS idx_audio_files_purpose   ON audio_files(purpose);
CREATE INDEX IF NOT EXISTS idx_audio_files_sha256    ON audio_files(sha256);

COMMENT ON TABLE audio_files IS 'File âm thanh user upload (đăng nhạc cộng đồng, nhạc cá nhân). Tối đa 20MB/file.';
COMMENT ON COLUMN audio_files.stored_filename IS 'Tên file trên filesystem = <uuid>.<ext>';
COMMENT ON COLUMN audio_files.duration_seconds IS 'Thời lượng âm thanh (giây) — NULL nếu không parse được';

-- 3. Thêm cột source_type + audio_file_upload_id + audio_duration_seconds vào user_music_submissions
ALTER TABLE user_music_submissions
    ADD COLUMN IF NOT EXISTS source_type TEXT NOT NULL DEFAULT 'youtube'
        CHECK (source_type IN ('youtube', 'audio_file')),
    ADD COLUMN IF NOT EXISTS audio_file_upload_id UUID REFERENCES audio_files(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS audio_duration_seconds INT;

CREATE INDEX IF NOT EXISTS idx_music_submissions_source_type ON user_music_submissions(source_type);
CREATE INDEX IF NOT EXISTS idx_music_submissions_audio_file ON user_music_submissions(audio_file_upload_id) WHERE audio_file_upload_id IS NOT NULL;

COMMENT ON COLUMN user_music_submissions.source_type IS 'youtube (link) hoặc audio_file (upload trực tiếp)';
COMMENT ON COLUMN user_music_submissions.audio_file_upload_id IS 'NULL cho YouTube, link tới audio_files.id cho upload';

-- 4. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '026', 'v0.9.36 — Giai đoạn 41: Community group logo upload + audio file uploads (MP3/M4A/OGG/WAV) for music submissions.'
) ON CONFLICT (version) DO NOTHING;

-- =====================================================================
-- Migration 025 — Giai đoạn 40: User Music Submissions
-- v0.9.35 — 2026-08-17
--
-- Mục tiêu:
--   Users submit YouTube music links, admin approves/rejects.
--   When playing, YouTube video opens INLINE (embedded iframe).
--
-- Thiết kế:
--   user_music_submissions — user-submitted music (YouTube links)
--     * status: pending / approved / rejected
--     * Admin review: reviewed_by, review_note, reviewed_at
--     * play_count for analytics
--     * Rate limit: max 5 submissions per user per day (app-level)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS — chạy lại không lỗi.
-- =====================================================================

-- 1. Bảng user_music_submissions — nhạc do user gửi (YouTube link)
CREATE TABLE IF NOT EXISTS user_music_submissions (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT            NOT NULL,                    -- Song title (required)
    artist          TEXT            NOT NULL DEFAULT '',         -- Artist name (required)
    category        TEXT            NOT NULL CHECK (category IN ('niem', 'thien', 'dao', 'khong_loi')),
    youtube_url     TEXT            NOT NULL,                    -- Full YouTube URL
    youtube_id      TEXT            NOT NULL,                    -- Extracted video ID (11 chars)
    description     TEXT            DEFAULT '',                  -- Optional description
    status          TEXT            NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    reviewed_by     UUID            REFERENCES users(id),       -- Admin who reviewed
    review_note     TEXT,                                        -- Admin note on approval/rejection
    reviewed_at     TIMESTAMPTZ,                                 -- When reviewed
    play_count      BIGINT          NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_music_submissions_user ON user_music_submissions(user_id);
CREATE INDEX IF NOT EXISTS idx_music_submissions_status ON user_music_submissions(status);
CREATE INDEX IF NOT EXISTS idx_music_submissions_category ON user_music_submissions(category);
CREATE INDEX IF NOT EXISTS idx_music_submissions_youtube_id ON user_music_submissions(youtube_id);

-- Migration log
INSERT INTO migration_log (version, description) VALUES (
    '025', 'v0.9.35 — User music submissions (YouTube links, admin approval).'
) ON CONFLICT (version) DO NOTHING;

-- =====================================================================
-- Migration 029 — Giai đoạn 45: Hoàn thiện Admin Moderation + Từ vựng cấm
-- v0.9.41 — 2026-08-17
--
-- Mục tiêu:
--   1. Tạo bảng `forbidden_words` — admin quản lý từ vựng cấm. Khi user
--      đăng bình luận / chủ đề / chat, server tự động kiểm tra và block /
--      flag nội dung chứa từ cấm.
--   2. Thêm cột `comments.is_pinned` + `comments.is_locked` — admin/mod
--      có thể ghim bình luận (hiển thị đầu topic) hoặc khoá (không cho
--      trả lời nhánh đó).
--   3. Thêm cột `comments.moderation_status` + `comments.moderated_by` +
--      `comments.moderated_at` — track trạng thái kiểm duyệt.
--   4. Thêm cột `groups.is_featured` + `groups.moderation_status` — admin
--      có thể feature nhóm (hiển thị đầu danh sách) và track kiểm duyệt.
--   5. Thêm cột `topics.moderation_status` — track kiểm duyệt chủ đề.
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ════════════════════════════════════════════════════════════════════════════
-- 1. Bảng forbidden_words — Từ vựng cấm
-- ════════════════════════════════════════════════════════════════════════════
-- Admin thêm các từ/cụm từ cấm. Khi user submit content (comment, topic,
-- chat message, mail), server kiểm tra nội dung có chứa từ cấm không.
-- Nếu có: tùy cấu hình `action` mà block (không lưu) hoặc flag (lưu nhưng
-- đánh dấu `moderation_status = 'pending'` để admin review).
CREATE TABLE IF NOT EXISTS forbidden_words (
    id              BIGSERIAL    PRIMARY KEY,
    -- Từ/cụm từ cấm (lowercase, so sánh ILIKE khi check)
    word            TEXT         NOT NULL UNIQUE,
    -- 'block' = chặn không cho đăng, 'flag' = đăng nhưng flag để admin review
    action          VARCHAR(10)  NOT NULL DEFAULT 'block'
                    CHECK (action IN ('block', 'flag')),
    -- Phân loại để admin dễ quản lý
    -- 'profanity' = tục tĩu, 'spam' = spam, 'politics' = chính trị nhạy cảm,
    -- 'religious' = xúc phạm tôn giáo, 'scam' = lừa đảo, 'other' = khác
    category        VARCHAR(20)  NOT NULL DEFAULT 'other'
                    CHECK (category IN ('profanity', 'spam', 'politics', 'religious', 'scam', 'other')),
    -- Mô t lý do cấm (hiển thị cho admin)
    reason          TEXT,
    -- Admin tạo / system seed
    is_system       BOOLEAN      NOT NULL DEFAULT false,
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    created_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forbidden_words_active ON forbidden_words(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_forbidden_words_category ON forbidden_words(category);

-- Seed một số từ cấm mặc định (is_system = true, không xóa được)
INSERT INTO forbidden_words (word, action, category, reason, is_system) VALUES
    ('địt',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
    ('lồn',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
    ('cặc',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
    ('buồi',      'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
    ('mẹ mày',    'block', 'profanity', 'Cụm từ xúc phạm — tự động cấm', true),
    ('đĩ',        'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
    ('chó chết',  'block', 'profanity', 'Cụm từ xúc phạm — tự động cấm', true),
    ('scam',      'flag',  'scam',      'Keyword lừa đảo — flag để admin review', true),
    ('lừa đảo',   'flag',  'scam',      'Keyword lừa đảo — flag để admin review', true)
ON CONFLICT (word) DO NOTHING;

-- ════════════════════════════════════════════════════════════════════════════
-- 2. Cột mới cho comments — ghim + khoá + moderation status
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE comments ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE comments ADD COLUMN IF NOT EXISTS is_locked BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
    CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_comments_pinned ON comments(topic_id, is_pinned) WHERE is_pinned = true;
CREATE INDEX IF NOT EXISTS idx_comments_moderation ON comments(moderation_status)
    WHERE moderation_status != 'approved';
CREATE INDEX IF NOT EXISTS idx_comments_active ON comments(topic_id, is_active, created_at DESC);

-- ════════════════════════════════════════════════════════════════════════════
-- 3. Cột mới cho groups — feature + moderation status
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE groups ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
    CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_groups_featured ON groups(is_featured, created_at DESC) WHERE is_featured = true;
CREATE INDEX IF NOT EXISTS idx_groups_moderation ON groups(moderation_status)
    WHERE moderation_status != 'approved';

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Cột mới cho topics — moderation status
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
    CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_topics_moderation ON topics(moderation_status)
    WHERE moderation_status != 'approved';

-- ════════════════════════════════════════════════════════════════════════════
-- 5. Trigger updated_at cho forbidden_words
-- ════════════════════════════════════════════════════════════════════════════
DROP TRIGGER IF EXISTS trg_forbidden_words_set_updated_at ON forbidden_words;
CREATE TRIGGER trg_forbidden_words_set_updated_at
    BEFORE UPDATE ON forbidden_words
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 6. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '029', 'v0.9.41 — Giai đoạn 45: Admin moderation hoàn thiện (comments pin/lock/moderation_status, groups feature/moderation_status, topics moderation_status) + forbidden_words table + 9 seed words.'
) ON CONFLICT (version) DO NOTHING;

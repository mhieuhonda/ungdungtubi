-- Migration 049 — Giai đoạn 71: Nhật Ký Tu Học (Practice Diary)
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx mục I.2 (Nhà Nhật Ký):
--   Nhật Ký Tu Học: Tương tự trang cá nhân trên Facebook, là nơi thành viên ghi
--   lại bút ký và cảm ngộ trong quá trình tu học. Có thể cài công khai hoặc
--   riêng tư. Nếu cho phép bình luận thì người khác có thể bình luận (mặc định
--   chỉ bạn bè được bình luận).
--
-- Giai đoạn 71: thành viên viết bút ký tu học, có thể công khai/riêng tư,
-- có thể bật bình luận, người khác có thể xem và bình luận.

-- Bảng nhật ký tu học — mỗi entry là một bài bút ký/cảm ngộ
CREATE TABLE IF NOT EXISTS practice_diaries (
    id              BIGSERIAL    PRIMARY KEY,
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(200) NOT NULL,
    content         TEXT         NOT NULL,
    mood            VARCHAR(20)  NOT NULL DEFAULT 'peace',  -- 'peace'|'joy'|'gratitude'|'repentance'|'dedication'|'reflection'
    is_public       BOOLEAN      NOT NULL DEFAULT true,
    allow_comments  BOOLEAN      NOT NULL DEFAULT true,
    view_count      INTEGER      NOT NULL DEFAULT 0,
    comment_count   INTEGER      NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_practice_diaries_user ON practice_diaries(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_practice_diaries_public_recent ON practice_diaries(created_at DESC) WHERE is_public = true;

-- Bình luận cho nhật ký tu học
CREATE TABLE IF NOT EXISTS diary_comments (
    id              BIGSERIAL    PRIMARY KEY,
    diary_id        BIGINT       NOT NULL REFERENCES practice_diaries(id) ON DELETE CASCADE,
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT         NOT NULL,
    is_hidden       BOOLEAN      NOT NULL DEFAULT false,  -- admin/mod có thể ẩn
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_diary_comments_diary ON diary_comments(diary_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_diary_comments_user ON diary_comments(user_id, created_at DESC);

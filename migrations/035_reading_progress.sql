-- =====================================================================
-- Migration 035 — Giai đoạn 56: Tiến Độ Đọc Sách + Bookmark
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Theo "ỨNG DỤNG TỪ BI.docx" mục I.4 (Kinh Sách):
--     Thành viên có thể đọc sách trực tuyến + tải sách ngoại tuyến.
--   Giai đoạn 56: theo dõi tiến độ đọc chương (mục cuối đọc) + bookmark chương
--     + tổng thời gian đọc + số chương đã đọc.
--   Khi user quay lại sách → "Tiếp tục đọc từ chương X" tự động.
--
-- Note: books.id và book_chapters.id đều là UUID (xem migration 012).
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Bảng reading_progress — tiến độ đọc của user cho từng sách
CREATE TABLE IF NOT EXISTS reading_progress (
    id                  BIGSERIAL       PRIMARY KEY,
    user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id             UUID            NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    -- Chương cuối đọc (UUID — book_chapters.id là UUID)
    last_chapter_id     UUID,
    -- % đọc (0-100, dựa trên số chương đã đọc / tổng số chương)
    progress_percent    SMALLINT        NOT NULL DEFAULT 0
                                    CHECK (progress_percent >= 0 AND progress_percent <= 100),
    -- Vị trí cuộn (pixel offset) trong lần đọc cuối
    scroll_position     INTEGER         NOT NULL DEFAULT 0,
    -- Tổng thời gian đọc (giây)
    total_reading_seconds BIGINT       NOT NULL DEFAULT 0,
    -- Số chương đã đọc
    chapters_read       BIGINT          NOT NULL DEFAULT 0,
    -- Lần đọc cuối
    last_read_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    -- Timestamps
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, book_id)
);

CREATE INDEX IF NOT EXISTS idx_reading_progress_user_last
    ON reading_progress(user_id, last_read_at DESC);
CREATE INDEX IF NOT EXISTS idx_reading_progress_book
    ON reading_progress(book_id);

COMMENT ON TABLE reading_progress IS 'Tiến độ đọc sách của user — lưu chương cuối, % đọc, thời gian đọc.';

-- 2. Bảng chapter_bookmarks — bookmark chương (đánh dấu đọc sau)
CREATE TABLE IF NOT EXISTS chapter_bookmarks (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id         UUID            NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_id      UUID            NOT NULL REFERENCES book_chapters(id) ON DELETE CASCADE,
    -- Ghi chú của user khi bookmark (optional)
    note            TEXT            NOT NULL DEFAULT '',
    -- Label/màu (vd: 'important', 'review_later', 'favorite')
    label           VARCHAR(30)     NOT NULL DEFAULT 'bookmark',
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, chapter_id)
);

CREATE INDEX IF NOT EXISTS idx_chapter_bookmarks_user_book
    ON chapter_bookmarks(user_id, book_id);
CREATE INDEX IF NOT EXISTS idx_chapter_bookmarks_user_created
    ON chapter_bookmarks(user_id, created_at DESC);

COMMENT ON TABLE chapter_bookmarks IS 'Bookmark chương — user có thể đánh dấu chương để đọc lại sau.';

-- 3. Trigger updated_at
DROP TRIGGER IF EXISTS trg_reading_progress_set_updated_at ON reading_progress;
CREATE TRIGGER trg_reading_progress_set_updated_at
    BEFORE UPDATE ON reading_progress
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 4. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '035', 'v0.9.45 — Giai đoạn 56: Tiến Độ Đọc Sách + Bookmark — reading_progress + chapter_bookmarks.'
) ON CONFLICT (version) DO NOTHING;

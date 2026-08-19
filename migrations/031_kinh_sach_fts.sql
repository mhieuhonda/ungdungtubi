-- =====================================================================
-- Migration 031 — Giai đoạn 51: Kinh Sách FTS (tsvector + GIN) for fast search
-- v0.9.44 — 2026-09-15
--
-- Mục tiêu:
--   Thay thế tìm kiếm ILIKE (chậm, không rank theo relevance) bằng PostgreSQL
--   Full-Text Search dùng:
--     * `search_tsv tsvector` column trên `books` và `book_chapters`
--     * GIN index để fast lookup
--     * `ts_rank_cd(search_tsv, plainto_tsquery('simple', $1))` để rank kết quả
--     * Trigger tự cập nhật `search_tsv` khi INSERT/UPDATE
--   Thêm bảng `user_search_history` để ghi lại 10 lần tìm kiếm gần nhất của user
--   (hiển thị dưới dạng clickable chips trên trang tìm kiếm).
--
-- Lưu ý:
--   * Dùng `to_tsvector('simple', ...)` — tiếng Việt không có stemming engine built-in
--     trong PostgreSQL, nhưng FTS vẫn giúp với word boundary detection.
--   * `plainto_tsquery` an toàn với ký tự đặc biệt (không cần escape user input).
--
-- Idempotent: tất cả ADD COLUMN IF NOT EXISTS / CREATE INDEX IF NOT EXISTS /
-- CREATE OR REPLACE FUNCTION / DROP TRIGGER IF EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ════════════════════════════════════════════════════════════════════════════
-- 1. Thêm cột search_tsv (tsvector) vào books và book_chapters
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE books ADD COLUMN IF NOT EXISTS search_tsv tsvector;
ALTER TABLE book_chapters ADD COLUMN IF NOT EXISTS search_tsv tsvector;

-- ════════════════════════════════════════════════════════════════════════════
-- 2. Populate dữ liệu hiện có (chạy 1 lần — trigger sẽ giữ dữ liệu mới nhất)
-- ════════════════════════════════════════════════════════════════════════════
-- books: title + description + author
UPDATE books
SET search_tsv = to_tsvector('simple',
    coalesce(title, '') || ' ' || coalesce(description, '') || ' ' || coalesce(author, '')
);

-- book_chapters: title + content
UPDATE book_chapters
SET search_tsv = to_tsvector('simple',
    coalesce(title, '') || ' ' || coalesce(content, '')
);

-- ════════════════════════════════════════════════════════════════════════════
-- 3. Tạo GIN indexes (cho fast full-text search)
-- ════════════════════════════════════════════════════════════════════════════
CREATE INDEX IF NOT EXISTS idx_books_search_tsv ON books USING gin(search_tsv);
CREATE INDEX IF NOT EXISTS idx_book_chapters_search_tsv ON book_chapters USING gin(search_tsv);

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Trigger functions + triggers để tự cập nhật search_tsv khi INSERT/UPDATE
-- ════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE FUNCTION books_search_tsv_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_tsv := to_tsvector('simple',
        coalesce(NEW.title, '') || ' ' || coalesce(NEW.description, '') || ' ' || coalesce(NEW.author, '')
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_books_search_tsv ON books;
CREATE TRIGGER trg_books_search_tsv
    BEFORE INSERT OR UPDATE OF title, description, author ON books
    FOR EACH ROW EXECUTE FUNCTION books_search_tsv_update();

CREATE OR REPLACE FUNCTION book_chapters_search_tsv_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_tsv := to_tsvector('simple',
        coalesce(NEW.title, '') || ' ' || coalesce(NEW.content, '')
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_book_chapters_search_tsv ON book_chapters;
CREATE TRIGGER trg_book_chapters_search_tsv
    BEFORE INSERT OR UPDATE OF title, content ON book_chapters
    FOR EACH ROW EXECUTE FUNCTION book_chapters_search_tsv_update();

-- ════════════════════════════════════════════════════════════════════════════
-- 5. Bảng user_search_history — 10 lần tìm kiếm gần nhất của user
-- ════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS user_search_history (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    query        TEXT         NOT NULL,
    searched_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_search_history_user_time
    ON user_search_history(user_id, searched_at DESC);

COMMENT ON TABLE user_search_history IS 'Lịch sử tìm kiếm Kinh Sách của user — hiển thị 10 chip gần nhất trên trang /kinh-sach/tim-kiem';

-- ════════════════════════════════════════════════════════════════════════════
-- 6. Migration log
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO migration_log (version, description) VALUES (
    '031', 'v0.9.44 — Kinh Sách FTS (tsvector + GIN) for fast search'
) ON CONFLICT (version) DO NOTHING;

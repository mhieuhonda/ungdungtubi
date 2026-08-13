-- Ứng Dụng Từ Bi - Migration 004: Storage ảnh + Audit log
-- Giai đoạn 5 (v0.5): Hạ tầng deploy + lưu trữ ảnh (tối đa 5MB/ảnh)
--
-- Mục tiêu:
--   * Tạo bảng images để lưu metadata ảnh user upload (avatar, ảnh bài viết, v.v.)
--   * Tạo bảng audit_log cho mọi giao dịch quan trọng (tiền tệ, quyền, xóa dữ liệu)
--   * Thêm cột avatar_upload_id vào users (ảnh avatar do user tự upload,
--     ưu tiên trước avatar_url từ Google)
--   * Tạo index cho tra cứu nhanh

-- 1. Bảng images — metadata file ảnh do user upload
CREATE TABLE IF NOT EXISTS images (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    -- UUID của file trên filesystem (ten file luu = <id>.<ext>)
    uploader_id     UUID         REFERENCES users(id) ON DELETE SET NULL,
    original_name   VARCHAR(255) NOT NULL,
    stored_filename VARCHAR(255) NOT NULL UNIQUE,
    mime_type       VARCHAR(100) NOT NULL,
    size_bytes      BIGINT       NOT NULL,
    sha256          VARCHAR(64)  NOT NULL,
    width           INT,
    height          INT,
    -- 'avatar' | 'post' | 'comment' | 'message' | 'other'
    purpose         VARCHAR(50)  NOT NULL DEFAULT 'other',
    is_public       BOOLEAN      NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_images_uploader  ON images(uploader_id);
CREATE INDEX IF NOT EXISTS idx_images_purpose   ON images(purpose);
CREATE INDEX IF NOT EXISTS idx_images_sha256    ON images(sha256);

-- 2. Cột avatar_upload_id — link tới ảnh avatar do user tự upload
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_upload_id UUID REFERENCES images(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_users_avatar_upload ON users(avatar_upload_id) WHERE avatar_upload_id IS NOT NULL;

-- 3. Bảng audit_log — append-only, ghi lại mọi giao dịch quan trọng
-- Dùng cho: giao dịch A/I/K/Bi, thay đổi quyền user, xóa dữ liệu, admin actions
CREATE TABLE IF NOT EXISTS audit_log (
    id              BIGSERIAL    PRIMARY KEY,
    actor_id        UUID         REFERENCES users(id) ON DELETE SET NULL,
    action          VARCHAR(100) NOT NULL,
    -- 'transaction' | 'permission' | 'admin' | 'auth' | 'upload' | 'delete'
    category        VARCHAR(50)  NOT NULL,
    -- JSONB chứa context cụ thể (amount, target_user, before/after, ...)
    details         JSONB        NOT NULL DEFAULT '{}'::jsonb,
    ip_address      VARCHAR(45),
    user_agent      VARCHAR(500),
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_log_actor    ON audit_log(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_category ON audit_log(category);
CREATE INDEX IF NOT EXISTS idx_audit_log_action   ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_created   ON audit_log(created_at DESC);

-- 4. Comments
COMMENT ON TABLE images IS 'Ảnh user upload (avatar, bài viết, bình luận). Giới hạn 5MB/ảnh.';
COMMENT ON COLUMN images.stored_filename IS 'Tên file trên filesystem = <uuid>.<ext>';
COMMENT ON COLUMN images.sha256 IS 'Checksum SHA-256 để chống trùng lặp';
COMMENT ON COLUMN images.purpose IS 'Mục đích: avatar | post | comment | message | other';
COMMENT ON COLUMN users.avatar_upload_id IS 'ID ảnh avatar user tự upload (ưu tiên trước Google avatar_url)';

COMMENT ON TABLE audit_log IS 'Nhật ký giao dịch — append-only, không xoá.';
COMMENT ON COLUMN audit_log.category IS 'transaction | permission | admin | auth | upload | delete';
COMMENT ON COLUMN audit_log.details IS 'JSONB context: {amount, currency, target_user_id, before, after, ...}';

-- 5. Trigger: tự động cập nhật updated_at khi UPDATE users
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_users_set_updated_at ON users;
CREATE TRIGGER trg_users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- Ứng Dụng Từ Bi - Migration 002: Google OAuth
-- Giai đoạn 3 (v0.3): Chuyển sang đăng nhập/đăng ký duy nhất bằng Google
--
-- Mục tiêu:
--   * Cho phép password_hash NULL (người dùng Google không có mật khẩu)
--   * Thêm cột google_sub (Google unique user ID) làm định danh OAuth
--   * Thêm avatar_url, email_verified lấy từ Google userinfo
--   * Giữ lại tài khoản cũ đã đăng ký bằng email/password (không xoá dữ liệu)

-- 1. Cho phép password_hash NULL (người dùng Google không có mật khẩu)
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- 2. Thêm các cột OAuth
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_sub VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url VARCHAR(500);
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT false;

-- 3. google_sub là duy nhất — mỗi tài khoản Google chỉ映射 về 1 user
ALTER TABLE users ADD CONSTRAINT uq_users_google_sub UNIQUE (google_sub);

-- 4. Index tra cứu nhanh theo google_sub
CREATE INDEX IF NOT EXISTS idx_users_google_sub ON users(google_sub) WHERE google_sub IS NOT NULL;

-- 5. Cập nhật comment
COMMENT ON COLUMN users.password_hash IS 'Mật khẩu Argon2 (chỉ dùng cho tài khoản cũ email/password). NULL đối với người dùng Google-only.';
COMMENT ON COLUMN users.google_sub IS 'Google unique user ID (sub claim). Dùng cho đăng nhập Google OAuth.';
COMMENT ON COLUMN users.avatar_url IS 'URL ảnh đại diện từ Google';
COMMENT ON COLUMN users.email_verified IS 'Trạng thái xác thực email từ Google';

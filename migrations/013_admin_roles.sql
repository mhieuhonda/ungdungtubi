-- Ứng Dụng Từ Bi - Migration 013: Hệ thống vai trò Admin & Phân quyền
-- Giai đoạn 11 (v0.9.7): Hệ thống vai trò Admin + Phân quyền cộng đồng
--
-- Mục tiêu:
--   * Thêm cột `role` vào bảng users (mặc định 'member')
--   * Thiết lập hệ thống phân quyền 4 cấp:
--       1. admin_quan_li   — Admin Quản Lý (quyền cao nhất — super admin)
--       2. admin_cong_dong — Admin Cộng Đồng (quản trị cộng đồng, duyệt nội dung)
--       3. admin_ky_thuat  — Admin Kỹ Thuật (hệ thống, server, database, mã nguồn)
--       4. member          — Thành Viên (người dùng thông thường)
--   * Tự động gán khongdich.admin@gmail.com làm admin_ky_thuat
--   * Index trên cột role để tra cứu nhanh khi kiểm tra quyền

-- 1. Thêm cột role vào bảng users
ALTER TABLE users ADD COLUMN IF NOT EXISTS role VARCHAR(30) NOT NULL DEFAULT 'member';

-- 2. Comment cho cột role
COMMENT ON COLUMN users.role IS 'Vai trò: member | admin_ky_thuat | admin_cong_dong | admin_quan_li';

-- 3. CHECK constraint — chỉ cho phép 4 giá trị hợp lệ
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_role_check'
    ) THEN
        ALTER TABLE users
        ADD CONSTRAINT users_role_check
        CHECK (role IN ('member', 'admin_ky_thuat', 'admin_cong_dong', 'admin_quan_li'));
    END IF;
END$$;

-- 4. Index trên cột role — tối ưu cho truy vấn "list all admins"
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- 5. Tự động gán khongdich.admin@gmail.com làm admin_ky_thuat
--    (admin kỹ thuật do user yêu cầu — phụ trách hệ thống, server, DB, mã nguồn)
--    Dùng UPSERT: nếu user đã tồn tại thì UPDATE role, nếu chưa thì INSERT mới.
--    Note: password_hash NULL vì user này sẽ đăng nhập bằng Google OAuth.
INSERT INTO users (email, display_name, password_hash, rank, role, is_active, email_verified, created_at, updated_at)
VALUES (
    'khongdich.admin@gmail.com',
    'Admin Kỹ Thuật',
    NULL,
    'tycoon',           -- Đại Gia — cấp bậc cao nhất cho admin kỹ thuật
    'admin_ky_thuat',
    true,
    true,
    NOW(),
    NOW()
)
ON CONFLICT (email) DO UPDATE
SET role        = 'admin_ky_thuat',
    is_active   = true,
    updated_at  = NOW();

-- 6. Cập nhật comment cho bảng
COMMENT ON TABLE users IS 'Thành viên Ứng Dụng Từ Bi — kèm vai trò quản trị (role)';

-- 7. Seed note
-- Hierarchy:
--   admin_quan_li (cấp 4 — cao nhất) > admin_cong_dong (cấp 3) > admin_ky_thuat (cấp 2) > member (cấp 1)
-- Mọi kiểm tra quyền trong code nên dùng helper `user.role_level() >= N` để so sánh.

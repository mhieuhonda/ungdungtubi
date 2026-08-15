-- Ứng Dụng Từ Bi - Migration 020: Thêm chức vụ Mod (v0.9.19 — Giai đoạn 24)
--
-- Mục tiêu:
--   * Thêm role 'mod' vào hệ thống — dưới admin, trên thành viên
--   * Mod có quyền quản trị cơ bản:
--       - Xem danh sách thành viên (/admin/thanh-vien)
--       - Duyệt cảm ngộ (/admin/cong-dong/cam-ngo)
--       - Xem các trang placeholder quản trị (/admin/cong-dong/nhom, /admin/kinh-sach, ...)
--       - Chat trong BẤT KỲ nhóm nào (không cần membership)
--       - Hiển thị badge 📜 Mod trong chat và profile
--   * Mod KHÔNG có quyền:
--       - Đổi role user (chỉ admin_ky_thuat + admin_quan_li)
--       - Ban user (chỉ admin_ky_thuat)
--       - Truy cập 3 dashboard admin riêng (ky-thuat/cong-dong/quan-li)
--
-- Hierarchy mới: admin_ky_thuat (5) > admin_quan_li (4) > admin_cong_dong (3) > mod (2) > member (1)

-- 1. Drop old CHECK constraint (chỉ cho phép 4 giá trị cũ: member, admin_ky_thuat, admin_cong_dong, admin_quan_li)
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;

-- 2. Add new CHECK constraint cho phép 5 giá trị (thêm 'mod')
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_role_check'
    ) THEN
        ALTER TABLE users
        ADD CONSTRAINT users_role_check
        CHECK (role IN ('member', 'mod', 'admin_ky_thuat', 'admin_cong_dong', 'admin_quan_li'));
    END IF;
END$$;

-- 3. Update comment cho cột role
COMMENT ON COLUMN users.role IS 'Vai trò: member | mod | admin_ky_thuat | admin_cong_dong | admin_quan_li';

-- 4. Ghi chú hierarchy mới
-- Hierarchy (v0.9.19):
--   admin_ky_thuat (5 — cao nhất) > admin_quan_li (4) > admin_cong_dong (3) > mod (2) > member (1)
-- Mọi kiểm tra quyền trong code nên dùng helper:
--   - user.is_admin()     — true cho 3 role admin (KHÔNG bao gồm mod)
--   - user.is_mod()       — true chỉ cho mod
--   - user.is_staff()     — true cho admin HOẶC mod (dùng cho các quyền cơ bản)
--   - user.role_level()   — so sánh số (1-5)

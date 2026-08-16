-- ════════════════════════════════════════════════════════════════════════════
-- Ứng Dụng Từ Bi - Migration 022: Thêm chức vụ "Admin Phát Triển" (admin_phat_trien)
-- Giai đoạn 35 (v0.9.30)
--
-- Mục tiêu:
--   * Thêm role `admin_phat_trien` vào hệ thống — Admin Phát Triển
--     (phụ trách định hướng phát triển sản phẩm, CI/CD, roadmap, kỹ thuật xây dựng)
--   * Admin Phát Triển NGANG HÀNH với 3 admin kia (cùng cấp 3) — không phân cấp
--   * Scope quyền: system + development + deployment + analytics + navigation
--     (giao thoa với admin_ky_thuat nhưng tập trung vào "phát triển sản phẩm")
--
-- Triết lý:
--   v0.9.29 đã đổi Võ Đăng Trọng Nghĩa từ "Admin Phát Triển" (role không tồn tại)
--   sang "Admin Cộng Đồng" vì role `admin_phat_trien` chưa có trong code.
--   v0.9.30 CHÍNH THỨC thêm role này vào hệ thống — đáp ứng yêu cầu thực tế:
--   Võ Đăng Trọng Nghĩa phụ trách mảng phát triển sản phẩm, định hướng roadmap,
--   nên cần một role riêng phản ánh đúng vai trò "Admin Phát Triển".
--
--   Nguyên tắc ngang hàng vẫn giữ: 4 admin (Kỹ Thuật · Quản Lí · Cộng Đồng ·
--   Phát Triển) đều ở cấp 3, không ai cao hơn ai — mỗi người phụ trách một mảng.
--
--   Võ Đăng Trọng Nghĩa — cập nhật thông tin:
--     - Chức vụ: Admin Phát Triển (đúng với phụ trách thực tế)
--     - Tôn giáo: Không (cập nhật theo yêu cầu — không theo Phật giáo)
-- ════════════════════════════════════════════════════════════════════════════

-- 1. Drop old CHECK constraint (5 giá trị: member, mod, admin_ky_thuat, admin_cong_dong, admin_quan_li)
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;

-- 2. Add new CHECK constraint cho phép 6 giá trị (thêm 'admin_phat_trien')
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_role_check'
    ) THEN
        ALTER TABLE users
        ADD CONSTRAINT users_role_check
        CHECK (role IN (
            'member',
            'mod',
            'admin_ky_thuat',
            'admin_cong_dong',
            'admin_quan_li',
            'admin_phat_trien'
        ));
    END IF;
END$$;

-- 3. Update comment cho cột role
COMMENT ON COLUMN users.role IS 'Vai trò: member | mod | admin_ky_thuat | admin_cong_dong | admin_quan_li | admin_phat_trien (v0.9.30)';

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Seed role_permissions cho admin_phat_trien
--    Scope: system + development + deployment + analytics + navigation + api
--    (35 quyền — tập trung vào phát triển sản phẩm và hạ tầng kỹ thuật)
--
--    Admin Phát Triển có quyền:
--      - System (10): xem status, config, migrate, logs, cache, metrics, backup, debug
--      - Users (7): xem list/detail/sessions, change_role, activate, export_data
--      - Security (5): audit, login_log, session_revoke, spam_filter, report_manage
--      - Media (5): view_all, view_storage, delete_any, moderate, restore
--      - Analytics (6): dashboard, user_stats, content_stats, revenue, export_reports, realtime
--      - Navigation (5): edit_announce, manage_home, edit_meta, view_settings_log, manage_features
--      - API (1): manage_keys
--    Tổng: 39 quyền
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_phat_trien', code FROM permissions
WHERE code IN (
    -- System (10) — toàn quyền hệ thống (giao thoa với admin_ky_thuat)
    'system_view_status', 'system_manage_config', 'system_manage_migrate',
    'system_view_logs', 'system_manage_cache', 'system_restart_server',
    'system_manage_cron', 'system_view_metrics', 'system_manage_backup',
    'system_debug_mode',
    -- Users (7) — xem + đổi role + kỹ thuật (như admin_ky_thuat)
    'users_view_list', 'users_view_detail', 'users_view_sessions',
    'users_change_role', 'users_activate', 'users_ban', 'users_export_data',
    -- Security (5) — chuyên môn kỹ thuật
    'sec_view_audit', 'sec_view_login_log', 'sec_session_revoke',
    'sec_spam_filter', 'sec_report_manage',
    -- Media (5) — technical storage
    'media_view_all', 'media_view_storage', 'media_delete_any',
    'media_moderate', 'media_restore',
    -- Analytics (6) — theo dõi phát triển sản phẩm
    'an_view_dashboard', 'an_view_user_stats', 'an_view_content_stats',
    'an_view_revenue', 'an_export_reports', 'an_view_realtime',
    -- Navigation (5) — technical config (định hướng UI/UX phát triển)
    'nav_edit_announce', 'nav_manage_home', 'nav_edit_meta',
    'nav_view_settings_log', 'nav_manage_features',
    -- API keys
    'api_manage_keys'
)
ON CONFLICT (role, permission_code) DO NOTHING;

-- ════════════════════════════════════════════════════════════════════════════
-- 5. Update view v_user_permissions (đảm bảo view hoạt động với role mới)
-- ════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE VIEW v_user_permissions AS
SELECT
    u.id AS user_id,
    u.role,
    p.code AS permission_code,
    p.name_vi AS permission_name,
    p.category AS permission_category
FROM users u
JOIN role_permissions rp ON rp.role = u.role
JOIN permissions p ON p.code = rp.permission_code;

COMMENT ON VIEW v_user_permissions IS 'Quyền chi tiết của mỗi user — admin ngang hàng + admin_phat_trien (v0.9.30)';

-- ════════════════════════════════════════════════════════════════════════════
-- 6. Ghi chú hệ thống vai trò mới (v0.9.30)
-- ════════════════════════════════════════════════════════════════════════════
COMMENT ON TABLE role_permissions IS 'Gán quyền chi tiết cho role — admin ngang hàng (v0.9.30): 4 admin (Kỹ Thuật · Quản Lí · Cộng Đồng · Phát Triển) đều cấp 3, mỗi admin có scope riêng, không ai cao hơn ai';

-- Hierarchy (v0.9.30):
--   admin_ky_thuat   (cấp 3 — 41 quyền) — Hệ thống · Bảo mật · Infrastructure
--   admin_quan_li    (cấp 3 — 40 quyền) — Thành viên · Nội dung · Quỹ
--   admin_cong_dong  (cấp 3 — 45 quyền) — Cộng đồng · Sự kiện · Media
--   admin_phat_trien (cấp 3 — 39 quyền) — Phát triển · System · Analytics · Navigation
--   mod              (cấp 2 — 15 quyền) — Kiểm duyệt cơ bản
--   member           (cấp 1 —  0 quyền) — Người dùng thường
--
-- Mọi kiểm tra quyền trong code nên dùng helper:
--   - user.is_admin()          — true cho 4 role admin (KHÔNG bao gồm mod)
--   - user.is_mod()            — true chỉ cho mod
--   - user.is_staff()          — true cho admin HOẶC mod
--   - user.role_level()        — so sánh số (1-3)
--   - user.has_permission_code(code) — kiểm tra quyền chi tiết theo scope

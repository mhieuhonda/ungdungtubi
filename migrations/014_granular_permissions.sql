-- Ứng Dụng Từ Bi - Migration 014: Hệ thống 50 quyền chi tiết (Granular Permissions)
-- Giai đoạn 12 (v0.9.8): Nâng Admin Kỹ Thuật lên chức vụ cao nhất, toàn bộ 50 quyền
--
-- Mục tiêu:
--   * Tạo bảng `permissions` — 50 quyền chi tiết chia 5 nhóm
--   * Tạo bảng `role_permissions` — gán quyền cho từng role
--   * Cập nhật hierarchy: admin_ky_thuat (cấp 4 — cao nhất)
--   * admin_ky_thuat có TẤT CẢ 50 quyền
--   * admin_quan_li có 30 quyền quản trị (không có quyền kỹ thuật hệ thống)
--   * admin_cong_dong có 20 quyền cộng đồng (không có quyền user/system)
--   * member có 0 quyền admin (chỉ quyền cơ bản)

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Bảng permissions — 50 quyền chi tiết
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS permissions (
    id SERIAL PRIMARY KEY,
    code VARCHAR(60) NOT NULL UNIQUE,
    name_vi VARCHAR(200) NOT NULL,
    description_vi TEXT,
    category VARCHAR(30) NOT NULL,  -- system | users | content | community | kinh_sach
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE permissions IS '50 quyền chi tiết cho hệ thống phân quyền admin';
COMMENT ON COLUMN permissions.category IS 'Nhóm quyền: system | users | content | community | kinh_sach';

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Seed 50 permissions — 5 nhóm x 10 quyền
-- ══════════════════════════════════════════════════════════════════════════════

-- Nhóm 1: System & Infrastructure (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('system_view_status',    'Xem trạng thái hệ thống',    'Xem health check, server status, DB status', 'system', 1),
    ('system_manage_config',  'Quản lý cấu hình hệ thống',  'Thay đổi config, env variables, settings', 'system', 2),
    ('system_manage_migrate', 'Chạy database migrations',   'Thực hiện migrate, rollback schema changes', 'system', 3),
    ('system_view_logs',      'Xem log hệ thống',           'Đọc application logs, access logs, error logs', 'system', 4),
    ('system_manage_cache',   'Quản lý cache',              'Xóa cache, warm cache, cấu hình cache strategy', 'system', 5),
    ('system_restart_server', 'Khởi động lại server',       'Restart application server, graceful shutdown', 'system', 6),
    ('system_manage_cron',    'Quản lý cron jobs',          'Tạo/sửa/xóa scheduled tasks, cron jobs', 'system', 7),
    ('system_view_metrics',   'Xem metrics & performance',  'CPU, RAM, DB connections, response times, throughput', 'system', 8),
    ('system_manage_backup',  'Quản lý backup',             'Tạo/restore/xóa database backups, cấu hình backup schedule', 'system', 9),
    ('system_debug_mode',     'Bật debug mode',             'Kích hoạt debug mode, verbose logging, query profiling', 'system', 10);

-- Nhóm 2: User Management (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('users_view_list',       'Xem danh sách thành viên',     'Truy cập trang /admin/thanh-vien', 'users', 11),
    ('users_view_detail',     'Xem chi tiết thành viên',      'Xem hồ sơ đầy đủ của bất kỳ user nào', 'users', 12),
    ('users_edit_profile',    'Sửa hồ sơ thành viên khác',    'Chỉnh sửa tên, bio, pháp danh của user khác', 'users', 13),
    ('users_change_role',     'Đổi vai trò user',             'Thay đổi role của user (member ↔ admin)', 'users', 14),
    ('users_activate',        'Kích hoạt/vô hiệu user',       'Bật/tắt is_active của tài khoản', 'users', 15),
    ('users_delete',          'Xóa user',                     'Xóa vĩnh viễn tài khoản và dữ liệu liên quan', 'users', 16),
    ('users_ban',             'Cấm user',                     'Ban/kick user, đặt thời gian cấm, lý do cấm', 'users', 17),
    ('users_view_sessions',   'Xem sessions user',            'Xem active sessions, revoke sessions', 'users', 18),
    ('users_manage_oauth',    'Quản lý OAuth settings',       'Cấu hình Google OAuth, thêm/xóa providers', 'users', 19),
    ('users_export_data',     'Xuất dữ liệu user',            'Export CSV/JSON danh sách user, backup data', 'users', 20);

-- Nhóm 3: Content Management (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('content_view_pending',  'Xem nội dung chờ duyệt',       'Xem reviews/comments đang chờ approve', 'content', 21),
    ('content_approve',       'Duyệt/từ chối nội dung',       'Approve hoặc reject reviews, comments', 'content', 22),
    ('content_edit_any',      'Sửa bất kỳ nội dung nào',      'Chỉnh sửa topic/comment/review của user khác', 'content', 23),
    ('content_delete_any',    'Xóa bất kỳ nội dung nào',       'Xóa topic/comment/review/image của user khác', 'content', 24),
    ('content_pin_lock',      'Ghim/khoá chủ đề',             'Pin/unpin, lock/unlock topics', 'content', 25),
    ('content_manage_cat',    'Quản lý danh mục',              'Tạo/sửa/xóa group categories, book categories', 'content', 26),
    ('content_manage_tags',   'Quản lý tags',                  'Tạo/sửa/xóa tags, merge tags', 'content', 27),
    ('content_mod_comments',  'Kiểm duyệt bình luận',         'Mod/moderate comments, hide spam', 'content', 28),
    ('content_mod_reviews',   'Kiểm duyệt cảm ngộ',           'Duyệt/từ chối book reviews', 'content', 29),
    ('content_feature',       'Đề cử/nổi bật nội dung',       'Feature/unfeature content, homepage spotlight', 'content', 30);

-- Nhóm 4: Community Management (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('community_view_stats',  'Xem thống kê cộng đồng',        'Xem group stats, member activity, engagement', 'community', 31),
    ('community_manage_grp',  'Quản lý nhóm',                  'Tạo/sửa/xóa groups, đổi group settings', 'community', 32),
    ('community_create_off',  'Tạo nhóm chính thức',           'Tạo official/verified groups', 'community', 33),
    ('community_manage_evt',  'Quản lý sự kiện',               'Tạo/sửa/xóa events, cộng tu schedules', 'community', 34),
    ('community_manage_chat', 'Quản lý live chat',             'Mod chat, clear history, mute users trong chat', 'community', 35),
    ('community_manage_mem',  'Quản lý thành viên nhóm',       'Approve/reject join requests, remove members', 'community', 36),
    ('community_broadcast',   'Gửi thông báo toàn nhóm',       'Broadcast message to all members', 'community', 37),
    ('community_manage_inv',  'Quản lý lời mời',               'Tạo/revoke group invitations', 'community', 38),
    ('community_archive',     'Lưu trữ nhóm/chủ đề',           'Archive/unarchive groups và topics', 'community', 39),
    ('community_merge',       'Gộp chủ đề',                    'Merge topics, move topics giữa groups', 'community', 40);

-- Nhóm 5: Kinh Sách & Advanced (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('ksach_manage_books',    'Quản lý sách',                  'Thêm/sửa/xóa books, đổi book metadata', 'kinh_sach', 41),
    ('ksach_manage_chap',     'Quản lý chương sách',           'Thêm/sửa/xóa chapters, sắp xếp thứ tự', 'kinh_sach', 42),
    ('ksach_upload',          'Upload nội dung sách',          'Upload book files, cover images', 'kinh_sach', 43),
    ('ksach_manage_cat',      'Quản lý thư mục Kinh Sách',     'Tạo/sửa/xóa book categories, sort order', 'kinh_sach', 44),
    ('ksach_review_mod',      'Kiểm duyệt cảm ngộ sách',       'Approve/reject book reviews', 'kinh_sach', 45),
    ('ksach_donation_mgr',    'Quản lý quyên góp sách',        'Quản lý book donations, flower counts', 'kinh_sach', 46),
    ('mail_view_all',         'Xem thư của tất cả user',       'Truy cập inbox/sent của bất kỳ user nào', 'kinh_sach', 47),
    ('notif_send_all',        'Gửi thông báo toàn hệ thống',   'Send system notification to all users', 'kinh_sach', 48),
    ('analytics_view',        'Xem phân tích dữ liệu',         'View dashboards, charts, usage analytics', 'kinh_sach', 49),
    ('api_manage_keys',       'Quản lý API keys',              'Tạo/revoke API keys, rate limiting', 'kinh_sach', 50);

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Bảng role_permissions — gán quyền cho role
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS role_permissions (
    role VARCHAR(30) NOT NULL,
    permission_code VARCHAR(60) NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role, permission_code)
);

COMMENT ON TABLE role_permissions IS 'Gán quyền chi tiết cho từng role admin';

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. Seed role_permissions
-- ══════════════════════════════════════════════════════════════════════════════
--
-- Hierarchy MỚI (v0.9.8):
--   admin_ky_thuat  (cấp 4 — CAO NHẤT) → TẤT CẢ 50 quyền
--   admin_quan_li   (cấp 3) → 30 quyền quản trị (không quyền kỹ thuật nguy hiểm)
--   admin_cong_dong (cấp 2) → 20 quyền cộng đồng + content
--   member          (cấp 1) → 0 quyền admin
--

-- 4a. Admin Kỹ Thuật — TẤT CẢ 50 quyền (toàn quyền hệ thống)
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_ky_thuat', code FROM permissions;

-- 4b. Admin Quản Lý — 30 quyền quản trị
--     Có: users (10) + content (10) + community (10)
--     Không có: system (quyền nguy hiểm) + kinh_sach (quyền kỹ thuật)
INSERT INTO role_permissions (role, permission_code) VALUES
    -- Users (10/10)
    ('admin_quan_li', 'users_view_list'),
    ('admin_quan_li', 'users_view_detail'),
    ('admin_quan_li', 'users_edit_profile'),
    ('admin_quan_li', 'users_change_role'),
    ('admin_quan_li', 'users_activate'),
    ('admin_quan_li', 'users_delete'),
    ('admin_quan_li', 'users_ban'),
    ('admin_quan_li', 'users_view_sessions'),
    ('admin_quan_li', 'users_manage_oauth'),
    ('admin_quan_li', 'users_export_data'),
    -- Content (10/10)
    ('admin_quan_li', 'content_view_pending'),
    ('admin_quan_li', 'content_approve'),
    ('admin_quan_li', 'content_edit_any'),
    ('admin_quan_li', 'content_delete_any'),
    ('admin_quan_li', 'content_pin_lock'),
    ('admin_quan_li', 'content_manage_cat'),
    ('admin_quan_li', 'content_manage_tags'),
    ('admin_quan_li', 'content_mod_comments'),
    ('admin_quan_li', 'content_mod_reviews'),
    ('admin_quan_li', 'content_feature'),
    -- Community (10/10)
    ('admin_quan_li', 'community_view_stats'),
    ('admin_quan_li', 'community_manage_grp'),
    ('admin_quan_li', 'community_create_off'),
    ('admin_quan_li', 'community_manage_evt'),
    ('admin_quan_li', 'community_manage_chat'),
    ('admin_quan_li', 'community_manage_mem'),
    ('admin_quan_li', 'community_broadcast'),
    ('admin_quan_li', 'community_manage_inv'),
    ('admin_quan_li', 'community_archive'),
    ('admin_quan_li', 'community_merge');

-- 4c. Admin Cộng Đồng — 20 quyền cộng đồng + content
--     Có: content (10) + community (10)
--     Không có: system + users + kinh_sach
INSERT INTO role_permissions (role, permission_code) VALUES
    -- Content (10/10)
    ('admin_cong_dong', 'content_view_pending'),
    ('admin_cong_dong', 'content_approve'),
    ('admin_cong_dong', 'content_edit_any'),
    ('admin_cong_dong', 'content_delete_any'),
    ('admin_cong_dong', 'content_pin_lock'),
    ('admin_cong_dong', 'content_manage_cat'),
    ('admin_cong_dong', 'content_manage_tags'),
    ('admin_cong_dong', 'content_mod_comments'),
    ('admin_cong_dong', 'content_mod_reviews'),
    ('admin_cong_dong', 'content_feature'),
    -- Community (10/10)
    ('admin_cong_dong', 'community_view_stats'),
    ('admin_cong_dong', 'community_manage_grp'),
    ('admin_cong_dong', 'community_create_off'),
    ('admin_cong_dong', 'community_manage_evt'),
    ('admin_cong_dong', 'community_manage_chat'),
    ('admin_cong_dong', 'community_manage_mem'),
    ('admin_cong_dong', 'community_broadcast'),
    ('admin_cong_dong', 'community_manage_inv'),
    ('admin_cong_dong', 'community_archive'),
    ('admin_cong_dong', 'community_merge');

-- ══════════════════════════════════════════════════════════════════════════════
-- 5. Indexes
-- ══════════════════════════════════════════════════════════════════════════════
CREATE INDEX IF NOT EXISTS idx_permissions_category ON permissions(category);
CREATE INDEX IF NOT EXISTS idx_permissions_code ON permissions(code);
CREATE INDEX IF NOT EXISTS idx_role_permissions_role ON role_permissions(role);

-- ══════════════════════════════════════════════════════════════════════════════
-- 6. Cập nhật UPSERT admin_ky_thuat — đảm bảo khongdich.admin@gmail.com
--    vẫn là admin_ky_thuat (không đổi role, chỉ nâng cấp quyền)
-- ══════════════════════════════════════════════════════════════════════════════
-- Note: role đã được set ở migration 013, không cần UPDATE lại
-- Chỉ cần đảm bảo user tồn tại và active
UPDATE users SET is_active = true, updated_at = NOW()
WHERE email = 'khongdich.admin@gmail.com';

-- ══════════════════════════════════════════════════════════════════════════════
-- 7. View: quyền của user (join role_permissions + permissions)
-- ══════════════════════════════════════════════════════════════════════════════
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

COMMENT ON VIEW v_user_permissions IS 'Quyền chi tiết của mỗi user — join users + role_permissions + permissions';

-- ══════════════════════════════════════════════════════════════════════════════
-- 8. Function: kiểm tra user có quyền cụ thể
-- ══════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE FUNCTION user_has_permission(p_user_id UUID, p_permission_code VARCHAR)
RETURNS BOOLEAN LANGUAGE sql STABLE AS
$$
    SELECT EXISTS (
        SELECT 1
        FROM users u
        JOIN role_permissions rp ON rp.role = u.role
        WHERE u.id = p_user_id
          AND rp.permission_code = p_permission_code
    );
$$;

COMMENT ON FUNCTION user_has_permission IS 'Kiểm tra nhanh user có quyền cụ thể không — dùng trong code và SQL';

-- ════════════════════════════════════════════════════════════════════════════
-- Ứng Dụng Từ Bi - Migration 021: Redesign phân quyền — Admin ngang hàng
-- Giai đoạn 29 (v0.9.24)
--
-- Mục tiêu:
--   * Bỏ hệ thống phân cấp cũ: admin_ky_thuat(5) > admin_quan_li(4) > admin_cong_dong(3)
--   * Áp dụng nguyên tắc MỚI: tất cả admin NGANG HÀNG nhau (cùng level 3),
--     mỗi admin có quyền khác nhau theo phần mình phụ trách.
--   * admin_ky_thuat: system + security + technical infrastructure (40 quyền)
--   * admin_quan_li:  users + content + community + fund (40 quyền)
--   * admin_cong_dong: content + community + friends + mail + events (45 quyền)
--   * mod: basic moderation (15 quyền)
--   * member: 0 quyền admin
--
-- Triết lý:
--   "Các admin đều bằng nhau ngang hàng, nhưng mỗi người phụ trách một mảng
--    khác nhau. Không ai cao hơn ai — quyền hạn được phân theo lĩnh vực."
--
-- Lợi ích:
--   - Tránh 1 admin có toàn quyền quá mức (single point of failure)
--   - Mỗi admin chỉ có quyền trong scope mình phụ trách → an toàn hơn
--   - Mod được giữ ở level 2 (dưới admin, trên member)
--   - Cột role trên users giữ nguyên — chỉ role_permissions thay đổi
-- ════════════════════════════════════════════════════════════════════════════

-- 1. Xoá toàn bộ role_permissions cũ để re-seed
TRUNCATE TABLE role_permissions;

-- 2. Re-seed role_permissions theo phân quyền mới (admin ngang hàng)

-- ════════════════════════════════════════════════════════════════════════════
-- 2a. admin_ky_thuat — Phụ trách KỸ THUẬT (40 quyền)
--     Scope: system, infrastructure, security, audit, technical users
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_ky_thuat', code FROM permissions
WHERE code IN (
    -- System (10) — toàn quyền hệ thống
    'system_view_status', 'system_manage_config', 'system_manage_migrate',
    'system_view_logs', 'system_manage_cache', 'system_restart_server',
    'system_manage_cron', 'system_view_metrics', 'system_manage_backup',
    'system_debug_mode',
    -- Users — chỉ xem + kỹ thuật (không change_role — đó là job của admin_quan_li)
    'users_view_list', 'users_view_detail', 'users_view_sessions',
    'users_activate', 'users_ban', 'users_export_data',
    -- Security (toàn quyền — đây là chuyên môn của admin_ky_thuat)
    'sec_view_audit', 'sec_view_login_log', 'sec_session_revoke',
    'sec_spam_filter', 'sec_report_manage',
    -- Media — technical storage
    'media_view_all', 'media_view_storage', 'media_delete_any',
    'media_moderate', 'media_restore',
    -- Analytics — technical metrics
    'an_view_dashboard', 'an_view_user_stats', 'an_view_content_stats',
    'an_view_revenue', 'an_export_reports', 'an_view_realtime',
    -- Navigation — technical config
    'nav_edit_announce', 'nav_manage_home', 'nav_edit_meta',
    'nav_view_settings_log', 'nav_manage_features',
    -- API keys
    'api_manage_keys'
);

-- ════════════════════════════════════════════════════════════════════════════
-- 2b. admin_quan_li — Phụ trách QUẢN LÝ (40 quyền)
--     Scope: user management, content moderation, community management, fund
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_quan_li', code FROM permissions
WHERE code IN (
    -- Users (10) — quản lý thành viên đầy đủ (bao gồm change_role)
    'users_view_list', 'users_view_detail', 'users_edit_profile',
    'users_change_role', 'users_activate', 'users_delete',
    'users_ban', 'users_view_sessions', 'users_manage_oauth', 'users_export_data',
    -- Content (10) — kiểm duyệt nội dung
    'content_view_pending', 'content_approve', 'content_edit_any',
    'content_delete_any', 'content_pin_lock', 'content_manage_cat',
    'content_manage_tags', 'content_mod_comments', 'content_mod_reviews', 'content_feature',
    -- Community (10) — quản lý cộng đồng
    'community_view_stats', 'community_manage_grp', 'community_create_off',
    'community_manage_evt', 'community_manage_chat', 'community_manage_mem',
    'community_broadcast', 'community_manage_inv', 'community_archive', 'community_merge',
    -- Fund (5) — quản lý quỹ từ bi
    'fund_view_all', 'fund_approve', 'fund_view_anonymous',
    'fund_audit_log', 'fund_export',
    -- Mail/Notif (5) — thông báo hệ thống
    'mail_view_all', 'notif_send_all', 'mail_broadcast',
    'notif_template', 'mail_view_queue'
);

-- ════════════════════════════════════════════════════════════════════════════
-- 2c. admin_cong_dong — Phụ trách CỘNG ĐỒNG (45 quyền)
--     Scope: content, community, friends, mail, events, achievements
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_cong_dong', code FROM permissions
WHERE code IN (
    -- Content (10) — kiểm duyệt nội dung cộng đồng
    'content_view_pending', 'content_approve', 'content_edit_any',
    'content_delete_any', 'content_pin_lock', 'content_manage_cat',
    'content_manage_tags', 'content_mod_comments', 'content_mod_reviews', 'content_feature',
    -- Community (10) — quản lý cộng đồng
    'community_view_stats', 'community_manage_grp', 'community_create_off',
    'community_manage_evt', 'community_manage_chat', 'community_manage_mem',
    'community_broadcast', 'community_manage_inv', 'community_archive', 'community_merge',
    -- Friends (5) — quản lý kết bạn
    'fr_view_all_friends', 'fr_view_all_dm', 'fr_delete_message',
    'fr_view_dm_reports', 'fr_manage_groups',
    -- Mail (5) — quản lý thư
    'mail_view_all', 'mail_delete_any', 'mail_broadcast',
    'mail_view_queue', 'mail_manage_filters',
    -- Events (5) — quản lý sự kiện cộng tu
    'evt_create', 'evt_edit_any', 'evt_manage_attendance',
    'evt_broadcast', 'evt_view_stats',
    -- Achievements (5) — quản lý thành tích
    'ach_view_all', 'ach_view_progress', 'ach_view_history',
    'ach_grant', 'ach_export',
    -- Media moderation (5)
    'media_view_all', 'media_approve', 'media_moderate',
    'media_delete_any', 'media_view_storage'
);

-- ════════════════════════════════════════════════════════════════════════════
-- 2d. mod — Moderator cơ bản (15 quyền)
--     Scope: content moderation, chat moderation, basic community
-- ════════════════════════════════════════════════════════════════════════════
INSERT INTO role_permissions (role, permission_code)
SELECT 'mod', code FROM permissions
WHERE code IN (
    -- Content moderation (5)
    'content_view_pending', 'content_approve', 'content_mod_comments',
    'content_mod_reviews', 'content_pin_lock',
    -- Community moderation (5)
    'community_view_stats', 'community_manage_chat', 'community_manage_mem',
    'community_broadcast', 'community_archive',
    -- Friends/DM moderation (3)
    'fr_view_dm_reports', 'fr_delete_message', 'fr_manage_groups',
    -- Security reporting (2)
    'sec_view_audit', 'sec_report_manage'
);

-- ════════════════════════════════════════════════════════════════════════════
-- 3. Cập nhật view v_user_permissions (đảm bảo view hoạt động với data mới)
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

COMMENT ON VIEW v_user_permissions IS 'Quyền chi tiết của mỗi user — admin ngang hàng (v0.9.24)';

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Thêm cột csrf_token vào sessions (cho v0.9.24 security hardening)
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS csrf_token VARCHAR(64) UNIQUE;

-- Backfill csrf_token cho sessions cũ (random UUID-based hex)
UPDATE sessions
SET csrf_token = encode(gen_random_bytes(32), 'hex')
WHERE csrf_token IS NULL;

-- Đảm bảo NOT NULL sau khi backfill
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_name = 'sessions' AND column_name = 'csrf_token'
               AND is_nullable = 'YES') THEN
        -- Chỉ set NOT NULL nếu không còn row nào NULL
        IF NOT EXISTS (SELECT 1 FROM sessions WHERE csrf_token IS NULL) THEN
            ALTER TABLE sessions ALTER COLUMN csrf_token SET NOT NULL;
        END IF;
    END IF;
END $$;

-- ════════════════════════════════════════════════════════════════════════════
-- 5. Thêm cột last_login_ip + last_login_at vào users (cho audit log)
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_ip VARCHAR(45);
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- ════════════════════════════════════════════════════════════════════════════
-- 6. Thêm cột ip_address vào audit_log (nếu chưa có)
-- ════════════════════════════════════════════════════════════════════════════
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'audit_log' AND column_name = 'ip_address') THEN
        ALTER TABLE audit_log ADD COLUMN ip_address VARCHAR(45);
    END IF;
END $$;

-- ════════════════════════════════════════════════════════════════════════════
-- 7. Tạo bảng rate_limit_log (cho security hardening — track failed logins)
-- ════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS rate_limit_log (
    id           BIGSERIAL    PRIMARY KEY,
    ip_address   VARCHAR(45)  NOT NULL,
    endpoint     VARCHAR(200) NOT NULL,
    hit_count    INT          NOT NULL DEFAULT 1,
    last_hit_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    blocked_until TIMESTAMPTZ,
    UNIQUE (ip_address, endpoint)
);

CREATE INDEX IF NOT EXISTS idx_rate_limit_ip ON rate_limit_log(ip_address);
CREATE INDEX IF NOT EXISTS idx_rate_limit_blocked ON rate_limit_log(blocked_until);

-- ════════════════════════════════════════════════════════════════════════════
-- 8. Tạo bảng login_attempts (chống brute-force OAuth)
-- ════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS login_attempts (
    id            BIGSERIAL    PRIMARY KEY,
    ip_address    VARCHAR(45)  NOT NULL,
    email         VARCHAR(200),
    success       BOOLEAN      NOT NULL,
    attempted_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    user_agent    TEXT
);

CREATE INDEX IF NOT EXISTS idx_login_attempts_ip ON login_attempts(ip_address, attempted_at);
CREATE INDEX IF NOT EXISTS idx_login_attempts_email ON login_attempts(email, attempted_at);

-- ════════════════════════════════════════════════════════════════════════════
-- 9. Update comment cho permissions table
-- ════════════════════════════════════════════════════════════════════════════
COMMENT ON TABLE role_permissions IS 'Gán quyền chi tiết cho role — admin ngang hàng (v0.9.24): mỗi admin có scope riêng, không ai cao hơn ai';

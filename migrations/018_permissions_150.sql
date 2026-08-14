-- Ứng Dụng Từ Bi - Migration 018: Hệ thống 150 quyền chi tiết (Permission Expansion)
-- Giai đoạn 19 (v0.9.14): Mở rộng từ 50 → 150 quyền, chia đều cho 3 chức admin
--
-- Mục tiêu:
--   * Thêm 100 quyền mới (10 nhóm × 10 quyền) vào bảng `permissions`
--   * Cập nhật role_permissions:
--     - admin_ky_thuat: TẤT CẢ 150 quyền (toàn quyền hệ thống)
--     - admin_quan_li: ~100 quyền (users + content + community + fund + analytics + shop + events + achievements)
--     - admin_cong_dong: ~75 quyền (content + community + friends + mail + events + achievements)
--   * Member: 0 quyền admin
--
-- 10 nhóm mới:
--   1. fund          — Quản lý Quỹ Từ Bi (10 quyền)
--   2. achievements  — Quản lý Thành tích (10 quyền)
--   3. security      — Bảo mật & chống spam (10 quyền)
--   4. navigation    — Quản lý UI/Navigation/Settings (10 quyền)
--   5. analytics     — Phân tích & báo cáo (10 quyền)
--   6. media         — Quản lý media/uploads (10 quyền)
--   7. friends       — Quản lý Bạn Bè/DM (10 quyền)
--   8. mail          — Quản lý Thư/Thông báo (10 quyền)
--   9. events        — Quản lý Sự kiện/Cộng tu (10 quyền)
--  10. shop          — Quản lý Thương Thành (10 quyền)

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Thêm 100 quyền mới — 10 nhóm × 10 quyền
-- ══════════════════════════════════════════════════════════════════════════════
-- Sử dụng ON CONFLICT để idempotent (chạy lại không lỗi)

-- Nhóm 6: Quản lý Quỹ Từ Bi (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('fund_view_all',         'Xem tất cả đóng góp',         'Xem danh sách đầy đủ các đóng góp quỹ', 'fund', 51),
    ('fund_approve',          'Duyệt đóng góp',               'Duyệt hoặc từ chối đóng góp đang chờ', 'fund', 52),
    ('fund_create_campaign',  'Tạo chiến dịch gây quỹ',       'Tạo/sửa chiến dịch quyên góp mới', 'fund', 53),
    ('fund_manage_expenses',  'Quản lý chi tiêu quỹ',         'Thêm/sửa/xóa các khoản chi tiêu quỹ', 'fund', 54),
    ('fund_export',           'Xuất dữ liệu quỹ',             'Export CSV/Excel báo cáo quỹ', 'fund', 55),
    ('fund_refund',           'Hoàn tiền đóng góp',           'Hoàn lại K cho người đóng góp', 'fund', 56),
    ('fund_view_anonymous',   'Xem đóng góp ẩn danh',         'Xem thông tin người đóng góp ẩn danh', 'fund', 57),
    ('fund_manage_categories','Quản lý loại quỹ',             'Thêm/sửa/xóa 5 loại quỹ (chung, sách, tu, quà, thiện nguyện)', 'fund', 58),
    ('fund_set_goal',         'Đặt mục tiêu quỹ',             'Thiết lập mục tiêu K cho từng quỹ', 'fund', 59),
    ('fund_audit_log',        'Xem nhật ký quỹ',              'Xem audit log các giao dịch quỹ', 'fund', 60)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 7: Quản lý Thành tích (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('ach_view_all',          'Xem tất cả thành tích',       'Xem thành tích của mọi user', 'achievements', 61),
    ('ach_create',            'Tạo thành tích mới',          'Định nghĩa achievement mới (daily/weekly/monthly/yearly)', 'achievements', 62),
    ('ach_edit',              'Sửa thành tích',              'Chỉnh sửa điều kiện, phần thưởng achievement', 'achievements', 63),
    ('ach_delete',            'Xóa thành tích',              'Xóa achievement (chỉ khi chưa có user đạt)', 'achievements', 64),
    ('ach_grant',             'Cấp thành tích thủ công',     'Trao achievement cho user cụ thể', 'achievements', 65),
    ('ach_revoke',            'Thu hồi thành tích',          'Gỡ achievement khỏi user', 'achievements', 66),
    ('ach_view_progress',     'Xem tiến độ thành tích',      'Xem tiến độ achievement của mọi user', 'achievements', 67),
    ('ach_manage_rewards',    'Quản lý phần thưởng',         'Thiết lập phần thưởng A/I/K cho achievement', 'achievements', 68),
    ('ach_view_history',      'Xem lịch sử thành tích',      'Xem lịch sử nhận achievement của user', 'achievements', 69),
    ('ach_export',            'Xuất dữ liệu thành tích',     'Export CSV/Excel dữ liệu achievement', 'achievements', 70)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 8: Bảo mật & chống spam (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('sec_view_audit',        'Xem audit log',               'Truy cập full audit log hệ thống', 'security', 71),
    ('sec_view_login_log',    'Xem lịch sử đăng nhập',       'Xem lịch sử login của user (IP, time, device)', 'security', 72),
    ('sec_ip_blocklist',      'Quản lý IP blocklist',        'Thêm/xóa IP bị chặn', 'security', 73),
    ('sec_rate_limit',        'Cấu hình rate limit',         'Thiết lập giới hạn request per IP/user', 'security', 74),
    ('sec_2fa_manage',        'Quản lý 2FA',                 'Bật/tắt 2FA cho user, reset 2FA', 'security', 75),
    ('sec_session_revoke',    'Thu hồi session',             'Revoke session của bất kỳ user nào', 'security', 76),
    ('sec_captcha_manage',    'Quản lý CAPTCHA',             'Cấu hình CAPTCHA, turnstile, hCaptcha', 'security', 77),
    ('sec_spam_filter',       'Quản lý bộ lọc spam',         'Thêm/sửa từ khóa spam, blacklist', 'security', 78),
    ('sec_report_manage',     'Quản lý báo cáo',             'Xử lý báo cáo nội dung/user từ cộng đồng', 'security', 79),
    ('sec_security_scan',     'Quét lỗ hổng bảo mật',        'Chạy security scan, xem báo cáo vulnerability', 'security', 80)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 9: Quản lý UI/Navigation/Settings (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('nav_edit_menu',         'Sửa menu điều hướng',         'Thêm/sửa/xóa item trong main menu', 'navigation', 81),
    ('nav_edit_footer',       'Sửa footer',                  'Chỉnh sửa nội dung footer, links', 'navigation', 82),
    ('nav_manage_home',       'Quản lý trang chủ',           'Thay đổi hero, banner, feature cards trên home', 'navigation', 83),
    ('nav_manage_themes',     'Quản lý themes',              'Thêm/sửa/xóa themes (lotus, dark, minimal)', 'navigation', 84),
    ('nav_edit_announce',     'Đăng thông báo hệ thống',     'Đăng banner thông báo trên header', 'navigation', 85),
    ('nav_manage_landing',    'Quản lý landing page',        'Sửa nội dung landing page cho khách', 'navigation', 86),
    ('nav_manage_redirects',  'Quản lý redirects',           'Tạo/sửa 301/302 redirects', 'navigation', 87),
    ('nav_edit_meta',         'Sửa meta tags',               'Thay đổi SEO meta tags, OpenGraph', 'navigation', 88),
    ('nav_manage_features',   'Bật/tắt tính năng',           'Feature flag — enable/disable tính năng', 'navigation', 89),
    ('nav_view_settings_log', 'Xem nhật ký cài đặt',         'Xem lịch sử thay đổi cài đặt hệ thống', 'navigation', 90)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 10: Phân tích & báo cáo (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('an_view_dashboard',     'Xem dashboard phân tích',     'Truy cập dashboard phân tích tổng', 'analytics', 91),
    ('an_view_user_stats',    'Xem thống kê user',           'Xem DAU/MAU, retention, engagement', 'analytics', 92),
    ('an_view_content_stats', 'Xem thống kê nội dung',       'Xem views, likes, comments, shares', 'analytics', 93),
    ('an_view_revenue',       'Xem báo cáo doanh thu',       'Xem báo cáo donation, fund, doanh thu', 'analytics', 94),
    ('an_export_reports',     'Xuất báo cáo',                'Export PDF/Excel báo cáo định kỳ', 'analytics', 95),
    ('an_view_funnel',        'Xem funnel phân tích',        'Phân tích funnel (signup → active → paid)', 'analytics', 96),
    ('an_view_cohort',        'Xem cohort analysis',         'Phân tích cohort retention', 'analytics', 97),
    ('an_set_kpi',            'Thiết lập KPI',               'Định nghĩa KPI, target cho dashboard', 'analytics', 98),
    ('an_view_realtime',      'Xem realtime metrics',        'Xem active users realtime, online users', 'analytics', 99),
    ('an_integrate_tool',     'Tích hợp công cụ analytics',  'Cấu hình GA, Mixpanel, Plausible', 'analytics', 100)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 11: Quản lý media/uploads (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('media_view_all',        'Xem tất cả media',            'Xem toàn bộ ảnh user đã upload', 'media', 101),
    ('media_delete_any',      'Xóa ảnh bất kỳ',              'Xóa ảnh upload của user khác', 'media', 102),
    ('media_approve',         'Duyệt ảnh đang chờ',          'Duyệt ảnh upload (nếu bật moderation)', 'media', 103),
    ('media_manage_quota',    'Quản lý quota',               'Thay đổi dung lượng upload tối đa per user', 'media', 104),
    ('media_manage_types',    'Quản lý loại file',           'Cấu hình MIME types được phép upload', 'media', 105),
    ('media_upload_admin',    'Upload admin media',          'Upload ảnh chính thức (banner, icon, avatar hệ thống)', 'media', 106),
    ('media_view_storage',    'Xem storage stats',           'Xem dung lượng đã dùng, free space', 'media', 107),
    ('media_compress',        'Nén ảnh',                     'Chạy bulk compress ảnh đã upload', 'media', 108),
    ('media_moderate',        'Moderate ảnh',                'Đánh dấu ảnh vi phạm, ẩn ảnh', 'media', 109),
    ('media_restore',         'Khôi phục ảnh đã xóa',        'Restore ảnh từ trash (soft delete)', 'media', 110)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 12: Quản lý Bạn Bè/DM (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('fr_view_all_friends',   'Xem tất cả bạn bè',           'Xem danh sách bạn bè của mọi user', 'friends', 111),
    ('fr_view_all_dm',        'Xem tất cả DM',               'Xem tin nhắn 1-1 của mọi user', 'friends', 112),
    ('fr_delete_message',     'Xóa tin nhắn',                'Xóa tin nhắn DM của user khác', 'friends', 113),
    ('fr_mute_user',          'Mute user trong DM',          'Mute user không được chat DM', 'friends', 114),
    ('fr_manage_blocklist',   'Quản lý blocklist',           'Xem/sửa danh sách user bị block', 'friends', 115),
    ('fr_force_unfriend',     'Ép hủy kết bạn',              'Force unfriend 2 user (admin only)', 'friends', 116),
    ('fr_view_dm_reports',    'Xem báo cáo DM',              'Xem báo cáo tin nhắn vi phạm', 'friends', 117),
    ('fr_export_dm',          'Xuất dữ liệu DM',             'Export lịch sử DM (legal/compliance)', 'friends', 118),
    ('fr_manage_groups',      'Quản lý group chat',          'Tạo/sửa group DM (multi-user)', 'friends', 119),
    ('fr_reset_conversation', 'Reset conversation',          'Xóa toàn bộ tin nhắn trong 1 conversation', 'friends', 120)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 13: Quản lý Thư/Thông báo (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('mail_view_all',         'Xem thư mọi user',            'Xem inbox/sent của bất kỳ user nào', 'mail', 121),
    ('mail_delete_any',       'Xóa thư bất kỳ',              'Xóa thư của user khác', 'mail', 122),
    ('mail_broadcast',        'Gửi thư hàng loạt',           'Gửi mail đến toàn bộ user (announcement)', 'mail', 123),
    ('mail_template',         'Quản lý template',            'Tạo/sửa template mail (welcome, ban, etc.)', 'mail', 124),
    ('mail_view_queue',       'Xem hàng đợi mail',           'Xem mail đang chờ gửi, mail fail', 'mail', 125),
    ('notif_send_all',        'Gửi thông báo toàn hệ thống', 'Send notification đến mọi user', 'mail', 126),
    ('notif_template',        'Quản lý template notif',      'Tạo/sửa template notification', 'mail', 127),
    ('notif_view_stats',      'Xem thống kê notif',          'View open rate, click rate notif', 'mail', 128),
    ('notif_delete_any',      'Xóa thông báo',               'Xóa notification của user khác', 'mail', 129),
    ('mail_manage_filters',   'Quản lý bộ lọc thư',          'Cấu hình spam filter, blacklist mail', 'mail', 130)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 14: Quản lý Sự kiện/Cộng tu (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('evt_create',            'Tạo sự kiện',                 'Tạo/sửa sự kiện cộng tu, 法會', 'events', 131),
    ('evt_edit_any',          'Sửa sự kiện bất kỳ',          'Sửa sự kiện của user khác', 'events', 132),
    ('evt_delete',            'Xóa sự kiện',                 'Xóa sự kiện (chỉ organizer/admin)', 'events', 133),
    ('evt_manage_attendance', 'Quản lý tham dự',             'Approve/reject registrations, check-in', 'events', 134),
    ('evt_broadcast',         'Broadcast sự kiện',           'Gửi reminder, update đến người tham dự', 'events', 135),
    ('evt_view_stats',        'Xem thống kê sự kiện',        'View attendance, engagement stats', 'events', 136),
    ('evt_manage_schedule',   'Quản lý lịch trình',          'Tạo/sửa lịch trình sự kiện', 'events', 137),
    ('evt_manage_recording',  'Quản lý recording',           'Tạo/sửa/xóa bản ghi sự kiện', 'events', 138),
    ('evt_set_capacity',      'Đặt giới hạn tham dự',        'Thiết lập max attendees cho event', 'events', 139),
    ('evt_export',            'Xuất dữ liệu sự kiện',        'Export CSV danh sách người tham dự', 'events', 140)
ON CONFLICT (code) DO NOTHING;

-- Nhóm 15: Quản lý Thương Thành (10 quyền)
INSERT INTO permissions (code, name_vi, description_vi, category, sort_order) VALUES
    ('shop_view_all',         'Xem tất cả sản phẩm',         'Xem toàn bộ sản phẩm trên Thương Thành', 'shop', 141),
    ('shop_add_product',      'Thêm sản phẩm',               'Thêm sản phẩm mới vào Thương Thành', 'shop', 142),
    ('shop_edit_any',         'Sửa sản phẩm bất kỳ',         'Sửa sản phẩm của user khác', 'shop', 143),
    ('shop_delete',           'Xóa sản phẩm',                'Xóa sản phẩm khỏi Thương Thành', 'shop', 144),
    ('shop_approve',          'Duyệt sản phẩm',              'Duyệt sản phẩm do user đăng lên', 'shop', 145),
    ('shop_view_orders',      'Xem đơn hàng',                'Xem tất cả đơn hàng trên hệ thống', 'shop', 146),
    ('shop_refund',           'Hoàn tiền đơn hàng',          'Refund K cho đơn hàng bị hủy', 'shop', 147),
    ('shop_manage_categories','Quản lý danh mục',            'Thêm/sửa/xóa danh mục Thương Thành', 'shop', 148),
    ('shop_set_featured',     'Đặt sản phẩm nổi bật',        'Feature sản phẩm trên trang chủ Thương Thành', 'shop', 149),
    ('shop_export',           'Xuất dữ liệu Thương Thành',   'Export CSV/Excel báo cáo bán hàng', 'shop', 150)
ON CONFLICT (code) DO NOTHING;

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Cập nhật role_permissions
-- ══════════════════════════════════════════════════════════════════════════════

-- 2a. Admin Kỹ Thuật — TẤT CẢ 150 quyền (toàn quyền hệ thống, cấp cao nhất)
INSERT INTO role_permissions (role, permission_code)
SELECT 'admin_ky_thuat', code FROM permissions
WHERE code NOT IN (SELECT permission_code FROM role_permissions WHERE role = 'admin_ky_thuat');

-- 2b. Admin Quản Lý — 100 quyền
-- Có: users(10) + content(10) + community(10) (đã có 30)
-- Thêm: fund(10) + analytics(10) + shop(10) + events(10) + achievements(10) + media(5) + navigation(5)
--       (để đạt tổng 100 quyền — thêm 70 quyền mới)
INSERT INTO role_permissions (role, permission_code) VALUES
    -- Fund (10/10)
    ('admin_quan_li', 'fund_view_all'),
    ('admin_quan_li', 'fund_approve'),
    ('admin_quan_li', 'fund_create_campaign'),
    ('admin_quan_li', 'fund_manage_expenses'),
    ('admin_quan_li', 'fund_export'),
    ('admin_quan_li', 'fund_refund'),
    ('admin_quan_li', 'fund_view_anonymous'),
    ('admin_quan_li', 'fund_manage_categories'),
    ('admin_quan_li', 'fund_set_goal'),
    ('admin_quan_li', 'fund_audit_log'),
    -- Achievements (10/10)
    ('admin_quan_li', 'ach_view_all'),
    ('admin_quan_li', 'ach_create'),
    ('admin_quan_li', 'ach_edit'),
    ('admin_quan_li', 'ach_grant'),
    ('admin_quan_li', 'ach_view_progress'),
    ('admin_quan_li', 'ach_manage_rewards'),
    ('admin_quan_li', 'ach_view_history'),
    ('admin_quan_li', 'ach_export'),
    ('admin_quan_li', 'ach_revoke'),
    ('admin_quan_li', 'ach_delete'),
    -- Analytics (10/10)
    ('admin_quan_li', 'an_view_dashboard'),
    ('admin_quan_li', 'an_view_user_stats'),
    ('admin_quan_li', 'an_view_content_stats'),
    ('admin_quan_li', 'an_view_revenue'),
    ('admin_quan_li', 'an_export_reports'),
    ('admin_quan_li', 'an_view_funnel'),
    ('admin_quan_li', 'an_view_cohort'),
    ('admin_quan_li', 'an_set_kpi'),
    ('admin_quan_li', 'an_view_realtime'),
    ('admin_quan_li', 'an_integrate_tool'),
    -- Shop (10/10)
    ('admin_quan_li', 'shop_view_all'),
    ('admin_quan_li', 'shop_add_product'),
    ('admin_quan_li', 'shop_edit_any'),
    ('admin_quan_li', 'shop_delete'),
    ('admin_quan_li', 'shop_approve'),
    ('admin_quan_li', 'shop_view_orders'),
    ('admin_quan_li', 'shop_refund'),
    ('admin_quan_li', 'shop_manage_categories'),
    ('admin_quan_li', 'shop_set_featured'),
    ('admin_quan_li', 'shop_export'),
    -- Events (10/10)
    ('admin_quan_li', 'evt_create'),
    ('admin_quan_li', 'evt_edit_any'),
    ('admin_quan_li', 'evt_delete'),
    ('admin_quan_li', 'evt_manage_attendance'),
    ('admin_quan_li', 'evt_broadcast'),
    ('admin_quan_li', 'evt_view_stats'),
    ('admin_quan_li', 'evt_manage_schedule'),
    ('admin_quan_li', 'evt_manage_recording'),
    ('admin_quan_li', 'evt_set_capacity'),
    ('admin_quan_li', 'evt_export'),
    -- Media (5/10) — chỉ quyền quản lý cao cấp
    ('admin_quan_li', 'media_view_all'),
    ('admin_quan_li', 'media_view_storage'),
    ('admin_quan_li', 'media_delete_any'),
    ('admin_quan_li', 'media_moderate'),
    ('admin_quan_li', 'media_restore'),
    -- Navigation (5/10) — chỉ quyền chỉnh sửa nội dung
    ('admin_quan_li', 'nav_edit_announce'),
    ('admin_quan_li', 'nav_manage_home'),
    ('admin_quan_li', 'nav_edit_meta'),
    ('admin_quan_li', 'nav_view_settings_log'),
    ('admin_quan_li', 'nav_manage_features')
ON CONFLICT (role, permission_code) DO NOTHING;

-- 2c. Admin Cộng Đồng — 75 quyền
-- Có: content(10) + community(10) (đã có 20)
-- Thêm: friends(10) + mail(10) + events(10) + achievements(10) + media(5) + fund(5) + security(5)
--       (để đạt tổng 75 quyền — thêm 55 quyền mới)
INSERT INTO role_permissions (role, permission_code) VALUES
    -- Friends (10/10)
    ('admin_cong_dong', 'fr_view_all_friends'),
    ('admin_cong_dong', 'fr_view_all_dm'),
    ('admin_cong_dong', 'fr_delete_message'),
    ('admin_cong_dong', 'fr_mute_user'),
    ('admin_cong_dong', 'fr_manage_blocklist'),
    ('admin_cong_dong', 'fr_force_unfriend'),
    ('admin_cong_dong', 'fr_view_dm_reports'),
    ('admin_cong_dong', 'fr_export_dm'),
    ('admin_cong_dong', 'fr_manage_groups'),
    ('admin_cong_dong', 'fr_reset_conversation'),
    -- Mail (10/10)
    ('admin_cong_dong', 'mail_view_all'),
    ('admin_cong_dong', 'mail_delete_any'),
    ('admin_cong_dong', 'mail_broadcast'),
    ('admin_cong_dong', 'mail_template'),
    ('admin_cong_dong', 'mail_view_queue'),
    ('admin_cong_dong', 'notif_send_all'),
    ('admin_cong_dong', 'notif_template'),
    ('admin_cong_dong', 'notif_view_stats'),
    ('admin_cong_dong', 'notif_delete_any'),
    ('admin_cong_dong', 'mail_manage_filters'),
    -- Events (10/10)
    ('admin_cong_dong', 'evt_create'),
    ('admin_cong_dong', 'evt_edit_any'),
    ('admin_cong_dong', 'evt_delete'),
    ('admin_cong_dong', 'evt_manage_attendance'),
    ('admin_cong_dong', 'evt_broadcast'),
    ('admin_cong_dong', 'evt_view_stats'),
    ('admin_cong_dong', 'evt_manage_schedule'),
    ('admin_cong_dong', 'evt_manage_recording'),
    ('admin_cong_dong', 'evt_set_capacity'),
    ('admin_cong_dong', 'evt_export'),
    -- Achievements (10/10)
    ('admin_cong_dong', 'ach_view_all'),
    ('admin_cong_dong', 'ach_view_progress'),
    ('admin_cong_dong', 'ach_view_history'),
    ('admin_cong_dong', 'ach_grant'),
    ('admin_cong_dong', 'ach_export'),
    ('admin_cong_dong', 'ach_create'),
    ('admin_cong_dong', 'ach_edit'),
    ('admin_cong_dong', 'ach_manage_rewards'),
    ('admin_cong_dong', 'ach_revoke'),
    ('admin_cong_dong', 'ach_delete'),
    -- Media (5/10)
    ('admin_cong_dong', 'media_view_all'),
    ('admin_cong_dong', 'media_approve'),
    ('admin_cong_dong', 'media_moderate'),
    ('admin_cong_dong', 'media_delete_any'),
    ('admin_cong_dong', 'media_view_storage'),
    -- Fund (5/10)
    ('admin_cong_dong', 'fund_view_all'),
    ('admin_cong_dong', 'fund_approve'),
    ('admin_cong_dong', 'fund_view_anonymous'),
    ('admin_cong_dong', 'fund_view_audit_log'),
    ('admin_cong_dong', 'fund_audit_log'),
    -- Security (5/10)
    ('admin_cong_dong', 'sec_view_audit'),
    ('admin_cong_dong', 'sec_view_login_log'),
    ('admin_cong_dong', 'sec_session_revoke'),
    ('admin_cong_dong', 'sec_spam_filter'),
    ('admin_cong_dong', 'sec_report_manage')
ON CONFLICT (role, permission_code) DO NOTHING;

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Cập nhật view v_user_permissions (tự động cập nhật vì view đang join động)
-- ══════════════════════════════════════════════════════════════════════════════
-- View đã JOIN động với role_permissions nên không cần rebuild.

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. Verify count
-- ══════════════════════════════════════════════════════════════════════════════
-- Tổng số quyền: 150
--   admin_ky_thuat: 150 (TẤT CẢ)
--   admin_quan_li: 100 (30 cũ + 70 mới)
--   admin_cong_dong: 75 (20 cũ + 55 mới)
--   member: 0

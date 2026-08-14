-- Ứng Dụng Từ Bi - Migration 019: Hệ thống Thành Tích (Achievements)
-- Giai đoạn 19 (v0.9.14): Hệ thống thành tích daily/weekly/monthly/yearly/total
--
-- Mục tiêu:
--   * Tạo bảng `achievements` — định nghĩa các thành tích
--   * Tạo bảng `user_achievements` — thành tích user đã đạt
--   * Tạo bảng `achievement_progress` — tiến độ đang thực hiện
--   * Seed ~30 thành tích mẫu (theo HieuLouis/Hệ Thống Và Chức Năng Chi Tiết)
--   * Trigger cập nhật achievement khi user niệm Phật / viết cảm ngộ / kết bạn

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Bảng achievements — định nghĩa thành tích
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS achievements (
    id SERIAL PRIMARY KEY,
    code VARCHAR(60) NOT NULL UNIQUE,
    name_vi VARCHAR(200) NOT NULL,
    description_vi TEXT,
    icon VARCHAR(20) NOT NULL DEFAULT '🪷',
    -- Loại週期: daily | weekly | monthly | yearly | total | one_time
    period VARCHAR(20) NOT NULL DEFAULT 'one_time',
    -- Nhóm: niem_phat | tuong_phat | community | kinh_sach | friends | fund | special
    category VARCHAR(30) NOT NULL DEFAULT 'special',
    -- Điều kiện đạt (JSONB): {"metric": "niem_count", "operator": ">=", "value": 100}
    criteria JSONB NOT NULL,
    -- Phần thưởng
    reward_a BIGINT NOT NULL DEFAULT 0,
    reward_i BIGINT NOT NULL DEFAULT 0,
    reward_k BIGINT NOT NULL DEFAULT 0,
    -- Độ hiếm
    rarity VARCHAR(20) NOT NULL DEFAULT 'common',  -- common | rare | epic | legendary | mythic
    -- Số điểm achievement (để xếp hạng)
    achievement_points INT NOT NULL DEFAULT 10,
    sort_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE achievements IS 'Định nghĩa các thành tích — Giai đoạn 19 (v0.9.14)';
COMMENT ON COLUMN achievements.period IS 'daily | weekly | monthly | yearly | total | one_time';
COMMENT ON COLUMN achievements.category IS 'niem_phat | tuong_phat | community | kinh_sach | friends | fund | special';
COMMENT ON COLUMN achievements.rarity IS 'common | rare | epic | legendary | mythic';
COMMENT ON COLUMN achievements.criteria IS 'JSONB: {"metric":"niem_count","operator":">=","value":100}';

CREATE INDEX IF NOT EXISTS idx_achievements_period ON achievements(period);
CREATE INDEX IF NOT EXISTS idx_achievements_category ON achievements(category);
CREATE INDEX IF NOT EXISTS idx_achievements_active ON achievements(is_active);

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Bảng user_achievements — thành tích user đã đạt
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS user_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id INT NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    -- Cho thành tích periodic: chu kỳ cụ thể (vd '2026-08-15' cho daily, '2026-W33' cho weekly)
    period_key VARCHAR(30),
    -- Tiến độ lúc đạt
    progress_value BIGINT NOT NULL DEFAULT 0,
    -- Phần thưởng đã nhận?
    reward_claimed BOOLEAN NOT NULL DEFAULT false,
    claimed_at TIMESTAMPTZ,
    -- Metadata (JSONB)
    metadata JSONB,
    achieved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Unique: 1 user chỉ đạt 1 lần cho one_time; periodic thì unique per (user, achievement, period_key)
    UNIQUE (user_id, achievement_id, period_key)
);

CREATE INDEX IF NOT EXISTS idx_user_ach_user ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_ach_achievement ON user_achievements(achievement_id);
CREATE INDEX IF NOT EXISTS idx_user_ach_period ON user_achievements(period_key);
CREATE INDEX IF NOT EXISTS idx_user_ach_achieved_at ON user_achievements(achieved_at DESC);

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Bảng achievement_progress — tiến độ đang thực hiện (cho periodic)
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS achievement_progress (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id INT NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    period_key VARCHAR(30),
    current_value BIGINT NOT NULL DEFAULT 0,
    target_value BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, achievement_id, period_key)
);

CREATE INDEX IF NOT EXISTS idx_ach_progress_user ON achievement_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_ach_progress_achievement ON achievement_progress(achievement_id);
CREATE INDEX IF NOT EXISTS idx_ach_progress_period ON achievement_progress(period_key);

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. Trigger cập nhật updated_at
-- ══════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE FUNCTION trigger_update_achievements()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_achievements_updated
    BEFORE UPDATE ON achievements
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_achievements();

CREATE TRIGGER trg_ach_progress_updated
    BEFORE UPDATE ON achievement_progress
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_achievements();

-- ══════════════════════════════════════════════════════════════════════════════
-- 5. View: thành tích của user kèm thông tin chi tiết
-- ══════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE VIEW v_user_achievements AS
SELECT
    ua.id AS user_achievement_id,
    ua.user_id,
    ua.achievement_id,
    a.code AS achievement_code,
    a.name_vi AS achievement_name,
    a.description_vi AS achievement_desc,
    a.icon,
    a.period,
    a.category,
    a.rarity,
    a.achievement_points,
    a.reward_a,
    a.reward_i,
    a.reward_k,
    ua.period_key,
    ua.progress_value,
    ua.reward_claimed,
    ua.achieved_at
FROM user_achievements ua
JOIN achievements a ON a.id = ua.achievement_id;

COMMENT ON VIEW v_user_achievements IS 'Thành tích user đã đạt — kèm chi tiết achievement';

-- ══════════════════════════════════════════════════════════════════════════════
-- 6. View: tiến độ achievement của user
-- ══════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE VIEW v_user_achievement_progress AS
SELECT
    ap.id AS progress_id,
    ap.user_id,
    ap.achievement_id,
    a.code,
    a.name_vi,
    a.icon,
    a.period,
    a.category,
    a.rarity,
    a.achievement_points,
    ap.period_key,
    ap.current_value,
    ap.target_value,
    CASE WHEN ap.target_value > 0
         THEN LEAST(100, (ap.current_value * 100 / ap.target_value)::INT)
         ELSE 0 END AS percent_complete,
    CASE WHEN ap.current_value >= ap.target_value THEN true ELSE false END AS is_complete
FROM achievement_progress ap
JOIN achievements a ON a.id = ap.achievement_id;

-- ══════════════════════════════════════════════════════════════════════════════
-- 7. Seed ~30 thành tích mẫu
-- ══════════════════════════════════════════════════════════════════════════════
INSERT INTO achievements (code, name_vi, description_vi, icon, period, category, criteria, reward_a, reward_i, reward_k, rarity, achievement_points, sort_order) VALUES
    -- Niệm Phật (10)
    ('niem_first',         'Lần Đầu Niệm Phật',       'Niệm Phật lần đầu tiên',                '🙏', 'one_time', 'niem_phat', '{"metric":"total_niem","op":">=","value":1}'::jsonb, 10, 0, 0, 'common', 5, 1),
    ('niem_100',           'Trăm Niệm Khởi Đầu',       'Đạt 100 lần niệm Phật tổng',            '✨', 'total',    'niem_phat', '{"metric":"total_niem","op":">=","value":100}'::jsonb, 50, 1, 0, 'common', 10, 2),
    ('niem_1000',          'Nghìn Niệm Tinh Tấn',      'Đạt 1.000 lần niệm Phật tổng',          '🌟', 'total',    'niem_phat', '{"metric":"total_niem","op":">=","value":1000}'::jsonb, 200, 5, 1, 'rare', 25, 3),
    ('niem_10000',         'Vạn Niệm Thành Tâm',       'Đạt 10.000 lần niệm Phật tổng',         '🌸', 'total',    'niem_phat', '{"metric":"total_niem","op":">=","value":10000}'::jsonb, 1000, 20, 5, 'epic', 50, 4),
    ('niem_100000',        'Thành Niệm Viên Mãn',      'Đạt 100.000 lần niệm Phật tổng',        '🪷', 'total',    'niem_phat', '{"metric":"total_niem","op":">=","value":100000}'::jsonb, 5000, 100, 50, 'legendary', 100, 5),
    ('niem_daily_10',      'Niệm 10 Lần Mỗi Ngày',     'Niệm 10 lần trong 1 ngày',              '🌿', 'daily',    'niem_phat', '{"metric":"daily_niem","op":">=","value":10}'::jsonb, 5, 0, 0, 'common', 5, 6),
    ('niem_daily_100',     'Tinh Tấn Hằng Ngày',       'Niệm 100 lần trong 1 ngày',             '🍃', 'daily',    'niem_phat', '{"metric":"daily_niem","op":">=","value":100}'::jsonb, 30, 1, 0, 'rare', 15, 7),
    ('niem_streak_7',      'Tuần Niệm Liên Tục',       'Niệm Phật 7 ngày liên tiếp',            '📅', 'total',    'niem_phat', '{"metric":"streak","op":">=","value":7}'::jsonb, 50, 2, 0, 'rare', 20, 8),
    ('niem_streak_30',     'Tháng Niệm Liên Tục',      'Niệm Phật 30 ngày liên tiếp',           '🗓️', 'total',    'niem_phat', '{"metric":"streak","op":">=","value":30}'::jsonb, 200, 10, 1, 'epic', 40, 9),
    ('niem_streak_100',    'Trăm Ngày Tinh Tấn',       'Niệm Phật 100 ngày liên tiếp',          '🏅', 'total',    'niem_phat', '{"metric":"streak","op":">=","value":100}'::jsonb, 1000, 50, 10, 'legendary', 80, 10),
    -- Tượng Phật (5)
    ('vow_first_prayer',   'Lần Đầu Cầu Nguyện',       'Cầu nguyện lần đầu tiên tại Tượng Phật','🙏', 'one_time', 'tuong_phat', '{"metric":"vow_prayer","op":">=","value":1}'::jsonb, 10, 1, 0, 'common', 5, 11),
    ('vow_first_repent',   'Lần Đầu Sám Hối',          'Sám hối lần đầu tiên tại Tượng Phật',   '🙇', 'one_time', 'tuong_phat', '{"metric":"vow_repentance","op":">=","value":1}'::jsonb, 10, 2, 0, 'common', 5, 12),
    ('vow_first_dedicate', 'Lần Đầu Hồi Hướng',        'Hồi hướng lần đầu tiên tại Tượng Phật','🌸', 'one_time', 'tuong_phat', '{"metric":"vow_dedication","op":">=","value":1}'::jsonb, 10, 3, 0, 'common', 5, 13),
    ('vow_total_50',       'Phát Nguyện 50 Lần',       'Phát nguyện 50 lần tại Tượng Phật',     '🪷', 'total',    'tuong_phat', '{"metric":"vow_total","op":">=","value":50}'::jsonb, 100, 20, 0, 'rare', 25, 14),
    ('vow_total_500',      'Bồ Tát Tâm Nguyện',         'Phát nguyện 500 lần tại Tượng Phật',    '🕉️', 'total',    'tuong_phat', '{"metric":"vow_total","op":">=","value":500}'::jsonb, 500, 100, 5, 'epic', 50, 15),
    -- Cộng Đồng (5)
    ('comm_first_post',    'Bài Viết Đầu Tiên',         'Tạo chủ đề đầu tiên trong Cộng Đồng',   '📝', 'one_time', 'community', '{"metric":"topics_created","op":">=","value":1}'::jsonb, 20, 0, 0, 'common', 10, 16),
    ('comm_first_comment', 'Bình Luận Đầu Tiên',        'Bình luận lần đầu tiên',                '💬', 'one_time', 'community', '{"metric":"comments_created","op":">=","value":1}'::jsonb, 5, 0, 0, 'common', 5, 17),
    ('comm_create_group',  'Nhóm Trưởng',               'Tạo nhóm cộng đồng đầu tiên',           '👥', 'one_time', 'community', '{"metric":"groups_created","op":">=","value":1}'::jsonb, 50, 5, 0, 'rare', 20, 18),
    ('comm_10_groups',     'Kết Nộp Rộng Rãi',          'Tham gia 10 nhóm cộng đồng',            '🌐', 'total',    'community', '{"metric":"groups_joined","op":">=","value":10}'::jsonb, 100, 5, 0, 'rare', 25, 19),
    ('comm_100_comments',  'Tích Cực Thảo Luận',        'Đăng 100 bình luận trong cộng đồng',    '💬', 'total',    'community', '{"metric":"comments_created","op":">=","value":100}'::jsonb, 200, 10, 1, 'epic', 40, 20),
    -- Kinh Sách (5)
    ('book_first_read',    'Lần Đầu Đọc Sách',          'Đọc chương sách đầu tiên',              '📖', 'one_time', 'kinh_sach', '{"metric":"chapters_read","op":">=","value":1}'::jsonb, 10, 0, 0, 'common', 10, 21),
    ('book_first_review',  'Cảm Ngộ Đầu Tiên',          'Viết cảm ngộ đầu tiên',                 '✍️', 'one_time', 'kinh_sach', '{"metric":"reviews_written","op":">=","value":1}'::jsonb, 50, 5, 0, 'rare', 25, 22),
    ('book_10_reviews',    'Học Giả Kinh Sách',         'Viết 10 cảm ngộ đã được duyệt',         '📚', 'total',    'kinh_sach', '{"metric":"reviews_approved","op":">=","value":10}'::jsonb, 300, 20, 5, 'epic', 50, 23),
    ('book_give_50_flowers','Kính Dâng 50 Hoa',         'Tặng 50 hoa cho các sách',              '🌸', 'total',    'kinh_sach', '{"metric":"flowers_given","op":">=","value":50}'::jsonb, 100, 10, 0, 'rare', 25, 24),
    ('book_read_100_chapters','Đọc Giả Tinh Tấn',       'Đọc 100 chương sách',                   '📜', 'total',    'kinh_sach', '{"metric":"chapters_read","op":">=","value":100}'::jsonb, 500, 30, 5, 'epic', 50, 25),
    -- Bạn Bè (3)
    ('friend_first',       'Bạn Đầu Tiên',              'Kết bạn lần đầu tiên',                  '🤝', 'one_time', 'friends', '{"metric":"friends_count","op":">=","value":1}'::jsonb, 20, 0, 0, 'common', 10, 26),
    ('friend_10',          'Vòng Bạn Bè',               'Có 10 người bạn',                       '👥', 'total',    'friends', '{"metric":"friends_count","op":">=","value":10}'::jsonb, 50, 5, 0, 'common', 15, 27),
    ('friend_50',          'Nhiều Bạn Nhiều Quý',       'Có 50 người bạn',                       '🌟', 'total',    'friends', '{"metric":"friends_count","op":">=","value":50}'::jsonb, 200, 20, 1, 'rare', 30, 28),
    -- Quỹ Từ Bi (2)
    ('fund_first_donate',  'Lần Đầu Đóng Góp',          'Đóng góp Quỹ Từ Bi lần đầu',            '🪷', 'one_time', 'fund', '{"metric":"donations_count","op":">=","value":1}'::jsonb, 30, 5, 0, 'common', 15, 29),
    ('fund_total_1000',    'Nhà Hảo Tâm',               'Đóng góp tổng 1.000 K vào quỹ',         '💝', 'total',    'fund', '{"metric":"donations_total_k","op":">=","value":1000}'::jsonb, 1000, 50, 10, 'legendary', 80, 30)
ON CONFLICT (code) DO NOTHING;

-- ══════════════════════════════════════════════════════════════════════════════
-- 8. Function: kiểm tra & trao achievement cho user
-- ══════════════════════════════════════════════════════════════════════════════
-- Sử dụng sau khi user thực hiện hành động (niem_phat, vow, comment, v.v.)
-- Function này sẽ được gọi từ Rust handler để check & grant achievement.
CREATE OR REPLACE FUNCTION check_and_grant_achievement(
    p_user_id UUID,
    p_achievement_code VARCHAR
) RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE
    ach RECORD;
    already_granted BOOLEAN;
    period_key_val VARCHAR;
BEGIN
    -- Lấy achievement info
    SELECT * INTO ach FROM achievements WHERE code = p_achievement_code AND is_active = true;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    -- Tính period_key cho periodic achievements
    period_key_val := CASE ach.period
        WHEN 'daily'   THEN TO_CHAR(CURRENT_DATE, 'YYYY-MM-DD')
        WHEN 'weekly'  THEN TO_CHAR(DATE_TRUNC('week', CURRENT_DATE), 'YYYY-WW')
        WHEN 'monthly' THEN TO_CHAR(CURRENT_DATE, 'YYYY-MM')
        WHEN 'yearly'  THEN TO_CHAR(CURRENT_DATE, 'YYYY')
        ELSE NULL  -- one_time, total
    END;

    -- Kiểm tra đã grant chưa (tránh duplicate)
    SELECT EXISTS(
        SELECT 1 FROM user_achievements
        WHERE user_id = p_user_id
          AND achievement_id = ach.id
          AND (period_key = period_key_val OR (period_key IS NULL AND period_key_val IS NULL))
    ) INTO already_granted;

    IF already_granted THEN
        RETURN false;
    END IF;

    -- Grant achievement + reward
    INSERT INTO user_achievements (user_id, achievement_id, period_key, progress_value, reward_claimed, claimed_at)
    VALUES (p_user_id, ach.id, period_key_val, 0, true, NOW());

    -- Cộng phần thưởng vào balance
    UPDATE users
    SET a_balance = a_balance + ach.reward_a,
        i_balance = i_balance + ach.reward_i,
        k_balance = k_balance + ach.reward_k,
        updated_at = NOW()
    WHERE id = p_user_id;

    RETURN true;
END;
$$;

COMMENT ON FUNCTION check_and_grant_achievement IS 'Kiểm tra & trao achievement cho user — gọi từ Rust handler sau hành động';

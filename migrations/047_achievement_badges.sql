-- Migration 047 — Giai đoạn 69: Huy Hiệu Thành Tích (Achievement Badges)
-- Theo tài liệu ỨNG DỤNG TỪ BI.docx mục I.4 (Hệ Thống Thành Tích):
--   Hệ thống thành tích gồm: thành tích cá nhân + thành tích cộng đồng.
-- Giai đoạn 69: hiển thị huy hiệu trên profile (visual badges).

-- Bảng định nghĩa huy hiệu
CREATE TABLE IF NOT EXISTS achievement_badges (
    id            BIGSERIAL    PRIMARY KEY,
    code          VARCHAR(40)  NOT NULL UNIQUE,
    name          VARCHAR(100) NOT NULL,
    emoji         VARCHAR(10)  NOT NULL DEFAULT '🏆',
    description   TEXT,
    category      VARCHAR(30)  NOT NULL,  -- 'tu_hoc' | 'cong_dong' | 'tai_chinh' | 'dac_biet'
    requirement_type VARCHAR(30) NOT NULL,  -- 'niem_count' | 'friend_count' | 'k_balance' | 'a_balance' | 'group_count' | 'topic_count' | 'comment_count' | 'tu_si_rank' | 'days_active'
    requirement_value BIGINT   NOT NULL,
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Seed 18 huy hiệu
INSERT INTO achievement_badges (code, name, emoji, description, category, requirement_type, requirement_value, is_active) VALUES
    -- Tu học
    ('niem_100',       'Người Tu Tập',      '📿', 'Niệm Phật 100 lần',           'tu_hoc',   'niem_count',     100, true),
    ('niem_1000',      'Niệm Phật 1000 lần', '📿', 'Niệm Phật 1000 lần',          'tu_hoc',   'niem_count',    1000, true),
    ('niem_10000',     'Đại Niệm Phật',     '🪷', 'Niệm Phật 10000 lần',         'tu_hoc',   'niem_count',   10000, true),
    ('days_7',         'Kiên Trì 7 Ngày',   '🔥', 'Niệm Phật 7 ngày liên tiếp',  'tu_hoc',   'days_active',      7, true),
    ('days_30',       'Tinh Tấn 30 Ngày',  '🌟', 'Niệm Phật 30 ngày liên tiếp', 'tu_hoc',   'days_active',     30, true),
    -- Cộng đồng
    ('friend_10',     'Kết Duyên',         '👥', 'Có 10 người bạn',              'cong_dong','friend_count',    10, true),
    ('friend_50',     'Đa Duyên',          '🤝', 'Có 50 người bạn',             'cong_dong','friend_count',    50, true),
    ('group_1',       'Thành Viên Tích Cực', '✨', 'Tham gia 1 nhóm cộng đồng',  'cong_dong','group_count',      1, true),
    ('group_5',       'Cộng Đồng Viên',    '🌐', 'Tham gia 5 nhóm cộng đồng',   'cong_dong','group_count',      5, true),
    ('topic_1',       'Người Khởi Xướng',  '📝', 'Tạo 1 chủ đề cộng đồng',       'cong_dong','topic_count',      1, true),
    ('topic_10',      'Tác Giả Cộng Đồng', '📚', 'Tạo 10 chủ đề cộng đồng',      'cong_dong','topic_count',     10, true),
    -- Tài chính
    ('a_100',         'Tiểu Niệm Lực',     '⚡', 'Tích lũy 100 A',               'tai_chinh','a_balance',      100, true),
    ('a_1000',        'Niệm Lực Sơ',       '⚡', 'Tích lũy 1000 A',              'tai_chinh','a_balance',     1000, true),
    ('k_100',         'Tiểu Tài Phú',      '💰', 'Tích lũy 100 K',              'tai_chinh','k_balance',      100, true),
    ('k_1000',        'Tài Phú',           '💰', 'Tích lũy 1000 K',             'tai_chinh','k_balance',     1000, true),
    ('k_10000',       'Đại Tài Phú',       '💎', 'Tích lũy 10000 K',            'tai_chinh','k_balance',    10000, true),
    -- Đặc biệt
    ('tu_si_1',       'Tu Sĩ Tập Sự',      '⭐', 'Trở thành Tu Sĩ 1 sao',       'dac_biet', 'tu_si_rank',       1, true),
    ('tu_si_5',       'Đại Tu Sĩ',         '🌟', 'Trở thành Tu Sĩ 5 sao',       'dac_biet', 'tu_si_rank',       5, true)
ON CONFLICT (code) DO NOTHING;

-- Huy hiệu user đã đạt
CREATE TABLE IF NOT EXISTS user_badges (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_id     BIGINT       NOT NULL REFERENCES achievement_badges(id) ON DELETE CASCADE,
    awarded_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, badge_id)
);

CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges(user_id, awarded_at DESC);

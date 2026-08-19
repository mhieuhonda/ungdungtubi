-- Migration 040 — Giai đoạn 62: Vòng Quay May Mắn
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx mục I.6 (Tượng Phật):
--   Vòng Quay May Mắn gồm các phần thưởng (tỉ lệ dự kiến):
--   50%: Phần thưởng A (1-1000 A)
--   10%: Phần thưởng K (1-10 K)
--   10%: Đạo cụ dưới 10 K
--   10%: Tinh Thể
--   5%: Tinh Thạch
--   5%: Linh Thạch
--   4.9%: Bao Lì Xì Từ Bi (10K)
--   1%: Phần thưởng dưới 100 K
--   1%: Sách kỹ năng
--   1%: Đá Thức Tỉnh Thiên Phú
--   1%: Tiên Thạch
--   0.1%: Đạo cụ từ 100 K trở lên
--   1%: Xịt (no reward)

-- Bảng định nghĩa prize pools (admin có thể edit sau)
CREATE TABLE IF NOT EXISTS lucky_wheel_prizes (
    id            BIGSERIAL    PRIMARY KEY,
    code          VARCHAR(40)  NOT NULL UNIQUE,  -- 'a_small' | 'a_big' | 'k_small' | 'k_big' | 'tinh_the' | ...
    label         VARCHAR(100) NOT NULL,
    emoji         VARCHAR(10)  NOT NULL DEFAULT '🎁',
    reward_type   VARCHAR(20)  NOT NULL,  -- 'a' | 'k' | 'bi' | 'item' | 'nothing'
    reward_amount BIGINT      NOT NULL DEFAULT 0,  -- số lượng (theo reward_type)
    reward_item_code VARCHAR(40),  -- mã đạo cụ nếu reward_type='item'
    weight        DOUBLE PRECISION NOT NULL,  -- tỉ lệ (tổng = 100)
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Seed 12 prize tiers theo tài liệu
INSERT INTO lucky_wheel_prizes (code, label, emoji, reward_type, reward_amount, weight, is_active)
VALUES
    ('a_small',      'Niệm Lực A (1-100)',     '✨', 'a', 50,   25.0, true),
    ('a_medium',    'Niệm Lực A (100-500)',    '💫', 'a', 250,  15.0, true),
    ('a_big',       'Niệm Lực A (500-1000)',   '🌟', 'a', 750,  10.0, true),
    ('k_small',     'Tiền K (1-5)',            '🪙', 'k', 3,    5.0,  true),
    ('k_big',       'Tiền K (5-10)',           '💰', 'k', 7,    5.0,  true),
    ('tinh_the',    'Tinh Thể',                '🔮', 'item', 1, 10.0, true),
    ('tinh_thach',  'Tinh Thạch',             '💎', 'item', 1, 5.0,  true),
    ('linh_thach',  'Linh Thạch',              '🌈', 'item', 1, 5.0,  true),
    ('tien_thach',  'Tiên Thạch',             '✨', 'item', 1, 1.0,  true),
    ('bao_li_xi',   'Bao Lì Xì Từ Bi (10K)',   '🧧', 'item', 1, 4.9,  true),
    ('da_thuc_tinh','Đá Thức Tỉnh Thiên Phú',  '⚡', 'item', 1, 1.0,  true),
    ('nothing',     'Xịt (chưa may mắn)',      '💦', 'nothing', 0, 12.1, true)
ON CONFLICT (code) DO NOTHING;

-- Lịch sử quay vòng
CREATE TABLE IF NOT EXISTS lucky_wheel_spins (
    id            BIGSERIAL    PRIMARY KEY,
    user_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    prize_id      BIGINT       NOT NULL REFERENCES lucky_wheel_prizes(id),
    source        VARCHAR(20)  NOT NULL,  -- 'ad_watch' | 'daily_login' | 'event' | 'free_daily'
    reward_given  VARCHAR(20)  NOT NULL,  -- 'a' | 'k' | 'bi' | 'item' | 'nothing'
    reward_amount BIGINT       NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lucky_wheel_spins_user_recent ON lucky_wheel_spins(user_id, created_at DESC);

-- Spin quota per user per day (free daily spin = 1)
CREATE TABLE IF NOT EXISTS lucky_wheel_daily_quota (
    user_id        UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    quota_date    DATE         NOT NULL DEFAULT CURRENT_DATE,
    free_spins_used SMALLINT   NOT NULL DEFAULT 0,
    ad_spins_used  SMALLINT    NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, quota_date)
);

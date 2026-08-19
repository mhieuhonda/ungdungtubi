-- Migration 046 — Giai đoạn 68: Sự Kiện Phật Lịch (Buddhist Calendar)
-- Theo tài liệu ỨNG DỤNG TỪ BI.docx — các ngày lễ Phật giáo Việt Nam.
-- Giai đoạn 68: hiển thị lịch sự kiện + tặng thưởng đặc biệt vào ngày lễ.

-- Bảng sự kiện Phật lịch
CREATE TABLE IF NOT EXISTS buddhist_events (
    id            BIGSERIAL    PRIMARY KEY,
    code          VARCHAR(40)  NOT NULL UNIQUE,
    name          VARCHAR(200) NOT NULL,
    emoji         VARCHAR(10)  NOT NULL DEFAULT '🪷',
    description   TEXT,
    event_date    DATE         NOT NULL,  -- ngày lễ (lunar date âm lịch — admin cấu hình)
    is_recurring  BOOLEAN      NOT NULL DEFAULT true,  -- true: diễn ra hàng năm
    bonus_a       BIGINT       NOT NULL DEFAULT 0,     -- thưởng A thêm khi niệm phật trong ngày
    bonus_k       BIGINT       NOT NULL DEFAULT 0,     -- thưởng K thêm khi đăng nhập trong ngày
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Seed các ngày lễ Phật giáo chính
INSERT INTO buddhist_events (code, name, emoji, description, event_date, is_recurring, bonus_a, bonus_k, is_active) VALUES
    ('phat_dan',          'Lễ Phật Đản',                '🪷', 'Ngày Đức Phật Thích Ca Mâu Ni đản sinh.',                      '0015-04-08', true, 100, 1, true),
    ('thanh_dinh',       'Lễ Thanh Đinh',              '🕯️', 'Ngày Đức Phật thành đạo.',                                       '0015-12-08', true, 100, 1, true),
    ('tiet_tu_lan_bon',   'Lễ Tết Trung Nguyên (Vu Lan)','👁️', 'Ngày báo hiếu cha mẹ, xá tội vong nhân.',                       '0015-07-15', true, 100, 1, true),
    ('ky_niem_duc_phat',  'Kỷ niệm Đức Phật nhập niết bàn','🪷', 'Ngày Đức Phật nhập Niết bàn.',                                 '0015-02-15', true, 50,  0, true),
    ('dong_chi',          'Tết Đông Chí',               '❄️', 'Ngày đông chí — truyền thống Á Đông.',                          '0015-11-22', true, 30,  0, true),
    ('tet_nguyen_dan',    'Tết Nguyên Đán',             '🧧', 'Tết cổ truyền Việt Nam.',                                      '0015-01-01', true, 50,  1, true),
    ('tet_nguyen_tieu',   'Tết Nguyên Tiêu (Rằm tháng Giêng)', '🏮', 'Tết Nguyên Tiêu — đầu năm âm lịch.',                  '0015-01-15', true, 30,  0, true)
ON CONFLICT (code) DO NOTHING;

-- Lịch sử nhận thưởng sự kiện
CREATE TABLE IF NOT EXISTS event_reward_claims (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id     BIGINT       NOT NULL REFERENCES buddhist_events(id) ON DELETE CASCADE,
    event_date   DATE         NOT NULL,
    reward_a     BIGINT       NOT NULL DEFAULT 0,
    reward_k     BIGINT       NOT NULL DEFAULT 0,
    claimed_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, event_id, event_date)
);

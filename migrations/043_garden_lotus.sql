-- Migration 043 — Giai đoạn 65: Nhà Vườn (Lotus Garden)
-- Theo tài liệu ỨNG DỤNG TỪ BI.docx mục I.1.b (Nhà Vườn):
--   "Đây là nơi thành viên có thể: Trồng cây, chăn nuôi, trang trí không gian giả lập.
--    Tính năng này chưa được ưu tiên phát triển trong giai đoạn đầu."
-- Giai đoạn 65 triển khai version đơn giản: trồng cây sen, tưới nước, thu hoạch A.

-- Loại cây trong vườn (seed data)
CREATE TABLE IF NOT EXISTS garden_plant_types (
    id            BIGSERIAL    PRIMARY KEY,
    code          VARCHAR(40)  NOT NULL UNIQUE,
    name          VARCHAR(100) NOT NULL,
    emoji         VARCHAR(10)  NOT NULL DEFAULT '🪷',
    growth_seconds BIGINT      NOT NULL,     -- thời gian trưởng thành (giây)
    cost_k        BIGINT       NOT NULL DEFAULT 0,  -- giá mua hạt giống
    reward_a      BIGINT       NOT NULL,     -- A thu được khi thu hoạch
    is_active     BOOLEAN      NOT NULL DEFAULT true
);

INSERT INTO garden_plant_types (code, name, emoji, growth_seconds, cost_k, reward_a, is_active) VALUES
    ('hoa_sen_nho',   'Hoa Sen Nhỏ',      '🪷',  300,    0,  5,    true),   -- 5 phút
    ('hoa_sen_trung', 'Hoa Sen Trung',    '🌸',  1800,   1,  20,   true),   -- 30 phút
    ('hoa_sen_lon',   'Hoa Sen Lớn',      '🌺',  7200,   5,  100,  true),   -- 2 giờ
    ('cay_bo_de',     'Cây Bồ Đề',        '🌳',  86400,  20, 500,  true)    -- 1 ngày
ON CONFLICT (code) DO NOTHING;

-- Vườn của user (mỗi user 1 vườn, có tối đa 9 ô trồng)
CREATE TABLE IF NOT EXISTS user_gardens (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    max_slots    SMALLINT     NOT NULL DEFAULT 9,
    total_harvest BIGINT      NOT NULL DEFAULT 0,  -- tổng số lần thu hoạch
    total_a_earned BIGINT     NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Slot trồng cây trong vườn
CREATE TABLE IF NOT EXISTS garden_slots (
    id           BIGSERIAL    PRIMARY KEY,
    garden_id    BIGINT       NOT NULL REFERENCES user_gardens(id) ON DELETE CASCADE,
    slot_index   SMALLINT     NOT NULL,  -- 1-9
    plant_type_id BIGINT      REFERENCES garden_plant_types(id),
    planted_at   TIMESTAMPTZ,
    is_ready     BOOLEAN      NOT NULL DEFAULT false,  -- true khi đã trưởng thành
    ready_at     TIMESTAMPTZ,
    UNIQUE (garden_id, slot_index)
);

CREATE INDEX IF NOT EXISTS idx_garden_slots_ready ON garden_slots(ready_at) WHERE is_ready = false AND plant_type_id IS NOT NULL;

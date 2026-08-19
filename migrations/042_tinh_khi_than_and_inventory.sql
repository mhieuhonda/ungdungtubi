-- Migration 042 — Giai đoạn 64: Tinh Khí Thần + Kho Đạo Cụ
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx:
--   - Tinh Khí Thần: chỉ số chơi game ở bản đồ cấp 100.
--   - Tinh Thể (1K): nuốt để tăng 1 điểm Tinh Khí Thần (max 10/cấp).
--   - Tinh Thạch (2K): tăng cấp kỹ năng.
--   - Linh Thạch (5K): tăng cấp nhân vật game.
--   - Tiên Thạch (100K): tăng điểm ưu tiên cho Thiên Phú.
--   - Đá Thức Tỉnh Thiên Phú (10K): mở Vòng Quay Thức Tỉnh Thiên Phú.

-- Thêm cột tinh_khi_than vào users (level 1-100)
ALTER TABLE users ADD COLUMN IF NOT EXISTS tinh_khi_than SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS max_tinh_khi_than SMALLINT NOT NULL DEFAULT 100;

-- Bảng định nghĩa đạo cụ hệ thống (system items)
CREATE TABLE IF NOT EXISTS system_items (
    id            BIGSERIAL    PRIMARY KEY,
    code          VARCHAR(40)  NOT NULL UNIQUE,  -- 'tinh_the' | 'tinh_thach' | 'linh_thach' | 'tien_thach' | 'da_thuc_tinh' | 'bao_li_xi'
    name          VARCHAR(100) NOT NULL,
    emoji         VARCHAR(10)  NOT NULL DEFAULT '🎁',
    description   TEXT,
    price_k       BIGINT       NOT NULL DEFAULT 0,
    category      VARCHAR(30)  NOT NULL,  -- 'crystal' | 'passive' | 'consumable'
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

INSERT INTO system_items (code, name, emoji, description, price_k, category, is_active) VALUES
    ('tinh_the',     'Tinh Thể',                 '🔮', 'Nuốt để tăng 1 điểm Tinh Khí Thần. Tối đa 10 Tinh Thể cho mỗi cấp.', 1,   'crystal',   true),
    ('tinh_thach',   'Tinh Thạch',                '💎', 'Dùng để tăng cấp kỹ năng bị động.',                                  2,   'crystal',   true),
    ('linh_thach',   'Linh Thạch',                '🌈', 'Dùng để tăng cấp nhân vật game.',                                    5,   'crystal',   true),
    ('tien_thach',   'Tiên Thạch',                '✨', 'Dùng để tăng điểm ưu tiên cho Thiên Phú.',                          100, 'crystal',   true),
    ('da_thuc_tinh', 'Đá Thức Tỉnh Thiên Phú',   '⚡', 'Mở Vòng Quay Thức Tỉnh Thiên Phú.',                                 10,  'crystal',   true),
    ('bao_li_xi',    'Bao Lì Xì Từ Bi (10K)',     '🧧', 'Tạo 1 bao lì xì 10K chia cho nhiều người.',                          10,  'consumable', true),
    ('the_ung_ho',   'Thẻ Ủng Hộ',                '🙏', 'Quay vòng miễn phí, không phải xem quảng cáo.',                       5,  'consumable', true)
ON CONFLICT (code) DO NOTHING;

-- Inventory của user (số lượng mỗi đạo cụ)
CREATE TABLE IF NOT EXISTS user_inventories (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id      BIGINT       NOT NULL REFERENCES system_items(id),
    quantity     BIGINT       NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, item_id)
);

CREATE INDEX IF NOT EXISTS idx_user_inventories_user ON user_inventories(user_id) WHERE quantity > 0;

-- Lịch sử sử dụng tinh thể / đạo cụ
CREATE TABLE IF NOT EXISTS item_use_log (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_code    VARCHAR(40)  NOT NULL,
    quantity_used SMALLINT    NOT NULL DEFAULT 1,
    effect       VARCHAR(100),  -- vd: 'tinh_khi_than +1'
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_item_use_log_user ON item_use_log(user_id, created_at DESC);

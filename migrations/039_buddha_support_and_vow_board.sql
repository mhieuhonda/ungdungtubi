-- Migration 039 — Giai đoạn 61: Tượng Phật Ủng Hộ + Bảng Kính Nguyện
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx mục I.6 (Tượng Phật):
--   - Mỗi ngày user nhận 1 lần Cầu Nguyện, 1 lần Sám Hối, 1 lần Hồi Hướng.
--   - Ủng Hộ: xem quảng cáo, tối đa 10 lần/ngày, mỗi lượt = 1 lượt Quay May Mắn.
--   - Mỗi lần thực hiện → thông báo lên Bảng Kính Nguyện (vd: "Thành viên xxx đã sám hối").

-- Bảng theo dõi lượt Tượng Phật mỗi ngày (Cầu Nguyện / Sám Hối / Hồi Hướng / Ủng Hộ)
CREATE TABLE IF NOT EXISTS buddha_daily_uses (
    id              BIGSERIAL    PRIMARY KEY,
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    use_date        DATE         NOT NULL DEFAULT CURRENT_DATE,
    prayer_count    SMALLINT    NOT NULL DEFAULT 0,
    repentance_count SMALLINT   NOT NULL DEFAULT 0,
    dedication_count SMALLINT   NOT NULL DEFAULT 0,
    support_count   SMALLINT    NOT NULL DEFAULT 0,  -- Ủng Hộ (xem quảng cáo) — max 10/ngày
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, use_date)
);

CREATE INDEX IF NOT EXISTS idx_buddha_daily_uses_user_date ON buddha_daily_uses(user_id, use_date DESC);

-- Bảng Kính Nguyện — public feed cho Cầu Nguyện / Sám Hối / Hồi Hướng
-- Mỗi row = 1 thông báo hiển thị lên Bảng Kính Nguyện công khai
CREATE TABLE IF NOT EXISTS kinh_nguyen_board (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vow_type     VARCHAR(20)  NOT NULL,  -- 'prayer' | 'repentance' | 'dedication'
    content      TEXT         NOT NULL,  -- nội dung user nhập (sám hối / hồi hướng) hoặc rút gọn cầu nguyện
    is_public    BOOLEAN      NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kinh_nguyen_board_public_recent ON kinh_nguyen_board(created_at DESC) WHERE is_public = true;
CREATE INDEX IF NOT EXISTS idx_kinh_nguyen_board_user ON kinh_nguyen_board(user_id, created_at DESC);

-- Spin log cho Ủng Hộ (lượt quay may mắn nhận được từ việc xem quảng cáo)
CREATE TABLE IF NOT EXISTS lucky_spin_grants (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source       VARCHAR(20)  NOT NULL,  -- 'ad_watch' | 'daily_login' | 'event'
    granted_date DATE         NOT NULL DEFAULT CURRENT_DATE,
    is_used      BOOLEAN      NOT NULL DEFAULT false,
    used_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lucky_spin_grants_user_unused ON lucky_spin_grants(user_id) WHERE is_used = false;

-- Ứng Dụng Từ Bi - Migration 015: Không Gian Cá Nhân & Niệm Phật
-- Giai đoạn 13 (v0.9.9): Không Gian Cá Nhân + Tượng Phật + Nhật ký tu học
--
-- Mục tiêu:
--   * Thêm cột `i_balance` (Nguyên lực I) vào users — đơn vị phần thưởng từ Tượng Phật
--   * Tạo bảng `practice_logs` — nhật ký niệm Phật theo ngày (1 row/user/day)
--   * Tạo bảng `buddha_vows` — lưu Cầu Nguyện / Sám Hối / Hồi Hướng trước Tượng Phật
--   * Index + trigger cho performance & integrity
--
-- Thiết kế:
--   * Mỗi lần niệm Phật (POST /api/niem-phat):
--       - INSERT INTO practice_logs (user_id, log_date=today) ON CONFLICT DO UPDATE SET niem_count = niem_count + 1
--       - UPDATE users SET a_balance = a_balance + 1
--   * 1000 A = 1 K (quy ước hiện có, áp dụng qua rank promotion)
--   * Mỗi vow (cầu nguyện / sám hối / hồi hướng) tặng 1 I (Nguyên lực)
--   * Vows có thể public (hiển thị trên bảng Kính Nguyện) hoặc private

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Thêm cột i_balance vào users (Nguyên lực I)
-- ══════════════════════════════════════════════════════════════════════════════
ALTER TABLE users ADD COLUMN IF NOT EXISTS i_balance BIGINT NOT NULL DEFAULT 0;
COMMENT ON COLUMN users.i_balance IS 'Nguyên lực I — đơn vị phần thưởng từ Tượng Phật (cầu nguyện / sám hối / hồi hướng)';

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Bảng practice_logs — nhật ký niệm Phật theo ngày
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS practice_logs (
    id            BIGSERIAL    PRIMARY KEY,
    user_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    log_date      DATE         NOT NULL DEFAULT CURRENT_DATE,
    niem_count    BIGINT       NOT NULL DEFAULT 0,
    last_niem_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, log_date)
);

COMMENT ON TABLE practice_logs IS 'Nhật ký niệm Phật theo ngày — 1 row/user/day';
COMMENT ON COLUMN practice_logs.niem_count IS 'Số lần niệm Phật trong ngày';
COMMENT ON COLUMN practice_logs.last_niem_at IS 'Thời điểm niệm Phật gần nhất trong ngày';

CREATE INDEX IF NOT EXISTS idx_practice_logs_user_id ON practice_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_practice_logs_log_date ON practice_logs(log_date DESC);

-- Trigger tự động cập nhật updated_at
CREATE OR REPLACE FUNCTION trg_practice_logs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS practice_logs_updated_at ON practice_logs;
CREATE TRIGGER practice_logs_updated_at
    BEFORE UPDATE ON practice_logs
    FOR EACH ROW
    EXECUTE FUNCTION trg_practice_logs_updated_at();

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Bảng buddha_vows — Cầu Nguyện / Sám Hối / Hồi Hướng trước Tượng Phật
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS buddha_vows (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vow_type     VARCHAR(20)  NOT NULL,  -- prayer | repentance | dedication
    content      TEXT         NOT NULL,
    is_public    BOOLEAN      NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE buddha_vows IS 'Lời Cầu Nguyện / Sám Hối / Hồi Hướng trước Tượng Phật';
COMMENT ON COLUMN buddha_vows.vow_type IS 'Loại vow: prayer (cầu nguyện) | repentance (sám hối) | dedication (hồi hướng)';
COMMENT ON COLUMN buddha_vows.is_public IS 'True = hiển thị trên bảng Kính Nguyện công khai';

-- CHECK constraint cho vow_type
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'buddha_vows_vow_type_check'
    ) THEN
        ALTER TABLE buddha_vows
        ADD CONSTRAINT buddha_vows_vow_type_check
        CHECK (vow_type IN ('prayer', 'repentance', 'dedication'));
    END IF;
END$$;

CREATE INDEX IF NOT EXISTS idx_buddha_vows_user_id ON buddha_vows(user_id);
CREATE INDEX IF NOT EXISTS idx_buddha_vows_created_at ON buddha_vows(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_buddha_vows_public ON buddha_vows(is_public, created_at DESC) WHERE is_public = true;

-- Trigger updated_at không cần vì buddha_vows là append-only (no UPDATE).

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. Seed: tặng 10 I cho admin kỹ thuật (để test UI)
-- ══════════════════════════════════════════════════════════════════════════════
UPDATE users
SET i_balance = GREATEST(i_balance, 10),
    updated_at = NOW()
WHERE role = 'admin_ky_thuat' AND i_balance = 0;

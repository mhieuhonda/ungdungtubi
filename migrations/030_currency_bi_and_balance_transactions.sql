-- =====================================================================
-- Migration 030 — Giai đoạn 46: Hệ Thống Tiền Tệ Bi + Balance Transactions
-- v0.9.42 — 2026-08-18
--
-- Mục tiêu:
--   1. Thêm cột `bi_balance` vào bảng `users` — đồng tiền Bi (Từ Bi/Compassion),
--      loại tiền cao cấp nhất trong hệ thống 3 tiền tệ A/K/Bi.
--      Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx" mục "Hệ Thống Tiền Tệ":
--        - Ấm (A): tiền cơ bản, kiếm qua hoạt động thường
--        - K: tiền công đức, kiếm qua việc lành, dùng trong Thương Thành
--        - Bi: tiền từ bi, loại cao cấp, kiếm qua cống hiến đặc biệt hoặc quy đổi
--   2. Tạo bảng `balance_transactions` — lịch sử giao dịch A/K/Bi
--      (chuyển kho, nhận thưởng, mua hàng, quy đổi, v.v.)
--   3. Tạo bảng `currency_exchange_rates` — tỷ lệ quy đổi giữa A ↔ K ↔ Bi
--      (admin quản lý, mặc định: 100 A = 1 K, 100 K = 1 Bi)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ════════════════════════════════════════════════════════════════════════════
-- 1. Thêm cột bi_balance vào users
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE users ADD COLUMN IF NOT EXISTS bi_balance BIGINT NOT NULL DEFAULT 0;
COMMENT ON COLUMN users.bi_balance IS 'Số dư Bi (Từ Bi) — loại tiền cao cấp nhất. Kiếm qua cống hiến đặc biệt hoặc quy đổi từ K.';

CREATE INDEX IF NOT EXISTS idx_users_bi_balance ON users(bi_balance) WHERE bi_balance > 0;

-- ════════════════════════════════════════════════════════════════════════════
-- 2. Bảng balance_transactions — lịch sử giao dịch A/K/Bi
-- ════════════════════════════════════════════════════════════════════════════
-- Ghi lại mọi thay đổi số dư: mua hàng, nhận thưởng, quy đổi, admin adjust, v.v.
CREATE TABLE IF NOT EXISTS balance_transactions (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Loại tiền: 'a' (Ấm), 'k' (Karmic), 'bi' (Từ Bi)
    currency        VARCHAR(5)      NOT NULL CHECK (currency IN ('a', 'k', 'bi')),
    -- Số thay đổi: dương = nhận, âm = trừ
    amount          BIGINT          NOT NULL,
    -- Số dư sau giao dịch (snapshot)
    balance_after   BIGINT          NOT NULL,
    -- Loại giao dịch
    tx_type         VARCHAR(30)     NOT NULL CHECK (tx_type IN (
        'purchase',          -- Mua hàng trong Thương Thành
        'sale',              -- Bán hàng trong Thương Thành
        'reward',            -- Nhận thưởng (niệm Phật, thành tích, admin grant)
        'exchange_in',       -- Quy đổi vào (vd: 100A → 1K)
        'exchange_out',     -- Quy đổi ra (vd: 1K → 100A)
        'donation',          -- Đóng góp Quỹ Từ Bi
        'admin_adjust',      -- Admin điều chỉnh thủ công
        'dao_huu_payment',   -- Thanh toán Chợ Đạo Hữu
        'signup_bonus',      -- Thưởng đăng ký
        'daily_login',       -- Thưởng đăng nhập hàng ngày
        'other'              -- Khác
    )),
    -- Mô tả chi tiết giao dịch
    description     TEXT            NOT NULL DEFAULT '',
    -- ID tham chiếu (vd: item_id khi mua hàng, achievement_id khi nhận thưởng)
    reference_id    VARCHAR(100),
    -- Admin thực hiện (cho tx_type = 'admin_adjust')
    performed_by    UUID            REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_balance_tx_user ON balance_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_balance_tx_currency ON balance_transactions(currency);
CREATE INDEX IF NOT EXISTS idx_balance_tx_type ON balance_transactions(tx_type);
CREATE INDEX IF NOT EXISTS idx_balance_tx_user_currency ON balance_transactions(user_id, currency);
CREATE INDEX IF NOT EXISTS idx_balance_tx_created ON balance_transactions(created_at DESC);

COMMENT ON TABLE balance_transactions IS 'Lịch sử giao dịch tiền tệ A/K/Bi — mọi thay đổi số dư đều được ghi lại.';
COMMENT ON COLUMN balance_transactions.amount IS 'Số thay đổi: dương = nhận thêm, âm = trừ đi';
COMMENT ON COLUMN balance_transactions.balance_after IS 'Số dư sau giao dịch (snapshot để truy vết nhanh)';

-- ════════════════════════════════════════════════════════════════════════════
-- 3. Bảng currency_exchange_rates — tỷ lệ quy đổi
-- ════════════════════════════════════════════════════════════════════════════
-- Admin quản lý tỷ lệ. Mặc định: 100A = 1K, 100K = 1Bi
CREATE TABLE IF NOT EXISTS currency_exchange_rates (
    id              BIGSERIAL       PRIMARY KEY,
    from_currency   VARCHAR(5)      NOT NULL CHECK (from_currency IN ('a', 'k', 'bi')),
    to_currency     VARCHAR(5)      NOT NULL CHECK (to_currency IN ('a', 'k', 'bi')),
    -- Tỷ lệ: from_amount đơn vị from_currency = 1 đơn vị to_currency
    -- Vd: from_currency='a', to_currency='k', from_amount=100 → 100A = 1K
    from_amount     BIGINT          NOT NULL DEFAULT 100,
    is_active       BOOLEAN         NOT NULL DEFAULT true,
    updated_by      UUID            REFERENCES users(id) ON DELETE SET NULL,
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (from_currency, to_currency)
);

-- Seed tỷ lệ mặc định
INSERT INTO currency_exchange_rates (from_currency, to_currency, from_amount) VALUES
    ('a',  'k',  100),   -- 100 Ấm = 1 Karmic
    ('k',  'bi', 100),   -- 100 Karmic = 1 Bi
    ('a',  'bi', 10000)  -- 10000 Ấm = 1 Bi (qua trung gian K)
ON CONFLICT (from_currency, to_currency) DO NOTHING;

COMMENT ON TABLE currency_exchange_rates IS 'Tỷ lệ quy đổi tiền tệ A ↔ K ↔ Bi — admin quản lý.';

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Trigger updated_at cho currency_exchange_rates
-- ════════════════════════════════════════════════════════════════════════════
DROP TRIGGER IF EXISTS trg_currency_exchange_rates_set_updated_at ON currency_exchange_rates;
CREATE TRIGGER trg_currency_exchange_rates_set_updated_at
    BEFORE UPDATE ON currency_exchange_rates
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '030', 'v0.9.42 — Giai đoạn 46: Hệ Thống Tiền Tệ Bi (bi_balance) + balance_transactions + currency_exchange_rates.'
) ON CONFLICT (version) DO NOTHING;

-- Ứng Dụng Từ Bi - Migration 016: Quỹ Từ Bi
-- Giai đoạn 15 (v0.9.11): Hệ thống quỹ cộng đồng Từ Bi
--
-- Mục tiêu:
--   * Bảng `fund_donations` — lưu các đóng góp vào Quỹ Từ Bi từ thành viên
--   * Bảng `fund_campaigns` — các chiến dịch gây quỹ chuyên đề (Sách/Tu/Quà/Thiện Nguyện)
--   * Bảng `fund_expenses` — các khoản chi tiêu từ quỹ (công khai, minh bạch)
--   * Index + trigger cho performance & integrity
--
-- Thiết kế (theo HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx mục VI):
--   * Quỹ Từ Bi là quỹ chung của toàn bộ cộng đồng
--   * Nguồn hình thành: đóng góp của thành viên, hỗ trợ mạnh thường quân, lợi nhuận dự án
--   * Nguyên tắc: Công khai · Minh bạch · Cùng quản lý · Cùng phát triển · Cùng chung số phận
--   * Tương lai: chia thành nhiều quỹ thành phần (Sách/Tu/Quà/Thiện Nguyện)
--
-- Quy ước:
--   * Đơn vị đóng góp: K (Tiền K — 1000 A = 1 K)
--   * Mỗi donation trừ K từ user.k_balance và cộng vào tổng quỹ
--   * Quỹ tổng được tính bằng SUM(amount_k) WHERE status = 'completed'

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Bảng fund_donations — đóng góp vào Quỹ Từ Bi
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS fund_donations (
    id              BIGSERIAL    PRIMARY KEY,
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_k        BIGINT       NOT NULL CHECK (amount_k > 0),
    -- Loại quỹ: general | sach | tu | qua | thien_nguyen
    donation_type   VARCHAR(20)  NOT NULL DEFAULT 'general',
    message         VARCHAR(500),
    is_anonymous    BOOLEAN      NOT NULL DEFAULT false,
    -- pending = vừa tạo (chưa trừ K), completed = đã trừ K xong, refunded = hoàn tiền
    status          VARCHAR(20)  NOT NULL DEFAULT 'completed',
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE fund_donations IS 'Đóng góp vào Quỹ Từ Bi — K trừ từ k_balance của user';
COMMENT ON COLUMN fund_donations.amount_k IS 'Số K đóng góp (phải > 0)';
COMMENT ON COLUMN fund_donations.donation_type IS 'Loại quỹ: general | sach | tu | qua | thien_nguyen';
COMMENT ON COLUMN fund_donations.is_anonymous IS 'True = ẩn tên đóng góp, chỉ hiện "Đạo hữu ẩn danh"';
COMMENT ON COLUMN fund_donations.status IS 'Trạng thái: pending | completed | refunded';

-- CHECK constraint cho donation_type
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fund_donations_donation_type_check'
    ) THEN
        ALTER TABLE fund_donations
        ADD CONSTRAINT fund_donations_donation_type_check
        CHECK (donation_type IN ('general', 'sach', 'tu', 'qua', 'thien_nguyen'));
    END IF;
END$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fund_donations_status_check'
    ) THEN
        ALTER TABLE fund_donations
        ADD CONSTRAINT fund_donations_status_check
        CHECK (status IN ('pending', 'completed', 'refunded'));
    END IF;
END$$;

CREATE INDEX IF NOT EXISTS idx_fund_donations_user_id ON fund_donations(user_id);
CREATE INDEX IF NOT EXISTS idx_fund_donations_created_at ON fund_donations(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fund_donations_type ON fund_donations(donation_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fund_donations_status ON fund_donations(status, created_at DESC);

-- Trigger updated_at
CREATE OR REPLACE FUNCTION trg_fund_donations_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS fund_donations_updated_at ON fund_donations;
CREATE TRIGGER fund_donations_updated_at
    BEFORE UPDATE ON fund_donations
    FOR EACH ROW
    EXECUTE FUNCTION trg_fund_donations_updated_at();

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Bảng fund_campaigns — chiến dịch gây quỹ chuyên đề
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS fund_campaigns (
    id              BIGSERIAL    PRIMARY KEY,
    name            VARCHAR(200) NOT NULL,
    slug            VARCHAR(220) NOT NULL UNIQUE,
    description     TEXT,
    -- Loại chiến dịch: same enum as donation_type
    campaign_type   VARCHAR(20)  NOT NULL DEFAULT 'general',
    target_amount_k BIGINT       NOT NULL DEFAULT 0 CHECK (target_amount_k >= 0),
    -- current_amount_k được tính bằng SUM của donations thuộc campaign (qua trigger)
    current_amount_k BIGINT      NOT NULL DEFAULT 0,
    start_date      DATE         NOT NULL DEFAULT CURRENT_DATE,
    end_date        DATE,
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    created_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE fund_campaigns IS 'Chiến dịch gây quỹ chuyên đề (Sách/Tu/Quà/Thiện Nguyện)';
COMMENT ON COLUMN fund_campaigns.target_amount_k IS 'Mục tiêu gây quỹ (K). 0 = không giới hạn';
COMMENT ON COLUMN fund_campaigns.current_amount_k IS 'Số K đã quyên góp (cập nhật qua trigger)';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fund_campaigns_campaign_type_check'
    ) THEN
        ALTER TABLE fund_campaigns
        ADD CONSTRAINT fund_campaigns_campaign_type_check
        CHECK (campaign_type IN ('general', 'sach', 'tu', 'qua', 'thien_nguyen'));
    END IF;
END$$;

CREATE INDEX IF NOT EXISTS idx_fund_campaigns_slug ON fund_campaigns(slug);
CREATE INDEX IF NOT EXISTS idx_fund_campaigns_active ON fund_campaigns(is_active, end_date);

CREATE OR REPLACE FUNCTION trg_fund_campaigns_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS fund_campaigns_updated_at ON fund_campaigns;
CREATE TRIGGER fund_campaigns_updated_at
    BEFORE UPDATE ON fund_campaigns
    FOR EACH ROW
    EXECUTE FUNCTION trg_fund_campaigns_updated_at();

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Bảng fund_expenses — chi tiêu từ quỹ (công khai, minh bạch)
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS fund_expenses (
    id              BIGSERIAL    PRIMARY KEY,
    amount_k        BIGINT       NOT NULL CHECK (amount_k > 0),
    expense_type    VARCHAR(20)  NOT NULL DEFAULT 'general',
    description     TEXT         NOT NULL,
    -- Đường dẫn ảnh hóa đơn / bằng chứng (tùy chọn)
    receipt_url     VARCHAR(500),
    spent_at        DATE         NOT NULL DEFAULT CURRENT_DATE,
    approved_by     UUID         REFERENCES users(id) ON DELETE SET NULL,
    is_public       BOOLEAN      NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE fund_expenses IS 'Chi tiêu từ Quỹ Từ Bi — công khai minh bạch';
COMMENT ON COLUMN fund_expenses.amount_k IS 'Số K chi tiêu (phải > 0)';
COMMENT ON COLUMN fund_expenses.expense_type IS 'Loại chi: same enum as donation_type';
COMMENT ON COLUMN fund_expenses.is_public IS 'True = hiển thị công khai';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fund_expenses_expense_type_check'
    ) THEN
        ALTER TABLE fund_expenses
        ADD CONSTRAINT fund_expenses_expense_type_check
        CHECK (expense_type IN ('general', 'sach', 'tu', 'qua', 'thien_nguyen'));
    END IF;
END$$;

CREATE INDEX IF NOT EXISTS idx_fund_expenses_spent_at ON fund_expenses(spent_at DESC);
CREATE INDEX IF NOT EXISTS idx_fund_expenses_type ON fund_expenses(expense_type, spent_at DESC);

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. View: v_fund_summary — tổng quan Quỹ Từ Bi (cho dashboard)
-- ══════════════════════════════════════════════════════════════════════════════
CREATE OR REPLACE VIEW v_fund_summary AS
SELECT
    -- Tổng thu (donations completed)
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed'), 0)::BIGINT AS total_income_k,
    -- Tổng chi (expenses public)
    COALESCE((SELECT SUM(amount_k) FROM fund_expenses WHERE is_public = true), 0)::BIGINT AS total_expense_k,
    -- Số dư = thu - chi
    (
        COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed'), 0)
        - COALESCE((SELECT SUM(amount_k) FROM fund_expenses WHERE is_public = true), 0)
    )::BIGINT AS balance_k,
    -- Số lượt đóng góp
    (SELECT COUNT(*) FROM fund_donations WHERE status = 'completed')::BIGINT AS total_donations_count,
    -- Số nhà hảo tâm (unique donors)
    (SELECT COUNT(DISTINCT user_id) FROM fund_donations WHERE status = 'completed')::BIGINT AS unique_donors,
    -- Theo loại quỹ
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed' AND donation_type = 'general'), 0)::BIGINT AS fund_general,
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed' AND donation_type = 'sach'), 0)::BIGINT AS fund_sach,
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed' AND donation_type = 'tu'), 0)::BIGINT AS fund_tu,
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed' AND donation_type = 'qua'), 0)::BIGINT AS fund_qua,
    COALESCE((SELECT SUM(amount_k) FROM fund_donations WHERE status = 'completed' AND donation_type = 'thien_nguyen'), 0)::BIGINT AS fund_thien_nguyen;

COMMENT ON VIEW v_fund_summary IS 'Tổng quan Quỹ Từ Bi — thu/chi/số dư/theo loại quỹ';

-- ══════════════════════════════════════════════════════════════════════════════
-- 5. Seed: tặng 50 K cho admin_ky_thuat để test donation (nếu chưa có)
-- ══════════════════════════════════════════════════════════════════════════════
-- Chỉ tặng nếu user chưa có K (tránh ảnh hưởng nếu đã có dư)
UPDATE users
SET k_balance = GREATEST(k_balance, 50),
    updated_at = NOW()
WHERE role = 'admin_ky_thuat' AND k_balance < 50;

-- =====================================================================
-- Migration 032 — Giai đoạn 53: Hệ Thống Tu Sĩ (1-5 Sao)
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Theo tài liệu "ỨNG DỤNG TỪ BI.docx" mục II.3.c (Hệ thống Tu Sĩ):
--     Thành viên có thể đăng ký trở thành Tu Sĩ — được xét duyệt bởi hệ thống
--     và đội ngũ quản lý. Các cấp bậc Tu Sĩ:
--       Tu Sĩ Một Sao: hỗ trợ từ 100 K/tháng.
--       Tu Sĩ Hai Sao: hỗ trợ từ 200 K/tháng.
--       Tu Sĩ Ba Sao: hỗ trợ từ 500 K/tháng.
--       Tu Sĩ Bốn Sao: hỗ trợ từ 1000 K/tháng.
--       Tu Sĩ Năm Sao: hỗ trợ từ 5000 K/tháng.
--   Giai đoạn 53 triển khai: bảng `tu_si_applications` (đăng ký) +
--   cột `users.tu_si_rank` (1-5 sao sau khi duyệt) + bảng `tu_si_monthly_supports`
--   (theo dõi đóng góp hàng tháng).
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Thêm cột tu_si_rank vào users (NULL = chưa phải Tu Sĩ)
ALTER TABLE users ADD COLUMN IF NOT EXISTS tu_si_rank SMALLINT;
COMMENT ON COLUMN users.tu_si_rank IS 'Cấp bậc Tu Sĩ 1-5 sao (NULL nếu chưa đăng ký/chưa duyệt).';

-- 2. Thêm cột tu_si_approved_at để theo dõi thời điểm được duyệt
ALTER TABLE users ADD COLUMN IF NOT EXISTS tu_si_approved_at TIMESTAMPTZ;
COMMENT ON COLUMN users.tu_si_approved_at IS 'Thời điểm user được duyệt thành Tu Sĩ.';

-- 3. CHECK constraint cho tu_si_rank (1-5)
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'users_tu_si_rank_check') THEN
        ALTER TABLE users ADD CONSTRAINT users_tu_si_rank_check
        CHECK (tu_si_rank IS NULL OR (tu_si_rank >= 1 AND tu_si_rank <= 5));
    END IF;
END $$;

-- 4. Bảng tu_si_applications — đăng ký Tu Sĩ
CREATE TABLE IF NOT EXISTS tu_si_applications (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requested_rank SMALLINT        NOT NULL CHECK (requested_rank >= 1 AND requested_rank <= 5),
    -- Mức cam kết đóng góp hàng tháng (K) — min theo rank: 100/200/500/1000/5000
    monthly_k_pledge BIGINT         NOT NULL DEFAULT 0,
    -- Lý do đăng ký (tự nguyện, tâm nguyện, v.v.)
    motivation      TEXT            NOT NULL DEFAULT '',
    -- Trạng thái: pending | approved | rejected | withdrawn
    status          VARCHAR(20)     NOT NULL DEFAULT 'pending'
                                    CHECK (status IN ('pending', 'approved', 'rejected', 'withdrawn')),
    -- Review info
    reviewed_by     UUID            REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at     TIMESTAMPTZ,
    review_note     TEXT            NOT NULL DEFAULT '',
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tu_si_apps_user ON tu_si_applications(user_id);
CREATE INDEX IF NOT EXISTS idx_tu_si_apps_status ON tu_si_applications(status);
CREATE INDEX IF NOT EXISTS idx_tu_si_apps_created ON tu_si_applications(created_at DESC);

COMMENT ON TABLE tu_si_applications IS 'Đơn đăng ký Tu Sĩ (1-5 sao) — duyệt bởi admin.';

-- 5. Bảng tu_si_monthly_supports — theo dõi đóng góp hàng tháng của Tu Sĩ
CREATE TABLE IF NOT EXISTS tu_si_monthly_supports (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Năm-tháng (vd: 202608 cho Aug 2026)
    year_month      INTEGER         NOT NULL,
    -- Số K đã đóng góp trong tháng đó
    k_contributed   BIGINT          NOT NULL DEFAULT 0,
    -- Đã đạt mức cam kết?
    fulfilled       BOOLEAN         NOT NULL DEFAULT false,
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, year_month)
);

CREATE INDEX IF NOT EXISTS idx_tu_si_supports_user_month ON tu_si_monthly_supports(user_id, year_month);
CREATE INDEX IF NOT EXISTS idx_tu_si_supports_year_month ON tu_si_monthly_supports(year_month);

COMMENT ON TABLE tu_si_monthly_supports IS 'Theo dõi đóng góp K hàng tháng của Tu Sĩ (so với cam kết).';

-- 6. Trigger updated_at cho cả 2 bảng
DROP TRIGGER IF EXISTS trg_tu_si_apps_set_updated_at ON tu_si_applications;
CREATE TRIGGER trg_tu_si_apps_set_updated_at
    BEFORE UPDATE ON tu_si_applications
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_tu_si_supports_set_updated_at ON tu_si_monthly_supports;
CREATE TRIGGER trg_tu_si_supports_set_updated_at
    BEFORE UPDATE ON tu_si_monthly_supports
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 7. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '032', 'v0.9.45 — Giai đoạn 53: Hệ Thống Tu Sĩ (1-5 sao) — tu_si_applications + tu_si_monthly_supports + users.tu_si_rank.'
) ON CONFLICT (version) DO NOTHING;

-- =====================================================================
-- Migration 037 — Giai đoạn 58: Mục Tiêu Tu Học + Streak Bảo Vệ
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Giai đoạn 58: thêm 2 tính năng cho cá nhân người dùng:
--     (a) Mục tiêu tu học cá nhân (Tu Học Goals): "Mỗi ngày niệm 108 lần",
--         "Mỗi tuần đọc 1 chương", v.v. Track tiến độ + deadline.
--     (b) Streak Bảo Vệ (Streak Freeze): cơ chế giống Duolingo — nếu lỡ 1 ngày
--         không niệm Phật, dùng 1 "streak freeze" để giữ streak (giới hạn 2 cái/
--         tháng miễn phí, mua thêm bằng 100 A).
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Bảng tu_hoc_goals — mục tiêu tu học cá nhân
CREATE TABLE IF NOT EXISTS tu_hoc_goals (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Loại mục tiêu: 'daily_niem' | 'weekly_niem' | 'monthly_niem' | 'daily_read' |
    --                 'weekly_read' | 'daily_thien' | 'custom'
    goal_type       VARCHAR(30)     NOT NULL,
    -- Mục tiêu số (vd: 108 niệm, 1 chương, 30 phút thiền)
    target_value    BIGINT          NOT NULL,
    -- Đơn vị: 'count' | 'chapter' | 'minute'
    target_unit     VARCHAR(20)     NOT NULL DEFAULT 'count',
    -- Tiêu đề mô tả (vd: "Mỗi ngày niệm 108 lần A Di Đà Phật")
    title           VARCHAR(200)    NOT NULL,
    -- Trạng thái: 'active' | 'completed' | 'abandoned' | 'expired'
    status          VARCHAR(20)     NOT NULL DEFAULT 'active'
                                    CHECK (status IN ('active', 'completed', 'abandoned', 'expired')),
    -- Deadline (optional)
    deadline        DATE,
    -- Tiến độ hiện tại (đếm trong kỳ hiện tại)
    current_value   BIGINT          NOT NULL DEFAULT 0,
    -- Lần reset cuối (mỗi ngày/tuần reset về 0)
    last_reset_at   TIMESTAMPTZ,
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tu_hoc_goals_user_status
    ON tu_hoc_goals(user_id, status);
CREATE INDEX IF NOT EXISTS idx_tu_hoc_goals_user_type
    ON tu_hoc_goals(user_id, goal_type);

COMMENT ON TABLE tu_hoc_goals IS 'Mục tiêu tu học cá nhân — niệm/thiền/đọc sách + deadline.';

-- 2. Bảng streak_freezes — bảo vệ streak khi bỏ ngày
CREATE TABLE IF NOT EXISTS streak_freezes (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Ngày được freeze (ngày user BỎ không niệm Phật)
    freeze_date     DATE            NOT NULL,
    -- Nguồn: 'monthly_free' (2 cái/tháng miễn phí) | 'purchased' (mua 100 A)
    source          VARCHAR(20)     NOT NULL DEFAULT 'monthly_free'
                                    CHECK (source IN ('monthly_free', 'purchased', 'admin_grant')),
    -- Giá mua (A) nếu source='purchased'
    cost_a          BIGINT          NOT NULL DEFAULT 0,
    -- Đã áp dụng (sử dụng để bảo vệ streak)?
    applied         BOOLEAN         NOT NULL DEFAULT false,
    applied_at      TIMESTAMPTZ,
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_streak_freezes_user_date
    ON streak_freezes(user_id, freeze_date);
CREATE INDEX IF NOT EXISTS idx_streak_freezes_user_source
    ON streak_freezes(user_id, source);

COMMENT ON TABLE streak_freezes IS 'Streak freeze — bảo vệ chuỗi ngày niệm Phật khi user bỏ lỡ 1 ngày.';

-- 3. Bảng streak_freeze_quota — quota miễn phí hàng tháng (2 cái/tháng)
CREATE TABLE IF NOT EXISTS streak_freeze_quota (
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year_month      INTEGER         NOT NULL,
    -- Số freeze miễn phí đã dùng trong tháng (max 2)
    used_count      SMALLINT        NOT NULL DEFAULT 0,
    -- Timestamps
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, year_month)
);

COMMENT ON TABLE streak_freeze_quota IS 'Quota streak freeze miễn phí hàng tháng — max 2/tháng.';

-- 4. Trigger updated_at cho tu_hoc_goals + streak_freeze_quota
DROP TRIGGER IF EXISTS trg_tu_hoc_goals_set_updated_at ON tu_hoc_goals;
CREATE TRIGGER trg_tu_hoc_goals_set_updated_at
    BEFORE UPDATE ON tu_hoc_goals
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_streak_freeze_quota_set_updated_at ON streak_freeze_quota;
CREATE TRIGGER trg_streak_freeze_quota_set_updated_at
    BEFORE UPDATE ON streak_freeze_quota
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '037', 'v0.9.45 — Giai đoạn 58: Mục Tiêu Tu Học + Streak Bảo Vệ — tu_hoc_goals + streak_freezes + streak_freeze_quota.'
) ON CONFLICT (version) DO NOTHING;

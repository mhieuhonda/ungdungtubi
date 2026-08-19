-- =====================================================================
-- Migration 036 — Giai đoạn 57: Phần Thưởng Đăng Nhập Hàng Ngày
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Theo "ỨNG DỤNG TỪ BI.docx" mục I.1.b (Nút Niệm):
--     Một đơn vị niệm hợp lệ = 1 A. 1000 A = 1 K.
--   Giai đoạn 57: phần thưởng đăng nhập hàng ngày (Daily Login Reward):
--     - Ngày 1: +10 A (chào mừng)
--     - Ngày 2: +15 A
--     - Ngày 3: +20 A
--     - Ngày 4: +25 A
--     - Ngày 5: +30 A
--     - Ngày 6: +40 A
--     - Ngày 7 (streak 1 tuần): +100 A (đặc biệt)
--   Streak reset nếu bỏ ngày. Log vào balance_transactions (tx_type = 'daily_login').
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Bảng daily_login_rewards — log phần thưởng đăng nhập hàng ngày
CREATE TABLE IF NOT EXISTS daily_login_rewards (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reward_date     DATE            NOT NULL DEFAULT CURRENT_DATE,
    -- Số ngày streak (1 = ngày đầu, 7 = ngày thứ 7 trong tuần)
    streak_day      SMALLINT        NOT NULL CHECK (streak_day >= 1 AND streak_day <= 7),
    -- Số A thưởng
    reward_a        BIGINT          NOT NULL,
    -- Có bonus đặc biệt không (ngày 7)?
    is_bonus        BOOLEAN         NOT NULL DEFAULT false,
    -- Balance A sau khi thưởng
    balance_after   BIGINT         NOT NULL,
    -- Timestamp
    claimed_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, reward_date)
);

CREATE INDEX IF NOT EXISTS idx_daily_login_rewards_user_date
    ON daily_login_rewards(user_id, reward_date DESC);
CREATE INDEX IF NOT EXISTS idx_daily_login_rewards_date
    ON daily_login_rewards(reward_date);

COMMENT ON TABLE daily_login_rewards IS 'Log phần thưởng đăng nhập hàng ngày — mỗi user 1 row/ngày.';

-- 2. Bảng user_login_streaks — trạng thái streak hiện tại của user
CREATE TABLE IF NOT EXISTS user_login_streaks (
    user_id             UUID            PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- Số ngày streak hiện tại (1-7, reset về 1 sau ngày 7)
    current_streak      SMALLINT        NOT NULL DEFAULT 0,
    -- Streak dài nhất đã đạt
    max_streak          SMALLINT        NOT NULL DEFAULT 0,
    -- Ngày đăng nhập cuối
    last_login_date     DATE,
    -- Tổng số ngày đã nhận thưởng
    total_days_claimed  BIGINT          NOT NULL DEFAULT 0,
    -- Tổng A đã nhận từ daily login
    total_a_earned      BIGINT          NOT NULL DEFAULT 0,
    -- Timestamps
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE user_login_streaks IS 'Trạng thái streak đăng nhập của user.';

-- 3. Backfill: tạo row streaks cho users hiện tại (chưa có reward nào)
INSERT INTO user_login_streaks (user_id, current_streak, max_streak, last_login_date, total_days_claimed, total_a_earned)
SELECT u.id, 0, 0, NULL, 0, 0
FROM users u
WHERE NOT EXISTS (SELECT 1 FROM user_login_streaks s WHERE s.user_id = u.id);

-- 4. Trigger updated_at
DROP TRIGGER IF EXISTS trg_user_login_streaks_set_updated_at ON user_login_streaks;
CREATE TRIGGER trg_user_login_streaks_set_updated_at
    BEFORE UPDATE ON user_login_streaks
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '036', 'v0.9.45 — Giai đoạn 57: Phần Thưởng Đăng Nhập Hàng Ngày — daily_login_rewards + user_login_streaks.'
) ON CONFLICT (version) DO NOTHING;

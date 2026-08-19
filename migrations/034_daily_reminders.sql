-- =====================================================================
-- Migration 034 — Giai đoạn 55: Nhắc Nhở Tu Học Hàng Ngày
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Giai đoạn 55: thêm tùy chọn nhận nhắc nhở tu học hàng ngày qua:
--     - Notification trong app (default ON)
--     - Email (opt-in, nếu user có email từ Google OAuth)
--   Khi user có nhật ký tu học (practice_logs) và:
--     - Hôm nay chưa niệm Phật → nhắc vào 20:00 (giờ địa phương)
--     - Streak >= 7 ngày và hôm nay chưa niệm → nhắc nhẹ (đừng gãy streak)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Bảng notification_preferences — settings cho nhắc nhở
CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id                 UUID            PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- Bật/tắt nhắc nhở niệm Phật hàng ngày
    daily_niem_reminder     BOOLEAN         NOT NULL DEFAULT true,
    -- Bật/tắt nhắc nhở streak (khi sắp gãy)
    streak_warning          BOOLEAN         NOT NULL DEFAULT true,
    -- Bật/tắt nhắc nhở qua email
    email_reminders         BOOLEAN         NOT NULL DEFAULT false,
    -- Giờ nhắc (0-23, giờ địa phương — vd: 20 cho 8pm)
    reminder_hour           SMALLINT        NOT NULL DEFAULT 20
                                          CHECK (reminder_hour >= 0 AND reminder_hour <= 23),
    -- Channel: 'app' | 'email' | 'both'
    reminder_channel        VARCHAR(10)     NOT NULL DEFAULT 'app'
                                          CHECK (reminder_channel IN ('app', 'email', 'both')),
    -- Lần cuối remind đã gửi (tránh spam nhiều lần/ngày)
    last_reminder_sent_at   TIMESTAMPTZ,
    -- Timestamps
    created_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE notification_preferences IS 'Cài đặt nhắc nhở tu học hàng ngày của user.';

-- 2. Backfill: tạo preferences mặc định cho users hiện tại
INSERT INTO notification_preferences (user_id)
SELECT id FROM users
WHERE NOT EXISTS (SELECT 1 FROM notification_preferences np WHERE np.user_id = users.id);

-- 3. Bảng daily_reminder_log — log các nhắc nhở đã gửi (chống spam)
CREATE TABLE IF NOT EXISTS daily_reminder_log (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reminder_date   DATE            NOT NULL DEFAULT CURRENT_DATE,
    reminder_type   VARCHAR(30)     NOT NULL CHECK (reminder_type IN (
                        'daily_niem', 'streak_warning', 'goal_due', 'monthly_summary'
                    )),
    channel         VARCHAR(10)     NOT NULL CHECK (channel IN ('app', 'email')),
    sent_at         TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    -- Trạng thái gửi (success / failed)
    status          VARCHAR(20)     NOT NULL DEFAULT 'sent',
    error_message   TEXT            NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_daily_reminder_log_user_date
    ON daily_reminder_log(user_id, reminder_date);
CREATE INDEX IF NOT EXISTS idx_daily_reminder_log_date_type
    ON daily_reminder_log(reminder_date, reminder_type);

COMMENT ON TABLE daily_reminder_log IS 'Log các nhắc nhở đã gửi — chống spam nhiều lần/ngày.';

-- 4. Trigger updated_at
DROP TRIGGER IF EXISTS trg_notification_preferences_set_updated_at ON notification_preferences;
CREATE TRIGGER trg_notification_preferences_set_updated_at
    BEFORE UPDATE ON notification_preferences
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '034', 'v0.9.45 — Giai đoạn 55: Nhắc Nhở Tu Học Hàng Ngày — notification_preferences + daily_reminder_log.'
) ON CONFLICT (version) DO NOTHING;

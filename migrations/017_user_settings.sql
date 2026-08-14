-- Ứng Dụng Từ Bi - Migration 017: User Settings + Navigation Overhaul
-- Giai đoạn 18 (v0.9.14): Thêm bảng user_settings cho trang /cai-dat
--
-- Mục tiêu:
--   * Tạo bảng `user_settings` — cài đặt cá nhân của user
--   * Mở rộng notifications — thêm type 'system_announcement'
--   * Thêm cột `last_settings_update` vào users (audit)
--   * Seed một vài default settings cho user hiện có

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Bảng user_settings — cài đặt cá nhân
-- ══════════════════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS user_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- Riêng tư
    profile_visibility VARCHAR(20) NOT NULL DEFAULT 'public',  -- public | friends | private
    show_balance BOOLEAN NOT NULL DEFAULT true,
    show_activity BOOLEAN NOT NULL DEFAULT true,
    show_email BOOLEAN NOT NULL DEFAULT false,
    -- Thông báo
    notify_friends BOOLEAN NOT NULL DEFAULT true,
    notify_mail BOOLEAN NOT NULL DEFAULT true,
    notify_dm BOOLEAN NOT NULL DEFAULT true,
    notify_group BOOLEAN NOT NULL DEFAULT true,
    notify_system BOOLEAN NOT NULL DEFAULT true,
    -- Giao diện
    theme VARCHAR(20) NOT NULL DEFAULT 'lotus',  -- lotus | dark | minimal
    language VARCHAR(10) NOT NULL DEFAULT 'vi',
    -- Chat
    auto_join_global_chat BOOLEAN NOT NULL DEFAULT false,
    chat_sound_enabled BOOLEAN NOT NULL DEFAULT true,
    -- Niệm Phật
    niem_sound_enabled BOOLEAN NOT NULL DEFAULT true,
    niem_auto_convert_k BOOLEAN NOT NULL DEFAULT true,  -- tự động 1000 A = 1 K
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE user_settings IS 'Cài đặt cá nhân của user — Giai đoạn 18 (v0.9.14)';
COMMENT ON COLUMN user_settings.profile_visibility IS 'public | friends | private — kiểm soát ai xem được hồ sơ';
COMMENT ON COLUMN user_settings.theme IS 'lotus (mặc định) | dark | minimal — giao diện UI';

-- Trigger cập nhật updated_at tự động
CREATE OR REPLACE FUNCTION trigger_update_user_settings()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_user_settings_updated
    BEFORE UPDATE ON user_settings
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_user_settings();

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Seed default settings cho user hiện có (chưa có row)
-- ══════════════════════════════════════════════════════════════════════════════
INSERT INTO user_settings (user_id)
SELECT id FROM users
WHERE NOT EXISTS (SELECT 1 FROM user_settings WHERE user_id = users.id);

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Thêm notification type mới cho system_announcement
-- ══════════════════════════════════════════════════════════════════════════════
-- (notifications table đã có cột type VARCHAR — không cần ALTER, chỉ mở rộng giá trị)

-- ══════════════════════════════════════════════════════════════════════════════
-- 4. Index
-- ══════════════════════════════════════════════════════════════════════════════
CREATE INDEX IF NOT EXISTS idx_user_settings_theme ON user_settings(theme);
CREATE INDEX IF NOT EXISTS idx_user_settings_visibility ON user_settings(profile_visibility);

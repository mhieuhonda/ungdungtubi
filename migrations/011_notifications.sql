-- Ứng Dụng Từ Bi - Migration 011: Notifications (Hệ thống thông báo)
-- Giai đoạn 9 (v0.9.5): Notification center cho lời mời kết bạn, tin nhắn mới, v.v.
--
-- Mục tiêu:
--   * Bảng notifications — thông báo gửi đến user
--   * Type: friend_request | friend_accept | mail | dm | system
--   * Payload JSON linh hoạt cho mỗi loại
--   * Đánh dấu đã đọc/chưa đọc
--   * Index cho truy vấn thông báo theo user + thời gian

CREATE TABLE IF NOT EXISTS notifications (
    id           UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type         VARCHAR(50)   NOT NULL,
    -- Actor: user gây ra thông báo (vd: người gửi lời mời kết bạn)
    actor_id     UUID          REFERENCES users(id) ON DELETE SET NULL,
    -- Payload JSON linh hoạt — ví dụ: {"friendship_id": "...", "message": "..."}
    payload      JSONB,
    is_read      BOOLEAN       NOT NULL DEFAULT false,
    read_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_notification_type CHECK (
        type IN ('friend_request', 'friend_accept', 'friend_decline',
                 'mail', 'dm', 'system', 'group_invite')
    )
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_unread
    ON notifications(user_id, is_read, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_user_created
    ON notifications(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_type
    ON notifications(type);

COMMENT ON TABLE notifications IS 'Thông báo gửi đến user — Giai đoạn 9 v0.9.5';
COMMENT ON COLUMN notifications.type IS 'friend_request | friend_accept | friend_decline | mail | dm | system | group_invite';
COMMENT ON COLUMN notifications.payload IS 'JSONB payload linh hoạt cho mỗi loại thông báo';

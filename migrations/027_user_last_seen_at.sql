-- Ứng Dụng Từ Bi - Migration 027: User Activity Tracking (last_seen_at)
-- Giai đoạn 43 (v0.9.39): Fix bug "5 user active nhưng vào quản lý thành viên không thấy ai"
--
-- Mục tiêu:
--   * Thêm cột `last_seen_at TIMESTAMPTZ` vào users — track thời điểm user
--     active gần nhất (update qua /api/heartbeat).
--   * Trước v0.9.39: admin stats `active_users` đếm `WHERE is_active` (tức là
--     "không bị ban") chứ không phải "đang online" → admin thấy "5 user đang
--     hoạt động" nhưng vào /admin/thanh-vien không thấy ai online. Heartbeat
--     handler cũng không làm gì cả (chỉ trả về `{"status":"ok"}`).
--   * v0.9.39 fix:
--       - Heartbeat handler update `last_seen_at = NOW()` cho user đã login.
--       - Admin stats `active_users` đếm `WHERE last_seen_at > NOW() - INTERVAL '5 min'`.
--       - Admin user list hiển thị `last_seen_at` thay vì `MAX(sessions.created_at)`
--         (lúc tạo session = lúc login, không phải lúc user active gần nhất).
--
-- Lưu ý: Safety schema check (src/db/mod.rs::ensure_schema_safety) cũng chạy
-- idempotent DDL tương tự để đảm bảo cột này tồn tại ngay cả khi migration
-- chưa chạy (checksum mismatch, partial deploy, manual rollback).

-- ══════════════════════════════════════════════════════════════════════════════
-- 1. Thêm cột last_seen_at vào users
-- ══════════════════════════════════════════════════════════════════════════════
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

COMMENT ON COLUMN users.last_seen_at IS 'Thời điểm user active gần nhất — update qua /api/heartbeat (mỗi 10 phút). Dùng cho admin stats "active_users" (last_seen_at > NOW() - 5 phút).';

-- ══════════════════════════════════════════════════════════════════════════════
-- 2. Index cho last_seen_at để query "active trong 5 phút" nhanh
-- ══════════════════════════════════════════════════════════════════════════════
CREATE INDEX IF NOT EXISTS idx_users_last_seen_at
    ON users(last_seen_at DESC)
    WHERE last_seen_at IS NOT NULL;

-- ══════════════════════════════════════════════════════════════════════════════
-- 3. Seed last_seen_at cho user hiện có (lấy từ MAX(sessions.created_at))
--    — để tránh "rỗng" ban đầu, dùng session gần nhất làm baseline.
--    Sau khi user login + heartbeat chạy, last_seen_at sẽ được update real-time.
-- ══════════════════════════════════════════════════════════════════════════════
UPDATE users u
SET last_seen_at = sub.max_session_at,
    updated_at = NOW()
FROM (
    SELECT user_id, MAX(created_at) AS max_session_at
    FROM sessions
    GROUP BY user_id
) sub
WHERE u.id = sub.user_id
  AND u.last_seen_at IS NULL;

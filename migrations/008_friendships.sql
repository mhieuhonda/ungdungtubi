-- Ứng Dụng Từ Bi - Migration 008: Friendships (Bạn Bè)
-- Giai đoạn 9 (v0.9.5): Hệ thống kết bạn — gửi/nhận/hủy lời mời kết bạn
--
-- Mục tiêu:
--   * Bảng friendships lưu quan hệ bạn bè giữa 2 user
--   * Trạng thái: pending (đang chờ) → accepted (đã chấp nhận) | blocked (chặn) | declined (từ chối)
--   * Index cho truy vấn danh sách bạn bè + lời mời đang chờ
--   * Trigger updated_at tự động
--
-- Theo thiết kế trong HieuLouis/ỨNG DỤNG TỪ BI.docx mục 3 + sheet `06_Ban_Be`:
--   * BB-01 Kết bạn: gửi lời mời, chấp nhận, từ chối, hủy kết bạn
--   * Block user: chặn nhận tin nhắn/lời mời từ user cụ thể

CREATE TABLE IF NOT EXISTS friendships (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id    UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee_id    UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          VARCHAR(20)  NOT NULL DEFAULT 'pending',  -- pending | accepted | blocked | declined
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    -- Đảm bảo chỉ có 1 bản ghi friendship giữa 2 user (regardless of direction)
    CONSTRAINT uq_friendship_pair UNIQUE (requester_id, addressee_id),
    -- Không thể kết bạn với chính mình
    CONSTRAINT chk_no_self_friend CHECK (requester_id <> addressee_id)
);

-- Index cho truy vấn danh sách bạn bè của một user (cả 2 chiều)
CREATE INDEX IF NOT EXISTS idx_friendships_addressee ON friendships(addressee_id, status);
CREATE INDEX IF NOT EXISTS idx_friendships_requester ON friendships(requester_id, status);
CREATE INDEX IF NOT EXISTS idx_friendships_status ON friendships(status);

-- Trigger updated_at tự động
CREATE OR REPLACE FUNCTION trg_friendships_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS friendships_updated_at ON friendships;
CREATE TRIGGER friendships_updated_at
    BEFORE UPDATE ON friendships
    FOR EACH ROW
    EXECUTE FUNCTION trg_friendships_updated_at();

COMMENT ON TABLE friendships IS 'Quan hệ bạn bè giữa 2 user (Giai đoạn 9 — v0.9.5)';
COMMENT ON COLUMN friendships.status IS 'pending | accepted | blocked | declined';

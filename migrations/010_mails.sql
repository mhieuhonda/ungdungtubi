-- Ứng Dụng Từ Bi - Migration 010: Mails (Gửi thư)
-- Giai đoạn 9 (v0.9.5): Hệ thống gửi thư riêng (long-form, không realtime)
--
-- Mục tiêu:
--   * Bảng mails — thư gửi giữa 2 user (subject + body dài)
--   * Hộp thư đến/đi
--   * Đánh dấu đã đọc/chưa đọc
--   * Index cho truy vấn hộp thư theo recipient + thời gian
--
-- Theo thiết kế trong HieuLouis/ỨNG DỤNG TỪ BI.docx mục 3:
--   * BB-03 Gửi thư: thư dài, không realtime, có subject + body

CREATE TABLE IF NOT EXISTS mails (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id     UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id  UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject       VARCHAR(200)  NOT NULL,
    body          TEXT          NOT NULL,
    is_read       BOOLEAN       NOT NULL DEFAULT false,
    read_at       TIMESTAMPTZ,
    is_active     BOOLEAN       NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    -- Không thể gửi thư cho chính mình
    CONSTRAINT chk_no_self_mail CHECK (sender_id <> recipient_id)
);

CREATE INDEX IF NOT EXISTS idx_mails_recipient_created
    ON mails(recipient_id, is_read, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_mails_sender_created
    ON mails(sender_id, created_at DESC);

COMMENT ON TABLE mails IS 'Thư riêng giữa 2 user (long-form, không realtime) — Giai đoạn 9 v0.9.5';
COMMENT ON COLUMN mails.subject IS 'Tiêu đề thư (tối đa 200 ký tự)';
COMMENT ON COLUMN mails.body IS 'Nội dung thư (TEXT, không giới hạn độ dài)';
COMMENT ON COLUMN mails.is_read IS 'true nếu người nhận đã đọc thư';

-- Ứng Dụng Từ Bi - Migration 006: Live Chat trong Nhóm
-- Giai đoạn 7 (v0.9.2): Live Chat thời gian thực (WebSocket) trong nhóm
--
-- Mục tiêu:
--   * Tạo bảng group_chat_messages — tin nhắn real-time trong nhóm
--   * Theo thiết kế trong HieuLouis/Giao Diện Cộng Đồng Trong Ứng Dụng.docx:
--       "Live Chat kết hợp với list chủ đề cho mỗi nhóm.
--        Live Chat chỉ để giao lưu, kết bạn, tán gẫu, hỏi nhanh.
--        Mọi nội dung có giá trị sẽ được chuyển thành Chủ đề."
--   * Live Chat panel chiếm ~30-40% chiều cao, list Chủ Đề chiếm 60-70%
--   * Index cho truy vấn chat history theo nhóm + thời gian
--
-- Phân biệt với bảng `comments`:
--   * comments: bình luận trên Chủ Đề (diễn đàn, lưu trữ tri thức)
--   * group_chat_messages: chat real-time trong Nhóm (kết nối, giao lưu)

CREATE TABLE IF NOT EXISTS group_chat_messages (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID         NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    author_id   UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body        VARCHAR(500) NOT NULL,
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Index cho truy vấn chat history phổ biến:
--   * Theo group + thời gian giảm dần (load 50 tin mới nhất)
--   * Theo group + trước một message_id (pagination)
CREATE INDEX IF NOT EXISTS idx_group_chat_messages_group_created
    ON group_chat_messages(group_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_group_chat_messages_author
    ON group_chat_messages(author_id);

COMMENT ON TABLE group_chat_messages IS 'Tin nhắn Live Chat real-time trong nhóm (WebSocket)';
COMMENT ON COLUMN group_chat_messages.body IS 'Nội dung tin nhắn (tối đa 500 ký tự, plain text)';
COMMENT ON COLUMN group_chat_messages.is_active IS 'false nếu bị mod xoá (giữ lại để audit)';

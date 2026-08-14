-- Ứng Dụng Từ Bi - Migration 009: Conversations + Direct Messages
-- Giai đoạn 9 (v0.9.5): Nhắn tin 1-1 realtime qua WebSocket
--
-- Mục tiêu:
--   * Bảng conversations — đại diện cho một cuộc hội thoại (direct hoặc group)
--   * Bảng conversation_participants — danh sách user tham gia conversation
--   * Bảng direct_messages — tin nhắn trong conversation
--   * Index cho truy vấn history theo conversation + thời gian
--
-- Theo thiết kế trong HieuLouis/ỨNG DỤNG TỪ BI.docx mục 3:
--   * BB-02 Nhắn tin: chat 1-1 realtime qua WebSocket
--   * Endpoint WS: /ws/ban-be/chat/{conversation_id}
--   * Reuse ChatHub pattern (broadcast per-conversation)

CREATE TABLE IF NOT EXISTS conversations (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    type        VARCHAR(20)  NOT NULL DEFAULT 'direct',  -- direct | group
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_conversation_type CHECK (type IN ('direct', 'group'))
);

CREATE TABLE IF NOT EXISTS conversation_participants (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID         NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_read_at    TIMESTAMPTZ,
    -- Mỗi user chỉ tham gia 1 lần vào mỗi conversation
    CONSTRAINT uq_conversation_user UNIQUE (conversation_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_participants_user
    ON conversation_participants(user_id);
CREATE INDEX IF NOT EXISTS idx_conversation_participants_conv
    ON conversation_participants(conversation_id);

CREATE TABLE IF NOT EXISTS direct_messages (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID         NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    author_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body            VARCHAR(1000) NOT NULL,
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_direct_messages_conv_created
    ON direct_messages(conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_direct_messages_author
    ON direct_messages(author_id);

-- Trigger updated_at cho conversations
CREATE OR REPLACE FUNCTION trg_conversations_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS conversations_updated_at ON conversations;
CREATE TRIGGER conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION trg_conversations_updated_at();

COMMENT ON TABLE conversations IS 'Cuộc hội thoại 1-1 hoặc nhóm (Giai đoạn 9 — v0.9.5)';
COMMENT ON TABLE conversation_participants IS 'Thành viên tham gia conversation';
COMMENT ON TABLE direct_messages IS 'Tin nhắn trong conversation (max 1000 ký tự)';

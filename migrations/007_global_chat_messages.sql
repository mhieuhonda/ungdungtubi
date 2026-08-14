-- Migration 007: Global Chat Messages (v0.9.3)
-- Platform-wide chat that works across the entire app
-- Only store the 500 most recent messages (auto-prune via application logic)

CREATE TABLE global_chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body VARCHAR(500) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_global_chat_created ON global_chat_messages(created_at DESC);
CREATE INDEX idx_global_chat_author ON global_chat_messages(author_id);

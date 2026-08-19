-- Migration 041 — Giai đoạn 63: Bao Lì Xì Từ Bi
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx mục V (Thương Thành):
--   - Bao Lì Xì Từ Bi, giá 10K: tạo 1 bao lì xì 10K chia cho nhiều người.
--   - Bao Lì Xì Đại Bi, giá 100K: tạo 1 bao lì xì 100K chia cho nhiều người.
--   - Mở bao = nhận 1 phần ngẫu nhiên; hết tiền thì bao hết.

CREATE TABLE IF NOT EXISTS red_envelopes (
    id              BIGSERIAL    PRIMARY KEY,
    creator_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    envelope_type  VARCHAR(20)  NOT NULL,  -- 'tubi_10k' | 'dai_bi_100k'
    total_k         BIGINT       NOT NULL,  -- tổng K ban đầu (10 or 100)
    remaining_k     BIGINT       NOT NULL,  -- K còn lại
    total_claims    SMALLINT     NOT NULL DEFAULT 0,
    max_claims      SMALLINT     NOT NULL DEFAULT 20,  -- tối đa người nhận
    message         VARCHAR(200),
    scope           VARCHAR(20)  NOT NULL DEFAULT 'public',  -- 'public' | 'friends' | 'group'
    target_group_id BIGINT,
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    expires_at      TIMESTAMPTZ  NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_red_envelopes_active_recent ON red_envelopes(created_at DESC) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_red_envelopes_creator ON red_envelopes(creator_id, created_at DESC);

-- Lịch sử người nhận
CREATE TABLE IF NOT EXISTS red_envelope_claims (
    id           BIGSERIAL    PRIMARY KEY,
    envelope_id  BIGINT      NOT NULL REFERENCES red_envelopes(id) ON DELETE CASCADE,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_k     BIGINT       NOT NULL,  -- K user nhận được
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (envelope_id, user_id)  -- mỗi user chỉ claim 1 lần/envelope
);

CREATE INDEX IF NOT EXISTS idx_red_envelope_claims_user ON red_envelope_claims(user_id, created_at DESC);

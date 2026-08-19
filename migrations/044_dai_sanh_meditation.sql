-- Migration 044 — Giai đoạn 66: Đại Sảnh + Cộng Tu
-- Theo tài liệu Hệ Thống Và Chức Năng Chi Tiết.docx mục I.5 (Đại Sảnh):
--   "Đại Sảnh là vị trí trung tâm. Ở giữa nền là một bông sen lớn, bên cạnh là tượng Phật.
--    Đây là nơi để mọi người tự tu và cộng tu.
--    Đại Sảnh có 10 bồ đoàn để ngồi thiền. Vị trí chủ tọa được đặt gần tượng Phật."
-- Giai đoạn 66: phiên bản đơn giản — cộng tu session, 10 ghế, +bonus A khi ngồi chung.

-- Cộng tu session (admin hoặc user tạo)
CREATE TABLE IF NOT EXISTS meditation_sessions (
    id           BIGSERIAL    PRIMARY KEY,
    host_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        VARCHAR(200) NOT NULL,
    description  TEXT,
    start_at     TIMESTAMPTZ  NOT NULL,
    duration_minutes SMALLINT NOT NULL DEFAULT 30,
    max_seats    SMALLINT     NOT NULL DEFAULT 10,
    is_active    BOOLEAN      NOT NULL DEFAULT true,
    is_cancelled BOOLEAN      NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_meditation_sessions_active_recent ON meditation_sessions(start_at) WHERE is_active = true AND is_cancelled = false;

-- Thành viên tham gia cộng tu
CREATE TABLE IF NOT EXISTS meditation_participants (
    id            BIGSERIAL    PRIMARY KEY,
    session_id    BIGINT       NOT NULL REFERENCES meditation_sessions(id) ON DELETE CASCADE,
    user_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seat_number   SMALLINT     NOT NULL,  -- 1-10 (1 = chủ tọa gần tượng Phật)
    joined_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    a_earned     BIGINT       NOT NULL DEFAULT 0,
    UNIQUE (session_id, user_id),
    UNIQUE (session_id, seat_number)
);

CREATE INDEX IF NOT EXISTS idx_meditation_participants_session ON meditation_participants(session_id);

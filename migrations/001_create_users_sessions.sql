-- Ứng Dụng Từ Bi - Migration 001: Users & Sessions
-- Giai đoạn 1: Nền móng hạ tầng cốt lõi

-- Bảng users: Thành viên ứng dụng
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    rank VARCHAR(50) NOT NULL DEFAULT 'new',
    a_balance BIGINT NOT NULL DEFAULT 0,
    k_balance BIGINT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Bảng sessions: Phiên đăng nhập
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(255) PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- Index
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Comment
COMMENT ON TABLE users IS 'Thành viên Ứng Dụng Từ Bi';
COMMENT ON COLUMN users.a_balance IS 'Niệm Lực A: 1 lần niệm = 1 A, 1000 A = 1 K';
COMMENT ON COLUMN users.k_balance IS 'Tiền K: đơn vị giao dịch cơ bản';
COMMENT ON COLUMN users.rank IS 'Cấp bậc: new, normal, common, good, very_good, great, excellent, benevolent, tycoon';

-- Migration 048 — Giai đoạn 70: Bảng Vinh Danh (Hall of Fame)
-- Theo tài liệu ỨNG DỤNG TỪ BI.docx mục II.4 (Hệ Thống Thành Tích):
--   Bao gồm: BXH Niệm Phật tháng, BXH Niệm Phật tổng, BXH Tài Phú K, BXH Niệm Lực A,
--   BXH Phiếu Từ Bi, BXH Từ Bi.
-- Giai đoạn 70: trang Bảng Vinh Danh tổng hợp — vinh danh top contributors theo nhiều tiêu chí.

-- Bảng lưu top contributors theo tuần (lưu trữ lịch sử)
CREATE TABLE IF NOT EXISTS hall_of_fame_entries (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category     VARCHAR(30)  NOT NULL,  -- 'niem_month' | 'niem_total' | 'a' | 'k' | 'bi' | 'tu_si' | 'friend' | 'topic'
    rank_position SMALLINT    NOT NULL,  -- 1, 2, 3, ...
    score        BIGINT       NOT NULL,
    period_label VARCHAR(20),  -- vd: '2026-W33' (tuần) hoặc '2026-08' (tháng) hoặc 'all-time'
    snapshot_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    UNIQUE (category, period_label, rank_position)
);

CREATE INDEX IF NOT EXISTS idx_hof_entries_recent ON hall_of_fame_entries(snapshot_at DESC);
CREATE INDEX IF NOT EXISTS idx_hof_entries_category ON hall_of_fame_entries(category, rank_position);

-- Vinh danh vĩnh viễn (admin phong tặng)
CREATE TABLE IF NOT EXISTS hall_of_fame_honors (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        VARCHAR(100) NOT NULL,  -- vd: 'Đại Từ Bi', 'Tu Sĩ Xuất Sắc'
    description  TEXT,
    awarded_by   UUID         REFERENCES users(id),
    awarded_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    is_active    BOOLEAN      NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_hof_honors_recent ON hall_of_fame_honors(awarded_at DESC) WHERE is_active = true;

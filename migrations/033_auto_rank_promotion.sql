-- =====================================================================
-- Migration 033 — Giai đoạn 54: Hệ Thống Cấp Bậc Tự Động
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Theo tài liệu "ỨNG DỤNG TỪ BI.docx" mục II.3.b (Hệ thống cấp bậc):
--     Người Mới (new): Vừa tham gia, chưa có hoạt động.
--     Người Thường (normal): Đã cập nhật đầy đủ hồ sơ.
--     Người Bình Thường (common): Có ít nhất 10 người bạn.
--     Người Tốt (good): Đóng góp tối thiểu 100 K cho Quỹ Từ Bi.
--     Người Khá Tốt (very_good): Đóng góp từ 500 K.
--     Người Rất Tốt (great): Đóng góp từ 1000 K.
--     Người Cực Kỳ Tốt (excellent): Đóng góp từ 5000 K.
--     Thiện Nhân (benevolent): Đóng góp từ 10.000 K.
--     Đại Gia (tycoon): Những Thiện Nhân nằm trong Top 10 tài phú.
--
--   Giai đoạn 54: tự động cập nhật users.rank khi user đạt điều kiện.
--   Đồng thời ghi log vào bảng member_rank_history để admin xem được lịch sử.
--   Rank codes đồng bộ với migration 003_member_profile_ranks.sql.
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Bảng member_rank_history — lịch sử thay đổi rank của user
CREATE TABLE IF NOT EXISTS member_rank_history (
    id              BIGSERIAL       PRIMARY KEY,
    user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_rank       VARCHAR(40)     NOT NULL DEFAULT '',
    to_rank         VARCHAR(40)     NOT NULL,
    reason          VARCHAR(60)     NOT NULL DEFAULT 'auto',
                    -- 'auto' | 'admin_change' | 'signup' | 'backfill' | 'manual'
    changed_by      UUID            REFERENCES users(id) ON DELETE SET NULL,
    note            TEXT            NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_member_rank_history_user ON member_rank_history(user_id);
CREATE INDEX IF NOT EXISTS idx_member_rank_history_created ON member_rank_history(created_at DESC);

COMMENT ON TABLE member_rank_history IS 'Lịch sử thay đổi cấp bậc thành viên (auto + admin_change).';

-- 2. Backfill: rank "new" cho users chưa có rank hoặc rank trống
UPDATE users
SET rank = 'new', updated_at = NOW()
WHERE rank IS NULL OR rank = '';

-- 3. INSERT INTO member_rank_history cho users hiện tại (1 row mỗi user, đánh dấu "backfill")
INSERT INTO member_rank_history (user_id, from_rank, to_rank, reason, note)
SELECT id, '', COALESCE(NULLIF(rank, ''), 'new'), 'backfill', 'Backfill initial rank — Giai đoạn 54'
FROM users
WHERE NOT EXISTS (
    SELECT 1 FROM member_rank_history h WHERE h.user_id = users.id
);

-- 4. Function: tự động tính rank dựa trên conditions
--    Trả về rank code đồng bộ với migration 003:
--    'new' | 'normal' | 'common' | 'good' | 'very_good' | 'great' | 'excellent' | 'benevolent' | 'tycoon'
CREATE OR REPLACE FUNCTION calculate_member_rank(p_user_id UUID)
RETURNS VARCHAR(40) AS $$
DECLARE
    v_k_balance        BIGINT;
    v_total_donated    BIGINT;
    v_friend_count     BIGINT;
    v_profile_complete BOOLEAN;
    v_top10_count      INTEGER;
    v_rank             VARCHAR(40);
BEGIN
    -- Lấy K balance
    SELECT k_balance INTO v_k_balance FROM users WHERE id = p_user_id;

    -- Tổng đóng góp Quỹ Từ Bi (từ bảng fund_donations)
    SELECT COALESCE(SUM(amount_k), 0) INTO v_total_donated
    FROM fund_donations WHERE user_id = p_user_id;

    -- Số bạn bè (đã chấp nhận)
    SELECT COUNT(*) INTO v_friend_count
    FROM friendships
    WHERE (requester_id = p_user_id OR addressee_id = p_user_id)
      AND status = 'accepted';

    -- Profile đầy đủ? (display_name + gender + ít nhất 1 trong 4: phap_danh/phap_hieu/but_danh/bio)
    SELECT (
        display_name IS NOT NULL AND display_name != ''
        AND gender IS NOT NULL AND gender != ''
        AND (
            (phap_danh IS NOT NULL AND phap_danh != '')
            OR (phap_hieu IS NOT NULL AND phap_hieu != '')
            OR (but_danh IS NOT NULL AND but_danh != '')
            OR (bio IS NOT NULL AND bio != '')
        )
    ) INTO v_profile_complete
    FROM users WHERE id = p_user_id;

    -- Top 10 tài phú K (đếm xem user có trong top 10 không)
    WITH top_users AS (
        SELECT id FROM users
        WHERE is_active = true
        ORDER BY k_balance DESC LIMIT 10
    )
    SELECT COUNT(*) INTO v_top10_count FROM top_users WHERE id = p_user_id;

    -- Tính rank theo tài liệu II.3.b
    -- (tycoon ưu tiên cao nhất — phải vừa top 10 vừa đóng góp >= 10000)
    IF v_top10_count > 0 AND v_total_donated >= 10000 THEN
        v_rank := 'tycoon';
    ELSIF v_total_donated >= 10000 THEN
        v_rank := 'benevolent';
    ELSIF v_total_donated >= 5000 THEN
        v_rank := 'excellent';
    ELSIF v_total_donated >= 1000 THEN
        v_rank := 'great';
    ELSIF v_total_donated >= 500 THEN
        v_rank := 'very_good';
    ELSIF v_total_donated >= 100 THEN
        v_rank := 'good';
    ELSIF v_friend_count >= 10 THEN
        v_rank := 'common';
    ELSIF v_profile_complete THEN
        v_rank := 'normal';
    ELSE
        v_rank := 'new';
    END IF;

    RETURN v_rank;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION calculate_member_rank(UUID) IS
'Tính cấp bậc thành viên tự động dựa trên đóng góp Quỹ Từ Bi + bạn bè + profile.';

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '033', 'v0.9.45 — Giai đoạn 54: Hệ Thống Cấp Bậc Tự Động — member_rank_history + calculate_member_rank().'
) ON CONFLICT (version) DO NOTHING;

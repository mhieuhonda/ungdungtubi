-- =====================================================================
-- Migration 038 — Giai đoạn 59: Chủ Đề Nổi Bật Algorithm + Trang Khám Phá
-- v0.9.45 — 2026-08-19
--
-- Mục tiêu:
--   Giai đoạn 59: thêm thuật toán "chủ đề nổi bật" (hot topics) dựa trên:
--     - Số bình luận trong 24h qua (weight 40%)
--     - Số bình luận trong 7 ngày qua (weight 20%)
--     - Số thành viên nhóm (weight 15%)
--     - Số like / phản hồi (weight 15%)
--     - Độ mới (weight 10%)
--   Trang /cong-dong/kham-pha sẽ hiển thị:
--     - Top 10 hot topics hôm nay
--     - Nhóm nổi bật
--     - Sách được tặng hoa nhiều
--     - Nhạc mới được duyệt
--     - Thành viên mới tích cực
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- 0. Đảm bảo bảng migration_log tồn tại
CREATE TABLE IF NOT EXISTS migration_log (
    version      VARCHAR(20) PRIMARY KEY,
    description  TEXT NOT NULL,
    applied_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 1. Thêm cột hot_score + last_activity_at vào topics
ALTER TABLE topics ADD COLUMN IF NOT EXISTS hot_score DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE topics ADD COLUMN IF NOT EXISTS last_activity_at TIMESTAMPTZ;
COMMENT ON COLUMN topics.hot_score IS 'Điểm nổi bật — tính theo thuật toán Giai đoạn 59.';
COMMENT ON COLUMN topics.last_activity_at IS 'Thời điểm hoạt động cuối (comment mới nhất).';

-- 2. Index cho hot_score + last_activity_at để query nhanh
CREATE INDEX IF NOT EXISTS idx_topics_hot_score ON topics(hot_score DESC);
CREATE INDEX IF NOT EXISTS idx_topics_last_activity ON topics(last_activity_at DESC);

-- 3. Function: tính hot_score cho 1 topic (dựa trên comment count 24h/7d + age + group_size)
--    Score = (comments_24h * 4 + comments_7d * 2 + group_size * 0.5 + like_count * 1)
--          / (age_hours + 2)^0.5
--    Tác vụ: tunable weight, có thể điều chỉnh trong tương lai.
CREATE OR REPLACE FUNCTION calculate_topic_hot_score(p_topic_id UUID)
RETURNS DOUBLE PRECISION AS $$
DECLARE
    v_comments_24h   BIGINT;
    v_comments_7d    BIGINT;
    v_group_size     BIGINT;
    v_age_hours      DOUBLE PRECISION;
    v_created_at     TIMESTAMPTZ;
BEGIN
    -- Số comment trong 24h qua
    SELECT COUNT(*) INTO v_comments_24h
    FROM comments
    WHERE topic_id = p_topic_id
      AND created_at > NOW() - INTERVAL '24 hours'
      AND is_active = true;

    -- Số comment trong 7 ngày qua (trừ 24h)
    SELECT COUNT(*) INTO v_comments_7d
    FROM comments
    WHERE topic_id = p_topic_id
      AND created_at > NOW() - INTERVAL '7 days'
      AND created_at <= NOW() - INTERVAL '24 hours'
      AND is_active = true;

    -- Group size (số thành viên của nhóm chứa topic)
    SELECT COUNT(*) INTO v_group_size
    FROM group_members gm
    JOIN topics t ON t.group_id = gm.group_id
    WHERE t.id = p_topic_id AND gm.status = 'approved';

    -- Tuổi topic (giờ)
    SELECT created_at INTO v_created_at FROM topics WHERE id = p_topic_id;
    IF v_created_at IS NULL THEN
        RETURN 0;
    END IF;
    v_age_hours := EXTRACT(EPOCH FROM (NOW() - v_created_at)) / 3600.0;

    -- Tính score
    RETURN (
        (v_comments_24h * 4.0) +
        (v_comments_7d  * 2.0) +
        (v_group_size   * 0.5)
    ) / (POWER(v_age_hours + 2.0, 0.5));
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION calculate_topic_hot_score(UUID) IS
'Tính điểm nổi bật cho topic — dựa trên comments 24h/7d + group_size + age.';

-- 4. Thêm cột is_hot + hot_score_at vào topics (cache flag)
ALTER TABLE topics ADD COLUMN IF NOT EXISTS is_hot BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE topics ADD COLUMN IF NOT EXISTS hot_score_at TIMESTAMPTZ;
COMMENT ON COLUMN topics.is_hot IS 'Flag cache — true nếu topic nằm trong top 10 hot.';
COMMENT ON COLUMN topics.hot_score_at IS 'Thời điểm hot_score được tính lần cuối.';

CREATE INDEX IF NOT EXISTS idx_topics_is_hot ON topics(is_hot) WHERE is_hot = true;

-- 5. Migration log
INSERT INTO migration_log (version, description) VALUES (
    '038', 'v0.9.45 — Giai đoạn 59: Chủ Đề Nổi Bật Algorithm — topics.hot_score + calculate_topic_hot_score() + /cong-dong/kham-pha.'
) ON CONFLICT (version) DO NOTHING;

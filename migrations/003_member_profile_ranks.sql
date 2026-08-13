-- Ứng Dụng Từ Bi - Migration 003: Hồ sơ thành viên & Hệ thống cấp bậc
-- Giai đoạn 4 (v0.4): Hồ sơ cá nhân + bảng member_ranks
--
-- Mục tiêu:
--   * Thêm các trường hồ sơ: pháp danh, pháp hiệu, bút danh, giới tính, bio
--   * Tạo bảng member_ranks mô tả chi tiết các cấp bậc
--   * Seed dữ liệu cấp bậc mặc định
--   * Tạo index cho rank để tra cứu nhanh

-- 1. Thêm các cột hồ sơ cá nhân
ALTER TABLE users ADD COLUMN IF NOT EXISTS phap_danh   VARCHAR(100);
ALTER TABLE users ADD COLUMN IF NOT EXISTS phap_hieu   VARCHAR(100);
ALTER TABLE users ADD COLUMN IF NOT EXISTS but_danh    VARCHAR(100);
ALTER TABLE users ADD COLUMN IF NOT EXISTS gender      VARCHAR(20)  NOT NULL DEFAULT 'other';
ALTER TABLE users ADD COLUMN IF NOT EXISTS bio         TEXT;

-- 2. Comment cho các cột mới
COMMENT ON COLUMN users.phap_danh IS 'Pháp danh — tên Phật giáo khi quy y (tùy chọn)';
COMMENT ON COLUMN users.phap_hieu IS 'Pháp hiệu — tên đạo giáo khi truyền pháp (tùy chọn)';
COMMENT ON COLUMN users.but_danh  IS 'Bút danh — tên bút khi viết bài (tùy chọn)';
COMMENT ON COLUMN users.gender    IS 'Giới tính: male | female | other';
COMMENT ON COLUMN users.bio       IS 'Tiểu sử / lời giới thiệu ngắn';

-- 3. Constraint cho gender
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_gender_check'
    ) THEN
        ALTER TABLE users
        ADD CONSTRAINT users_gender_check
        CHECK (gender IN ('male', 'female', 'other'));
    END IF;
END$$;

-- 4. Tạo bảng member_ranks — mô tả chi tiết từng cấp bậc
CREATE TABLE IF NOT EXISTS member_ranks (
    code            VARCHAR(50)  PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT         NOT NULL,
    min_k_balance   BIGINT       NOT NULL DEFAULT 0,
    color           VARCHAR(20)  NOT NULL DEFAULT '#2E7D32',
    icon            VARCHAR(20)  NOT NULL DEFAULT '🪷',
    sort_order      INT          NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- 5. Index theo sort_order để hiển thị đúng thứ tự
CREATE INDEX IF NOT EXISTS idx_member_ranks_sort_order ON member_ranks(sort_order);

-- 6. Seed các cấp bậc mặc định
-- Thứ tự: Người Mới → Người Thường → Người Bình Thường → Người Tốt → Người Khá Tốt
--         → Người Rất Tốt → Người Cực Kỳ Tốt → Thiện Nhân → Đại Gia
INSERT INTO member_ranks (code, name, description, min_k_balance, color, icon, sort_order) VALUES
    ('new',        'Người Mới',          'Thành viên mới gia nhập, đang làm quen với Ứng Dụng Từ Bi.', 0,     '#9E9E9E', '🌱', 1),
    ('normal',     'Người Thường',       'Đã tích lũy 1K, bắt đầu hành trình tu học.',                1,     '#795548', '🍃', 2),
    ('common',     'Người Bình Thường',  'Tích cực tham gia cộng đồng, đã đóng góp 10K.',             10,    '#558B2F', '🌿', 3),
    ('good',       'Người Tốt',          'Người tu học chăm chỉ, tích lũy 100K.',                     100,   '#388E3C', '🌳', 4),
    ('very_good',  'Người Khá Tốt',      'Đạo hữu gương mẫu, có đóng góp cho cộng đồng 500K.',        500,   '#2E7D32', '🌲', 5),
    ('great',      'Người Rất Tốt',      'Tâm tu vững vàng, đạt 1.000K (1M).',                        1000,  '#1B5E20', '🎋', 6),
    ('excellent',  'Người Cực Kỳ Tốt',   'Đã giác ngộ sâu sắc, 5.000K.',                              5000,  '#00695C', '🏆', 7),
    ('benevolent', 'Thiện Nhân',         'Người làm nhiều việc thiện, 10.000K.',                      10000, '#FFB300', '🪷', 8),
    ('tycoon',     'Đại Gia',            'Đại hộ pháp của cộng đồng Từ Bi, 100.000K.',                100000,'#FF6F00', '👑', 9)
ON CONFLICT (code) DO NOTHING;

-- 7. Cập nhật comment cho bảng
COMMENT ON TABLE member_ranks IS 'Bảng cấp bậc thành viên Ứng Dụng Từ Bi';
COMMENT ON COLUMN member_ranks.code          IS 'Mã cấp bậc: new, normal, common, good, very_good, great, excellent, benevolent, tycoon';
COMMENT ON COLUMN member_ranks.min_k_balance IS 'Số K tối thiểu để đạt cấp bậc này';
COMMENT ON COLUMN member_ranks.color         IS 'Màu sắc đại diện cho cấp bậc (hex)';
COMMENT ON COLUMN member_ranks.icon          IS 'Emoji đại diện cho cấp bậc';
COMMENT ON COLUMN member_ranks.sort_order    IS 'Thứ tự hiển thị (1-9)';

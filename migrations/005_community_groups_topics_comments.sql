-- Ứng Dụng Từ Bi - Migration 005: Cộng Đồng Foundation
-- Giai đoạn 6 (v0.6): Nhóm + Chủ Đề + Bình luận + Thành viên nhóm
--
-- Mục tiêu:
--   * Tạo bảng groups (Nhóm) — đơn vị tổ chức cộng đồng
--   * Tạo bảng group_members — thành viên nhóm + vai trò (member/moderator/admin)
--   * Tạo bảng topics (Chủ Đề) — bài viết trong nhóm
--   * Tạo bảng comments (Bình luận) — bình luận trên chủ đề
--   * Tạo bảng group_categories — phân loại nhóm
--   * Trigger tự cập nhật updated_at + counter (member_count, topic_count, comment_count)
--   * Index cho các truy vấn phổ biến
--
-- Theo thiết kế trong HieuLouis/:
--   * Cộng Đồng = Lướt Nhóm + Lướt Chủ Đề (TikTok-style swipe)
--   * Mỗi nhóm = Danh sách Chủ Đề (diễn đàn) + Live Chat (sau này)
--   * Live Chat sẽ được thêm ở giai đoạn sau (WebSocket)

-- 1. Phân loại nhóm (group_categories)
CREATE TABLE IF NOT EXISTS group_categories (
    id          SERIAL       PRIMARY KEY,
    slug        VARCHAR(50)  UNIQUE NOT NULL,
    name        VARCHAR(100) NOT NULL,
    icon        VARCHAR(10)  NOT NULL DEFAULT '🪷',
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

INSERT INTO group_categories (slug, name, icon, sort_order)
VALUES
    ('tu-hoc',       'Tu Học',         '🧘', 1),
    ('niem-phat',    'Niệm Phật',      '🙏', 2),
    ('kinh-sach',    'Kinh Sách',      '📚', 3),
    ('thien',        'Thiền Định',     '🌿', 4),
    ('phap-thoai',   'Pháp Thoại',     '🎤', 5),
    ('chia-se',      'Chia Sẻ',        '💬', 6),
    ('thien-nguyen', 'Thiện Nguyện',   '🤲', 7),
    ('am-nhac',      'Âm Nhạc',        '🎵', 8),
    ('khac',         'Khác',           '🪷', 99)
ON CONFLICT (slug) DO NOTHING;

-- 2. Bảng groups — Nhóm cộng đồng
CREATE TABLE IF NOT EXISTS groups (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    slug             VARCHAR(100) UNIQUE NOT NULL,
    name             VARCHAR(100) NOT NULL,
    description      TEXT,
    category_id      INT          REFERENCES group_categories(id) ON DELETE SET NULL,
    owner_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cover_upload_id  UUID         REFERENCES images(id) ON DELETE SET NULL,
    -- 'public' | 'private' | 'hidden'
    visibility       VARCHAR(20)  NOT NULL DEFAULT 'public',
    require_approval BOOLEAN      NOT NULL DEFAULT false,
    -- Counters (denormalised cho perf, duy trì bằng trigger)
    member_count     INT          NOT NULL DEFAULT 0,
    topic_count      INT          NOT NULL DEFAULT 0,
    is_active        BOOLEAN      NOT NULL DEFAULT true,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_groups_owner       ON groups(owner_id);
CREATE INDEX IF NOT EXISTS idx_groups_category    ON groups(category_id);
CREATE INDEX IF NOT EXISTS idx_groups_visibility  ON groups(visibility);
CREATE INDEX IF NOT EXISTS idx_groups_active      ON groups(is_active);
CREATE INDEX IF NOT EXISTS idx_groups_created     ON groups(created_at DESC);

-- 3. Bảng group_members — thành viên nhóm
CREATE TABLE IF NOT EXISTS group_members (
    id         BIGSERIAL    PRIMARY KEY,
    group_id   UUID         NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id    UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- 'owner' | 'admin' | 'moderator' | 'member'
    role       VARCHAR(20)  NOT NULL DEFAULT 'member',
    -- 'active' | 'pending' | 'banned'
    status     VARCHAR(20)  NOT NULL DEFAULT 'active',
    joined_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE(group_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_group_members_group ON group_members(group_id);
CREATE INDEX IF NOT EXISTS idx_group_members_user  ON group_members(user_id);
CREATE INDEX IF NOT EXISTS idx_group_members_status ON group_members(status);

-- 4. Bảng topics — Chủ Đề (bài viết trong nhóm)
CREATE TABLE IF NOT EXISTS topics (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id        UUID         NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    author_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(200) NOT NULL,
    body            TEXT         NOT NULL,
    is_pinned       BOOLEAN      NOT NULL DEFAULT false,
    is_locked       BOOLEAN      NOT NULL DEFAULT false,
    -- Counters
    comment_count   INT          NOT NULL DEFAULT 0,
    view_count      BIGINT       NOT NULL DEFAULT 0,
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_topics_group    ON topics(group_id);
CREATE INDEX IF NOT EXISTS idx_topics_author   ON topics(author_id);
CREATE INDEX IF NOT EXISTS idx_topics_pinned   ON topics(group_id, is_pinned);
CREATE INDEX IF NOT EXISTS idx_topics_created  ON topics(created_at DESC);

-- 5. Bảng comments — Bình luận
CREATE TABLE IF NOT EXISTS comments (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id    UUID         NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    author_id   UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id   UUID         REFERENCES comments(id) ON DELETE CASCADE,
    body        TEXT         NOT NULL,
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_comments_topic  ON comments(topic_id);
CREATE INDEX IF NOT EXISTS idx_comments_author ON comments(author_id);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_id);
CREATE INDEX IF NOT EXISTS idx_comments_created ON comments(created_at DESC);

-- 6. Triggers: tự cập nhật updated_at
-- (Hàm trigger_set_updated_at đã có từ migration 004, dùng lại)

DROP TRIGGER IF EXISTS trg_groups_set_updated_at ON groups;
CREATE TRIGGER trg_groups_set_updated_at
    BEFORE UPDATE ON groups
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_topics_set_updated_at ON topics;
CREATE TRIGGER trg_topics_set_updated_at
    BEFORE UPDATE ON topics
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_comments_set_updated_at ON comments;
CREATE TRIGGER trg_comments_set_updated_at
    BEFORE UPDATE ON comments
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 7. Triggers: tự cập nhật counters
CREATE OR REPLACE FUNCTION update_group_member_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE groups SET member_count = member_count + 1 WHERE id = NEW.group_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE groups SET member_count = GREATEST(member_count - 1, 0) WHERE id = OLD.group_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_group_members_count ON group_members;
CREATE TRIGGER trg_group_members_count
    AFTER INSERT OR DELETE ON group_members
    FOR EACH ROW EXECUTE FUNCTION update_group_member_count();

CREATE OR REPLACE FUNCTION update_topic_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE groups SET topic_count = topic_count + 1 WHERE id = NEW.group_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE groups SET topic_count = GREATEST(topic_count - 1, 0) WHERE id = OLD.group_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_topics_count ON topics;
CREATE TRIGGER trg_topics_count
    AFTER INSERT OR DELETE ON topics
    FOR EACH ROW EXECUTE FUNCTION update_topic_count();

CREATE OR REPLACE FUNCTION update_comment_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE topics SET comment_count = comment_count + 1 WHERE id = NEW.topic_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE topics SET comment_count = GREATEST(comment_count - 1, 0) WHERE id = OLD.topic_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_comments_count ON comments;
CREATE TRIGGER trg_comments_count
    AFTER INSERT OR DELETE ON comments
    FOR EACH ROW EXECUTE FUNCTION update_comment_count();

-- 8. Comments
COMMENT ON TABLE group_categories IS 'Phân loại nhóm cộng đồng';
COMMENT ON TABLE groups IS 'Nhóm cộng đồng — đơn vị tổ chức chính của chuyên mục Cộng Đồng';
COMMENT ON COLUMN groups.visibility IS 'public | private | hidden';
COMMENT ON COLUMN groups.require_approval IS 'Có cần admin duyệt khi thành viên tham gia không';
COMMENT ON TABLE group_members IS 'Thành viên nhóm + vai trò (owner/admin/moderator/member)';
COMMENT ON COLUMN group_members.role IS 'owner | admin | moderator | member';
COMMENT ON COLUMN group_members.status IS 'active | pending | banned';
COMMENT ON TABLE topics IS 'Chủ Đề (bài viết) trong nhóm — giao diện giống diễn đàn';
COMMENT ON TABLE comments IS 'Bình luận trên chủ đề — hỗ trợ trả lời (parent_id)';

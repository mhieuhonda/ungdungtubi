-- Migration 045 — Giai đoạn 67: Nhà Truyền Tống
-- Theo tài liệu ỨNG DỤNG TỪ BI.docx mục I.1.b (Nhà Truyền Tống):
--   "Đây là nơi thành viên dịch chuyển đến:
--    - Không gian của người chơi khác.
--    - Không gian nhóm.
--    - Các bản đồ Du Hí.
--    Thành viên có thể nhập ID để truyền tống.
--    Điều kiện truyền tống: Phải là bạn bè / Phải đạt đủ đẳng cấp / Phải từng ghé thăm địa điểm đó."

-- Lịch sử truyền tống của user
CREATE TABLE IF NOT EXISTS teleport_visits (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_user_id UUID,       -- nếu truyền tống đến không gian user khác
    target_group_id BIGINT,    -- nếu truyền tống đến nhóm
    target_type  VARCHAR(20)  NOT NULL,  -- 'user_space' | 'group_space' | 'game_map'
    visited_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_teleport_visits_user ON teleport_visits(user_id, visited_at DESC);

-- Danh sách địa điểm đã ghé thăm (để user quay lại nhanh)
CREATE TABLE IF NOT EXISTS teleport_bookmarks (
    id           BIGSERIAL    PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type  VARCHAR(20)  NOT NULL,  -- 'user_space' | 'group_space'
    target_id    VARCHAR(60)  NOT NULL,  -- user_id hoặc group_id (string để flexible)
    label        VARCHAR(100) NOT NULL,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, target_type, target_id)
);

CREATE INDEX IF NOT EXISTS idx_teleport_bookmarks_user ON teleport_bookmarks(user_id, created_at DESC);

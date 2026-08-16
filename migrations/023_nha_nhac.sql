-- =====================================================================
-- Migration 023 — Giai đoạn 38: Nhà Nhạc (Music House)
-- v0.9.33 — 2026-08-16
--
-- Mục tiêu:
--   Tạo schema cho Nhà Nhạc — 1 trong 8 phòng của Không Gian (KG-03).
--   Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx":
--     * 5 thư mục nhạc: Niem, Thien, Dao, KhongLoi, CaNhan
--     * 5 chế độ phát: SingleRepeat, Shuffle, RepeatAll, Loop, SleepTimer
--     * Khi mở nhạc, thành viên trong Không Gian có thể nghe cùng
--     * Cá Nhân = danh sách nhạc do user tải lên hoặc thêm từ kho hệ thống
--
-- Thiết kế:
--   1. music_tracks — kho nhạc hệ thống (admin upload/seed)
--      Mỗi track có: title, category, duration, audio_url, artist (optional),
--                    is_public (mặc định true), upload_user_id (nullable)
--   2. user_music_prefs — preferences phát nhạc per-user
--      Lưu playback_mode, volume, sleep_timer_minutes, last_track_id
--   3. user_personal_tracks — shortcut "Cá Nhân" (user add track hệ thống vào list cá nhân)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS — chạy lại không lỗi.
-- =====================================================================

-- 1. Bảng music_tracks — kho nhạc hệ thống
CREATE TABLE IF NOT EXISTS music_tracks (
    id              BIGSERIAL PRIMARY KEY,
    title           TEXT        NOT NULL,
    -- Category nhạc: niem (Niệm), thien (Thiền), dao (Đạo), khong_loi (Không Lời)
    -- Cá nhân được lưu trong user_personal_tracks (separate table)
    category        TEXT        NOT NULL CHECK (category IN ('niem', 'thien', 'dao', 'khong_loi')),
    description     TEXT,
    artist          TEXT,
    audio_url       TEXT        NOT NULL,
    duration_seconds INTEGER    NOT NULL DEFAULT 0,
    cover_url       TEXT,
    is_public       BOOLEAN     NOT NULL DEFAULT true,
    upload_user_id  UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Sort order trong category (manual ordering)
    sort_order      INTEGER     NOT NULL DEFAULT 0,
    is_active       BOOLEAN     NOT NULL DEFAULT true,
    play_count      BIGINT      NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index cho query theo category + sort
CREATE INDEX IF NOT EXISTS idx_music_tracks_category_sort
    ON music_tracks (category, sort_order, id)
    WHERE is_active = true AND is_public = true;

CREATE INDEX IF NOT EXISTS idx_music_tracks_upload_user
    ON music_tracks (upload_user_id);

-- 2. Bảng user_music_prefs — preferences phát nhạc per-user
CREATE TABLE IF NOT EXISTS user_music_prefs (
    user_id              UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- Playback mode: single_repeat, shuffle, repeat_all, loop
    playback_mode        TEXT        NOT NULL DEFAULT 'repeat_all'
                                     CHECK (playback_mode IN ('single_repeat', 'shuffle', 'repeat_all', 'loop')),
    volume               INTEGER     NOT NULL DEFAULT 70
                                     CHECK (volume >= 0 AND volume <= 100),
    -- Sleep timer tính bằng phút (NULL = tắt). Khi hết thời gian → auto-pause.
    sleep_timer_minutes  INTEGER     CHECK (sleep_timer_minutes IS NULL OR sleep_timer_minutes > 0),
    -- Track cuối cùng user nghe (restore state khi quay lại Nhà Nhạc)
    last_track_id        BIGINT      REFERENCES music_tracks(id) ON DELETE SET NULL,
    -- Timestamp khi preferences được update lần cuối
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Bảng user_personal_tracks — "Cá Nhân" playlist
-- Mỗi user có thể add track hệ thống vào danh sách cá nhân của mình.
-- Sau này mở rộng: user upload track riêng (cần quota check + storage).
CREATE TABLE IF NOT EXISTS user_personal_tracks (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    track_id        BIGINT      NOT NULL REFERENCES music_tracks(id) ON DELETE CASCADE,
    -- Sort order trong playlist cá nhân
    sort_order      INTEGER     NOT NULL DEFAULT 0,
    added_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, track_id)
);

CREATE INDEX IF NOT EXISTS idx_user_personal_tracks_user
    ON user_personal_tracks (user_id, sort_order);

-- =====================================================================
-- Seed data — Một số track mẫu cho 4 category (Niem, Thien, Dao, KhongLoi)
-- URL audio là placeholder (sẽ thay bằng file thật khi có asset).
-- Empty audio_url vẫn hợp lệ cho UI — frontend sẽ disable nút Play nếu URL rỗng.
-- =====================================================================

-- Sử dụng ON CONFLICT DO NOTHING để idempotent — không insert lại nếu đã có
INSERT INTO music_tracks (title, category, description, artist, audio_url, duration_seconds, sort_order, is_public)
VALUES
    -- 📿 Nhạc Niệm — Phật hiệu, thần chú, danh xưng Phật
    ('Nam Mô A Di Đà Phật (Lộc Châu)', 'niem', 'Phật hiệu Lộc Châu — niệm Phật Nam Mô A Di Đà Phật.', 'Niệm Phật Đạo Tràng', '', 1800, 1, true),
    ('Lục Tự Đại Minh Chú (Om Mani Padme Hum)', 'niem', 'Chú Lục Tự Đại Minh — thần chú Quán Thế Âm Bồ Tát.', 'Tibetan Monks', '', 600, 2, true),
    ('Nam Mô Đại Từ Đại Bi Quán Thế Âm Bồ Tát', 'niem', 'Niệm danh hiệu Quán Thế Âm Bồ Tát.', 'Phật Giáo Audio', '', 900, 3, true),

    -- 🧘 Nhạc Thiền — thiền thanh tịnh, tĩnh tâm
    ('Thiền Chuông Tây Tạng', 'thien', 'Tiếng chuông Tây Tạng êm dịu cho thiền định.', 'Meditation Sounds', '', 1200, 1, true),
    ('Mưa Nhẹ Rơi Trên Lá Sen', 'thien', 'Tiếng mưa nhẹ trên lá sen — ambient thiền.', 'Nature & Zen', '', 1500, 2, true),
    ('Gió Trên Núi Linh Thứu', 'thien', 'Tiếng gió nhẹ — tưởng tượng núi Linh Thứu nơi Phật thuyết pháp.', 'Zen Garden', '', 1800, 3, true),

    -- 🛕 Nhạc Đạo — nhạc Phật giáo, ca khúc tu học
    ('Hymn To The Lotus (Tôn Ca Hoa Sen)', 'dao', 'Bài ca tôn vinh hoa sen — biểu tượng giác ngộ.', 'Buddhist Choir', '', 240, 1, true),
    ('Đường Về Tịnh Độ', 'dao', 'Ca khúc về con đường tu học hướng về Tịnh Độ.', 'Phật Giáo Việt Nam', '', 300, 2, true),
    ('Vầng Sáng Từ Bi', 'dao', 'Ca khúc tôn vinh lòng từ bi.', 'Buddhist Music Ensemble', '', 280, 3, true),

    -- 🎵 Không Lời — instrumental, ambient
    ('Mộc Tần (Guitar Thính Phòng)', 'khong_loi', 'Đàn guitar thính phòng êm dịu.', 'Acoustic Zen', '', 360, 1, true),
    ('Trúc Điếu (Sáo Trúc)', 'khong_loi', 'Tiếng sáo trúc mộc mạc.', 'Bamboo Flute', '', 420, 2, true),
    ('Cổ Cầm (Đàn Cổ)', 'khong_loi', 'Tiếng đàn cổ mộc mạc cổ truyền.', 'Guqin Master', '', 480, 3, true)
ON CONFLICT DO NOTHING;

-- Update play_count cho music_tracks — default 0 (already set in CREATE TABLE)
-- Cập nhật comment cho bảng
COMMENT ON TABLE music_tracks IS
    'v0.9.33 — Kho nhạc hệ thống cho Nhà Nhạc (KG-03). 4 category: niem, thien, dao, khong_loi. Cá nhân lưu trong user_personal_tracks.';
COMMENT ON TABLE user_music_prefs IS
    'v0.9.33 — Preferences phát nhạc per-user: playback_mode, volume, sleep_timer, last_track_id.';
COMMENT ON TABLE user_personal_tracks IS
    'v0.9.33 — Playlist Cá Nhân: user add track hệ thống vào danh sách riêng. Sau này: hỗ trợ user upload.';

-- Update view v_user_permissions (nếu có) — không cần thay đổi permissions cho nha-nhac
-- vì Nhà Nhạc là public feature cho mọi user đã đăng nhập.

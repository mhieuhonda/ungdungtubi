// Database module — helper functions for PostgreSQL interactions

use sqlx::PgPool;

/// Clean up expired sessions from the database.
/// Should be called periodically (e.g., every hour) via a background task.
pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Safety check: ensure critical columns/tables exist BEFORE sqlx migrations run.
///
/// This fixes the CRITICAL bug where login fails with
/// "Database (42703): column i_balance does not exist" because migration 015
/// hasn't been applied yet (checksum mismatch, partial deploy, etc.).
///
/// Runs idempotent DDL (`ADD COLUMN IF NOT EXISTS`, `CREATE TABLE IF NOT EXISTS`)
/// directly — no dependency on sqlx migration tracking table.
/// If these statements fail, we log a warning but do NOT crash — the server
/// still starts, and the sqlx migration system will try again afterward.
pub async fn ensure_schema_safety(pool: &PgPool) {
    log::info!("🔒 Đang chạy safety schema check...");

    // 1. Ensure `i_balance` column exists on users (v0.9.9 — Giai đoạn 13).
    //    This is THE fix for the "column i_balance does not exist" login error.
    match sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS i_balance BIGINT NOT NULL DEFAULT 0"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ users.i_balance column ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure users.i_balance: {e}"),
    }

    // 2. Ensure `role` column exists on users (v0.9.7 — Giai đoạn 11).
    //    v0.9.19: Update CHECK constraint để cho phép role 'mod' (Giai đoạn 24).
    match sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS role VARCHAR(30) NOT NULL DEFAULT 'member'"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ users.role column ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure users.role: {e}"),
    }

    // v0.9.30: Drop old CHECK constraint và thay bằng CHECK constraint mới cho phép
    // 6 giá trị: member, mod, admin_ky_thuat, admin_cong_dong, admin_quan_li, admin_phat_trien.
    // v0.9.19: thêm 'mod'. v0.9.30: thêm 'admin_phat_trien' (Admin Phát Triển).
    // Idempotent — nếu constraint cũ không tồn tại thì DROP IF EXISTS không lỗi.
    match sqlx::query("ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => log::warn!("⚠️ Could not drop old users_role_check constraint: {e}"),
    }
    match sqlx::query(
        "DO $$ BEGIN \
            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'users_role_check') THEN \
                ALTER TABLE users ADD CONSTRAINT users_role_check \
                CHECK (role IN ('member', 'mod', 'admin_ky_thuat', 'admin_cong_dong', 'admin_quan_li', 'admin_phat_trien')); \
            END IF; \
        END $$"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ users_role_check constraint updated (v0.9.30: + 'admin_phat_trien')"),
        Err(e) => log::error!("  ❌ Failed to update users_role_check constraint: {e}"),
    }

    // 3. Ensure practice_logs table exists (v0.9.9 — Giai đoạn 13).
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS practice_logs (
            id            BIGSERIAL    PRIMARY KEY,
            user_id       UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            log_date      DATE         NOT NULL DEFAULT CURRENT_DATE,
            niem_count    BIGINT       NOT NULL DEFAULT 0,
            last_niem_at  TIMESTAMPTZ,
            created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
            updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, log_date)
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ practice_logs table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure practice_logs: {e}"),
    }

    // 4. Ensure buddha_vows table exists (v0.9.9 — Giai đoạn 13).
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS buddha_vows (
            id           BIGSERIAL    PRIMARY KEY,
            user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            vow_type     VARCHAR(20)  NOT NULL,
            content      TEXT         NOT NULL,
            is_public    BOOLEAN      NOT NULL DEFAULT true,
            created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ buddha_vows table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure buddha_vows: {e}"),
    }

    // 5. Ensure permissions-related tables exist (v0.9.8 — Giai đoạn 12).
    // v0.9.25 FIX (bug B3): Đồng bộ column names với migration 014.
    //   - permissions.name → permissions.name_vi (+ description_vi)
    //   - role_permissions(role_code, permission_id, assigned_at)
    //       → role_permissions(role, permission_code, granted_at)
    // Trước v0.9.25, fresh deploy tạo bảng SAI → migration 014 `CREATE TABLE IF NOT EXISTS`
    // bị skip → INSERT fail vì column không tồn tại → cascading migration failure.
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS permissions (
            id            SERIAL       PRIMARY KEY,
            code          VARCHAR(60)  NOT NULL UNIQUE,
            name_vi       VARCHAR(200) NOT NULL,
            description_vi TEXT,
            category      VARCHAR(30)  NOT NULL,
            sort_order    INT          NOT NULL DEFAULT 0,
            created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ permissions table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure permissions: {e}"),
    }

    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS role_permissions (
            role            VARCHAR(30) NOT NULL,
            permission_code VARCHAR(60) NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
            granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (role, permission_code)
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ role_permissions table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure role_permissions: {e}"),
    }

    // ─── v0.9.38 — Giai đoạn 42: Safety schema cho migration 026 ─────────
    // Trên production, migration 026 có thể không được apply đầy đủ vì checksum
    // mismatch, partial deploy, hoặc DB bị rollback manual. Khi đó:
    //   - `UPDATE groups SET logo_upload_id = ...` fail với
    //     "column \"logo_upload_id\" does not exist" → "Lỗi cập nhật logo nhóm."
    //   - `INSERT INTO user_music_submissions (..., source_type, audio_file_upload_id, ...)`
    //     fail với "column \"source_type\" does not exist" → "lỗi gửi bài"
    //   - `INSERT INTO audio_files ...` fail vì bảng chưa tồn tại.
    // Fix: chạy idempotent DDL trực tiếp (ADD COLUMN IF NOT EXISTS / CREATE TABLE
    // IF NOT EXISTS) trước khi sqlx migrations chạy, để schema luôn nhất quán.
    // (Tương tự cơ chế đã fix v0.9.25 cho users.i_balance / permissions.)

    // 6. Ensure `groups.logo_upload_id` column (v0.9.36 — Giai đoạn 41).
    match sqlx::query(
        "ALTER TABLE groups ADD COLUMN IF NOT EXISTS logo_upload_id UUID REFERENCES images(id) ON DELETE SET NULL"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ groups.logo_upload_id column ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure groups.logo_upload_id: {e}"),
    }

    // 7. Ensure `audio_files` table (v0.9.36 — Giai đoạn 41).
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS audio_files (
            id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
            uploader_id      UUID         REFERENCES users(id) ON DELETE SET NULL,
            original_name    VARCHAR(255) NOT NULL,
            stored_filename  VARCHAR(255) NOT NULL UNIQUE,
            mime_type        VARCHAR(100) NOT NULL,
            size_bytes       BIGINT       NOT NULL,
            sha256           VARCHAR(64)  NOT NULL,
            duration_seconds INT,
            purpose          VARCHAR(50)  NOT NULL DEFAULT 'other',
            is_public        BOOLEAN      NOT NULL DEFAULT true,
            created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ audio_files table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure audio_files: {e}"),
    }
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_audio_files_uploader ON audio_files(uploader_id)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_audio_files_purpose ON audio_files(purpose)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_audio_files_sha256 ON audio_files(sha256)"
    ).execute(pool).await;

    // 8. Ensure `user_music_submissions` new columns (v0.9.36 — Giai đoạn 41).
    //    source_type có CHECK constraint — dùng DO $$ để idempotent.
    match sqlx::query(
        "ALTER TABLE user_music_submissions
            ADD COLUMN IF NOT EXISTS source_type TEXT NOT NULL DEFAULT 'youtube'"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ user_music_submissions.source_type ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure user_music_submissions.source_type: {e}"),
    }
    // Add CHECK constraint nếu chưa có (idempotent)
    let _ = sqlx::query(
        "DO $$ BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'user_music_submissions_source_type_check'
            ) THEN
                ALTER TABLE user_music_submissions
                ADD CONSTRAINT user_music_submissions_source_type_check
                CHECK (source_type IN ('youtube', 'audio_file'));
            END IF;
        END $$"
    ).execute(pool).await;

    match sqlx::query(
        "ALTER TABLE user_music_submissions
            ADD COLUMN IF NOT EXISTS audio_file_upload_id UUID REFERENCES audio_files(id) ON DELETE SET NULL"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ user_music_submissions.audio_file_upload_id ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure user_music_submissions.audio_file_upload_id: {e}"),
    }
    match sqlx::query(
        "ALTER TABLE user_music_submissions
            ADD COLUMN IF NOT EXISTS audio_duration_seconds INT"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ user_music_submissions.audio_duration_seconds ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure user_music_submissions.audio_duration_seconds: {e}"),
    }
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_source_type ON user_music_submissions(source_type)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_audio_file ON user_music_submissions(audio_file_upload_id) WHERE audio_file_upload_id IS NOT NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_groups_logo_upload ON groups(logo_upload_id) WHERE logo_upload_id IS NOT NULL"
    ).execute(pool).await;

    // ─── v0.9.39 — Giai đoạn 43: Safety schema cho user_settings + last_seen_at ─
    // Trên production, migration 017 (user_settings) có thể không được apply đầy đủ
    // vì checksum mismatch, partial deploy, hoặc DB rollback manual. Khi đó:
    //   - `INSERT INTO user_settings ...` fail với
    //     "relation \"user_settings\" does not exist" → user thấy
    //     "Lỗi database: error returned from database: relation user_settings does not exist"
    //   - `SELECT ... FROM user_settings` cũng fail → trang /cai-dat render với default
    //     settings (không giữ được preferences của user).
    // Fix: chạy idempotent DDL trực tiếp (CREATE TABLE IF NOT EXISTS) trước khi sqlx
    // migrations chạy, để schema luôn nhất quán.
    // (Tương tự cơ chế đã fix v0.9.25 cho users.i_balance / permissions,
    // v0.9.38 cho groups.logo_upload_id / audio_files.)

    // 9. Ensure `user_settings` table exists (v0.9.14 — Giai đoạn 18).
    //    Bảng cài đặt cá nhân của user — 15 cột + 1 trigger updated_at.
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_settings (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            profile_visibility VARCHAR(20) NOT NULL DEFAULT 'public',
            show_balance BOOLEAN NOT NULL DEFAULT true,
            show_activity BOOLEAN NOT NULL DEFAULT true,
            show_email BOOLEAN NOT NULL DEFAULT false,
            notify_friends BOOLEAN NOT NULL DEFAULT true,
            notify_mail BOOLEAN NOT NULL DEFAULT true,
            notify_dm BOOLEAN NOT NULL DEFAULT true,
            notify_group BOOLEAN NOT NULL DEFAULT true,
            notify_system BOOLEAN NOT NULL DEFAULT true,
            theme VARCHAR(20) NOT NULL DEFAULT 'lotus',
            language VARCHAR(10) NOT NULL DEFAULT 'vi',
            auto_join_global_chat BOOLEAN NOT NULL DEFAULT false,
            chat_sound_enabled BOOLEAN NOT NULL DEFAULT true,
            niem_sound_enabled BOOLEAN NOT NULL DEFAULT true,
            niem_auto_convert_k BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ user_settings table ensured (v0.9.39 fix)"),
        Err(e) => log::error!("  ❌ Failed to ensure user_settings: {e}"),
    }
    // Index cho user_settings
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_settings_theme ON user_settings(theme)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_settings_visibility ON user_settings(profile_visibility)"
    ).execute(pool).await;

    // 10. Ensure `users.last_seen_at` column (v0.9.39 — Giai đoạn 43).
    //    Cột này track thời điểm user active gần nhất (update qua /api/heartbeat).
    //    Trước v0.9.39: admin stats `active_users` đếm `WHERE is_active` (tức là
    //    "không bị ban") chứ không phải "đang online" → admin thấy "5 user đang hoạt
    //    động" nhưng vào /admin/thanh-vien không thấy ai online. Heartbeat handler
    //    cũng không làm gì cả. v0.9.39 fix:
    //      - Thêm cột `last_seen_at TIMESTAMPTZ` vào users.
    //      - Heartbeat handler update `last_seen_at = NOW()` cho user đã login.
    //      - Admin stats `active_users` đếm `WHERE last_seen_at > NOW() - INTERVAL '5 min'`.
    //      - Admin user list hiển thị `last_seen_at` thay vì `MAX(sessions.created_at)`.
    match sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ users.last_seen_at column ensured (v0.9.39 — active user sync)"),
        Err(e) => log::error!("  ❌ Failed to ensure users.last_seen_at: {e}"),
    }
    // Index cho last_seen_at để query "active trong 5 phút" nhanh
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_users_last_seen_at ON users(last_seen_at DESC) WHERE last_seen_at IS NOT NULL"
    ).execute(pool).await;

    // 11. Seed default settings cho user hiện có (chưa có row trong user_settings).
    //    Idempotent — chỉ INSERT nếu chưa có row.
    let _ = sqlx::query(
        "INSERT INTO user_settings (user_id)
         SELECT id FROM users
         WHERE NOT EXISTS (SELECT 1 FROM user_settings WHERE user_id = users.id)"
    ).execute(pool).await;

    // ─── v0.9.41 — Giai đoạn 44: Safety schema cho Chợ Đạo Hữu + Admin ──────
    // Migration 028 có thể không được apply đầy đủ trên production do checksum
    // mismatch, partial deploy, hoặc manual rollback. Khi đó:
    //   - `INSERT INTO shop_categories ...` fail với
    //     "relation \"shop_categories\" does not exist"
    //   - `UPDATE shop_items SET payment_method = ...` fail với
    //     "column \"payment_method\" does not exist"
    //   - `INSERT INTO shop_items (..., payment_method, price_vnd, bank_info, ...)`
    //     cũng fail.
    // Fix: chạy idempotent DDL trực tiếp (CREATE TABLE IF NOT EXISTS,
    // ALTER TABLE ... ADD COLUMN IF NOT EXISTS) trước khi sqlx migrations chạy.

    // 12. Ensure `shop_categories` table (v0.9.41 — Giai đoạn 44).
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS shop_categories (
            id              BIGSERIAL    PRIMARY KEY,
            slug            TEXT         NOT NULL UNIQUE,
            name_vi         TEXT         NOT NULL,
            description     TEXT,
            icon            TEXT         NOT NULL DEFAULT '📦',
            color           TEXT         NOT NULL DEFAULT '#0F766E',
            parent_id       BIGINT       REFERENCES shop_categories(id) ON DELETE SET NULL,
            sort_order      INTEGER      NOT NULL DEFAULT 0,
            is_system       BOOLEAN      NOT NULL DEFAULT false,
            is_approved     BOOLEAN      NOT NULL DEFAULT true,
            is_active       BOOLEAN      NOT NULL DEFAULT true,
            created_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
            created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ shop_categories table ensured (v0.9.41 — Chợ Đạo Hữu)"),
        Err(e) => log::error!("  ❌ Failed to ensure shop_categories: {e}"),
    }
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shop_categories_parent ON shop_categories(parent_id, sort_order) WHERE is_active = true"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shop_categories_active ON shop_categories(is_active, is_approved)"
    ).execute(pool).await;

    // Seed system categories (idempotent — ON CONFLICT DO NOTHING)
    let _ = sqlx::query(
        "INSERT INTO shop_categories (slug, name_vi, description, icon, color, sort_order, is_system, is_approved) VALUES
            ('the-tu-hoc',     'Thẻ Tu Học',       'Các thẻ hỗ trợ tu học: Tự Tu, Cộng Tu, Exp.',                    '📿', '#2E7D32', 1,  true, true),
            ('the-doi-ten',    'Thẻ Đổi Tên',      'Thẻ đổi tên, pháp danh, pháp hiệu.',                            '✏️', '#6A1B9A', 2,  true, true),
            ('the-ho-tro',     'Thẻ Hỗ Trợ',       'Thẻ hỗ trợ cộng đồng, ủng hộ quỹ, hộp quà.',                    '🤝', '#C62828', 3,  true, true),
            ('the-nhom',       'Thẻ Nhóm',         'Thẻ tạo nhóm, không gian nhóm, mời cộng tu.',                    '👥', '#3F51B5', 4,  true, true),
            ('the-bau-chon',   'Thẻ Bầu Chọn',     'Thẻ tạo cuộc bầu chọn trong nhóm/cộng đồng.',                   '🗳️', '#673AB7', 5,  true, true),
            ('vat-pham',       'Vật Phẩm',         'Vật phẩm chung: hoa hồng, ô vật phẩm, thẻ yêu cầu.',            '📦', '#795548', 6,  true, true),
            ('cao-cap',        'Cao Cấp',          'Vật phẩm cao cấp: Phiếu Từ Bi, Thẻ Người Tốt, Thẻ Thiện Nhân.', '🪷', '#0F766E', 7,  true, true),
            ('sach-phat-giao', 'Sách Phật Giáo',   'Sách điện tử, kinh sách do đạo hữu chia sẻ.',                   '📚', '#FF6F00', 8,  true, true),
            ('do-tho',         'Đồ Thờ',           'Đồ thờ cúng: tượng Phật, hoa sen, đèn nến.',                    '🪔', '#FFD600', 9,  true, true),
            ('dich-vu',        'Dịch Vụ',          'Dịch vụ Phật giáo: in kinh, tổ chức lễ, hướng dẫn tu.',         '🛎️', '#0288D1', 10, true, true),
            ('thuc-pham-chay', 'Thực Phẩm Chay',   'Thực phẩm chay, đồ hữu cơ.',                                    '🥬', '#43A047', 11, true, true),
            ('khac',           'Khác',             'Danh mục khác — không thuộc nhóm nào trên.',                     '🏷️', '#607D8B', 99, true, true)
         ON CONFLICT (slug) DO NOTHING"
    ).execute(pool).await;

    // 13. Ensure new columns on shop_items (v0.9.41 — Giai đoạn 44).
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS category_id BIGINT REFERENCES shop_categories(id) ON DELETE SET NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS payment_method TEXT NOT NULL DEFAULT 'k'"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS price_vnd BIGINT"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS bank_info JSONB DEFAULT '{}'"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT false"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS moderation_status TEXT NOT NULL DEFAULT 'approved'"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shop_items_moderation ON shop_items(moderation_status, created_at DESC) WHERE store IN ('pvp', 'dao_huu')"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shop_items_category_id ON shop_items(category_id) WHERE category_id IS NOT NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shop_items_featured ON shop_items(is_featured, sort_order) WHERE is_active = true AND is_featured = true"
    ).execute(pool).await;

    // Backfill category_id cho shop_items cũ (text category → slug mapping)
    let _ = sqlx::query(
        "UPDATE shop_items si SET category_id = sc.id, updated_at = NOW()
         FROM shop_categories sc
         WHERE si.category_id IS NULL AND si.category IS NOT NULL
           AND sc.slug = REPLACE(LOWER(si.category), '_', '-')"
    ).execute(pool).await;

    // 14. Ensure new columns on transactions (v0.9.41 — Giai đoạn 44).
    let _ = sqlx::query(
        "ALTER TABLE transactions ADD COLUMN IF NOT EXISTS payment_method TEXT NOT NULL DEFAULT 'k'"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE transactions ADD COLUMN IF NOT EXISTS price_vnd BIGINT"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE transactions ADD COLUMN IF NOT EXISTS bank_info JSONB DEFAULT '{}'"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE transactions ADD COLUMN IF NOT EXISTS buyer_contact TEXT"
    ).execute(pool).await;

    // ─── v0.9.41 — Giai đoạn 45: Safety schema cho Admin Moderation + Từ vựng cấm ─
    // Migration 029 có thể không được apply đầy đủ trên production do checksum
    // mismatch, partial deploy, hoặc manual rollback. Khi đó:
    //   - `INSERT INTO forbidden_words ...` fail với
    //     "relation \"forbidden_words\" does not exist"
    //   - `UPDATE comments SET is_pinned = ...` fail với
    //     "column \"is_pinned\" does not exist"
    //   - `UPDATE groups SET is_featured = ...` fail với
    //     "column \"is_featured\" does not exist"
    // Fix: chạy idempotent DDL trực tiếp trước khi sqlx migrations chạy.

    // 15. Ensure `forbidden_words` table (v0.9.41 — Giai đoạn 45).
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS forbidden_words (
            id              BIGSERIAL    PRIMARY KEY,
            word            TEXT         NOT NULL UNIQUE,
            action          VARCHAR(10)  NOT NULL DEFAULT 'block'
                            CHECK (action IN ('block', 'flag')),
            category        VARCHAR(20)  NOT NULL DEFAULT 'other'
                            CHECK (category IN ('profanity', 'spam', 'politics', 'religious', 'scam', 'other')),
            reason          TEXT,
            is_system       BOOLEAN      NOT NULL DEFAULT false,
            is_active       BOOLEAN      NOT NULL DEFAULT true,
            created_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
            created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ forbidden_words table ensured (v0.9.41 — admin moderation)"),
        Err(e) => log::error!("  ❌ Failed to ensure forbidden_words: {e}"),
    }
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_forbidden_words_active ON forbidden_words(is_active) WHERE is_active = true"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_forbidden_words_category ON forbidden_words(category)"
    ).execute(pool).await;

    // Seed system forbidden words (idempotent — ON CONFLICT DO NOTHING)
    let _ = sqlx::query(
        "INSERT INTO forbidden_words (word, action, category, reason, is_system) VALUES
            ('địt',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
            ('lồn',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
            ('cặc',       'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
            ('buồi',      'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
            ('mẹ mày',    'block', 'profanity', 'Cụm từ xúc phạm — tự động cấm', true),
            ('đĩ',        'block', 'profanity', 'Từ tục tĩu — tự động cấm', true),
            ('chó chết',  'block', 'profanity', 'Cụm từ xúc phạm — tự động cấm', true),
            ('scam',      'flag',  'scam',      'Keyword lừa đảo — flag để admin review', true),
            ('lừa đảo',   'flag',  'scam',      'Keyword lừa đảo — flag để admin review', true)
         ON CONFLICT (word) DO NOTHING"
    ).execute(pool).await;

    // 16. Ensure new columns on `comments` (v0.9.41 — Giai đoạn 45).
    let _ = sqlx::query(
        "ALTER TABLE comments ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT false"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE comments ADD COLUMN IF NOT EXISTS is_locked BOOLEAN NOT NULL DEFAULT false"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
         CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'))"
    ).execute(pool).await;
    // Add CHECK constraint idempotently (separate DO $$ because ADD COLUMN IF NOT EXISTS
    // không supports adding CHECK constraint inline idempotently if column already exists)
    let _ = sqlx::query(
        "DO $$ BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'comments_moderation_status_check'
            ) THEN
                ALTER TABLE comments
                ADD CONSTRAINT comments_moderation_status_check
                CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
            END IF;
        END $$"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE comments ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_comments_pinned ON comments(topic_id, is_pinned) WHERE is_pinned = true"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_comments_moderation ON comments(moderation_status) WHERE moderation_status != 'approved'"
    ).execute(pool).await;

    // 17. Ensure new columns on `groups` (v0.9.41 — Giai đoạn 45).
    let _ = sqlx::query(
        "ALTER TABLE groups ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT false"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
         CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'))"
    ).execute(pool).await;
    let _ = sqlx::query(
        "DO $$ BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'groups_moderation_status_check'
            ) THEN
                ALTER TABLE groups
                ADD CONSTRAINT groups_moderation_status_check
                CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
            END IF;
        END $$"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE groups ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_groups_featured ON groups(is_featured, created_at DESC) WHERE is_featured = true"
    ).execute(pool).await;

    // 18. Ensure new columns on `topics` (v0.9.41 — Giai đoạn 45).
    let _ = sqlx::query(
        "ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderation_status VARCHAR(20) NOT NULL DEFAULT 'approved'
         CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'))"
    ).execute(pool).await;
    let _ = sqlx::query(
        "DO $$ BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'topics_moderation_status_check'
            ) THEN
                ALTER TABLE topics
                ADD CONSTRAINT topics_moderation_status_check
                CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'flagged'));
            END IF;
        END $$"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES users(id) ON DELETE SET NULL"
    ).execute(pool).await;
    let _ = sqlx::query(
        "ALTER TABLE topics ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ"
    ).execute(pool).await;

    // ─── v0.9.42 — Giai đoạn 46: Safety schema cho user_music_submissions + bi_balance + balance_transactions ─
    // Root cause fix cho "Lỗi gửi bài — không thể lưu bài hát vào cơ sở dữ liệu":
    //   Nếu bảng user_music_submissions chưa tồn tại (migration 025 chưa chạy),
    //   INSERT INTO user_music_submissions sẽ fail với "relation does not exist".
    // Fix: tạo bảng trong safety schema.

    // 19. Ensure `user_music_submissions` table (v0.9.35 — Giai đoạn 40).
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_music_submissions (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title           TEXT            NOT NULL,
            artist          TEXT            NOT NULL DEFAULT '',
            category        TEXT            NOT NULL CHECK (category IN ('niem', 'thien', 'dao', 'khong_loi')),
            youtube_url     TEXT            NOT NULL,
            youtube_id      TEXT            NOT NULL,
            description     TEXT            DEFAULT '',
            status          TEXT            NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
            reviewed_by     UUID            REFERENCES users(id),
            review_note     TEXT,
            reviewed_at     TIMESTAMPTZ,
            play_count      BIGINT          NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            source_type     TEXT            NOT NULL DEFAULT 'youtube' CHECK (source_type IN ('youtube', 'audio_file')),
            audio_file_upload_id UUID      REFERENCES audio_files(id) ON DELETE SET NULL,
            audio_duration_seconds INT
        )"
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_user ON user_music_submissions(user_id)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_status ON user_music_submissions(status)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_category ON user_music_submissions(category)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_music_submissions_youtube_id ON user_music_submissions(youtube_id)"
    ).execute(pool).await;
    log::info!("  ✅ user_music_submissions table ensured (v0.9.42 — root cause fix)");

    // 20. Ensure `users.bi_balance` column (v0.9.42 — Giai đoạn 46).
    let _ = sqlx::query(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS bi_balance BIGINT NOT NULL DEFAULT 0"
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_users_bi_balance ON users(bi_balance) WHERE bi_balance > 0"
    ).execute(pool).await;
    log::info!("  ✅ users.bi_balance column ensured (v0.9.42)");

    // 21. Ensure `balance_transactions` table (v0.9.42 — Giai đoạn 46).
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS balance_transactions (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            currency        VARCHAR(5)      NOT NULL CHECK (currency IN ('a', 'k', 'bi')),
            amount          BIGINT          NOT NULL,
            balance_after   BIGINT          NOT NULL,
            tx_type         VARCHAR(30)     NOT NULL CHECK (tx_type IN (
                'purchase', 'sale', 'reward', 'exchange_in', 'exchange_out',
                'donation', 'admin_adjust', 'dao_huu_payment', 'signup_bonus',
                'daily_login', 'other'
            )),
            description     TEXT            NOT NULL DEFAULT '',
            reference_id    VARCHAR(100),
            performed_by    UUID            REFERENCES users(id) ON DELETE SET NULL,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_balance_tx_user ON balance_transactions(user_id)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_balance_tx_user_currency ON balance_transactions(user_id, currency)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_balance_tx_created ON balance_transactions(created_at DESC)"
    ).execute(pool).await;
    log::info!("  ✅ balance_transactions table ensured (v0.9.42)");

    // 22. Ensure `currency_exchange_rates` table (v0.9.42 — Giai đoạn 46).
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS currency_exchange_rates (
            id              BIGSERIAL       PRIMARY KEY,
            from_currency   VARCHAR(5)      NOT NULL CHECK (from_currency IN ('a', 'k', 'bi')),
            to_currency     VARCHAR(5)      NOT NULL CHECK (to_currency IN ('a', 'k', 'bi')),
            from_amount     BIGINT          NOT NULL DEFAULT 100,
            is_active       BOOLEAN         NOT NULL DEFAULT true,
            updated_by      UUID            REFERENCES users(id) ON DELETE SET NULL,
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            UNIQUE (from_currency, to_currency)
        )"
    )
    .execute(pool)
    .await;
    // Seed exchange rates (idempotent)
    let _ = sqlx::query(
        "INSERT INTO currency_exchange_rates (from_currency, to_currency, from_amount) VALUES
            ('a', 'k', 100), ('k', 'bi', 100), ('a', 'bi', 10000)
        ON CONFLICT (from_currency, to_currency) DO NOTHING"
    ).execute(pool).await;
    log::info!("  ✅ currency_exchange_rates table ensured (v0.9.42)");

    // ─── v0.9.44 — Giai đoạn 51: Safety schema cho books/book_chapters.search_tsv
    //   + user_search_history ─
    // Trên production, migration 031 có thể không được apply đầy đủ vì checksum
    // mismatch, partial deploy, hoặc DB rollback manual. Khi đó:
    //   - `SELECT ... WHERE b.search_tsv @@ plainto_tsquery(...)` fail với
    //     "column \"search_tsv\" does not exist" → trang /kinh-sach/tim-kiem crash
    //   - `INSERT INTO user_search_history ...` fail với
    //     "relation \"user_search_history\" does not exist"
    // Fix: chạy idempotent DDL trực tiếp (ADD COLUMN IF NOT EXISTS / CREATE INDEX
    // IF NOT EXISTS / CREATE TABLE IF NOT EXISTS) trước khi sqlx migrations chạy,
    // để schema luôn nhất quán (tương tự cơ chế đã fix v0.9.42).

    // 23. Ensure `books.search_tsv` column (v0.9.44 — Giai đoạn 51).
    let _ = sqlx::query(
        "ALTER TABLE books ADD COLUMN IF NOT EXISTS search_tsv tsvector"
    )
    .execute(pool)
    .await;
    log::info!("  ✅ books.search_tsv column ensured (v0.9.44)");

    // 24. Ensure `book_chapters.search_tsv` column (v0.9.44 — Giai đoạn 51).
    let _ = sqlx::query(
        "ALTER TABLE book_chapters ADD COLUMN IF NOT EXISTS search_tsv tsvector"
    )
    .execute(pool)
    .await;
    log::info!("  ✅ book_chapters.search_tsv column ensured (v0.9.44)");

    // 25. Ensure GIN indexes on search_tsv (v0.9.44 — Giai đoạn 51).
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_books_search_tsv ON books USING gin(search_tsv)"
    ).execute(pool).await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_book_chapters_search_tsv ON book_chapters USING gin(search_tsv)"
    ).execute(pool).await;
    log::info!("  ✅ GIN indexes on books/book_chapters.search_tsv ensured (v0.9.44)");

    // 26. Ensure `user_search_history` table (v0.9.44 — Giai đoạn 51).
    //    Bảng ghi lại 10 lần tìm kiếm Kinh Sách gần nhất của user.
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_search_history (
            id           BIGSERIAL    PRIMARY KEY,
            user_id      UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            query        TEXT         NOT NULL,
            searched_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_search_history_user_time \
         ON user_search_history(user_id, searched_at DESC)"
    ).execute(pool).await;
    log::info!("  ✅ user_search_history table ensured (v0.9.44)");

    // ─── v0.9.45 — Giai đoạn 53-60: Safety schema cho các bảng mới ─
    //   Mỗi bảng/mục mới phải chạy idempotent DDL trực tiếp để app sống sót
    //   qua migration drift (checksum mismatch, partial deploy, manual rollback).

    // 27. users.tu_si_rank + tu_si_approved_at (Giai đoạn 53)
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS tu_si_rank SMALLINT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS tu_si_approved_at TIMESTAMPTZ").execute(pool).await;
    log::info!("  ✅ users.tu_si_rank + tu_si_approved_at ensured (v0.9.45)");

    // 28. tu_si_applications (Giai đoạn 53)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS tu_si_applications (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            requested_rank  SMALLINT        NOT NULL,
            monthly_k_pledge BIGINT         NOT NULL DEFAULT 0,
            motivation      TEXT            NOT NULL DEFAULT '',
            status          VARCHAR(20)     NOT NULL DEFAULT 'pending',
            reviewed_by     UUID            REFERENCES users(id) ON DELETE SET NULL,
            reviewed_at     TIMESTAMPTZ,
            review_note     TEXT            NOT NULL DEFAULT '',
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tu_si_apps_user ON tu_si_applications(user_id)").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tu_si_apps_status ON tu_si_applications(status)").execute(pool).await;
    log::info!("  ✅ tu_si_applications table ensured (v0.9.45)");

    // 29. tu_si_monthly_supports (Giai đoạn 53)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS tu_si_monthly_supports (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            year_month      INTEGER         NOT NULL,
            k_contributed   BIGINT          NOT NULL DEFAULT 0,
            fulfilled       BOOLEAN         NOT NULL DEFAULT false,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, year_month)
        )"
    ).execute(pool).await;
    log::info!("  ✅ tu_si_monthly_supports table ensured (v0.9.45)");

    // 30. member_rank_history (Giai đoạn 54)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS member_rank_history (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            from_rank       VARCHAR(40)     NOT NULL DEFAULT '',
            to_rank         VARCHAR(40)     NOT NULL,
            reason          VARCHAR(60)     NOT NULL DEFAULT 'auto',
            changed_by      UUID            REFERENCES users(id) ON DELETE SET NULL,
            note            TEXT            NOT NULL DEFAULT '',
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_member_rank_history_user ON member_rank_history(user_id)").execute(pool).await;
    log::info!("  ✅ member_rank_history table ensured (v0.9.45)");

    // 31. notification_preferences (Giai đoạn 55)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS notification_preferences (
            user_id                 UUID            PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            daily_niem_reminder     BOOLEAN         NOT NULL DEFAULT true,
            streak_warning          BOOLEAN         NOT NULL DEFAULT true,
            email_reminders         BOOLEAN         NOT NULL DEFAULT false,
            reminder_hour           SMALLINT        NOT NULL DEFAULT 20,
            reminder_channel        VARCHAR(10)     NOT NULL DEFAULT 'app',
            last_reminder_sent_at   TIMESTAMPTZ,
            created_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    log::info!("  ✅ notification_preferences table ensured (v0.9.45)");

    // 32. daily_reminder_log (Giai đoạn 55)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS daily_reminder_log (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            reminder_date   DATE            NOT NULL DEFAULT CURRENT_DATE,
            reminder_type   VARCHAR(30)     NOT NULL,
            channel         VARCHAR(10)     NOT NULL,
            sent_at         TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            status          VARCHAR(20)     NOT NULL DEFAULT 'sent',
            error_message   TEXT            NOT NULL DEFAULT ''
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_daily_reminder_log_user_date ON daily_reminder_log(user_id, reminder_date)").execute(pool).await;
    log::info!("  ✅ daily_reminder_log table ensured (v0.9.45)");

    // 33. reading_progress (Giai đoạn 56) — books.id & book_chapters.id là UUID
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS reading_progress (
            id                  BIGSERIAL       PRIMARY KEY,
            user_id             UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            book_id             UUID            NOT NULL,
            last_chapter_id     UUID,
            progress_percent    SMALLINT        NOT NULL DEFAULT 0,
            scroll_position     INTEGER         NOT NULL DEFAULT 0,
            total_reading_seconds BIGINT       NOT NULL DEFAULT 0,
            chapters_read       BIGINT          NOT NULL DEFAULT 0,
            last_read_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, book_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_reading_progress_user_last ON reading_progress(user_id, last_read_at DESC)").execute(pool).await;
    log::info!("  ✅ reading_progress table ensured (v0.9.45)");

    // 34. chapter_bookmarks (Giai đoạn 56) — books.id & book_chapters.id là UUID
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS chapter_bookmarks (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            book_id         UUID            NOT NULL,
            chapter_id      UUID            NOT NULL,
            note            TEXT            NOT NULL DEFAULT '',
            label           VARCHAR(30)     NOT NULL DEFAULT 'bookmark',
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, chapter_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_chapter_bookmarks_user_book ON chapter_bookmarks(user_id, book_id)").execute(pool).await;
    log::info!("  ✅ chapter_bookmarks table ensured (v0.9.45)");

    // 35. daily_login_rewards + user_login_streaks (Giai đoạn 57)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS daily_login_rewards (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            reward_date     DATE            NOT NULL DEFAULT CURRENT_DATE,
            streak_day      SMALLINT        NOT NULL,
            reward_a        BIGINT          NOT NULL,
            is_bonus        BOOLEAN         NOT NULL DEFAULT false,
            balance_after   BIGINT          NOT NULL,
            claimed_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, reward_date)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_daily_login_rewards_user_date ON daily_login_rewards(user_id, reward_date DESC)").execute(pool).await;
    log::info!("  ✅ daily_login_rewards table ensured (v0.9.45)");

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_login_streaks (
            user_id             UUID            PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            current_streak      SMALLINT        NOT NULL DEFAULT 0,
            max_streak          SMALLINT        NOT NULL DEFAULT 0,
            last_login_date     DATE,
            total_days_claimed  BIGINT          NOT NULL DEFAULT 0,
            total_a_earned      BIGINT          NOT NULL DEFAULT 0,
            created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    log::info!("  ✅ user_login_streaks table ensured (v0.9.45)");

    // 36. tu_hoc_goals + streak_freezes + streak_freeze_quota (Giai đoạn 58)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS tu_hoc_goals (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            goal_type       VARCHAR(30)     NOT NULL,
            target_value    BIGINT          NOT NULL,
            target_unit     VARCHAR(20)     NOT NULL DEFAULT 'count',
            title           VARCHAR(200)    NOT NULL,
            status          VARCHAR(20)     NOT NULL DEFAULT 'active',
            deadline        DATE,
            current_value   BIGINT          NOT NULL DEFAULT 0,
            last_reset_at   TIMESTAMPTZ,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tu_hoc_goals_user_status ON tu_hoc_goals(user_id, status)").execute(pool).await;
    log::info!("  ✅ tu_hoc_goals table ensured (v0.9.45)");

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS streak_freezes (
            id              BIGSERIAL       PRIMARY KEY,
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            freeze_date     DATE            NOT NULL,
            source          VARCHAR(20)     NOT NULL DEFAULT 'monthly_free',
            cost_a          BIGINT          NOT NULL DEFAULT 0,
            applied         BOOLEAN         NOT NULL DEFAULT false,
            applied_at      TIMESTAMPTZ,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_streak_freezes_user_date ON streak_freezes(user_id, freeze_date)").execute(pool).await;
    log::info!("  ✅ streak_freezes table ensured (v0.9.45)");

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS streak_freeze_quota (
            user_id         UUID            NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            year_month      INTEGER         NOT NULL,
            used_count      SMALLINT        NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, year_month)
        )"
    ).execute(pool).await;
    log::info!("  ✅ streak_freeze_quota table ensured (v0.9.45)");

    // 37. topics.hot_score + is_hot + last_activity_at (Giai đoạn 59)
    let _ = sqlx::query("ALTER TABLE topics ADD COLUMN IF NOT EXISTS hot_score DOUBLE PRECISION NOT NULL DEFAULT 0").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE topics ADD COLUMN IF NOT EXISTS last_activity_at TIMESTAMPTZ").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE topics ADD COLUMN IF NOT EXISTS is_hot BOOLEAN NOT NULL DEFAULT false").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE topics ADD COLUMN IF NOT EXISTS hot_score_at TIMESTAMPTZ").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_topics_hot_score ON topics(hot_score DESC)").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_topics_is_hot ON topics(is_hot) WHERE is_hot = true").execute(pool).await;
    log::info!("  ✅ topics.hot_score + is_hot + last_activity_at ensured (v0.9.45)");

    // ─── v0.9.46 — Giai đoạn 61-70: safety schema cho 10 stages mới ───
    // 38. buddha_daily_uses + kinh_nguyen_board + lucky_spin_grants (Giai đoạn 61)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS buddha_daily_uses (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            use_date DATE NOT NULL DEFAULT CURRENT_DATE,
            prayer_count SMALLINT NOT NULL DEFAULT 0,
            repentance_count SMALLINT NOT NULL DEFAULT 0,
            dedication_count SMALLINT NOT NULL DEFAULT 0,
            support_count SMALLINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, use_date)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_buddha_daily_uses_user_date ON buddha_daily_uses(user_id, use_date DESC)").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS kinh_nguyen_board (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            vow_type VARCHAR(20) NOT NULL,
            content TEXT NOT NULL,
            is_public BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_kinh_nguyen_board_public_recent ON kinh_nguyen_board(created_at DESC) WHERE is_public = true").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_kinh_nguyen_board_user ON kinh_nguyen_board(user_id, created_at DESC)").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS lucky_spin_grants (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            source VARCHAR(20) NOT NULL,
            granted_date DATE NOT NULL DEFAULT CURRENT_DATE,
            is_used BOOLEAN NOT NULL DEFAULT false,
            used_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_lucky_spin_grants_user_unused ON lucky_spin_grants(user_id) WHERE is_used = false").execute(pool).await;
    log::info!("  ✅ buddha_daily_uses + kinh_nguyen_board + lucky_spin_grants ensured (v0.9.46 Giai đoạn 61)");

    // 39. lucky_wheel_prizes + lucky_wheel_spins + lucky_wheel_daily_quota (Giai đoạn 62)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS lucky_wheel_prizes (
            id BIGSERIAL PRIMARY KEY,
            code VARCHAR(40) NOT NULL UNIQUE,
            label VARCHAR(100) NOT NULL,
            emoji VARCHAR(10) NOT NULL DEFAULT '🎁',
            reward_type VARCHAR(20) NOT NULL,
            reward_amount BIGINT NOT NULL DEFAULT 0,
            reward_item_code VARCHAR(40),
            weight DOUBLE PRECISION NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    // Seed prizes nếu chưa có
    let _ = sqlx::query(
        "INSERT INTO lucky_wheel_prizes (code, label, emoji, reward_type, reward_amount, weight, is_active)
         SELECT * FROM (VALUES
            ('a_small','Niệm Lực A (1-100)','✨','a',50,25.0,true),
            ('a_medium','Niệm Lực A (100-500)','💫','a',250,15.0,true),
            ('a_big','Niệm Lực A (500-1000)','🌟','a',750,10.0,true),
            ('k_small','Tiền K (1-5)','🪙','k',3,5.0,true),
            ('k_big','Tiền K (5-10)','💰','k',7,5.0,true),
            ('tinh_the','Tinh Thể','🔮','item',1,10.0,true),
            ('tinh_thach','Tinh Thạch','💎','item',1,5.0,true),
            ('linh_thach','Linh Thạch','🌈','item',1,5.0,true),
            ('tien_thach','Tiên Thạch','✨','item',1,1.0,true),
            ('bao_li_xi','Bao Lì Xì Từ Bi (10K)','🧧','item',1,4.9,true),
            ('da_thuc_tinh','Đá Thức Tỉnh Thiên Phú','⚡','item',1,1.0,true),
            ('nothing','Xịt (chưa may mắn)','💦','nothing',0,12.1,true)
         ) AS t(code,label,emoji,reward_type,reward_amount,weight,is_active)
         WHERE NOT EXISTS (SELECT 1 FROM lucky_wheel_prizes LIMIT 1)"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS lucky_wheel_spins (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            prize_id BIGINT NOT NULL REFERENCES lucky_wheel_prizes(id),
            source VARCHAR(20) NOT NULL,
            reward_given VARCHAR(20) NOT NULL,
            reward_amount BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_lucky_wheel_spins_user_recent ON lucky_wheel_spins(user_id, created_at DESC)").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS lucky_wheel_daily_quota (
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            quota_date DATE NOT NULL DEFAULT CURRENT_DATE,
            free_spins_used SMALLINT NOT NULL DEFAULT 0,
            ad_spins_used SMALLINT NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, quota_date)
        )"
    ).execute(pool).await;
    log::info!("  ✅ lucky_wheel_prizes + spins + daily_quota ensured (v0.9.46 Giai đoạn 62)");

    // 40. red_envelopes + red_envelope_claims (Giai đoạn 63)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS red_envelopes (
            id BIGSERIAL PRIMARY KEY,
            creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            envelope_type VARCHAR(20) NOT NULL,
            total_k BIGINT NOT NULL,
            remaining_k BIGINT NOT NULL,
            total_claims SMALLINT NOT NULL DEFAULT 0,
            max_claims SMALLINT NOT NULL DEFAULT 20,
            message VARCHAR(200),
            scope VARCHAR(20) NOT NULL DEFAULT 'public',
            target_group_id BIGINT,
            is_active BOOLEAN NOT NULL DEFAULT true,
            expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_red_envelopes_active_recent ON red_envelopes(created_at DESC) WHERE is_active = true").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS red_envelope_claims (
            id BIGSERIAL PRIMARY KEY,
            envelope_id BIGINT NOT NULL REFERENCES red_envelopes(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            amount_k BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (envelope_id, user_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_red_envelope_claims_user ON red_envelope_claims(user_id, created_at DESC)").execute(pool).await;
    log::info!("  ✅ red_envelopes + claims ensured (v0.9.46 Giai đoạn 63)");

    // 41. tinh_khi_than + system_items + user_inventories + item_use_log (Giai đoạn 64)
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS tinh_khi_than SMALLINT NOT NULL DEFAULT 0").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS max_tinh_khi_than SMALLINT NOT NULL DEFAULT 100").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS system_items (
            id BIGSERIAL PRIMARY KEY,
            code VARCHAR(40) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            emoji VARCHAR(10) NOT NULL DEFAULT '🎁',
            description TEXT,
            price_k BIGINT NOT NULL DEFAULT 0,
            category VARCHAR(30) NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query(
        "INSERT INTO system_items (code, name, emoji, description, price_k, category, is_active)
         SELECT * FROM (VALUES
            ('tinh_the','Tinh Thể','🔮','Nuốt để tăng 1 điểm Tinh Khí Thần. Tối đa 10 Tinh Thể cho mỗi cấp.',1,'crystal',true),
            ('tinh_thach','Tinh Thạch','💎','Dùng để tăng cấp kỹ năng bị động.',2,'crystal',true),
            ('linh_thach','Linh Thạch','🌈','Dùng để tăng cấp nhân vật game.',5,'crystal',true),
            ('tien_thach','Tiên Thạch','✨','Dùng để tăng điểm ưu tiên cho Thiên Phú.',100,'crystal',true),
            ('da_thuc_tinh','Đá Thức Tỉnh Thiên Phú','⚡','Mở Vòng Quay Thức Tỉnh Thiên Phú.',10,'crystal',true),
            ('bao_li_xi','Bao Lì Xì Từ Bi (10K)','🧧','Tạo 1 bao lì xì 10K chia cho nhiều người.',10,'consumable',true),
            ('the_ung_ho','Thẻ Ủng Hộ','🙏','Quay vòng miễn phí, không phải xem quảng cáo.',5,'consumable',true)
         ) AS t(code,name,emoji,description,price_k,category,is_active)
         WHERE NOT EXISTS (SELECT 1 FROM system_items LIMIT 1)"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_inventories (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            item_id BIGINT NOT NULL REFERENCES system_items(id),
            quantity BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, item_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_inventories_user ON user_inventories(user_id) WHERE quantity > 0").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS item_use_log (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            item_code VARCHAR(40) NOT NULL,
            quantity_used SMALLINT NOT NULL DEFAULT 1,
            effect VARCHAR(100),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_item_use_log_user ON item_use_log(user_id, created_at DESC)").execute(pool).await;
    log::info!("  ✅ tinh_khi_than + system_items + inventories ensured (v0.9.46 Giai đoạn 64)");

    // 42. garden_plant_types + user_gardens + garden_slots (Giai đoạn 65)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS garden_plant_types (
            id BIGSERIAL PRIMARY KEY,
            code VARCHAR(40) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            emoji VARCHAR(10) NOT NULL DEFAULT '🪷',
            growth_seconds BIGINT NOT NULL,
            cost_k BIGINT NOT NULL DEFAULT 0,
            reward_a BIGINT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true
        )"
    ).execute(pool).await;
    let _ = sqlx::query(
        "INSERT INTO garden_plant_types (code, name, emoji, growth_seconds, cost_k, reward_a, is_active)
         SELECT * FROM (VALUES
            ('hoa_sen_nho','Hoa Sen Nhỏ','🪷',300,0,5,true),
            ('hoa_sen_trung','Hoa Sen Trung','🌸',1800,1,20,true),
            ('hoa_sen_lon','Hoa Sen Lớn','🌺',7200,5,100,true),
            ('cay_bo_de','Cây Bồ Đề','🌳',86400,20,500,true)
         ) AS t(code,name,emoji,growth_seconds,cost_k,reward_a,is_active)
         WHERE NOT EXISTS (SELECT 1 FROM garden_plant_types LIMIT 1)"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_gardens (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
            max_slots SMALLINT NOT NULL DEFAULT 9,
            total_harvest BIGINT NOT NULL DEFAULT 0,
            total_a_earned BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS garden_slots (
            id BIGSERIAL PRIMARY KEY,
            garden_id BIGINT NOT NULL REFERENCES user_gardens(id) ON DELETE CASCADE,
            slot_index SMALLINT NOT NULL,
            plant_type_id BIGINT REFERENCES garden_plant_types(id),
            planted_at TIMESTAMPTZ,
            is_ready BOOLEAN NOT NULL DEFAULT false,
            ready_at TIMESTAMPTZ,
            UNIQUE (garden_id, slot_index)
        )"
    ).execute(pool).await;
    log::info!("  ✅ garden_plant_types + user_gardens + garden_slots ensured (v0.9.46 Giai đoạn 65)");

    // 43. meditation_sessions + meditation_participants (Giai đoạn 66)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS meditation_sessions (
            id BIGSERIAL PRIMARY KEY,
            host_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title VARCHAR(200) NOT NULL,
            description TEXT,
            start_at TIMESTAMPTZ NOT NULL,
            duration_minutes SMALLINT NOT NULL DEFAULT 30,
            max_seats SMALLINT NOT NULL DEFAULT 10,
            is_active BOOLEAN NOT NULL DEFAULT true,
            is_cancelled BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_meditation_sessions_active_recent ON meditation_sessions(start_at) WHERE is_active = true AND is_cancelled = false").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS meditation_participants (
            id BIGSERIAL PRIMARY KEY,
            session_id BIGINT NOT NULL REFERENCES meditation_sessions(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            seat_number SMALLINT NOT NULL,
            joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            a_earned BIGINT NOT NULL DEFAULT 0,
            UNIQUE (session_id, user_id),
            UNIQUE (session_id, seat_number)
        )"
    ).execute(pool).await;
    log::info!("  ✅ meditation_sessions + participants ensured (v0.9.46 Giai đoạn 66)");

    // 44. teleport_visits + teleport_bookmarks (Giai đoạn 67)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS teleport_visits (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            target_user_id UUID,
            target_group_id BIGINT,
            target_type VARCHAR(20) NOT NULL,
            visited_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_teleport_visits_user ON teleport_visits(user_id, visited_at DESC)").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS teleport_bookmarks (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            target_type VARCHAR(20) NOT NULL,
            target_id VARCHAR(60) NOT NULL,
            label VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, target_type, target_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_teleport_bookmarks_user ON teleport_bookmarks(user_id, created_at DESC)").execute(pool).await;
    log::info!("  ✅ teleport_visits + bookmarks ensured (v0.9.46 Giai đoạn 67)");

    // 45. buddhist_events + event_reward_claims (Giai đoạn 68)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS buddhist_events (
            id BIGSERIAL PRIMARY KEY,
            code VARCHAR(40) NOT NULL UNIQUE,
            name VARCHAR(200) NOT NULL,
            emoji VARCHAR(10) NOT NULL DEFAULT '🪷',
            description TEXT,
            event_date DATE NOT NULL,
            is_recurring BOOLEAN NOT NULL DEFAULT true,
            bonus_a BIGINT NOT NULL DEFAULT 0,
            bonus_k BIGINT NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query(
        "INSERT INTO buddhist_events (code, name, emoji, description, event_date, is_recurring, bonus_a, bonus_k, is_active)
         SELECT * FROM (VALUES
            ('phat_dan','Lễ Phật Đản','🪷','Ngày Đức Phật Thích Ca Mâu Ni đản sinh.','0015-04-08',true,100,1,true),
            ('thanh_dinh','Lễ Thanh Đinh','🕯️','Ngày Đức Phật thành đạo.','0015-12-08',true,100,1,true),
            ('tiet_tu_lan_bon','Lễ Tết Trung Nguyên (Vu Lan)','👁️','Ngày báo hiếu cha mẹ, xá tội vong nhân.','0015-07-15',true,100,1,true),
            ('ky_niem_duc_phat','Kỷ niệm Đức Phật nhập niết bàn','🪷','Ngày Đức Phật nhập Niết bàn.','0015-02-15',true,50,0,true),
            ('dong_chi','Tết Đông Chí','❄️','Ngày đông chí — truyền thống Á Đông.','0015-11-22',true,30,0,true),
            ('tet_nguyen_dan','Tết Nguyên Đán','🧧','Tết cổ truyền Việt Nam.','0015-01-01',true,50,1,true),
            ('tet_nguyen_tieu','Tết Nguyên Tiêu (Rằm tháng Giêng)','🏮','Tết Nguyên Tiêu — đầu năm âm lịch.','0015-01-15',true,30,0,true)
         ) AS t(code,name,emoji,description,event_date,is_recurring,bonus_a,bonus_k,is_active)
         WHERE NOT EXISTS (SELECT 1 FROM buddhist_events LIMIT 1)"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS event_reward_claims (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            event_id BIGINT NOT NULL REFERENCES buddhist_events(id) ON DELETE CASCADE,
            event_date DATE NOT NULL,
            reward_a BIGINT NOT NULL DEFAULT 0,
            reward_k BIGINT NOT NULL DEFAULT 0,
            claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, event_id, event_date)
        )"
    ).execute(pool).await;
    log::info!("  ✅ buddhist_events + event_reward_claims ensured (v0.9.46 Giai đoạn 68)");

    // 46. achievement_badges + user_badges (Giai đoạn 69)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS achievement_badges (
            id BIGSERIAL PRIMARY KEY,
            code VARCHAR(40) NOT NULL UNIQUE,
            name VARCHAR(100) NOT NULL,
            emoji VARCHAR(10) NOT NULL DEFAULT '🏆',
            description TEXT,
            category VARCHAR(30) NOT NULL,
            requirement_type VARCHAR(30) NOT NULL,
            requirement_value BIGINT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query(
        "INSERT INTO achievement_badges (code, name, emoji, description, category, requirement_type, requirement_value, is_active)
         SELECT * FROM (VALUES
            ('niem_100','Người Tu Tập','📿','Niệm Phật 100 lần','tu_hoc','niem_count',100,true),
            ('niem_1000','Niệm Phật 1000 lần','📿','Niệm Phật 1000 lần','tu_hoc','niem_count',1000,true),
            ('niem_10000','Đại Niệm Phật','🪷','Niệm Phật 10000 lần','tu_hoc','niem_count',10000,true),
            ('days_7','Kiên Trì 7 Ngày','🔥','Niệm Phật 7 ngày liên tiếp','tu_hoc','days_active',7,true),
            ('days_30','Tinh Tấn 30 Ngày','🌟','Niệm Phật 30 ngày liên tiếp','tu_hoc','days_active',30,true),
            ('friend_10','Kết Duyên','👥','Có 10 người bạn','cong_dong','friend_count',10,true),
            ('friend_50','Đa Duyên','🤝','Có 50 người bạn','cong_dong','friend_count',50,true),
            ('group_1','Thành Viên Tích Cực','✨','Tham gia 1 nhóm cộng đồng','cong_dong','group_count',1,true),
            ('group_5','Cộng Đồng Viên','🌐','Tham gia 5 nhóm cộng đồng','cong_dong','group_count',5,true),
            ('topic_1','Người Khởi Xướng','📝','Tạo 1 chủ đề cộng đồng','cong_dong','topic_count',1,true),
            ('topic_10','Tác Giả Cộng Đồng','📚','Tạo 10 chủ đề cộng đồng','cong_dong','topic_count',10,true),
            ('a_100','Tiểu Niệm Lực','⚡','Tích lũy 100 A','tai_chinh','a_balance',100,true),
            ('a_1000','Niệm Lực Sơ','⚡','Tích lũy 1000 A','tai_chinh','a_balance',1000,true),
            ('k_100','Tiểu Tài Phú','💰','Tích lũy 100 K','tai_chinh','k_balance',100,true),
            ('k_1000','Tài Phú','💰','Tích lũy 1000 K','tai_chinh','k_balance',1000,true),
            ('k_10000','Đại Tài Phú','💎','Tích lũy 10000 K','tai_chinh','k_balance',10000,true),
            ('tu_si_1','Tu Sĩ Tập Sự','⭐','Trở thành Tu Sĩ 1 sao','dac_biet','tu_si_rank',1,true),
            ('tu_si_5','Đại Tu Sĩ','🌟','Trở thành Tu Sĩ 5 sao','dac_biet','tu_si_rank',5,true)
         ) AS t(code,name,emoji,description,category,requirement_type,requirement_value,is_active)
         WHERE NOT EXISTS (SELECT 1 FROM achievement_badges LIMIT 1)"
    ).execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_badges (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            badge_id BIGINT NOT NULL REFERENCES achievement_badges(id) ON DELETE CASCADE,
            awarded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (user_id, badge_id)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges(user_id, awarded_at DESC)").execute(pool).await;
    log::info!("  ✅ achievement_badges + user_badges ensured (v0.9.46 Giai đoạn 69)");

    // 47. hall_of_fame_entries + hall_of_fame_honors (Giai đoạn 70)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS hall_of_fame_entries (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            category VARCHAR(30) NOT NULL,
            rank_position SMALLINT NOT NULL,
            score BIGINT NOT NULL,
            period_label VARCHAR(20),
            snapshot_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (category, period_label, rank_position)
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_hof_entries_recent ON hall_of_fame_entries(snapshot_at DESC)").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_hof_entries_category ON hall_of_fame_entries(category, rank_position)").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS hall_of_fame_honors (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title VARCHAR(100) NOT NULL,
            description TEXT,
            awarded_by UUID REFERENCES users(id),
            awarded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            is_active BOOLEAN NOT NULL DEFAULT true
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_hof_honors_recent ON hall_of_fame_honors(awarded_at DESC) WHERE is_active = true").execute(pool).await;
    log::info!("  ✅ hall_of_fame_entries + honors ensured (v0.9.46 Giai đoạn 70)");

    // 48. practice_diaries + diary_comments (Giai đoạn 71 — v0.9.47)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS practice_diaries (
            id              BIGSERIAL    PRIMARY KEY,
            user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title           VARCHAR(200) NOT NULL,
            content         TEXT         NOT NULL,
            mood            VARCHAR(20)  NOT NULL DEFAULT 'peace',
            is_public       BOOLEAN      NOT NULL DEFAULT true,
            allow_comments  BOOLEAN      NOT NULL DEFAULT true,
            view_count      INTEGER      NOT NULL DEFAULT 0,
            comment_count   INTEGER      NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_practice_diaries_user ON practice_diaries(user_id, created_at DESC)").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_practice_diaries_public_recent ON practice_diaries(created_at DESC) WHERE is_public = true").execute(pool).await;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS diary_comments (
            id              BIGSERIAL    PRIMARY KEY,
            diary_id        BIGINT       NOT NULL REFERENCES practice_diaries(id) ON DELETE CASCADE,
            user_id         UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            content         TEXT         NOT NULL,
            is_hidden       BOOLEAN      NOT NULL DEFAULT false,
            created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )"
    ).execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_diary_comments_diary ON diary_comments(diary_id, created_at DESC)").execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_diary_comments_user ON diary_comments(user_id, created_at DESC)").execute(pool).await;
    log::info!("  ✅ practice_diaries + diary_comments ensured (v0.9.47 Giai đoạn 71)");

    log::info!("🔒 Safety schema check hoàn tất");
}

// ─── Forbidden Words Check ─────────────────────────────────────────────────

/// Kết quả kiểm tra từ vựng cấm.
///
/// v0.9.42 — Giai đoạn 46: Forbidden Words Auto-Check.
/// Khi user submit content (comment, topic, chat, mail, music),
/// gọi `check_forbidden_words` để kiểm tra nội dung có chứa từ cấm không.
#[derive(Debug, Clone)]
pub struct ForbiddenWordsResult {
    /// True nếu nội dung chứa từ cấm với action='block' — PHẢI chặn.
    pub should_block: bool,
    /// True nếu nội dung chứa từ cấm với action='flag' — cho phép nhưng flag.
    pub should_flag: bool,
    /// Danh sách từ cấm tìm thấy (để hiển thị cho user).
    pub matched_words: Vec<String>,
    /// Mô tả chi tiết (cho log).
    pub detail: String,
}

impl ForbiddenWordsResult {
    /// Không có từ cấm — nội dung an toàn.
    pub fn clean() -> Self {
        Self {
            should_block: false,
            should_flag: false,
            matched_words: vec![],
            detail: String::new(),
        }
    }

    /// Có từ cấm block — chặn nội dung.
    pub fn blocked(words: Vec<String>) -> Self {
        let detail = format!("Chặn vì chứa từ cấm: {}", words.join(", "));
        Self {
            should_block: true,
            should_flag: false,
            matched_words: words,
            detail,
        }
    }

    /// Có từ cấm flag — cho phép nhưng đánh dấu.
    pub fn flagged(words: Vec<String>) -> Self {
        let detail = format!("Flag vì chứa từ nhạy cảm: {}", words.join(", "));
        Self {
            should_block: false,
            should_flag: true,
            matched_words: words,
            detail,
        }
    }

    /// Vừa block vừa flag (nhiều loại từ cấm).
    pub fn blocked_and_flagged(block_words: Vec<String>, flag_words: Vec<String>) -> Self {
        let mut all = block_words.clone();
        all.extend_from_slice(&flag_words);
        let detail = format!(
            "Chặn vì chứa từ cấm: {} | Flag vì chứa từ nhạy cảm: {}",
            block_words.join(", "),
            flag_words.join(", ")
        );
        Self {
            should_block: true,
            should_flag: true,
            matched_words: all,
            detail,
        }
    }
}

/// Kiểm tra nội dung có chứa từ vựng cấm không.
///
/// v0.9.42 — Giai đoạn 46: Forbidden Words Auto-Check.
/// Query bảng `forbidden_words` (is_active=true) và kiểm tra từng từ
/// xem có xuất hiện trong nội dung không (case-insensitive).
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `content` — Nội dung cần kiểm tra (title, body, message, v.v.)
///
/// # Returns
/// * `ForbiddenWordsResult` — Kết quả kiểm tra (block/flag/clean)
///
/// # Performance
/// Chỉ query từ cấm active một lần, cache trong hàm gọi.
/// Với số lượng từ cấm nhỏ (< 100), linear scan là đủ.
pub async fn check_forbidden_words(pool: &PgPool, content: &str) -> ForbiddenWordsResult {
    // Lấy tất cả từ cấm đang active
    let rows = match sqlx::query_as::<_, (String, String)>(
        "SELECT word, action FROM forbidden_words WHERE is_active = true ORDER BY word"
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            // Bảng forbidden_words có thể chưa tồn tại — không chặn content.
            log::warn!("⚠️ check_forbidden_words: không query được forbidden_words: {e}");
            return ForbiddenWordsResult::clean();
        }
    };

    let content_lower = content.to_lowercase();
    let mut block_words: Vec<String> = vec![];
    let mut flag_words: Vec<String> = vec![];

    for (word, action) in &rows {
        let word_lower = word.to_lowercase();
        // Kiểm tra từ/cụm từ xuất hiện trong nội dung (case-insensitive)
        if content_lower.contains(&word_lower) {
            match action.as_str() {
                "block" => block_words.push(word.clone()),
                "flag" => flag_words.push(word.clone()),
                _ => {}
            }
        }
    }

    let has_block = !block_words.is_empty();
    let has_flag = !flag_words.is_empty();

    match (has_block, has_flag) {
        (true, true) => ForbiddenWordsResult::blocked_and_flagged(block_words, flag_words),
        (true, false) => ForbiddenWordsResult::blocked(block_words),
        (false, true) => ForbiddenWordsResult::flagged(flag_words),
        (false, false) => ForbiddenWordsResult::clean(),
    }
}

/// Kiểm tra nhiều trường nội dung (title + body + description, v.v.).
/// Gộp tất cả thành 1 chuỗi để kiểm tra 1 lần (hiệu quả hơn).
pub async fn check_forbidden_words_multi(pool: &PgPool, parts: &[&str]) -> ForbiddenWordsResult {
    let combined = parts.join(" ");
    check_forbidden_words(pool, &combined).await
}

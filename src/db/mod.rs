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

    log::info!("🔒 Safety schema check hoàn tất");
}

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

    // v0.9.19: Drop old CHECK constraint (chỉ cho phép 4 giá trị cũ) và thay bằng
    // CHECK constraint mới cho phép 5 giá trị: member, mod, admin_ky_thuat, admin_cong_dong, admin_quan_li.
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
                CHECK (role IN ('member', 'mod', 'admin_ky_thuat', 'admin_cong_dong', 'admin_quan_li')); \
            END IF; \
        END $$"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ users_role_check constraint updated (v0.9.19: + 'mod')"),
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
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS permissions (
            id         BIGSERIAL    PRIMARY KEY,
            code       VARCHAR(60)  NOT NULL UNIQUE,
            name       VARCHAR(200) NOT NULL,
            category   VARCHAR(30)  NOT NULL,
            sort_order INT          NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
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
            role_code      VARCHAR(30) NOT NULL,
            permission_id  BIGINT      NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
            assigned_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (role_code, permission_id)
        )"
    )
    .execute(pool)
    .await
    {
        Ok(_) => log::info!("  ✅ role_permissions table ensured"),
        Err(e) => log::error!("  ❌ Failed to ensure role_permissions: {e}"),
    }

    log::info!("🔒 Safety schema check hoàn tất");
}

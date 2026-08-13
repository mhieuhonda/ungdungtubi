// Database module — helper functions for PostgreSQL interactions
// Will be expanded in future phases with connection utilities,
// session management helpers, and migration runners.

use sqlx::PgPool;

/// Clean up expired sessions from the database.
/// Should be called periodically (e.g., every hour) via a background task.
pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use sqlx::SqlitePool;

use crate::beads::types::BeadsError;

/// Create the issues table schema if it does not exist.
///
/// # Errors
///
/// Returns `BeadsError` if the schema creation fails.
pub async fn ensure_schema(pool: &SqlitePool) -> Result<(), BeadsError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS issues (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            priority TEXT,
            type TEXT,
            description TEXT,
            labels TEXT,
            assignee TEXT,
            parent TEXT,
            depends_on TEXT,
            blocked_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| BeadsError::DatabaseError(format!("Failed to create issues schema: {e}")))?;
    Ok(())
}

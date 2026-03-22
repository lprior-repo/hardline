#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use sqlx::SqlitePool;

use crate::beads::types::BeadsError;
use crate::beads::db::parsing::parse_bead_row;

/// Enable `WAL` mode on the `SQLite` connection for better crash recovery.
///
/// # Errors
///
/// Returns `BeadsError` if the `PRAGMA` statement fails.
pub(crate) async fn enable_wal_mode(pool: &SqlitePool) -> Result<(), BeadsError> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to enable WAL mode: {e}")))?;
    Ok(())
}

/// Query all issues from the beads database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - The database cannot be opened or queried
/// - Any required field is missing or malformed
/// - Status or datetime values are invalid
pub async fn query_beads(workspace_path: &Path) -> Result<Vec<crate::beads::types::BeadIssue>, BeadsError> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        tracing::warn!(
            "Beads database not found at {}. It will be created when needed.",
            beads_db.display()
        );
        return Ok(Vec::new());
    }

    let path_str = beads_db.to_str().ok_or_else(|| {
        BeadsError::DatabaseError("Beads database path contains invalid UTF-8".to_string())
    })?;

    let db_url = format!("sqlite://{path_str}?mode=rw");
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to connect to beads.db: {e}")))?;

    // Enable WAL mode for better crash recovery
    enable_wal_mode(&pool).await?;

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, title, status, priority, type, description, labels, assignee,
                parent, depends_on, blocked_by, created_at, updated_at, closed_at
         FROM issues ORDER BY priority, created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.iter()
        .map(parse_bead_row)
        .collect::<Result<Vec<_>, BeadsError>>()
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to parse bead issues: {e}")))
}

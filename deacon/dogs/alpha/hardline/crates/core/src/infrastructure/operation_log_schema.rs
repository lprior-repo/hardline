//! Operation Log Schema Management
//!
//! This module provides SQLite schema creation and management for the operation_log table.

use sqlx::SqlitePool;

use super::operation_log_types::OperationLogError;

/// Create the operation_log table schema if it does not exist.
///
/// The table is append-only and uses auto-increment for IDs.
/// Indexes are created on stream_id and created_at for efficient querying.
///
/// # Errors
///
/// Returns `OperationLogError::DatabaseError` if the schema creation fails.
pub async fn ensure_operation_log_schema(pool: &SqlitePool) -> Result<(), OperationLogError> {
    // Create the main operation_log table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS operation_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            stream_id TEXT NOT NULL,
            stream_version INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        OperationLogError::DatabaseError(format!("Failed to create operation_log schema: {e}"))
    })?;

    // Create index on stream_id for efficient event sourcing queries
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_log_stream_id ON operation_log(stream_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        OperationLogError::DatabaseError(format!("Failed to create stream_id index: {e}"))
    })?;

    // Create index on created_at for efficient temporal queries
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_log_created_at ON operation_log(created_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        OperationLogError::DatabaseError(format!("Failed to create created_at index: {e}"))
    })?;

    // Create composite index for stream queries ordered by version
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_log_stream_version ON operation_log(stream_id, stream_version)",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        OperationLogError::DatabaseError(format!(
            "Failed to create stream_version index: {e}"
        ))
    })?;

    Ok(())
}

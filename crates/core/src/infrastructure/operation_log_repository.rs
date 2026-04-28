//! Operation Log Repository
//!
//! This module provides database operations for the operation_log event store.

use sqlx::{Row, SqlitePool};

use super::operation_log_types::{OperationLogEntry, OperationLogError};

/// Insert a new operation log entry.
///
/// # Errors
///
/// Returns `OperationLogError` if:
/// - Validation fails (empty event_type or stream_id)
/// - The insert operation fails
pub async fn insert_operation_log(
    pool: &SqlitePool,
    entry: &OperationLogEntry,
) -> Result<OperationLogEntry, OperationLogError> {
    // Validate input
    if entry.event_type.is_empty() {
        return Err(OperationLogError::ValidationFailed(
            "event_type cannot be empty".to_string(),
        ));
    }

    if entry.stream_id.is_empty() {
        return Err(OperationLogError::ValidationFailed(
            "stream_id cannot be empty".to_string(),
        ));
    }

    let created_at_str = entry.created_at.to_rfc3339();

    // Execute insert
    let result = sqlx::query(
        "INSERT INTO operation_log (event_type, payload, stream_id, stream_version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&entry.event_type)
    .bind(&entry.payload)
    .bind(&entry.stream_id)
    .bind(entry.stream_version)
    .bind(&created_at_str)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            // Get the last inserted ID using a separate query
            // Note: last_insert_rowid() is connection-specific in SQLite
            let row = sqlx::query("SELECT last_insert_rowid() as id")
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    OperationLogError::DatabaseError(format!("Failed to get insert ID: {e}"))
                })?;

            // If last_insert_rowid returns 0, query for the max id instead
            // (this handles cases where the connection was reused)
            let id: i64 = row
                .try_get("id")
                .map_err(|e| OperationLogError::QueryFailed(format!("Field 'id' error: {e}")))?;

            let final_id = if id == 0 {
                // Fallback: get the max id from the table
                let max_row = sqlx::query("SELECT MAX(id) as max_id FROM operation_log")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| {
                        OperationLogError::DatabaseError(format!("Failed to get max ID: {e}"))
                    })?;

                max_row
                    .try_get::<Option<i64>, _>("max_id")
                    .map_err(|e| {
                        OperationLogError::QueryFailed(format!("Field 'max_id' error: {e}"))
                    })?
                    .unwrap_or(1)
            } else {
                id
            };

            Ok(OperationLogEntry {
                id: final_id,
                event_type: entry.event_type.clone(),
                payload: entry.payload.clone(),
                stream_id: entry.stream_id.clone(),
                stream_version: entry.stream_version,
                created_at: entry.created_at,
            })
        }
        Err(e) => Err(OperationLogError::DatabaseError(format!(
            "Failed to insert operation log entry: {e}"
        ))),
    }
}

/// Query all operation log entries for a specific stream, ordered by version.
///
/// # Errors
///
/// Returns `OperationLogError::QueryFailed` if the query fails.
pub async fn query_stream_events(
    pool: &SqlitePool,
    stream_id: &str,
) -> Result<Vec<OperationLogEntry>, OperationLogError> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, event_type, payload, stream_id, stream_version, created_at
         FROM operation_log
         WHERE stream_id = ?1
         ORDER BY stream_version ASC",
    )
    .bind(stream_id)
    .fetch_all(pool)
    .await
    .map_err(|e| OperationLogError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.iter()
        .map(super::operation_log_types::parse_operation_log_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OperationLogError::QueryFailed(format!("Failed to parse results: {e}")))
}

/// Query all operation log entries, ordered by creation time.
///
/// # Errors
///
/// Returns `OperationLogError::QueryFailed` if the query fails.
pub async fn query_all_operations(
    pool: &SqlitePool,
    limit: Option<u32>,
) -> Result<Vec<OperationLogEntry>, OperationLogError> {
    let rows: Vec<sqlx::sqlite::SqliteRow> = match limit {
        Some(lim) => {
            sqlx::query(
                "SELECT id, event_type, payload, stream_id, stream_version, created_at
                 FROM operation_log
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .bind(lim)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT id, event_type, payload, stream_id, stream_version, created_at
                 FROM operation_log
                 ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| OperationLogError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.iter()
        .map(super::operation_log_types::parse_operation_log_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OperationLogError::QueryFailed(format!("Failed to parse results: {e}")))
}

/// Get the current version for a stream (for optimistic locking).
///
/// # Errors
///
/// Returns `OperationLogError::QueryFailed` if the query fails.
pub async fn get_stream_version(
    pool: &SqlitePool,
    stream_id: &str,
) -> Result<i64, OperationLogError> {
    let result = sqlx::query(
        "SELECT COALESCE(MAX(stream_version), 0) as version
         FROM operation_log
         WHERE stream_id = ?1",
    )
    .bind(stream_id)
    .fetch_one(pool)
    .await
    .map_err(|e| OperationLogError::QueryFailed(format!("Failed to execute query: {e}")))?;

    let version: i64 = result
        .try_get("version")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'version' error: {e}")))?;

    Ok(version)
}

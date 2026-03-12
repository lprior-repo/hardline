//! Operation Log Database Schema
//!
//! This module provides SQLite schema and operations for the operation_log table,
//! which implements event sourcing for tracking all state changes in the system.
//!
//! # Design
//!
//! The operation_log is an append-only event store that enables:
//! - Event sourcing (rebuilding state from event history)
//! - Audit logging (complete history of all changes)
//! - Projections (deriving read models from event stream)
//! - Temporal queries (state at any point in time)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Error types for operation log operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationLogError {
    /// Database query failed
    QueryFailed(String),
    /// Database connection failed
    DatabaseError(String),
    /// Serialization/deserialization failed
    SerializationError(String),
    /// Operation not found
    NotFound(String),
    /// Invalid input validation failed
    ValidationFailed(String),
}

impl std::fmt::Display for OperationLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueryFailed(msg) => write!(f, "Query failed: {msg}"),
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::NotFound(id) => write!(f, "Operation not found: {id}"),
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {msg}"),
        }
    }
}

impl std::error::Error for OperationLogError {}

/// An entry in the operation log (event store)
///
/// Each entry represents a single domain event that has occurred in the system.
/// The payload contains the serialized event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLogEntry {
    /// Unique identifier for this log entry (auto-increment)
    pub id: i64,
    /// Type of event (e.g., "session_created", "workspace_removed")
    pub event_type: String,
    /// Serialized event payload (JSON)
    pub payload: String,
    /// Stream identifier (e.g., "session-123", "workspace-456")
    /// Used for event sourcing to group related events
    pub stream_id: String,
    /// Version number for optimistic concurrency control
    pub stream_version: i64,
    /// When this event was created
    pub created_at: DateTime<Utc>,
}

impl OperationLogEntry {
    /// Create a new operation log entry
    ///
    /// # Errors
    ///
    /// Returns `OperationLogError::ValidationFailed` if:
    /// - event_type is empty
    /// - stream_id is empty
    pub fn new(
        event_type: impl Into<String>,
        payload: impl Into<String>,
        stream_id: impl Into<String>,
        stream_version: i64,
    ) -> Result<Self, OperationLogError> {
        let event_type = event_type.into();
        let stream_id = stream_id.into();

        if event_type.is_empty() {
            return Err(OperationLogError::ValidationFailed(
                "event_type cannot be empty".to_string(),
            ));
        }

        if stream_id.is_empty() {
            return Err(OperationLogError::ValidationFailed(
                "stream_id cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            id: 0, // Will be assigned by database
            event_type,
            payload: payload.into(),
            stream_id,
            stream_version,
            created_at: Utc::now(),
        })
    }
}

/// Parse a datetime string from SQLite TEXT format (RFC3339).
///
/// # Errors
///
/// Returns `OperationLogError::QueryFailed` if the datetime string is invalid.
fn parse_datetime(datetime_str: Option<String>) -> Result<DateTime<Utc>, OperationLogError> {
    datetime_str
        .ok_or_else(|| {
            OperationLogError::QueryFailed("Missing required datetime field".to_string())
        })
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    OperationLogError::QueryFailed(format!("Invalid datetime format '{s}': {e}"))
                })
        })
}

/// Parse a row from the operation_log table into an `OperationLogEntry`.
///
/// # Errors
///
/// Returns `OperationLogError::QueryFailed` if any required field is missing or malformed.
fn parse_operation_log_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<OperationLogEntry, OperationLogError> {
    let id: i64 = row
        .try_get("id")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'id' error: {e}")))?;

    let event_type: String = row
        .try_get("event_type")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'event_type' error: {e}")))?;

    let payload: String = row
        .try_get("payload")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'payload' error: {e}")))?;

    let stream_id: String = row
        .try_get("stream_id")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'stream_id' error: {e}")))?;

    let stream_version: i64 = row.try_get("stream_version").map_err(|e| {
        OperationLogError::QueryFailed(format!("Field 'stream_version' error: {e}"))
    })?;

    let created_at_str: Option<String> = row
        .try_get("created_at")
        .map_err(|e| OperationLogError::QueryFailed(format!("Field 'created_at' error: {e}")))?;
    let created_at = parse_datetime(created_at_str)?;

    Ok(OperationLogEntry {
        id,
        event_type,
        payload,
        stream_id,
        stream_version,
        created_at,
    })
}

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
        .map(parse_operation_log_row)
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
        .map(parse_operation_log_row)
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    async fn create_test_pool() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        // Ensure schema is created
        ensure_operation_log_schema(&pool)
            .await
            .expect("Failed to create schema");

        (pool, temp_dir)
    }

    // Behavior: Creating a valid operation log entry succeeds
    #[tokio::test]
    async fn given_valid_entry_when_create_then_succeeds() {
        let entry = OperationLogEntry::new(
            "session_created",
            r#"{"session_id": "s1", "name": "test"}"#,
            "session-s1",
            1,
        )
        .expect("Failed to create entry");

        assert!(!entry.event_type.is_empty());
        assert!(!entry.stream_id.is_empty());
    }

    // Behavior: Creating entry with empty event_type fails
    #[tokio::test]
    async fn given_empty_event_type_when_create_then_returns_validation_error() {
        let result = OperationLogEntry::new("", r#"{"data": "test"}"#, "stream-1", 1);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, OperationLogError::ValidationFailed(_)));
        }
    }

    // Behavior: Creating entry with empty stream_id fails
    #[tokio::test]
    async fn given_empty_stream_id_when_create_then_returns_validation_error() {
        let result = OperationLogEntry::new("test_event", r#"{"data": "test"}"#, "", 1);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, OperationLogError::ValidationFailed(_)));
        }
    }

    // Behavior: Insert a valid operation log entry into database
    #[tokio::test]
    async fn given_valid_entry_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        let entry = OperationLogEntry::new(
            "session_created",
            r#"{"session_id": "s1", "name": "test"}"#,
            "session-s1",
            1,
        )
        .expect("Failed to create entry");

        let result = insert_operation_log(&pool, &entry).await;
        assert!(result.is_ok());

        let inserted = result.unwrap();
        assert!(inserted.id > 0);
        assert_eq!(inserted.event_type, "session_created");
    }

    // Behavior: Query stream events returns all events for that stream
    #[tokio::test]
    async fn given_multiple_events_when_query_stream_then_returns_all() {
        let (pool, _temp_dir) = create_test_pool().await;

        // Insert multiple events for same stream
        let events = vec![
            OperationLogEntry::new(
                "session_created",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                1,
            )
            .unwrap(),
            OperationLogEntry::new(
                "session_activated",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                2,
            )
            .unwrap(),
            OperationLogEntry::new(
                "session_completed",
                r#"{"session_id": "s1"}"#,
                "session-s1",
                3,
            )
            .unwrap(),
        ];

        for event in &events {
            insert_operation_log(&pool, event)
                .await
                .expect("Insert failed");
        }

        // Query the stream
        let results = query_stream_events(&pool, "session-s1")
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].stream_version, 1);
        assert_eq!(results[1].stream_version, 2);
        assert_eq!(results[2].stream_version, 3);
    }

    // Behavior: Get stream version returns correct version
    #[tokio::test]
    async fn given_events_when_get_stream_version_then_returns_max() {
        let (pool, _temp_dir) = create_test_pool().await;

        // Insert events
        let events = vec![
            OperationLogEntry::new("event1", "{}", "stream-1", 1).unwrap(),
            OperationLogEntry::new("event2", "{}", "stream-1", 2).unwrap(),
            OperationLogEntry::new("event3", "{}", "stream-1", 3).unwrap(),
        ];

        for event in &events {
            insert_operation_log(&pool, event)
                .await
                .expect("Insert failed");
        }

        let version = get_stream_version(&pool, "stream-1")
            .await
            .expect("Query failed");

        assert_eq!(version, 3);
    }

    // Behavior: Query all operations with limit
    #[tokio::test]
    async fn given_many_events_when_query_with_limit_then_respects_limit() {
        let (pool, _temp_dir) = create_test_pool().await;

        // Insert multiple events across different streams
        for i in 0..10 {
            let entry = OperationLogEntry::new(
                format!("event_{}", i),
                format!(r#"{{"i": {}}}"#, i),
                format!("stream-{}", i % 3),
                1,
            )
            .unwrap();
            insert_operation_log(&pool, &entry)
                .await
                .expect("Insert failed");
        }

        let results = query_all_operations(&pool, Some(5))
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 5);
    }

    // Behavior: Schema creation is idempotent
    #[tokio::test]
    async fn given_schema_exists_when_create_again_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        // Create schema first time
        let result1 = ensure_operation_log_schema(&pool).await;
        assert!(result1.is_ok());

        // Create schema again (should be idempotent)
        let result2 = ensure_operation_log_schema(&pool).await;
        assert!(result2.is_ok());
    }
}

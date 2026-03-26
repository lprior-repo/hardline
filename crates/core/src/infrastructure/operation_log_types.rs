//! Operation Log Types and Helpers
//!
//! This module provides the core types for the operation_log event store:
//! - Error types for operation log operations
//! - OperationLogEntry struct representing an event
//! - Row parsing helpers for database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

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
pub fn parse_datetime(datetime_str: Option<String>) -> Result<DateTime<Utc>, OperationLogError> {
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
pub fn parse_operation_log_row(
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

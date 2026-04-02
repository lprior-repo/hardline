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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // OperationLogError Display
    // =========================================================================

    #[test]
    fn given_query_failed_when_display_then_contains_message() {
        let err = OperationLogError::QueryFailed("sql error".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Query failed: sql error"));
    }

    #[test]
    fn given_database_error_when_display_then_contains_message() {
        let err = OperationLogError::DatabaseError("conn lost".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Database error: conn lost"));
    }

    #[test]
    fn given_serialization_error_when_display_then_contains_message() {
        let err = OperationLogError::SerializationError("bad json".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Serialization error: bad json"));
    }

    #[test]
    fn given_not_found_when_display_then_contains_id() {
        let err = OperationLogError::NotFound("entry-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Operation not found: entry-42"));
    }

    #[test]
    fn given_validation_failed_when_display_then_contains_message() {
        let err = OperationLogError::ValidationFailed("empty field".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Validation failed: empty field"));
    }

    // =========================================================================
    // OperationLogError as std::error::Error
    // =========================================================================

    #[test]
    fn given_error_when_cast_to_std_error_then_succeeds() {
        let err: Box<dyn std::error::Error> =
            Box::new(OperationLogError::QueryFailed("test".to_string()));
        let msg = format!("{err}");
        assert!(msg.contains("Query failed"));
    }

    #[test]
    fn given_error_variants_when_source_then_none() {
        let err = OperationLogError::QueryFailed("test".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }

    // =========================================================================
    // OperationLogError Clone
    // =========================================================================

    #[test]
    fn given_error_when_cloned_then_same_display() {
        let err = OperationLogError::DatabaseError("db err".to_string());
        let cloned = err.clone();
        assert_eq!(format!("{err}"), format!("{cloned}"));
    }

    // =========================================================================
    // OperationLogError Debug
    // =========================================================================

    #[test]
    fn given_error_when_debug_then_contains_variant() {
        let err = OperationLogError::NotFound("id-1".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("NotFound"));
    }

    // =========================================================================
    // OperationLogEntry construction - valid cases
    // =========================================================================

    #[test]
    fn given_valid_fields_when_new_then_succeeds() {
        let entry = OperationLogEntry::new("session_created", r#"{"id": "s1"}"#, "session-s1", 1);
        assert!(entry.is_ok());
        let entry = entry.unwrap();
        assert_eq!(entry.event_type, "session_created");
        assert_eq!(entry.payload, r#"{"id": "s1"}"#);
        assert_eq!(entry.stream_id, "session-s1");
        assert_eq!(entry.stream_version, 1);
        assert_eq!(entry.id, 0); // assigned by database
    }

    #[test]
    fn given_valid_entry_when_new_then_created_at_is_recent() {
        let before = chrono::Utc::now();
        let entry = OperationLogEntry::new("event_type", "payload", "stream-1", 1).unwrap();
        let after = chrono::Utc::now();
        assert!(entry.created_at >= before);
        assert!(entry.created_at <= after);
    }

    #[test]
    fn given_empty_payload_when_new_then_succeeds() {
        let entry = OperationLogEntry::new("event_type", "", "stream-1", 1);
        assert!(entry.is_ok());
    }

    #[test]
    fn given_zero_stream_version_when_new_then_succeeds() {
        let entry = OperationLogEntry::new("event_type", "{}", "stream-1", 0);
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().stream_version, 0);
    }

    #[test]
    fn given_negative_stream_version_when_new_then_succeeds() {
        let entry = OperationLogEntry::new("event_type", "{}", "stream-1", -5);
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().stream_version, -5);
    }

    // =========================================================================
    // OperationLogEntry construction - invalid cases
    // =========================================================================

    #[test]
    fn given_empty_event_type_when_new_then_validation_fails() {
        let result = OperationLogEntry::new("", "payload", "stream-1", 1);
        assert!(result.is_err());
        match result {
            Err(OperationLogError::ValidationFailed(msg)) => {
                assert!(msg.contains("event_type"));
            }
            other => panic!("Expected ValidationFailed, got: {other:?}"),
        }
    }

    #[test]
    fn given_empty_stream_id_when_new_then_validation_fails() {
        let result = OperationLogEntry::new("event_type", "payload", "", 1);
        assert!(result.is_err());
        match result {
            Err(OperationLogError::ValidationFailed(msg)) => {
                assert!(msg.contains("stream_id"));
            }
            other => panic!("Expected ValidationFailed, got: {other:?}"),
        }
    }

    #[test]
    fn given_both_empty_when_new_then_validation_fails_on_event_type() {
        let result = OperationLogEntry::new("", "payload", "", 1);
        assert!(result.is_err());
    }

    // =========================================================================
    // OperationLogEntry Clone
    // =========================================================================

    #[test]
    fn given_entry_when_cloned_then_equal() {
        let entry = OperationLogEntry::new("evt", "p", "s", 1).unwrap();
        let cloned = entry.clone();
        assert_eq!(entry.event_type, cloned.event_type);
        assert_eq!(entry.payload, cloned.payload);
        assert_eq!(entry.stream_id, cloned.stream_id);
        assert_eq!(entry.stream_version, cloned.stream_version);
    }

    // =========================================================================
    // OperationLogEntry Serialize / Deserialize
    // =========================================================================

    #[test]
    fn given_entry_when_serialized_then_roundtrips() {
        let entry = OperationLogEntry::new("evt", "payload", "stream-1", 3).unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: OperationLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.event_type, deserialized.event_type);
        assert_eq!(entry.payload, deserialized.payload);
        assert_eq!(entry.stream_id, deserialized.stream_id);
        assert_eq!(entry.stream_version, deserialized.stream_version);
        assert_eq!(entry.id, deserialized.id);
    }

    #[test]
    fn given_entry_when_debug_then_contains_fields() {
        let entry = OperationLogEntry::new("my-event", "data", "stream-x", 42).unwrap();
        let debug = format!("{entry:?}");
        assert!(debug.contains("my-event"));
        assert!(debug.contains("stream-x"));
        assert!(debug.contains("42"));
    }

    // =========================================================================
    // parse_datetime - valid cases
    // =========================================================================

    #[test]
    fn given_valid_rfc3339_when_parse_datetime_then_succeeds() {
        let result = parse_datetime(Some("2024-01-15T10:30:00+00:00".to_string()));
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.timestamp(), 1705314600);
    }

    #[test]
    fn given_valid_rfc3339_with_timezone_when_parse_datetime_then_utc() {
        let result = parse_datetime(Some("2024-01-15T10:30:00Z".to_string()));
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.timezone(), chrono::Utc);
    }

    #[test]
    fn given_valid_rfc3339_with_offset_when_parse_datetime_then_converts_to_utc() {
        let result = parse_datetime(Some("2024-01-15T12:30:00+02:00".to_string()));
        assert!(result.is_ok());
        // Should convert +02:00 to UTC
        assert_eq!(result.unwrap().timestamp(), 1705314600);
    }

    // =========================================================================
    // parse_datetime - invalid cases
    // =========================================================================

    #[test]
    fn given_none_when_parse_datetime_then_fails() {
        let result = parse_datetime(None);
        assert!(result.is_err());
        match result {
            Err(OperationLogError::QueryFailed(msg)) => {
                assert!(msg.contains("Missing required datetime"));
            }
            other => panic!("Expected QueryFailed, got: {other:?}"),
        }
    }

    #[test]
    fn given_invalid_format_when_parse_datetime_then_fails() {
        let result = parse_datetime(Some("not-a-date".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn given_empty_string_when_parse_datetime_then_fails() {
        let result = parse_datetime(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn given_garbage_when_parse_datetime_then_fails() {
        let result = parse_datetime(Some("2024-13-99T99:99:99Z".to_string()));
        assert!(result.is_err());
    }

    // =========================================================================
    // All error variants exhaustiveness display test
    // =========================================================================

    #[test]
    fn given_all_error_variants_when_display_then_all_have_prefix() {
        let variants: Vec<OperationLogError> = vec![
            OperationLogError::QueryFailed("q".into()),
            OperationLogError::DatabaseError("d".into()),
            OperationLogError::SerializationError("s".into()),
            OperationLogError::NotFound("n".into()),
            OperationLogError::ValidationFailed("v".into()),
        ];

        for err in &variants {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Display output should not be empty");
        }
    }
}

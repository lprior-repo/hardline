//! Event Store Lock Types
//!
//! This module provides the core types for the event_store_locks table:
//! - Error types for event store lock operations
//! - EventStoreLock struct representing a distributed lock on a stream position
//! - Row parsing helpers for database operations
//!
//! # Design
//!
//! Event store locks enforce ordered event processing across multiple agents.
//! Each lock is identified by a composite key of (stream_id, stream_seq), ensuring
//! that only one agent processes a specific position in an event stream at a time.

use sqlx::Row;

/// Error types for event store lock operations
#[derive(Debug, Clone)]
pub enum EventStoreLockError {
    /// Database query failed
    QueryFailed(String),
    /// Database connection or operation failed
    DatabaseError(String),
    /// Lock not found
    NotFound(String),
    /// Input validation failed
    ValidationFailed(String),
    /// Lock already held by another agent
    LockConflict(String),
}

impl std::fmt::Display for EventStoreLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueryFailed(msg) => write!(f, "Event store lock query failed: {msg}"),
            Self::DatabaseError(msg) => write!(f, "Event store lock database error: {msg}"),
            Self::NotFound(id) => write!(f, "Event store lock not found: {id}"),
            Self::ValidationFailed(msg) => write!(f, "Event store lock validation failed: {msg}"),
            Self::LockConflict(msg) => write!(f, "Event store lock conflict: {msg}"),
        }
    }
}

impl std::error::Error for EventStoreLockError {}

/// A lock entry in the event_store_locks table.
///
/// Each lock represents exclusive access to process a specific position
/// (stream_seq) within a named event stream (stream_id). The lock is held
/// by an agent (holder_id) and expires after a configurable TTL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStoreLock {
    /// Stream identifier (e.g., "session-queue", "workspace-events")
    pub stream_id: String,
    /// Sequence number within the stream (enforces ordering)
    pub stream_seq: i64,
    /// Agent holding the lock
    pub holder_id: String,
    /// Lock acquisition timestamp (Unix epoch seconds)
    pub acquired_at: i64,
    /// Lock expiration timestamp (Unix epoch seconds, TTL-based auto-release)
    pub expires_at: i64,
}

impl EventStoreLock {
    /// Create a new event store lock.
    ///
    /// # Errors
    ///
    /// Returns `EventStoreLockError::ValidationFailed` if:
    /// - stream_id is empty
    /// - holder_id is empty
    /// - stream_seq is negative
    /// - expires_at <= acquired_at (lock would be immediately expired)
    pub fn new(
        stream_id: impl Into<String>,
        stream_seq: i64,
        holder_id: impl Into<String>,
        acquired_at: i64,
        expires_at: i64,
    ) -> Result<Self, EventStoreLockError> {
        let stream_id = stream_id.into();
        let holder_id = holder_id.into();

        if stream_id.is_empty() {
            return Err(EventStoreLockError::ValidationFailed(
                "stream_id cannot be empty".to_string(),
            ));
        }

        if holder_id.is_empty() {
            return Err(EventStoreLockError::ValidationFailed(
                "holder_id cannot be empty".to_string(),
            ));
        }

        if stream_seq < 0 {
            return Err(EventStoreLockError::ValidationFailed(
                "stream_seq cannot be negative".to_string(),
            ));
        }

        if expires_at <= acquired_at {
            return Err(EventStoreLockError::ValidationFailed(
                "expires_at must be greater than acquired_at".to_string(),
            ));
        }

        Ok(Self {
            stream_id,
            stream_seq,
            holder_id,
            acquired_at,
            expires_at,
        })
    }
}

/// Parse a row from the event_store_locks table into an `EventStoreLock`.
///
/// # Errors
///
/// Returns `EventStoreLockError::QueryFailed` if any required field is missing or malformed.
pub fn parse_event_store_lock_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<EventStoreLock, EventStoreLockError> {
    let stream_id: String = row
        .try_get("stream_id")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'stream_id' error: {e}")))?;

    let stream_seq: i64 = row
        .try_get("stream_seq")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'stream_seq' error: {e}")))?;

    let holder_id: String = row
        .try_get("holder_id")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'holder_id' error: {e}")))?;

    let acquired_at: i64 = row
        .try_get("acquired_at")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'acquired_at' error: {e}")))?;

    let expires_at: i64 = row
        .try_get("expires_at")
        .map_err(|e| EventStoreLockError::QueryFailed(format!("Field 'expires_at' error: {e}")))?;

    Ok(EventStoreLock {
        stream_id,
        stream_seq,
        holder_id,
        acquired_at,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_valid_fields_when_new_then_succeeds() {
        let lock = EventStoreLock::new("session-queue", 1, "agent-1", 1000, 1600);
        assert!(lock.is_ok());
        let lock = lock.unwrap();
        assert_eq!(lock.stream_id, "session-queue");
        assert_eq!(lock.stream_seq, 1);
        assert_eq!(lock.holder_id, "agent-1");
        assert_eq!(lock.acquired_at, 1000);
        assert_eq!(lock.expires_at, 1600);
    }

    #[test]
    fn given_empty_stream_id_when_new_then_fails() {
        let result = EventStoreLock::new("", 1, "agent-1", 1000, 1600);
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("stream_id"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn given_empty_holder_id_when_new_then_fails() {
        let result = EventStoreLock::new("stream-1", 1, "", 1000, 1600);
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("holder_id"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn given_negative_stream_seq_when_new_then_fails() {
        let result = EventStoreLock::new("stream-1", -1, "agent-1", 1000, 1600);
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("stream_seq"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn given_expires_at_not_greater_than_acquired_at_when_new_then_fails() {
        let result = EventStoreLock::new("stream-1", 1, "agent-1", 1600, 1000);
        assert!(result.is_err());
        match result {
            Err(EventStoreLockError::ValidationFailed(msg)) => {
                assert!(msg.contains("expires_at"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn given_equal_timestamps_when_new_then_fails() {
        let result = EventStoreLock::new("stream-1", 1, "agent-1", 1000, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn given_locks_when_compare_then_equal() {
        let a = EventStoreLock::new("s", 1, "h", 100, 200).unwrap();
        let b = EventStoreLock::new("s", 1, "h", 100, 200).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn given_locks_when_compare_then_not_equal() {
        let a = EventStoreLock::new("s", 1, "h", 100, 200).unwrap();
        let b = EventStoreLock::new("s", 2, "h", 100, 200).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn given_error_when_display_then_contains_context() {
        let err = EventStoreLockError::QueryFailed("some sql error".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("some sql error"));

        let err = EventStoreLockError::ValidationFailed("field empty".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("field empty"));

        let err = EventStoreLockError::LockConflict("already held".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("already held"));
    }

    // =========================================================================
    // Additional Display coverage for all error variants
    // =========================================================================

    #[test]
    fn given_query_failed_error_when_display_then_prefix_present() {
        let err = EventStoreLockError::QueryFailed("sql err".to_string());
        let msg = format!("{err}");
        assert!(msg.starts_with("Event store lock query failed:"));
        assert!(msg.contains("sql err"));
    }

    #[test]
    fn given_database_error_when_display_then_prefix_present() {
        let err = EventStoreLockError::DatabaseError("connection lost".to_string());
        let msg = format!("{err}");
        assert!(msg.starts_with("Event store lock database error:"));
        assert!(msg.contains("connection lost"));
    }

    #[test]
    fn given_not_found_error_when_display_then_prefix_present() {
        let err = EventStoreLockError::NotFound("lock-42".to_string());
        let msg = format!("{err}");
        assert!(msg.starts_with("Event store lock not found:"));
        assert!(msg.contains("lock-42"));
    }

    #[test]
    fn given_validation_failed_error_when_display_then_prefix_present() {
        let err = EventStoreLockError::ValidationFailed("bad input".to_string());
        let msg = format!("{err}");
        assert!(msg.starts_with("Event store lock validation failed:"));
        assert!(msg.contains("bad input"));
    }

    #[test]
    fn given_lock_conflict_error_when_display_then_prefix_present() {
        let err = EventStoreLockError::LockConflict("held by other".to_string());
        let msg = format!("{err}");
        assert!(msg.starts_with("Event store lock conflict:"));
        assert!(msg.contains("held by other"));
    }

    // =========================================================================
    // Error trait / std::error::Error impl
    // =========================================================================

    #[test]
    fn given_error_when_cast_to_std_error_then_succeeds() {
        let err: Box<dyn std::error::Error> =
            Box::new(EventStoreLockError::DatabaseError("db err".to_string()));
        let msg = format!("{err}");
        assert!(msg.contains("db err"));
    }

    #[test]
    fn given_error_variants_when_source_then_none() {
        // EventStoreLockError has no inner error source
        let err = EventStoreLockError::QueryFailed("test".to_string());
        assert!(std::error::Error::source(&err).is_none());

        let err = EventStoreLockError::DatabaseError("test".to_string());
        assert!(std::error::Error::source(&err).is_none());

        let err = EventStoreLockError::NotFound("test".to_string());
        assert!(std::error::Error::source(&err).is_none());

        let err = EventStoreLockError::ValidationFailed("test".to_string());
        assert!(std::error::Error::source(&err).is_none());

        let err = EventStoreLockError::LockConflict("test".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }

    // =========================================================================
    // Clone
    // =========================================================================

    #[test]
    fn given_lock_when_cloned_then_equal() {
        let lock = EventStoreLock::new("stream", 5, "holder", 100, 200).unwrap();
        let cloned = lock.clone();
        assert_eq!(lock, cloned);
    }

    #[test]
    fn given_error_when_cloned_then_independent() {
        let err = EventStoreLockError::QueryFailed("original".to_string());
        let cloned = err.clone();
        // Both produce the same display output
        assert_eq!(format!("{err}"), format!("{cloned}"));
    }

    // =========================================================================
    // Debug formatting
    // =========================================================================

    #[test]
    fn given_lock_when_debug_then_contains_fields() {
        let lock = EventStoreLock::new("my-stream", 42, "agent-x", 1000, 2000).unwrap();
        let debug = format!("{lock:?}");
        assert!(debug.contains("my-stream"));
        assert!(debug.contains("42"));
        assert!(debug.contains("agent-x"));
        assert!(debug.contains("1000"));
        assert!(debug.contains("2000"));
    }

    #[test]
    fn given_error_when_debug_then_contains_variant() {
        let err = EventStoreLockError::LockConflict("test".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("LockConflict"));
    }

    // =========================================================================
    // Boundary conditions
    // =========================================================================

    #[test]
    fn given_zero_stream_seq_when_new_then_succeeds() {
        let lock = EventStoreLock::new("stream", 0, "holder", 100, 200);
        assert!(lock.is_ok());
        assert_eq!(lock.unwrap().stream_seq, 0);
    }

    #[test]
    fn given_expires_at_one_greater_than_acquired_when_new_then_succeeds() {
        let lock = EventStoreLock::new("stream", 1, "holder", 100, 101);
        assert!(lock.is_ok());
    }

    #[test]
    fn given_negative_acquired_at_with_positive_expires_when_new_then_succeeds() {
        // Edge case: negative timestamps (should be allowed by validation)
        let lock = EventStoreLock::new("stream", 1, "holder", -100, 0);
        assert!(lock.is_ok());
    }

    #[test]
    fn given_whitespace_strings_when_new_then_succeeds() {
        // Whitespace is not empty, so it should pass validation
        let lock = EventStoreLock::new("  ", 1, "  ", 100, 200);
        assert!(lock.is_ok());
    }

    #[test]
    fn given_unicode_stream_id_when_new_then_succeeds() {
        let lock = EventStoreLock::new("stream-cafe\u{00e9}", 1, "holder", 100, 200);
        assert!(lock.is_ok());
        assert_eq!(lock.unwrap().stream_id, "stream-cafe\u{00e9}");
    }

    // =========================================================================
    // Equality / PartialOrd edge cases
    // =========================================================================

    #[test]
    fn given_locks_differing_by_stream_id_then_not_equal() {
        let a = EventStoreLock::new("a", 1, "h", 100, 200).unwrap();
        let b = EventStoreLock::new("b", 1, "h", 100, 200).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn given_locks_differing_by_holder_id_then_not_equal() {
        let a = EventStoreLock::new("s", 1, "h1", 100, 200).unwrap();
        let b = EventStoreLock::new("s", 1, "h2", 100, 200).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn given_locks_differing_by_acquired_at_then_not_equal() {
        let a = EventStoreLock::new("s", 1, "h", 100, 200).unwrap();
        let b = EventStoreLock::new("s", 1, "h", 101, 200).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn given_locks_differing_by_expires_at_then_not_equal() {
        let a = EventStoreLock::new("s", 1, "h", 100, 200).unwrap();
        let b = EventStoreLock::new("s", 1, "h", 100, 201).unwrap();
        assert_ne!(a, b);
    }

    // =========================================================================
    // Display for all error variant types via match exhaustiveness
    // =========================================================================

    #[test]
    fn given_all_error_variants_when_display_then_all_have_prefix() {
        let variants: Vec<EventStoreLockError> = vec![
            EventStoreLockError::QueryFailed("q".into()),
            EventStoreLockError::DatabaseError("d".into()),
            EventStoreLockError::NotFound("n".into()),
            EventStoreLockError::ValidationFailed("v".into()),
            EventStoreLockError::LockConflict("l".into()),
        ];

        for err in &variants {
            let msg = format!("{err}");
            assert!(
                msg.contains("Event store lock"),
                "Display output should contain 'Event store lock': {msg}"
            );
        }
    }
}

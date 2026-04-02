#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution error conversion.
//!
//! This module provides the `From<ConflictResolutionError>` implementation
//! for converting domain errors to crate-level errors.

pub use super::conflict_resolutions_entities::ConflictResolutionError;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ERROR CONVERSION
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl From<ConflictResolutionError> for crate::Error {
    fn from(err: ConflictResolutionError) -> Self {
        match err {
            ConflictResolutionError::SchemaInitializationError {
                operation, source, ..
            } => Self::database(format!(
                "Schema initialization failed for '{operation}': {source}"
            )),
            ConflictResolutionError::InsertError { file, source, .. } => Self::database(format!(
                "Failed to insert conflict resolution for '{file}': {source}"
            )),
            ConflictResolutionError::QueryError {
                operation, source, ..
            } => Self::database(format!("Failed to execute query '{operation}': {source}")),
            ConflictResolutionError::InvalidDeciderError { decider, .. } => {
                Self::validation_field_error(
                    "decider",
                    format!("invalid decider '{decider}': must be 'ai' or 'human'"),
                    Some(decider),
                )
            }
            ConflictResolutionError::InvalidTimestampError { timestamp, .. } => {
                Self::validation_field_error(
                    "timestamp",
                    format!("invalid timestamp '{timestamp}': must be ISO 8601 format"),
                    Some(timestamp),
                )
            }
            ConflictResolutionError::EmptyFieldError { field } => Self::validation_field_error(
                &field,
                format!("empty required field '{field}'"),
                Some(String::new()),
            ),
            ConflictResolutionError::InvalidTimeRangeError {
                start_time,
                end_time,
            } => Self::validation_field_error(
                "time_range",
                format!("invalid time range: start_time '{start_time}' >= end_time '{end_time}'"),
                Some(format!("{start_time}..{end_time}")),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::conflict_resolutions_entities::{
        validate_decider, validate_timestamp,
    };

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Validation function tests
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_validate_decider_ai() {
        assert_eq!(validate_decider("ai"), Ok(()));
    }

    #[test]
    fn test_validate_decider_human() {
        assert_eq!(validate_decider("human"), Ok(()));
    }

    #[test]
    fn test_validate_decider_invalid() {
        let result = validate_decider("robot");
        assert!(result.is_err());
        match result {
            Err(ConflictResolutionError::InvalidDeciderError { decider, .. }) => {
                assert_eq!(decider, "robot");
            }
            _ => panic!("Expected InvalidDeciderError"),
        }
    }

    #[test]
    fn test_validate_timestamp_valid() {
        assert_eq!(validate_timestamp("2025-02-18T12:34:56Z"), Ok(()));
    }

    #[test]
    fn test_validate_timestamp_empty() {
        let result = validate_timestamp("");
        assert!(result.is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // From<ConflictResolutionError> -> crate::Error conversion tests
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_from_schema_initialization_error() {
        let err = ConflictResolutionError::SchemaInitializationError {
            operation: "CREATE TABLE conflict_resolutions".to_string(),
            source: "disk I/O error".to_string(),
            recovery: "check permissions".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("Schema initialization failed"));
        assert!(msg.contains("CREATE TABLE conflict_resolutions"));
        assert!(msg.contains("disk I/O error"));
    }

    #[test]
    fn test_from_insert_error() {
        let err = ConflictResolutionError::InsertError {
            file: "src/main.rs".to_string(),
            source: "UNIQUE constraint failed".to_string(),
            constraint: Some("2067".to_string()),
            recovery: "check for duplicates".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("Failed to insert conflict resolution"));
        assert!(msg.contains("src/main.rs"));
        assert!(msg.contains("UNIQUE constraint failed"));
    }

    #[test]
    fn test_from_insert_error_no_constraint() {
        let err = ConflictResolutionError::InsertError {
            file: "src/lib.rs".to_string(),
            source: "general error".to_string(),
            constraint: None,
            recovery: "retry".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("Failed to insert conflict resolution"));
        assert!(msg.contains("general error"));
    }

    #[test]
    fn test_from_query_error() {
        let err = ConflictResolutionError::QueryError {
            operation: "get_conflict_resolutions".to_string(),
            source: "table not found".to_string(),
            recovery: "run schema init".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("Failed to execute query"));
        assert!(msg.contains("get_conflict_resolutions"));
        assert!(msg.contains("table not found"));
    }

    #[test]
    fn test_from_invalid_decider_error() {
        let err = ConflictResolutionError::InvalidDeciderError {
            decider: "robot".to_string(),
            expected: vec!["ai".to_string(), "human".to_string()],
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("invalid decider"));
        assert!(msg.contains("robot"));
        assert!(msg.contains("ai"));
        assert!(msg.contains("human"));
    }

    #[test]
    fn test_from_invalid_timestamp_error() {
        let err = ConflictResolutionError::InvalidTimestampError {
            timestamp: "not-a-timestamp".to_string(),
            expected_format: "ISO 8601".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("invalid timestamp"));
        assert!(msg.contains("not-a-timestamp"));
        assert!(msg.contains("ISO 8601"));
    }

    #[test]
    fn test_from_empty_field_error() {
        let err = ConflictResolutionError::EmptyFieldError {
            field: "strategy".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("empty required field"));
        assert!(msg.contains("strategy"));
    }

    #[test]
    fn test_from_invalid_time_range_error() {
        let err = ConflictResolutionError::InvalidTimeRangeError {
            start_time: "2025-12-31T23:59:59Z".to_string(),
            end_time: "2025-01-01T00:00:00Z".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("invalid time range"));
        assert!(msg.contains("2025-12-31T23:59:59Z"));
        assert!(msg.contains("2025-01-01T00:00:00Z"));
    }

    #[test]
    fn test_from_invalid_time_range_equal_times() {
        let err = ConflictResolutionError::InvalidTimeRangeError {
            start_time: "2025-01-01T00:00:00Z".to_string(),
            end_time: "2025-01-01T00:00:00Z".to_string(),
        };
        let crate_err: crate::Error = err.into();
        let msg = format!("{crate_err}");
        assert!(msg.contains("invalid time range"));
    }
}

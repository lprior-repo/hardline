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
            } => Self::Database(format!(
                "Schema initialization failed for '{operation}': {source}"
            )),
            ConflictResolutionError::InsertError { file, source, .. } => Self::Database(format!(
                "Failed to insert conflict resolution for '{file}': {source}"
            )),
            ConflictResolutionError::QueryError {
                operation, source, ..
            } => Self::Database(format!("Failed to execute query '{operation}': {source}")),
            ConflictResolutionError::InvalidDeciderError { decider, .. } => {
                Self::ValidationFieldError {
                    message: format!("invalid decider '{decider}': must be 'ai' or 'human'"),
                    field: "decider".to_string(),
                    value: Some(decider),
                }
            }
            ConflictResolutionError::InvalidTimestampError { timestamp, .. } => {
                Self::ValidationFieldError {
                    message: format!("invalid timestamp '{timestamp}': must be ISO 8601 format"),
                    field: "timestamp".to_string(),
                    value: Some(timestamp),
                }
            }
            ConflictResolutionError::EmptyFieldError { field } => Self::ValidationFieldError {
                message: format!("empty required field '{field}'"),
                field: field,
                value: Some(String::new()),
            },
            ConflictResolutionError::InvalidTimeRangeError {
                start_time,
                end_time,
            } => Self::ValidationFieldError {
                message: format!(
                    "invalid time range: start_time '{start_time}' >= end_time '{end_time}'"
                ),
                field: "time_range".to_string(),
                value: Some(format!("{start_time}..{end_time}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests are in conflict_resolutions_tests.rs
    // This module contains only unit tests for pure functions

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
}

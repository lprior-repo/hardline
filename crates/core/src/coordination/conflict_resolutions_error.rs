//! Error conversion for conflict resolution operations.
//!
//! Provides the `From` implementation for converting
//! `ConflictResolutionError` into the crate's `Error` type.

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
                field,
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

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Conflict resolution entities (infrastructure layer).
//!
//! This module contains `sqlx::FromRow` structs that directly map to
//! the database schema. These are infrastructure types separated from
//! domain logic.
//!
//! Domain logic and validation are in `conflict_resolutions.rs`.

use serde::{Deserialize, Serialize};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFLICT RESOLUTION (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A row in the `conflict_resolutions` table.
///
/// This is the infrastructure representation of a conflict resolution,
/// directly mapping to the database schema.
///
/// # Fields
///
/// * `id` - Primary key (auto-increment)
/// * `timestamp` - ISO 8601 timestamp of resolution
/// * `session` - Session name where conflict occurred
/// * `file` - File path with conflict
/// * `strategy` - Resolution strategy used
/// * `reason` - Human-readable reason for resolution (optional)
/// * `confidence` - Confidence score for AI decisions (optional)
/// * `decider` - Who made the decision ("ai" or "human")
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct ConflictResolution {
    /// Primary key (auto-increment)
    pub id: i64,

    /// ISO 8601 timestamp of resolution
    pub timestamp: String,

    /// Session name where conflict occurred
    pub session: String,

    /// File path with conflict
    pub file: String,

    /// Resolution strategy used
    /// Examples: "`accept_theirs`", "`accept_ours`", "`manual_merge`", "skip"
    pub strategy: String,

    /// Human-readable reason for resolution (optional)
    pub reason: Option<String>,

    /// Confidence score for AI decisions (optional)
    /// Examples: "high", "medium", "low", "0.95"
    pub confidence: Option<String>,

    /// Who made the decision
    /// Must be "ai" or "human"
    pub decider: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONFLICT RESOLUTION ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for conflict resolution operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolutionError {
    /// Schema initialization failed
    SchemaInitializationError {
        operation: String,
        source: String,
        recovery: String,
    },

    /// Insert operation failed
    InsertError {
        file: String,
        source: String,
        constraint: Option<String>,
        recovery: String,
    },

    /// Query operation failed
    QueryError {
        operation: String,
        source: String,
        recovery: String,
    },

    /// Invalid decider type
    InvalidDeciderError {
        decider: String,
        expected: Vec<String>,
    },

    /// Invalid timestamp format
    InvalidTimestampError {
        timestamp: String,
        expected_format: String,
    },

    /// Empty required field
    EmptyFieldError { field: String },

    /// Invalid time range
    InvalidTimeRangeError {
        start_time: String,
        end_time: String,
    },
}

impl std::fmt::Display for ConflictResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaInitializationError {
                operation, source, ..
            } => {
                write!(
                    f,
                    "schema initialization failed for operation '{operation}': {source}"
                )
            }
            Self::InsertError { file, source, .. } => {
                write!(f, "insert failed for file '{file}': {source}")
            }
            Self::QueryError {
                operation, source, ..
            } => {
                write!(f, "query failed for operation '{operation}': {source}")
            }
            Self::InvalidDeciderError { decider, expected } => {
                write!(
                    f,
                    "invalid decider '{decider}': expected one of {expected:?}"
                )
            }
            Self::InvalidTimestampError {
                timestamp,
                expected_format,
            } => {
                write!(
                    f,
                    "invalid timestamp '{timestamp}': expected {expected_format}"
                )
            }
            Self::EmptyFieldError { field } => {
                write!(f, "empty required field: {field}")
            }
            Self::InvalidTimeRangeError {
                start_time,
                end_time,
            } => {
                write!(
                    f,
                    "invalid time range: start_time '{start_time}' >= end_time '{end_time}'"
                )
            }
        }
    }
}

impl std::error::Error for ConflictResolutionError {}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// VALIDATION HELPERS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Validate that a decider is either "ai" or "human".
///
/// # Returns
///
/// * `Ok(())` if decider is valid
/// * `Err(ConflictResolutionError::InvalidDeciderError)` otherwise
///
/// # Errors
///
/// Returns `InvalidDeciderError` if decider is not "ai" or "human".
pub fn validate_decider(decider: &str) -> Result<(), ConflictResolutionError> {
    match decider {
        "ai" | "human" => Ok(()),
        _ => Err(ConflictResolutionError::InvalidDeciderError {
            decider: decider.to_string(),
            expected: vec!["ai".to_string(), "human".to_string()],
        }),
    }
}

/// Validate that a timestamp is valid ISO 8601 format (basic check).
///
/// # Returns
///
/// * `Ok(())` if timestamp is non-empty
/// * `Err(ConflictResolutionError::InvalidTimestampError)` otherwise
///
/// # Errors
///
/// Returns `InvalidTimestampError` if timestamp is empty.
pub fn validate_timestamp(timestamp: &str) -> Result<(), ConflictResolutionError> {
    if timestamp.is_empty() {
        return Err(ConflictResolutionError::InvalidTimestampError {
            timestamp: timestamp.to_string(),
            expected_format: "ISO 8601".to_string(),
        });
    }
    Ok(())
}

/// Validate that a required field is non-empty.
///
/// # Returns
///
/// * `Ok(())` if field is non-empty
/// * `Err(ConflictResolutionError::EmptyFieldError)` otherwise
///
/// # Errors
///
/// Returns `EmptyFieldError` if field is empty.
pub fn validate_non_empty(field: &str, field_name: &str) -> Result<(), ConflictResolutionError> {
    if field.is_empty() {
        return Err(ConflictResolutionError::EmptyFieldError {
            field: field_name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_decider_valid() {
        assert_eq!(validate_decider("ai"), Ok(()));
        assert_eq!(validate_decider("human"), Ok(()));
    }

    #[test]
    fn test_validate_decider_invalid() {
        let result = validate_decider("robot");
        assert_eq!(
            result,
            Err(ConflictResolutionError::InvalidDeciderError {
                decider: "robot".to_string(),
                expected: vec!["ai".to_string(), "human".to_string()],
            })
        );
    }

    #[test]
    fn test_validate_timestamp_valid() {
        assert_eq!(validate_timestamp("2025-02-18T12:34:56Z"), Ok(()));
        assert_eq!(validate_timestamp("2025-02-18T12:34:56.789Z"), Ok(()));
    }

    #[test]
    fn test_validate_timestamp_invalid() {
        let result = validate_timestamp("");
        assert_eq!(
            result,
            Err(ConflictResolutionError::InvalidTimestampError {
                timestamp: String::new(),
                expected_format: "ISO 8601".to_string(),
            })
        );
    }

    #[test]
    fn test_validate_non_empty_valid() {
        assert_eq!(validate_non_empty("test", "field_name"), Ok(()));
    }

    #[test]
    fn test_validate_non_empty_invalid() {
        let result = validate_non_empty("", "field_name");
        assert_eq!(
            result,
            Err(ConflictResolutionError::EmptyFieldError {
                field: "field_name".to_string(),
            })
        );
    }
}

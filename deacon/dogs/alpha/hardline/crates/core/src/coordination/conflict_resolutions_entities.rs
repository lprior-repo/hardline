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

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Type construction
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_conflict_resolution_construction_with_all_fields() {
        let resolution = ConflictResolution {
            id: 42,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "my-session".to_string(),
            file: "src/main.rs".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: Some("Automatic resolution".to_string()),
            confidence: Some("high".to_string()),
            decider: "ai".to_string(),
        };

        assert_eq!(resolution.id, 42);
        assert_eq!(resolution.timestamp, "2025-02-18T12:34:56Z");
        assert_eq!(resolution.session, "my-session");
        assert_eq!(resolution.file, "src/main.rs");
        assert_eq!(resolution.strategy, "accept_theirs");
        assert_eq!(resolution.reason.as_deref(), Some("Automatic resolution"));
        assert_eq!(resolution.confidence.as_deref(), Some("high"));
        assert_eq!(resolution.decider, "ai");
    }

    #[test]
    fn test_conflict_resolution_construction_minimal() {
        let resolution = ConflictResolution {
            id: 1,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session-1".to_string(),
            file: "src/lib.rs".to_string(),
            strategy: "skip".to_string(),
            reason: None,
            confidence: None,
            decider: "human".to_string(),
        };

        assert!(resolution.reason.is_none());
        assert!(resolution.confidence.is_none());
    }

    #[test]
    fn test_conflict_resolution_equality() {
        let a = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "s".to_string(),
            file: "f".to_string(),
            strategy: "accept_ours".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        let b = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "s".to_string(),
            file: "f".to_string(),
            strategy: "accept_ours".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_conflict_resolution_clone() {
        let original = ConflictResolution {
            id: 5,
            timestamp: "2025-03-01T00:00:00Z".to_string(),
            session: "clone-test".to_string(),
            file: "src/clone.rs".to_string(),
            strategy: "manual_merge".to_string(),
            reason: Some("reason".to_string()),
            confidence: Some("0.8".to_string()),
            decider: "human".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_conflict_resolution_debug() {
        let resolution = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "debug-test".to_string(),
            file: "src/debug.rs".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        let debug_str = format!("{resolution:?}");
        assert!(debug_str.contains("ConflictResolution"));
        assert!(debug_str.contains("debug-test"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Serialization / Deserialization
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_serialization_json_roundtrip_with_all_fields() {
        let resolution = ConflictResolution {
            id: 10,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "json-session".to_string(),
            file: "src/json.rs".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: Some("Test reason".to_string()),
            confidence: Some("0.95".to_string()),
            decider: "ai".to_string(),
        };

        let json = serde_json::to_string(&resolution).expect("serialization should succeed");
        let deserialized: ConflictResolution =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(resolution, deserialized);
    }

    #[test]
    fn test_serialization_json_roundtrip_minimal() {
        let resolution = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "minimal".to_string(),
            file: "f.rs".to_string(),
            strategy: "skip".to_string(),
            reason: None,
            confidence: None,
            decider: "human".to_string(),
        };

        let json = serde_json::to_string(&resolution).expect("serialization should succeed");
        let deserialized: ConflictResolution =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(resolution, deserialized);
    }

    #[test]
    fn test_serialization_json_none_fields_are_null() {
        let resolution = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "null-test".to_string(),
            file: "f.rs".to_string(),
            strategy: "skip".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };

        let json = serde_json::to_value(&resolution).expect("serialization should succeed");
        assert_eq!(json["reason"], serde_json::Value::Null);
        assert_eq!(json["confidence"], serde_json::Value::Null);
    }

    #[test]
    fn test_serialization_json_some_fields_are_strings() {
        let resolution = ConflictResolution {
            id: 1,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: "some-test".to_string(),
            file: "f.rs".to_string(),
            strategy: "skip".to_string(),
            reason: Some("my reason".to_string()),
            confidence: Some("0.99".to_string()),
            decider: "ai".to_string(),
        };

        let json = serde_json::to_value(&resolution).expect("serialization should succeed");
        assert_eq!(
            json["reason"],
            serde_json::Value::String("my reason".to_string())
        );
        assert_eq!(
            json["confidence"],
            serde_json::Value::String("0.99".to_string())
        );
    }

    #[test]
    fn test_deserialization_from_json_object() {
        let json_str = r#"{
            "id": 99,
            "timestamp": "2025-06-15T10:30:00Z",
            "session": "deser-session",
            "file": "src/deser.rs",
            "strategy": "manual_merge",
            "reason": "manual fix",
            "confidence": "medium",
            "decider": "human"
        }"#;

        let resolution: ConflictResolution =
            serde_json::from_str(json_str).expect("deserialization should succeed");
        assert_eq!(resolution.id, 99);
        assert_eq!(resolution.session, "deser-session");
        assert_eq!(resolution.decider, "human");
        assert_eq!(resolution.reason.as_deref(), Some("manual fix"));
    }

    #[test]
    fn test_serialization_yaml_roundtrip() {
        let resolution = ConflictResolution {
            id: 3,
            timestamp: "2025-03-01T00:00:00Z".to_string(),
            session: "yaml-test".to_string(),
            file: "src/yaml.rs".to_string(),
            strategy: "accept_ours".to_string(),
            reason: Some("yaml reason".to_string()),
            confidence: None,
            decider: "human".to_string(),
        };

        let yaml = serde_yaml::to_string(&resolution).expect("YAML serialization should succeed");
        let deserialized: ConflictResolution =
            serde_yaml::from_str(&yaml).expect("YAML deserialization should succeed");
        assert_eq!(resolution, deserialized);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Validation functions
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
    fn test_validate_decider_empty_string() {
        let result = validate_decider("");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(ConflictResolutionError::InvalidDeciderError { decider, .. } ) if decider.is_empty())
        );
    }

    #[test]
    fn test_validate_decider_case_sensitive() {
        assert!(validate_decider("AI").is_err());
        assert!(validate_decider("Human").is_err());
        assert!(validate_decider("AI ").is_err());
    }

    #[test]
    fn test_validate_timestamp_valid() {
        assert_eq!(validate_timestamp("2025-02-18T12:34:56Z"), Ok(()));
        assert_eq!(validate_timestamp("2025-02-18T12:34:56.789Z"), Ok(()));
    }

    #[test]
    fn test_validate_timestamp_invalid_empty() {
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
    fn test_validate_timestamp_non_empty_always_passes() {
        // The current implementation only checks for empty strings
        assert_eq!(validate_timestamp("not-a-real-timestamp"), Ok(()));
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

    #[test]
    fn test_validate_non_empty_whitespace_only_passes() {
        // Current implementation only checks is_empty(), not trim()
        assert_eq!(validate_non_empty("   ", "field"), Ok(()));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Error Display impl
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_display_schema_initialization_error() {
        let err = ConflictResolutionError::SchemaInitializationError {
            operation: "CREATE TABLE".to_string(),
            source: "disk full".to_string(),
            recovery: "free space".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("schema initialization failed"));
        assert!(display.contains("CREATE TABLE"));
        assert!(display.contains("disk full"));
    }

    #[test]
    fn test_display_insert_error() {
        let err = ConflictResolutionError::InsertError {
            file: "src/main.rs".to_string(),
            source: "UNIQUE constraint".to_string(),
            constraint: Some("2067".to_string()),
            recovery: "check duplicates".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("insert failed"));
        assert!(display.contains("src/main.rs"));
        assert!(display.contains("UNIQUE constraint"));
    }

    #[test]
    fn test_display_insert_error_no_constraint() {
        let err = ConflictResolutionError::InsertError {
            file: "src/lib.rs".to_string(),
            source: "I/O error".to_string(),
            constraint: None,
            recovery: "retry".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("insert failed"));
        assert!(display.contains("I/O error"));
    }

    #[test]
    fn test_display_query_error() {
        let err = ConflictResolutionError::QueryError {
            operation: "SELECT *".to_string(),
            source: "table not found".to_string(),
            recovery: "run migrations".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("query failed"));
        assert!(display.contains("SELECT *"));
        assert!(display.contains("table not found"));
    }

    #[test]
    fn test_display_invalid_decider_error() {
        let err = ConflictResolutionError::InvalidDeciderError {
            decider: "robot".to_string(),
            expected: vec!["ai".to_string(), "human".to_string()],
        };
        let display = format!("{err}");
        assert!(display.contains("invalid decider"));
        assert!(display.contains("robot"));
        assert!(display.contains(r#"["ai", "human"]"#));
    }

    #[test]
    fn test_display_invalid_timestamp_error() {
        let err = ConflictResolutionError::InvalidTimestampError {
            timestamp: "".to_string(),
            expected_format: "ISO 8601".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("invalid timestamp"));
        assert!(display.contains("ISO 8601"));
    }

    #[test]
    fn test_display_empty_field_error() {
        let err = ConflictResolutionError::EmptyFieldError {
            field: "strategy".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("empty required field"));
        assert!(display.contains("strategy"));
    }

    #[test]
    fn test_display_invalid_time_range_error() {
        let err = ConflictResolutionError::InvalidTimeRangeError {
            start_time: "2025-12-31T23:59:59Z".to_string(),
            end_time: "2025-01-01T00:00:00Z".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("invalid time range"));
        assert!(display.contains("2025-12-31T23:59:59Z"));
        assert!(display.contains("2025-01-01T00:00:00Z"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Error std::error::Error impl
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(ConflictResolutionError::EmptyFieldError {
            field: "test".to_string(),
        });
        let _display = format!("{err}");
    }

    #[test]
    fn test_error_debug_clone_and_eq() {
        let err1 = ConflictResolutionError::InvalidDeciderError {
            decider: "x".to_string(),
            expected: vec!["ai".to_string()],
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
        let debug_str = format!("{err1:?}");
        assert!(debug_str.contains("InvalidDeciderError"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Strategy values (known valid strategies)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_known_strategies_serialize_correctly() {
        let strategies = ["accept_theirs", "accept_ours", "manual_merge", "skip"];
        for strategy in strategies {
            let resolution = ConflictResolution {
                id: 1,
                timestamp: "2025-01-01T00:00:00Z".to_string(),
                session: "test".to_string(),
                file: "f.rs".to_string(),
                strategy: strategy.to_string(),
                reason: None,
                confidence: None,
                decider: "ai".to_string(),
            };
            let json = serde_json::to_value(&resolution).expect("serialize");
            assert_eq!(json["strategy"], strategy);
        }
    }
}

// //! Builder error types
//!
//! Errors that can occur during builder operations.

use thiserror::Error;

/// Errors that can occur during builder operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BuilderError {
    /// Required field not set
    #[error("missing required field: {field}")]
    MissingRequired { field: &'static str },

    /// Invalid value provided
    #[error("invalid value for field '{field}': {reason}")]
    InvalidValue { field: &'static str, reason: String },

    /// Collection overflow
    #[error("field '{field}' exceeds capacity of {capacity}")]
    Overflow {
        field: &'static str,
        capacity: usize,
    },

    /// Invalid state transition
    #[error("invalid transition from '{from}' to '{to}': {reason}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_required_display() {
        let err = BuilderError::MissingRequired { field: "name" };
        let msg = format!("{err}");
        assert!(msg.contains("name"));
        assert!(msg.contains("missing required field"));
    }

    #[test]
    fn invalid_value_display() {
        let err = BuilderError::InvalidValue { field: "age", reason: "negative".to_string() };
        let msg = format!("{err}");
        assert!(msg.contains("age"));
        assert!(msg.contains("negative"));
    }

    #[test]
    fn overflow_display() {
        let err = BuilderError::Overflow { field: "tags", capacity: 10 };
        let msg = format!("{err}");
        assert!(msg.contains("tags"));
        assert!(msg.contains("10"));
        assert!(msg.contains("exceeds capacity"));
    }

    #[test]
    fn invalid_transition_display() {
        let err = BuilderError::InvalidTransition {
            from: "draft",
            to: "published",
            reason: "must be reviewed first".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("draft"));
        assert!(msg.contains("published"));
        assert!(msg.contains("must be reviewed first"));
    }

    #[test]
    fn error_is_debug() {
        let err = BuilderError::MissingRequired { field: "x" };
        let debug = format!("{err:?}");
        assert!(debug.contains("MissingRequired"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = BuilderError::MissingRequired { field: "a" };
        let _ = BuilderError::InvalidValue { field: "b", reason: String::new() };
        let _ = BuilderError::Overflow { field: "c", capacity: 0 };
        let _ = BuilderError::InvalidTransition {
            from: "d",
            to: "e",
            reason: String::new(),
        };
    }
}

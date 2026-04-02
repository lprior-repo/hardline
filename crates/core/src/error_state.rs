//! State and validation errors.
//!
//! Error codes: 7xxx, 8xxx

use crate::error::Error;
use thiserror::Error;

/// State and validation errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct StateError {
    #[from]
    inner: StateErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum StateErrorKind {
    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Validation failed with field context
    #[error("Validation error on '{field}': {message}")]
    ValidationFieldError {
        /// Human-readable error message
        message: String,
        /// Field name that failed validation
        field: String,
        /// Invalid value provided
        value: Option<String>,
    },

    /// Invalid identifier format
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
}

impl From<StateErrorKind> for Error {
    fn from(e: StateErrorKind) -> Self {
        Error::State(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl StateError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &StateErrorKind {
        &self.inner
    }

    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match &self.inner {
            StateErrorKind::NotFound(_) => {
                Some("Use 'scp session list' to see available sessions".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            StateErrorKind::InvalidState(_) => 70,
            StateErrorKind::NotFound(_) => 71,
            StateErrorKind::ValidationError(_) => 80,
            StateErrorKind::ValidationFieldError { .. } => 81,
            StateErrorKind::InvalidIdentifier(_) => 82,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_error_kind_invalid_state_display() {
        let err = StateErrorKind::InvalidState("bad state".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad state"));
        assert!(msg.contains("Invalid state"));
    }

    #[test]
    fn state_error_kind_not_found_display() {
        let err = StateErrorKind::NotFound("resource-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("resource-42"));
        assert!(msg.contains("Not found"));
    }

    #[test]
    fn state_error_kind_validation_error_display() {
        let err = StateErrorKind::ValidationError("field X required".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("field X required"));
        assert!(msg.contains("Validation error"));
    }

    #[test]
    fn state_error_kind_validation_field_error_display() {
        let err = StateErrorKind::ValidationFieldError {
            field: "name".to_string(),
            message: "cannot be empty".to_string(),
            value: Some("".to_string()),
        };
        let msg = format!("{err}");
        assert!(msg.contains("name"));
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn state_error_kind_validation_field_error_no_value_display() {
        let err = StateErrorKind::ValidationFieldError {
            field: "age".to_string(),
            message: "must be positive".to_string(),
            value: None,
        };
        let msg = format!("{err}");
        assert!(msg.contains("age"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn state_error_kind_invalid_identifier_display() {
        let err = StateErrorKind::InvalidIdentifier("bad@id!".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad@id!"));
        assert!(msg.contains("Invalid identifier"));
    }

    #[test]
    fn state_error_exit_codes() {
        assert_eq!(
            StateError::from(StateErrorKind::InvalidState("x".into())).exit_code(),
            70
        );
        assert_eq!(
            StateError::from(StateErrorKind::NotFound("x".into())).exit_code(),
            71
        );
        assert_eq!(
            StateError::from(StateErrorKind::ValidationError("x".into())).exit_code(),
            80
        );
        assert_eq!(
            StateError::from(StateErrorKind::ValidationFieldError {
                field: "f".into(),
                message: "m".into(),
                value: None,
            })
            .exit_code(),
            81
        );
        assert_eq!(
            StateError::from(StateErrorKind::InvalidIdentifier("x".into())).exit_code(),
            82
        );
    }

    #[test]
    fn state_error_kind_accessor() {
        let err = StateError::from(StateErrorKind::InvalidState("bad".to_string()));
        assert!(matches!(err.kind(), StateErrorKind::InvalidState(_)));
    }

    #[test]
    fn state_error_suggestion_not_found() {
        let err = StateError::from(StateErrorKind::NotFound("x".to_string()));
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("session list"));
    }

    #[test]
    fn state_error_suggestion_none_for_invalid_state() {
        let err = StateError::from(StateErrorKind::InvalidState("x".to_string()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn from_state_error_kind_to_error() {
        let err: Error = StateErrorKind::InvalidIdentifier("x".to_string()).into();
        assert!(matches!(err, Error::State(_)));
    }
}

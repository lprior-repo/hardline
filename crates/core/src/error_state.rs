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

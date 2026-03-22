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

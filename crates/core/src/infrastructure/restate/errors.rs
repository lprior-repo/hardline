//! Error types for Restate SDK integration.
//!
//! ## Error Classification
//!
//! | Type | Behavior | Use Case |
//! |------|---------|----------|
//! | `TerminalError` | Non-retryable, stops retries | Business logic failures |
//! | `HandlerError` | Retryable with backoff | Transient failures |
//!
//! ## Usage
//!
//! ```rust
//! use scp_core::infrastructure::restate::errors::{HandlerError, TerminalError};
//!
//! // Return TerminalError to stop retries
//! let _: HandlerError = TerminalError::new("Business logic failure").into();
//!
//! // Return TerminalError with code
//! let _: HandlerError = TerminalError::new_with_code(404, "Not found").into();
//! ```

use std::error::Error as StdError;
use std::fmt;

use thiserror::Error;

/// Non-retryable error that stops automatic retry behavior.
///
/// Use `TerminalError` when:
/// - The failure is permanent (e.g., business logic validation)
/// - Retrying will never succeed
/// - You want to explicitly halt retry attempts
///
/// ## Example
///
/// ```rust
/// use scp_core::infrastructure::restate::errors::{HandlerError, TerminalError};
///
/// fn validate_input(input: &str) -> Result<(), HandlerError> {
///     if input.is_empty() {
///         return Err(TerminalError::new("Input cannot be empty").into());
///     }
///     Ok(())
/// }
/// ```
#[derive(Error, Debug)]
pub struct TerminalError {
    code: u16,
    message: String,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl TerminalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: 1,
            message: message.into(),
            source: None,
        }
    }

    pub fn new_with_code(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn from_error<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self {
            code: 1,
            message: e.to_string(),
            source: Some(Box::new(e)),
        }
    }

    pub fn code(&self) -> u16 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_error(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.source.as_ref().map(|e| e.as_ref())
    }
}

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TerminalError({}): {}", self.code, self.message)
    }
}

impl From<&str> for TerminalError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TerminalError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

pub type HandlerResult<T> = Result<T, HandlerError>;

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Terminal: {0}")]
    Terminal(TerminalError),

    #[error("Retryable: {0}")]
    Retryable(String),
}

impl HandlerError {
    pub fn retryable(message: impl Into<String>) -> Self {
        Self::Retryable(message.into())
    }

    pub fn from_std_error<E>(e: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Retryable(e.to_string())
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, HandlerError::Terminal(_))
    }

    pub fn is_retryable(&self) -> bool {
        !self.is_terminal()
    }

    pub fn terminal(e: TerminalError) -> Self {
        Self::Terminal(e)
    }
}

impl From<TerminalError> for HandlerError {
    fn from(e: TerminalError) -> Self {
        Self::Terminal(e)
    }
}

impl From<&str> for HandlerError {
    fn from(s: &str) -> Self {
        Self::retryable(s)
    }
}

impl From<String> for HandlerError {
    fn from(s: String) -> Self {
        Self::retryable(s)
    }
}

impl From<std::io::Error> for HandlerError {
    fn from(e: std::io::Error) -> Self {
        Self::from_std_error(e)
    }
}

impl From<serde_json::Error> for HandlerError {
    fn from(e: serde_json::Error) -> Self {
        Self::from_std_error(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_error_creation() {
        let e = TerminalError::new("test error");
        assert_eq!(e.code(), 1);
        assert_eq!(e.message(), "test error");
    }

    #[test]
    fn test_terminal_error_with_code() {
        let e = TerminalError::new_with_code(404, "not found");
        assert_eq!(e.code(), 404);
        assert_eq!(e.message(), "not found");
    }

    #[test]
    fn test_handler_error_terminal() {
        let e: HandlerError = TerminalError::new("terminal").into();
        assert!(e.is_terminal());
        assert!(!e.is_retryable());
    }

    #[test]
    fn test_handler_error_retryable() {
        let e = HandlerError::retryable("transient");
        assert!(!e.is_terminal());
        assert!(e.is_retryable());
    }
}

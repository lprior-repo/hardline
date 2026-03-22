//! Internal errors.
//!
//! Error codes: 9xxx

use thiserror::Error;
use crate::error::Error;

/// Internal errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct InternalError {
    #[from]
    inner: InternalErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum InternalErrorKind {
    /// Internal invariant violation
    #[error("Internal error: {0}")]
    Internal(String),

    /// Unimplemented feature
    #[error("Not implemented: {0}")]
    Unimplemented(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Clone operation failed
    #[error("Clone failed: {0}")]
    CloneFailed(String),

    /// Record operation failed
    #[error("Record failed: {0}")]
    RecordFailed(String),

    /// Invalid repository URL
    #[error("Invalid repository URL: {0}")]
    InvalidRepoUrl(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl From<InternalErrorKind> for Error {
    fn from(e: InternalErrorKind) -> Self {
        Error::Internal(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl InternalError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            InternalErrorKind::Internal(_) => 90,
            InternalErrorKind::Unimplemented(_) => 91,
            InternalErrorKind::InvalidConfig(_) => 92,
            InternalErrorKind::CloneFailed(_) => 93,
            InternalErrorKind::RecordFailed(_) => 94,
            InternalErrorKind::InvalidRepoUrl(_) => 95,
            InternalErrorKind::InvalidOperation(_) => 96,
        }
    }
}

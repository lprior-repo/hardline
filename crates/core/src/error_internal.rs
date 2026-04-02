//! Internal errors.
//!
//! Error codes: 9xxx

use crate::error::Error;
use thiserror::Error;

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
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &InternalErrorKind {
        &self.inner
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    // -- InternalErrorKind Display --

    #[test]
    fn internal_error_kind_display() {
        assert_eq!(
            format!("{}", InternalErrorKind::Internal("boom".to_string())),
            "Internal error: boom"
        );
        assert_eq!(
            format!(
                "{}",
                InternalErrorKind::Unimplemented("not yet".to_string())
            ),
            "Not implemented: not yet"
        );
        assert_eq!(
            format!(
                "{}",
                InternalErrorKind::InvalidConfig("bad key".to_string())
            ),
            "Invalid configuration: bad key"
        );
        assert_eq!(
            format!("{}", InternalErrorKind::CloneFailed("network".to_string())),
            "Clone failed: network"
        );
        assert_eq!(
            format!(
                "{}",
                InternalErrorKind::RecordFailed("db error".to_string())
            ),
            "Record failed: db error"
        );
        assert_eq!(
            format!(
                "{}",
                InternalErrorKind::InvalidRepoUrl("ftp://bad".to_string())
            ),
            "Invalid repository URL: ftp://bad"
        );
        assert_eq!(
            format!(
                "{}",
                InternalErrorKind::InvalidOperation("delete root".to_string())
            ),
            "Invalid operation: delete root"
        );
    }

    // -- InternalError exit codes --

    #[test]
    fn internal_error_exit_codes() {
        assert_eq!(
            InternalError::from(InternalErrorKind::Internal("x".into())).exit_code(),
            90
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::Unimplemented("x".into())).exit_code(),
            91
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::InvalidConfig("x".into())).exit_code(),
            92
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::CloneFailed("x".into())).exit_code(),
            93
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::RecordFailed("x".into())).exit_code(),
            94
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::InvalidRepoUrl("x".into())).exit_code(),
            95
        );
        assert_eq!(
            InternalError::from(InternalErrorKind::InvalidOperation("x".into())).exit_code(),
            96
        );
    }

    // -- InternalError kind() --

    #[test]
    fn internal_error_kind_accessor() {
        let err = InternalError::from(InternalErrorKind::Unimplemented("test".to_string()));
        assert!(matches!(err.kind(), InternalErrorKind::Unimplemented(_)));
    }

    // -- From<InternalErrorKind> for Error --

    #[test]
    fn from_internal_error_kind_to_error() {
        let err: Error = InternalErrorKind::Internal("test".to_string()).into();
        assert!(matches!(err, Error::Internal(_)));
    }
}

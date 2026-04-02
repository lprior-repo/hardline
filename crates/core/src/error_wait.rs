//! Wait and Batch command errors.
//!
//! Error codes: 5xxx, 8xxx

use crate::error::Error;
use thiserror::Error;

/// Wait and Batch command errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct WaitError {
    #[from]
    inner: WaitErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum WaitErrorKind {
    /// Wait operation timed out
    #[error("Wait timeout for session '{0}' waiting for {1}")]
    Timeout(String, String),

    /// Invalid wait mode specified
    #[error("Invalid wait mode: {0}")]
    InvalidWaitMode(String),

    /// Invalid session name
    #[error("Invalid session name: {0}")]
    InvalidSessionName(String),

    /// Batch command list is empty
    #[error("Batch command list is empty")]
    BatchEmpty,

    /// A command in the batch failed
    #[error("Batch command failed: {0}")]
    BatchCommandFailed(String),

    /// Rollback of batch failed
    #[error("Batch rollback failed: {0}")]
    BatchRollbackFailed(String),

    /// Checkpoint operation failed
    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    /// Batch size exceeds maximum allowed
    #[error("Batch size exceeds maximum of {0}")]
    BatchSizeExceeded(usize),
}

impl From<WaitErrorKind> for Error {
    fn from(e: WaitErrorKind) -> Self {
        Error::Wait(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl WaitError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &WaitErrorKind {
        &self.inner
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            WaitErrorKind::Timeout(_, _) => 55,
            WaitErrorKind::InvalidWaitMode(_) => 80,
            WaitErrorKind::InvalidSessionName(_) => 82,
            WaitErrorKind::BatchEmpty => 80,
            WaitErrorKind::BatchCommandFailed(_) => 56,
            WaitErrorKind::BatchRollbackFailed(_) => 57,
            WaitErrorKind::CheckpointError(_) => 58,
            WaitErrorKind::BatchSizeExceeded(_) => 80,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_error_kind_timeout_display() {
        let err = WaitErrorKind::Timeout("session-1".to_string(), "ready state".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("session-1"));
        assert!(msg.contains("ready state"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn wait_error_kind_invalid_wait_mode_display() {
        let err = WaitErrorKind::InvalidWaitMode("invalid-mode".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("invalid-mode"));
        assert!(msg.contains("Invalid wait mode"));
    }

    #[test]
    fn wait_error_kind_invalid_session_name_display() {
        let err = WaitErrorKind::InvalidSessionName("".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid session name"));
    }

    #[test]
    fn wait_error_kind_batch_empty_display() {
        let err = WaitErrorKind::BatchEmpty;
        let msg = format!("{err}");
        assert!(msg.contains("empty"));
    }

    #[test]
    fn wait_error_kind_batch_command_failed_display() {
        let err = WaitErrorKind::BatchCommandFailed("disk full".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("disk full"));
        assert!(msg.contains("Batch command failed"));
    }

    #[test]
    fn wait_error_kind_batch_rollback_failed_display() {
        let err = WaitErrorKind::BatchRollbackFailed("lock lost".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("lock lost"));
        assert!(msg.contains("rollback"));
    }

    #[test]
    fn wait_error_kind_checkpoint_error_display() {
        let err = WaitErrorKind::CheckpointError("fs error".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("fs error"));
        assert!(msg.contains("Checkpoint"));
    }

    #[test]
    fn wait_error_kind_batch_size_exceeded_display() {
        let err = WaitErrorKind::BatchSizeExceeded(500);
        let msg = format!("{err}");
        assert!(msg.contains("500"));
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn wait_error_exit_codes() {
        assert_eq!(WaitError::from(WaitErrorKind::Timeout("s".into(), "w".into())).exit_code(), 55);
        assert_eq!(WaitError::from(WaitErrorKind::InvalidWaitMode("x".into())).exit_code(), 80);
        assert_eq!(WaitError::from(WaitErrorKind::InvalidSessionName("x".into())).exit_code(), 82);
        assert_eq!(WaitError::from(WaitErrorKind::BatchEmpty).exit_code(), 80);
        assert_eq!(WaitError::from(WaitErrorKind::BatchCommandFailed("x".into())).exit_code(), 56);
        assert_eq!(WaitError::from(WaitErrorKind::BatchRollbackFailed("x".into())).exit_code(), 57);
        assert_eq!(WaitError::from(WaitErrorKind::CheckpointError("x".into())).exit_code(), 58);
        assert_eq!(WaitError::from(WaitErrorKind::BatchSizeExceeded(1)).exit_code(), 80);
    }

    #[test]
    fn wait_error_kind_accessor() {
        let err = WaitError::from(WaitErrorKind::BatchEmpty);
        assert!(matches!(err.kind(), WaitErrorKind::BatchEmpty));
    }

    #[test]
    fn from_wait_error_kind_to_error() {
        let err: Error = WaitErrorKind::Timeout("s".to_string(), "w".to_string()).into();
        assert!(matches!(err, Error::Wait(_)));
    }
}

//! Wait and Batch command errors.
//!
//! Error codes: 5xxx, 8xxx

use thiserror::Error;
use crate::error::Error;

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

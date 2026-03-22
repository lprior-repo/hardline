//! Queue-related errors.
//!
//! Error codes: 2xxx

use thiserror::Error;
use crate::error::Error;

/// Queue-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct QueueError {
    #[from]
    inner: QueueErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum QueueErrorKind {
    /// Queue is empty
    #[error("Queue is empty")]
    Empty,

    /// Queue item not found
    #[error("Queue item not found: {0}")]
    ItemNotFound(String),

    /// Queue is locked
    #[error("Queue is locked by '{0}'")]
    Locked(String),

    /// Queue operation already in progress
    #[error("Queue operation already in progress")]
    Processing,

    /// Invalid queue position
    #[error("Invalid queue position: {0}")]
    InvalidPosition(usize),

    /// Queue full (if there's a max size)
    #[error("Queue is full (max: {0})")]
    Full(usize),
}

impl From<QueueErrorKind> for Error {
    fn from(e: QueueErrorKind) -> Self {
        Error::Queue(e.into())
    }
}

// ========================================================================
// Suggestion & Exit Code
// ========================================================================

impl QueueError {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self.inner {
            QueueErrorKind::Empty => {
                Some("No items in queue. Use 'scp queue enqueue <branch>' to add one".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            QueueErrorKind::Empty => 20,
            QueueErrorKind::ItemNotFound(_) => 21,
            QueueErrorKind::Locked(_) => 22,
            QueueErrorKind::Processing => 23,
            QueueErrorKind::InvalidPosition(_) => 24,
            QueueErrorKind::Full(_) => 25,
        }
    }
}

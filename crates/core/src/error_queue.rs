//! Queue-related errors.
//!
//! Error codes: 2xxx

use crate::error::Error;
use thiserror::Error;

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
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &QueueErrorKind {
        &self.inner
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_error_kind_empty_display() {
        let err = QueueErrorKind::Empty;
        let msg = format!("{err}");
        assert!(msg.contains("empty"));
    }

    #[test]
    fn queue_error_kind_item_not_found_display() {
        let err = QueueErrorKind::ItemNotFound("entry-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("entry-42"));
    }

    #[test]
    fn queue_error_kind_locked_display() {
        let err = QueueErrorKind::Locked("agent-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent-1"));
        assert!(msg.contains("locked"));
    }

    #[test]
    fn queue_error_kind_processing_display() {
        let err = QueueErrorKind::Processing;
        let msg = format!("{err}");
        assert!(msg.contains("already in progress"));
    }

    #[test]
    fn queue_error_kind_invalid_position_display() {
        let err = QueueErrorKind::InvalidPosition(999);
        let msg = format!("{err}");
        assert!(msg.contains("999"));
    }

    #[test]
    fn queue_error_kind_full_display() {
        let err = QueueErrorKind::Full(100);
        let msg = format!("{err}");
        assert!(msg.contains("100"));
        assert!(msg.contains("full"));
    }

    #[test]
    fn queue_error_exit_codes() {
        assert_eq!(QueueError::from(QueueErrorKind::Empty).exit_code(), 20);
        assert_eq!(QueueError::from(QueueErrorKind::ItemNotFound("x".into())).exit_code(), 21);
        assert_eq!(QueueError::from(QueueErrorKind::Locked("x".into())).exit_code(), 22);
        assert_eq!(QueueError::from(QueueErrorKind::Processing).exit_code(), 23);
        assert_eq!(QueueError::from(QueueErrorKind::InvalidPosition(1)).exit_code(), 24);
        assert_eq!(QueueError::from(QueueErrorKind::Full(100)).exit_code(), 25);
    }

    #[test]
    fn queue_error_kind_accessor() {
        let err = QueueError::from(QueueErrorKind::Locked("agent-1".to_string()));
        assert!(matches!(err.kind(), QueueErrorKind::Locked(_)));
    }

    #[test]
    fn queue_error_suggestion_empty() {
        let err = QueueError::from(QueueErrorKind::Empty);
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("enqueue"));
    }

    #[test]
    fn queue_error_suggestion_none_for_locked() {
        let err = QueueError::from(QueueErrorKind::Locked("x".into()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn from_queue_error_kind_to_error() {
        let err: Error = QueueErrorKind::ItemNotFound("x".to_string()).into();
        assert!(matches!(err, Error::Queue(_)));
    }
}

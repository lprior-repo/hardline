//! Task-related errors.
//!
//! Error codes: 6xxx

use crate::error::Error;
use thiserror::Error;

/// Task-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct TaskError {
    #[from]
    pub inner: TaskErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum TaskErrorKind {
    /// Task not found
    #[error("Task not found: {0}")]
    NotFound(String),

    /// Task already claimed
    #[error("Task '{0}' is already claimed by '{1}'")]
    AlreadyClaimed(String, String),

    /// Task not claimed
    #[error("Task '{0}' is not claimed")]
    NotClaimed(String),

    /// Task locked
    #[error("Task '{0}' is locked")]
    Locked(String),

    /// Invalid task ID
    #[error("Invalid task ID: {0}")]
    InvalidId(String),

    /// Invalid task state transition
    #[error("Invalid state transition for task '{0}': {1}")]
    InvalidStateTransition(String, String),
}

impl From<TaskErrorKind> for Error {
    fn from(e: TaskErrorKind) -> Self {
        Error::Task(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl TaskError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            TaskErrorKind::NotFound(_) => 60,
            TaskErrorKind::AlreadyClaimed(_, _) => 61,
            TaskErrorKind::NotClaimed(_) => 62,
            TaskErrorKind::Locked(_) => 63,
            TaskErrorKind::InvalidId(_) => 64,
            TaskErrorKind::InvalidStateTransition(_, _) => 65,
        }
    }
}

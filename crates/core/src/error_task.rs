//! Task-related errors.
//!
//! Error codes: 3xxx (ADR-007, aligned with Bead category)

use thiserror::Error;

use crate::error::Error;

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
        Self::Task(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl TaskError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub const fn kind(&self) -> &TaskErrorKind {
        &self.inner
    }

    /// Returns exit code for CLI.
    /// Task errors use range 30-35 (ADR-007: 3xxx, aligned with Bead).
    pub const fn exit_code(&self) -> i32 {
        match self.inner {
            TaskErrorKind::NotFound(_) => 30,
            TaskErrorKind::AlreadyClaimed(_, _) => 31,
            TaskErrorKind::NotClaimed(_) => 32,
            TaskErrorKind::Locked(_) => 33,
            TaskErrorKind::InvalidId(_) => 34,
            TaskErrorKind::InvalidStateTransition(_, _) => 35,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_error_kind_not_found_display() {
        let err = TaskErrorKind::NotFound("task-42".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("task-42"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn task_error_kind_already_claimed_display() {
        let err = TaskErrorKind::AlreadyClaimed("task-1".to_string(), "agent-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("task-1"));
        assert!(msg.contains("agent-1"));
        assert!(msg.contains("already claimed"));
    }

    #[test]
    fn task_error_kind_not_claimed_display() {
        let err = TaskErrorKind::NotClaimed("task-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("task-1"));
        assert!(msg.contains("not claimed"));
    }

    #[test]
    fn task_error_kind_locked_display() {
        let err = TaskErrorKind::Locked("task-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("task-1"));
        assert!(msg.contains("locked"));
    }

    #[test]
    fn task_error_kind_invalid_id_display() {
        let err = TaskErrorKind::InvalidId("bad-id".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad-id"));
        assert!(msg.contains("Invalid task ID"));
    }

    #[test]
    fn task_error_kind_invalid_state_transition_display() {
        let err = TaskErrorKind::InvalidStateTransition(
            "task-1".to_string(),
            "Open -> Closed".to_string(),
        );
        let msg = format!("{err}");
        assert!(msg.contains("task-1"));
        assert!(msg.contains("Open -> Closed"));
    }

    #[test]
    fn task_error_exit_codes() {
        assert_eq!(
            TaskError::from(TaskErrorKind::NotFound("x".into())).exit_code(),
            30
        );
        assert_eq!(
            TaskError::from(TaskErrorKind::AlreadyClaimed("x".into(), "y".into())).exit_code(),
            31
        );
        assert_eq!(
            TaskError::from(TaskErrorKind::NotClaimed("x".into())).exit_code(),
            32
        );
        assert_eq!(
            TaskError::from(TaskErrorKind::Locked("x".into())).exit_code(),
            33
        );
        assert_eq!(
            TaskError::from(TaskErrorKind::InvalidId("x".into())).exit_code(),
            34
        );
        assert_eq!(
            TaskError::from(TaskErrorKind::InvalidStateTransition(
                "x".into(),
                "y".into()
            ))
            .exit_code(),
            35
        );
    }

    #[test]
    fn task_error_kind_accessor() {
        let err = TaskError::from(TaskErrorKind::NotFound("task-1".to_string()));
        assert!(matches!(err.kind(), TaskErrorKind::NotFound(_)));
    }

    #[test]
    fn from_task_error_kind_to_error() {
        let err: Error = TaskErrorKind::Locked("x".to_string()).into();
        assert!(matches!(err, Error::Task(_)));
    }
}

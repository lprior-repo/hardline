//! Workspace and Session errors.
//!
//! Error codes: 1xxx

use thiserror::Error;
use crate::error::Error;

/// Workspace-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct WorkspaceError {
    #[from]
    inner: WorkspaceErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum WorkspaceErrorKind {
    /// Workspace not found
    #[error("Workspace not found: {0}")]
    NotFound(String),

    /// Workspace already exists
    #[error("Workspace already exists: {0}")]
    Exists(String),

    /// Workspace is locked by another process
    #[error("Workspace '{0}' is locked by '{1}'")]
    Locked(String, String),

    /// Workspace conflict during operation
    #[error("Workspace conflict: {0}")]
    Conflict(String),
}

impl From<WorkspaceErrorKind> for Error {
    fn from(e: WorkspaceErrorKind) -> Self {
        Error::Workspace(e.into())
    }
}

/// Session-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct SessionError {
    #[from]
    inner: SessionErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum SessionErrorKind {
    /// Session not found
    #[error("Session not found: {0}")]
    NotFound(String),

    /// Session already exists
    #[error("Session already exists: {0}")]
    Exists(String),

    /// Session is locked
    #[error("Session '{0}' is locked by '{1}'")]
    Locked(String, String),

    /// Not the lock holder
    #[error("Agent '{1}' does not hold lock on session '{0}'")]
    NotLockHolder(String, String),

    /// Session in invalid state for operation
    #[error("Session '{0}' is {1}, expected {2}")]
    InvalidState(String, String, String),
}

impl From<SessionErrorKind> for Error {
    fn from(e: SessionErrorKind) -> Self {
        Error::Session(e.into())
    }
}

// ========================================================================
// Suggestion & Exit Code
// ========================================================================

impl WorkspaceError {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self.inner {
            WorkspaceErrorKind::NotFound(_) => {
                Some("Try 'scp workspace list' to see available workspaces".to_string())
            }
            WorkspaceErrorKind::Locked(_, holder) => {
                Some(format!("Use 'scp agent kill {}' to force release", holder))
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            WorkspaceErrorKind::NotFound(_) => 10,
            WorkspaceErrorKind::Exists(_) => 11,
            WorkspaceErrorKind::Locked(_, _) => 12,
            WorkspaceErrorKind::Conflict(_) => 13,
        }
    }
}

impl SessionError {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self.inner {
            SessionErrorKind::NotFound(_) => {
                Some("Try 'scp session list' to see available sessions".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            SessionErrorKind::NotFound(_) => 14,
            SessionErrorKind::Exists(_) => 15,
            SessionErrorKind::Locked(_, _) => 16,
            SessionErrorKind::NotLockHolder(_, _) => 17,
            SessionErrorKind::InvalidState(_, _, _) => 18,
        }
    }
}

//! Workspace and Session errors.
//!
//! Error codes: Workspace 1xxx, Session 2xxx (ADR-007)

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
        Self::Workspace(e.into())
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
        Self::Session(e.into())
    }
}

// ========================================================================
// Suggestion & Exit Code
// ========================================================================

impl WorkspaceError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub const fn kind(&self) -> &WorkspaceErrorKind {
        &self.inner
    }

    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match &self.inner {
            WorkspaceErrorKind::NotFound(_) => {
                Some("Try 'scp workspace list' to see available workspaces".to_string())
            }
            WorkspaceErrorKind::Locked(_, holder) => {
                Some(format!("Use 'scp agent kill {holder}' to force release"))
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    /// Workspace errors use range 10-13 (ADR-007: 1xxx).
    pub const fn exit_code(&self) -> i32 {
        match self.inner {
            WorkspaceErrorKind::NotFound(_) => 10,
            WorkspaceErrorKind::Exists(_) => 11,
            WorkspaceErrorKind::Locked(_, _) => 12,
            WorkspaceErrorKind::Conflict(_) => 13,
        }
    }
}

impl SessionError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub const fn kind(&self) -> &SessionErrorKind {
        &self.inner
    }

    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match &self.inner {
            SessionErrorKind::NotFound(_) => {
                Some("Try 'scp session list' to see available sessions".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    /// Session errors use range 20-24 (ADR-007: 2xxx).
    pub const fn exit_code(&self) -> i32 {
        match self.inner {
            SessionErrorKind::NotFound(_) => 20,
            SessionErrorKind::Exists(_) => 21,
            SessionErrorKind::Locked(_, _) => 22,
            SessionErrorKind::NotLockHolder(_, _) => 23,
            SessionErrorKind::InvalidState(_, _, _) => 24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- WorkspaceErrorKind Display --

    #[test]
    fn workspace_error_kind_not_found_display() {
        let err = WorkspaceErrorKind::NotFound("my-workspace".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("my-workspace"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn workspace_error_kind_exists_display() {
        let err = WorkspaceErrorKind::Exists("my-workspace".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("my-workspace"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn workspace_error_kind_locked_display() {
        let err = WorkspaceErrorKind::Locked("ws".to_string(), "agent-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("ws"));
        assert!(msg.contains("agent-1"));
        assert!(msg.contains("locked"));
    }

    #[test]
    fn workspace_error_kind_conflict_display() {
        let err = WorkspaceErrorKind::Conflict("concurrent modification".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("concurrent modification"));
    }

    // -- WorkspaceError suggestion --

    #[test]
    fn workspace_error_suggestion_not_found() {
        let err = WorkspaceError::from(WorkspaceErrorKind::NotFound("ws".to_string()));
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("workspace list"));
    }

    #[test]
    fn workspace_error_suggestion_locked() {
        let err = WorkspaceError::from(WorkspaceErrorKind::Locked(
            "ws".to_string(),
            "agent-1".to_string(),
        ));
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("agent kill agent-1"));
    }

    #[test]
    fn workspace_error_suggestion_none_for_exists() {
        let err = WorkspaceError::from(WorkspaceErrorKind::Exists("ws".to_string()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn workspace_error_suggestion_none_for_conflict() {
        let err = WorkspaceError::from(WorkspaceErrorKind::Conflict("x".to_string()));
        assert!(err.suggestion().is_none());
    }

    // -- WorkspaceError exit codes --

    #[test]
    fn workspace_error_exit_codes() {
        assert_eq!(
            WorkspaceError::from(WorkspaceErrorKind::NotFound("x".into())).exit_code(),
            10
        );
        assert_eq!(
            WorkspaceError::from(WorkspaceErrorKind::Exists("x".into())).exit_code(),
            11
        );
        assert_eq!(
            WorkspaceError::from(WorkspaceErrorKind::Locked("x".into(), "y".into())).exit_code(),
            12
        );
        assert_eq!(
            WorkspaceError::from(WorkspaceErrorKind::Conflict("x".into())).exit_code(),
            13
        );
    }

    // -- WorkspaceError kind() --

    #[test]
    fn workspace_error_kind_accessor() {
        let err = WorkspaceError::from(WorkspaceErrorKind::NotFound("ws".to_string()));
        assert!(matches!(err.kind(), WorkspaceErrorKind::NotFound(_)));
    }

    // -- From<WorkspaceErrorKind> for Error --

    #[test]
    fn from_workspace_error_kind_to_error() {
        let err: Error = WorkspaceErrorKind::NotFound("ws".to_string()).into();
        assert!(matches!(err, Error::Workspace(_)));
    }

    // -- SessionErrorKind Display --

    #[test]
    fn session_error_kind_not_found_display() {
        let err = SessionErrorKind::NotFound("my-session".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("my-session"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn session_error_kind_exists_display() {
        let err = SessionErrorKind::Exists("my-session".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("my-session"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn session_error_kind_locked_display() {
        let err = SessionErrorKind::Locked("session-1".to_string(), "agent-1".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("session-1"));
        assert!(msg.contains("agent-1"));
    }

    #[test]
    fn session_error_kind_not_lock_holder_display() {
        let err = SessionErrorKind::NotLockHolder("session-1".to_string(), "agent-2".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("agent-2"));
        assert!(msg.contains("does not hold lock"));
    }

    #[test]
    fn session_error_kind_invalid_state_display() {
        let err = SessionErrorKind::InvalidState(
            "session-1".to_string(),
            "active".to_string(),
            "paused".to_string(),
        );
        let msg = format!("{err}");
        assert!(msg.contains("session-1"));
        assert!(msg.contains("active"));
        assert!(msg.contains("paused"));
    }

    // -- SessionError suggestion --

    #[test]
    fn session_error_suggestion_not_found() {
        let err = SessionError::from(SessionErrorKind::NotFound("s".to_string()));
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("session list"));
    }

    #[test]
    fn session_error_suggestion_none_for_locked() {
        let err = SessionError::from(SessionErrorKind::Locked("s".into(), "a".into()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn session_error_suggestion_none_for_exists() {
        let err = SessionError::from(SessionErrorKind::Exists("s".into()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn session_error_suggestion_none_for_not_lock_holder() {
        let err = SessionError::from(SessionErrorKind::NotLockHolder("s".into(), "a".into()));
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn session_error_suggestion_none_for_invalid_state() {
        let err = SessionError::from(SessionErrorKind::InvalidState(
            "s".into(),
            "a".into(),
            "b".into(),
        ));
        assert!(err.suggestion().is_none());
    }

    // -- SessionError exit codes --

    #[test]
    fn session_error_exit_codes() {
        assert_eq!(
            SessionError::from(SessionErrorKind::NotFound("x".into())).exit_code(),
            20
        );
        assert_eq!(
            SessionError::from(SessionErrorKind::Exists("x".into())).exit_code(),
            21
        );
        assert_eq!(
            SessionError::from(SessionErrorKind::Locked("x".into(), "y".into())).exit_code(),
            22
        );
        assert_eq!(
            SessionError::from(SessionErrorKind::NotLockHolder("x".into(), "y".into())).exit_code(),
            23
        );
        assert_eq!(
            SessionError::from(SessionErrorKind::InvalidState(
                "x".into(),
                "y".into(),
                "z".into()
            ))
            .exit_code(),
            24
        );
    }

    // -- SessionError kind() --

    #[test]
    fn session_error_kind_accessor() {
        let err = SessionError::from(SessionErrorKind::Locked("s".into(), "a".into()));
        assert!(matches!(err.kind(), SessionErrorKind::Locked(_, _)));
    }

    // -- From<SessionErrorKind> for Error --

    #[test]
    fn from_session_error_kind_to_error() {
        let err: Error = SessionErrorKind::NotFound("s".to_string()).into();
        assert!(matches!(err, Error::Session(_)));
    }
}

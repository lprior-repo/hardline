//! JJ-specific errors.
//!
//! Error codes: 3xxx

use crate::error::Error;
use crate::error_types::JjConflictType;
use thiserror::Error;

/// JJ-specific errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct JjError {
    #[from]
    inner: JjErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum JjErrorKind {
    /// JJ command execution failed
    #[error("JJ command '{operation}' failed: {msg}")]
    CommandError {
        /// Operation that was attempted
        operation: String,
        /// Error message from JJ
        msg: String,
        /// Whether JJ binary was not found
        is_not_found: bool,
    },

    /// JJ workspace conflict detected
    #[error("JJ workspace conflict: {conflict_type:?} for '{workspace_name}': {msg}")]
    WorkspaceConflict {
        /// Type of conflict detected
        conflict_type: JjConflictType,
        /// Workspace name
        workspace_name: String,
        /// Raw error output
        msg: String,
        /// Recovery hint
        recovery_hint: String,
    },

    /// Lock acquisition timeout
    #[error("Lock acquisition timeout for '{operation}' after {timeout_ms}ms ({retries} retries)")]
    LockTimeout {
        /// Operation that was being locked
        operation: String,
        /// Timeout in milliseconds
        timeout_ms: u64,
        /// Number of retry attempts
        retries: usize,
    },
}

impl From<JjErrorKind> for Error {
    fn from(e: JjErrorKind) -> Self {
        Error::Jj(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl JjError {
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &JjErrorKind {
        &self.inner
    }

    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match &self.inner {
            JjErrorKind::CommandError {
                is_not_found: true,
                ..
            } => Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
            JjErrorKind::WorkspaceConflict {
                recovery_hint, ..
            } => Some(recovery_hint.clone()),
            JjErrorKind::LockTimeout { .. } => {
                Some("System is under heavy load. Wait a few moments and retry".to_string())
            }
            JjErrorKind::CommandError { .. } => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            JjErrorKind::CommandError { .. } => 39,
            JjErrorKind::WorkspaceConflict { .. } => 39,
            JjErrorKind::LockTimeout { .. } => 37,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jj_error_kind_command_error_display() {
        let err = JjErrorKind::CommandError {
            operation: "new".to_string(),
            msg: "binary not found".to_string(),
            is_not_found: true,
        };
        let msg = format!("{err}");
        assert!(msg.contains("new"));
        assert!(msg.contains("binary not found"));
    }

    #[test]
    fn jj_error_kind_command_error_not_found_flag() {
        let err = JjErrorKind::CommandError {
            operation: "status".to_string(),
            msg: "access denied".to_string(),
            is_not_found: false,
        };
        let msg = format!("{err}");
        assert!(msg.contains("status"));
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn jj_error_kind_workspace_conflict_display() {
        let err = JjErrorKind::WorkspaceConflict {
            conflict_type: JjConflictType::ConcurrentModification,
            workspace_name: "my-ws".to_string(),
            msg: "conflict detected".to_string(),
            recovery_hint: "resolve manually".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("my-ws"));
        assert!(msg.contains("conflict detected"));
    }

    #[test]
    fn jj_error_kind_lock_timeout_display() {
        let err = JjErrorKind::LockTimeout {
            operation: "commit".to_string(),
            timeout_ms: 5000,
            retries: 3,
        };
        let msg = format!("{err}");
        assert!(msg.contains("commit"));
        assert!(msg.contains("5000"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn jj_error_exit_codes() {
        assert_eq!(JjError::from(JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "y".into(),
            is_not_found: false,
        }).exit_code(), 39);
        assert_eq!(JjError::from(JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "y".into(),
            is_not_found: true,
        }).exit_code(), 39);
        assert_eq!(JjError::from(JjErrorKind::WorkspaceConflict {
            conflict_type: JjConflictType::Stale,
            workspace_name: "x".into(),
            msg: "y".into(),
            recovery_hint: "z".into(),
        }).exit_code(), 39);
        assert_eq!(JjError::from(JjErrorKind::LockTimeout {
            operation: "x".into(),
            timeout_ms: 1000,
            retries: 1,
        }).exit_code(), 37);
    }

    #[test]
    fn jj_error_kind_accessor() {
        let err = JjError::from(JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "y".into(),
            is_not_found: false,
        });
        assert!(matches!(err.kind(), JjErrorKind::CommandError { .. }));
    }

    #[test]
    fn jj_error_suggestion_command_not_found() {
        let err = JjError::from(JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "not found".into(),
            is_not_found: true,
        });
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("Install JJ"));
    }

    #[test]
    fn jj_error_suggestion_command_other() {
        let err = JjError::from(JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "failed".into(),
            is_not_found: false,
        });
        assert!(err.suggestion().is_none());
    }

    #[test]
    fn jj_error_suggestion_workspace_conflict() {
        let err = JjError::from(JjErrorKind::WorkspaceConflict {
            conflict_type: JjConflictType::Abandoned,
            workspace_name: "x".into(),
            msg: "y".into(),
            recovery_hint: "check lock".into(),
        });
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap(), "check lock");
    }

    #[test]
    fn jj_error_suggestion_lock_timeout() {
        let err = JjError::from(JjErrorKind::LockTimeout {
            operation: "x".into(),
            timeout_ms: 1000,
            retries: 1,
        });
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("heavy load"));
    }

    #[test]
    fn from_jj_error_kind_to_error() {
        let err: Error = JjErrorKind::CommandError {
            operation: "x".into(),
            msg: "y".into(),
            is_not_found: false,
        }.into();
        assert!(matches!(err, Error::Jj(_)));
    }
}

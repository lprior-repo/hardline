//! Session sync error types - domain errors using thiserror
//!
//! # Architecture
//!
//! - **Errors**: `SyncError` enum with domain-specific error variants

use thiserror::Error;

use crate::error::Error as CoreError;
use crate::error_io::IoErrorKind;
use crate::error_vcs::VcsErrorKind;
use crate::error_workspace::SessionErrorKind;

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR LAYER - Domain errors using thiserror
// ═══════════════════════════════════════════════════════════════════════════════

/// Domain errors for session sync operations
#[derive(Debug, Clone, Error)]
pub enum SyncError {
    /// Session does not exist in database
    #[error("Session '{0}' not found")]
    SessionNotFound(String),

    /// Session status does not allow sync
    #[error("Invalid session status '{actual}' for sync operation. Expected: Active or Failed")]
    InvalidSessionStatus {
        /// Actual status of the session
        actual: String,
        /// Allowed statuses
        allowed: Vec<String>,
    },

    /// Workspace has uncommitted changes
    #[error("Workspace at '{0}' has uncommitted changes. Use --allow-dirty to sync anyway")]
    DirtyWorkspace(String),

    /// Rebase resulted in conflicts
    #[error("Rebase conflicts in workspace '{workspace}'. Resolve with 'jj resolve' and retry")]
    Conflict {
        /// Workspace path
        workspace: String,
        /// Conflicted files
        conflicted_files: Vec<String>,
    },

    /// Rebase operation failed
    #[error("Rebase failed for workspace '{workspace}': {reason}")]
    RebaseFailure {
        /// Workspace path
        workspace: String,
        /// Underlying error
        reason: String,
    },

    /// JJ command execution failed
    #[error("JJ command failed: {0}")]
    JjCommandError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<SyncError> for CoreError {
    fn from(err: SyncError) -> Self {
        match err {
            SyncError::SessionNotFound(session) => SessionErrorKind::NotFound(session).into(),
            SyncError::InvalidSessionStatus { actual, .. } => {
                crate::error_state::StateErrorKind::ValidationFieldError {
                    field: "status".to_string(),
                    message: format!("Invalid session status: {actual}"),
                    value: Some(actual),
                }
                .into()
            }
            SyncError::DirtyWorkspace(path) => {
                crate::error_state::StateErrorKind::ValidationFieldError {
                    field: "workspace".to_string(),
                    message: format!("Workspace at '{path}' has uncommitted changes"),
                    value: Some(path),
                }
                .into()
            }
            SyncError::Conflict {
                workspace,
                conflicted_files,
            } => VcsErrorKind::Conflict(
                workspace,
                format!("Conflicted files: {}", conflicted_files.join(", ")),
            )
            .into(),
            SyncError::RebaseFailure {
                workspace: _,
                reason,
            } => VcsErrorKind::RebaseFailed(reason).into(),
            SyncError::JjCommandError(msg) => VcsErrorKind::Conflict("jj".to_string(), msg).into(),
            SyncError::IoError(msg) => IoErrorKind::IoError(msg).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Display tests — verify error message content for each variant
    // ========================================================================

    #[test]
    fn session_not_found_display_contains_name() {
        let err = SyncError::SessionNotFound("my-session".into());
        let msg = err.to_string();
        assert!(msg.contains("my-session"), "Display should contain session name");
        assert!(msg.contains("not found"), "Display should contain 'not found'");
    }

    #[test]
    fn invalid_session_status_display_contains_actual_and_allowed() {
        let err = SyncError::InvalidSessionStatus {
            actual: "Paused".into(),
            allowed: vec!["Active".into(), "Failed".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Paused"), "Display should contain actual status");
        assert!(msg.contains("Active"), "Display should mention Active");
        assert!(msg.contains("Failed"), "Display should mention Failed");
    }

    #[test]
    fn dirty_workspace_display_contains_path() {
        let err = SyncError::DirtyWorkspace("/tmp/my-workspace".into());
        let msg = err.to_string();
        assert!(msg.contains("/tmp/my-workspace"), "Display should contain workspace path");
        assert!(msg.contains("--allow-dirty"), "Display should mention --allow-dirty flag");
    }

    #[test]
    fn conflict_display_contains_workspace_and_resolve_hint() {
        let err = SyncError::Conflict {
            workspace: "proj-a".into(),
            conflicted_files: vec!["a.rs".into(), "b.rs".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("proj-a"), "Display should contain workspace name");
        assert!(msg.contains("jj resolve"), "Display should suggest jj resolve");
    }

    #[test]
    fn rebase_failure_display_contains_workspace_and_reason() {
        let err = SyncError::RebaseFailure {
            workspace: "proj-b".into(),
            reason: "divergent revisions".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("proj-b"), "Display should contain workspace name");
        assert!(msg.contains("divergent revisions"), "Display should contain reason");
    }

    #[test]
    fn jj_command_error_display_contains_message() {
        let err = SyncError::JjCommandError("command not found".into());
        let msg = err.to_string();
        assert!(msg.contains("command not found"), "Display should contain error message");
        assert!(msg.contains("JJ command failed"), "Display should mention JJ command");
    }

    #[test]
    fn io_error_display_contains_message() {
        let err = SyncError::IoError("permission denied".into());
        let msg = err.to_string();
        assert!(msg.contains("permission denied"), "Display should contain error message");
        assert!(msg.contains("IO error"), "Display should mention IO error");
    }

    #[test]
    fn sync_error_is_clone() {
        let err = SyncError::SessionNotFound("s1".into());
        let cloned = err.clone();
        assert!(matches!(cloned, SyncError::SessionNotFound(s) if s == "s1"));
    }

    #[test]
    fn sync_error_is_debug() {
        let err = SyncError::Conflict {
            workspace: "w".into(),
            conflicted_files: vec!["f.txt".into()],
        };
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Conflict"));
    }

    // ========================================================================
    // From<SyncError> for CoreError tests — verify conversion for each variant
    // ========================================================================

    #[test]
    fn session_not_found_converts_to_core_session_error() {
        let err = SyncError::SessionNotFound("test".into());
        let core = CoreError::from(err);
        assert!(matches!(core, CoreError::Session(_)));
        assert!(core.to_string().contains("test"));
    }

    #[test]
    fn invalid_status_converts_to_core_error() {
        let err = SyncError::InvalidSessionStatus {
            actual: "Paused".into(),
            allowed: vec![],
        };
        let core = CoreError::from(err);
        // The InvalidSessionStatus maps through StateErrorKind::ValidationFieldError
        let msg = core.to_string();
        assert!(msg.contains("Paused") || msg.contains("status"), "Conversion should preserve status info: {msg}");
    }

    #[test]
    fn dirty_workspace_converts_to_core_error() {
        let err = SyncError::DirtyWorkspace("/path/ws".into());
        let core = CoreError::from(err);
        let msg = core.to_string();
        assert!(msg.contains("/path/ws") || msg.contains("uncommitted"), "Conversion should preserve workspace info: {msg}");
    }

    #[test]
    fn conflict_converts_to_core_vcs_error() {
        let err = SyncError::Conflict {
            workspace: "ws".into(),
            conflicted_files: vec!["a.rs".into()],
        };
        let core = CoreError::from(err);
        assert!(matches!(core, CoreError::Vcs(_)));
    }

    #[test]
    fn rebase_failure_converts_to_core_vcs_error() {
        let err = SyncError::RebaseFailure {
            workspace: "ws".into(),
            reason: "err".into(),
        };
        let core = CoreError::from(err);
        assert!(matches!(core, CoreError::Vcs(_)));
    }

    #[test]
    fn jj_command_error_converts_to_core_vcs_error() {
        let err = SyncError::JjCommandError("bad".into());
        let core = CoreError::from(err);
        assert!(matches!(core, CoreError::Vcs(_)));
    }

    #[test]
    fn io_error_converts_to_core_io_error() {
        let err = SyncError::IoError("disk full".into());
        let core = CoreError::from(err);
        assert!(matches!(core, CoreError::Io(_)));
    }
}

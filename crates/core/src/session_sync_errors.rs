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

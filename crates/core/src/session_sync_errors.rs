//! Session sync error types - domain errors using thiserror
//!
//! # Architecture
//!
//! - **Errors**: `SyncError` enum with domain-specific error variants

use thiserror::Error;

use crate::error::Error as CoreError;

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
            SyncError::SessionNotFound(session) => CoreError::SessionNotFound(session),
            SyncError::InvalidSessionStatus { actual, .. } => CoreError::ValidationFieldError {
                message: format!("Invalid session status: {actual}"),
                field: "status".to_string(),
                value: Some(actual),
            },
            SyncError::DirtyWorkspace(path) => CoreError::ValidationFieldError {
                message: format!("Workspace at '{path}' has uncommitted changes"),
                field: "workspace".to_string(),
                value: Some(path),
            },
            SyncError::Conflict {
                workspace,
                conflicted_files,
            } => CoreError::VcsConflict(
                workspace,
                format!("Conflicted files: {}", conflicted_files.join(", ")),
            ),
            SyncError::RebaseFailure {
                workspace: _,
                reason,
            } => CoreError::VcsRebaseFailed(reason),
            SyncError::JjCommandError(msg) => CoreError::VcsConflict("jj".to_string(), msg),
            SyncError::IoError(msg) => CoreError::Io(std::io::Error::other(msg)),
        }
    }
}

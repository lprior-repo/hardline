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
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            JjErrorKind::CommandError { .. } => 39,
            JjErrorKind::WorkspaceConflict { .. } => 39,
            JjErrorKind::LockTimeout { .. } => 37,
        }
    }
}

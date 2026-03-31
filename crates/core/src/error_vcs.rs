//! VCS-related errors.
//!
//! Error codes: 3xxx

use crate::error::Error;
use thiserror::Error;

/// VCS-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct VcsError {
    #[from]
    inner: VcsErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum VcsErrorKind {
    /// VCS not initialized
    #[error("VCS not initialized in this directory")]
    NotInitialized,

    /// VCS conflict detected
    #[error("VCS conflict in {0}: {1}")]
    Conflict(String, String),

    /// Push failed
    #[error("Failed to push: {0}")]
    PushFailed(String),

    /// Pull failed
    #[error("Failed to pull: {0}")]
    PullFailed(String),

    /// Rebase failed
    #[error("Failed to rebase: {0}")]
    RebaseFailed(String),

    /// Branch not found
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Branch already exists
    #[error("Branch already exists: {0}")]
    BranchExists(String),

    /// Commit not found
    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    /// Working copy is dirty
    #[error("Working copy has uncommitted changes")]
    WorkingCopyDirty,
}

impl From<VcsErrorKind> for Error {
    fn from(e: VcsErrorKind) -> Self {
        Error::Vcs(e.into())
    }
}

// ========================================================================
// Suggestion & Exit Code
// ========================================================================

impl VcsError {
    /// Returns a human-readable suggestion for fixing the error.
    pub fn suggestion(&self) -> Option<String> {
        match self.inner {
            VcsErrorKind::NotInitialized => Some("Run 'scp init' to initialize VCS".to_string()),
            VcsErrorKind::WorkingCopyDirty => {
                Some("Commit or stash your changes before continuing".to_string())
            }
            _ => None,
        }
    }

    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            VcsErrorKind::NotInitialized => 30,
            VcsErrorKind::Conflict(_, _) => 31,
            VcsErrorKind::PushFailed(_) => 32,
            VcsErrorKind::PullFailed(_) => 33,
            VcsErrorKind::RebaseFailed(_) => 34,
            VcsErrorKind::BranchNotFound(_) => 35,
            VcsErrorKind::BranchExists(_) => 36,
            VcsErrorKind::CommitNotFound(_) => 37,
            VcsErrorKind::WorkingCopyDirty => 38,
        }
    }
}

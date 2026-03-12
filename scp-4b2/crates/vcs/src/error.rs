//! VCS Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VcsError {
    #[error("VCS not initialized in this directory")]
    NotInitialized,

    #[error("VCS conflict in {0}: {1}")]
    Conflict(String, String),

    #[error("Failed to push: {0}")]
    PushFailed(String),

    #[error("Failed to pull: {0}")]
    PullFailed(String),

    #[error("Failed to rebase: {0}")]
    RebaseFailed(String),

    #[error("Branch already exists: {0}")]
    BranchExists(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Feature not implemented: {0}")]
    Unimplemented(String),
}

pub type Result<T> = std::result::Result<T, VcsError>;

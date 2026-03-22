//! VCS Error Types

use std::path::PathBuf;

use thiserror::Error;

/// Railway-oriented Git errors for gitoxide operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Repository not found at {0}")]
    NotFound(PathBuf),

    #[error("Invalid reference '{name}': {reason}")]
    InvalidRef { name: String, reason: String },

    #[error("Merge conflict: {message}\nConflicted files: {conflicted_files:?}")]
    Conflict {
        message: String,
        conflicted_files: Vec<PathBuf>,
    },

    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Gitoxide error: {0}")]
    Gix(#[from] gix::Error),

    #[error("Gitoxide discover error: {0}")]
    GixDiscover(#[from] gix::discover::Error),

    #[error("Gitoxide init error: {0}")]
    GixInit(#[from] gix::init::Error),

    #[error("Gitoxide status error: {0}")]
    GixStatus(#[from] gix::status::Error),
    #[error("Gitoxide status iter error: {0}")]
    GixStatusIter(#[from] gix::status::into_iter::Error),
}

/// Result type for GitError operations
pub type GitResult<T> = std::result::Result<T, GitError>;

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

    #[error("Git CLI not found in PATH")]
    GitNotInstalled,

    #[error("Failed to parse git output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Feature not implemented: {0}")]
    Unimplemented(String),
}

/// Result type for VcsError operations (backward compatible)
pub type Result<T> = std::result::Result<T, VcsError>;

impl From<GitError> for VcsError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::NotFound(_path) => VcsError::NotInitialized,
            GitError::InvalidRef { name, reason: _ } => VcsError::BranchNotFound(name),
            GitError::Conflict { message, .. } => VcsError::Conflict(message, String::new()),
            GitError::Unauthorized(msg) => VcsError::PushFailed(msg),
            GitError::Network(msg) => VcsError::PullFailed(msg),
            GitError::Io(io_err) => VcsError::Io(io_err),
            GitError::Gix(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixDiscover(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixInit(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            GitError::GixStatus(err) => VcsError::Unimplemented(err.to_string()),
            GitError::GixStatusIter(err) => VcsError::Unimplemented(err.to_string()),
        }
    }
}

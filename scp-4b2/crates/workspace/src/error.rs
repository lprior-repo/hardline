use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace is locked by: {0}")]
    WorkspaceLocked(String, String),

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Invalid workspace id: {0}")]
    InvalidWorkspaceId(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    #[error("Workspace operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

pub type Result<T> = std::result::Result<T, WorkspaceError>;

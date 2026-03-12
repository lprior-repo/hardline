use thiserror::Error;

use crate::domain::entities::session::SessionState;
use crate::domain::workspace_state::WorkspaceState;

#[derive(Error, Debug)]
pub enum SessionError {
    // Session errors
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session already active: {0}")]
    AlreadyActive(String),

    #[error("Session expired: {0}")]
    Expired(String),

    // State transition errors
    #[error("Invalid workspace state transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: WorkspaceState,
        to: WorkspaceState,
    },

    #[error("Invalid session state transition: {from:?} -> {to:?}")]
    InvalidSessionTransition {
        from: SessionState,
        to: SessionState,
    },

    #[error("Invalid branch transition: {from:?} -> {to:?}")]
    InvalidBranchTransition { from: String, to: String },

    // Workspace errors (P1-P6 preconditions)
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Workspace already exists: {0}")]
    WorkspaceExists(String),

    #[error("Workspace is locked by: {0}")]
    WorkspaceLocked(String),

    #[error("Invalid workspace ID: {0}")]
    InvalidWorkspaceId(String),

    #[error("Invalid workspace name: {0}")]
    InvalidWorkspaceName(String),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    // Bead errors (P7-P10 preconditions)
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    #[error("Bead already exists: {0}")]
    BeadAlreadyExists(String),

    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),

    #[error("Invalid bead ID: {0}")]
    InvalidBeadId(String),

    #[error("Invalid bead title: {0}")]
    InvalidBeadTitle(String),

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Bead is blocked by: {0}")]
    BlockedBy(String),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    // General identifier/path errors
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid priority: {0}")]
    InvalidPriority(String),

    #[error("Invalid issue type: {0}")]
    InvalidIssueType(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;

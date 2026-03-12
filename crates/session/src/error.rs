use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session already active: {0}")]
    AlreadyActive(String),

    #[error("Session expired: {0}")]
    Expired(String),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Invalid branch transition: {from:?} -> {to:?}")]
    InvalidBranchTransition { from: String, to: String },

    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),

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

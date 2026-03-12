use thiserror::Error;

#[derive(Error, Debug)]
pub enum BeadError {
    #[error("Bead not found: {0}")]
    NotFound(String),

    #[error("Bead already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid bead ID: {0}")]
    InvalidId(String),

    #[error("Invalid title: {0}")]
    InvalidTitle(String),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Bead is blocked by: {0:?}")]
    BlockedBy(Vec<String>),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, BeadError>;

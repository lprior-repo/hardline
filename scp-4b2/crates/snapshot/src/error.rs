use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Snapshot corrupt: {0}")]
    Corrupt(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Git error: {0}")]
    GitError(String),

    #[error("Invalid snapshot: {0}")]
    InvalidSnapshot(String),
}

pub type Result<T> = std::result::Result<T, SnapshotError>;

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

    #[error("Snapshot creation failed: {0}")]
    CreationFailed(String),

    #[error("Snapshot not ready: {0}")]
    SnapshotNotReady(String),

    #[error("Snapshot restore failed: {0}")]
    RestoreFailed(String),
}

#[derive(Error, Debug)]
pub enum SnapshotRepoError {
    #[error("Failed to save snapshot: {0}")]
    SaveFailed(String),

    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Failed to delete snapshot: {0}")]
    DeleteFailed(String),
}

pub type Result<T> = std::result::Result<T, SnapshotError>;
pub type RepoResult<T> = std::result::Result<T, SnapshotRepoError>;
pub type StorageResult<T> = std::result::Result<T, SnapshotError>;

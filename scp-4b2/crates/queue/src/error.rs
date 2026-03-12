use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Queue entry not found: {0}")]
    QueueEntryNotFound(String),

    #[error("Queue is empty")]
    QueueEmpty,

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Invalid queue entry id: {0}")]
    InvalidQueueEntryId(String),

    #[error("Invalid priority value: {0}")]
    InvalidPriority(String),

    #[error("Invalid queue position: {0}")]
    InvalidQueuePosition(String),

    #[error("Queue operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

pub type Result<T> = std::result::Result<T, QueueError>;

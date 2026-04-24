use chrono::{DateTime, Utc};

use super::entity::{QueueEntry, FailedRetryable, FailedTerminal, Testing};
use crate::domain::queue::status::QueueStatus;
use crate::error::QueueError;

impl QueueEntry<FailedRetryable> {
    #[must_use]
    pub fn retry_metadata(&self) -> RetryMetadata {
        RetryMetadata {
            retry_count: self.retry_count,
            can_retry: self.can_retry(),
            error_message: self.error_message.clone(),
        }
    }
}

impl QueueEntry<FailedTerminal> {
    #[must_use]
    pub fn terminal_metadata(&self) -> TerminalMetadata {
        TerminalMetadata {
            error_message: self.error_message.clone(),
        }
    }
}

impl QueueEntry<Testing> {
    pub fn test_metadata(&self) -> TestMetadata {
        TestMetadata {
            retry_count: self.retry_count,
            error_message: self.error_message.clone(),
            status: self.status,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryMetadata {
    pub retry_count: u32,
    pub can_retry: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TerminalMetadata {
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestMetadata {
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub status: QueueStatus,
}

impl<S> QueueEntry<S> {
    pub fn metadata(&self) -> EntryMetadata {
        EntryMetadata {
            id: self.id().as_str().to_string(),
            session_id: self.session_id().to_string(),
            bead_id: self.bead_id().map(String::from),
            priority_value: self.priority().value(),
            position_value: self.position().value(),
            status: self.status(),
            enqueued_at: self.enqueued_at(),
            updated_at: self.updated_at(),
            retry_count: self.retry_count(),
            error_message: self.error_message().map(String::from),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntryMetadata {
    pub id: String,
    pub session_id: String,
    pub bead_id: Option<String>,
    pub priority_value: u32,
    pub position_value: u32,
    pub status: QueueStatus,
    pub enqueued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retry_count: u32,
    pub error_message: Option<String>,
}

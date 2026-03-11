use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueEvent {
    EntryEnqueued {
        entry_id: String,
        session_id: String,
        priority: u8,
        timestamp: DateTime<Utc>,
    },
    EntryClaimed {
        entry_id: String,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    RebaseStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    RebaseCompleted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    TestingStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    TestingCompleted {
        entry_id: String,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    MergeReady {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    MergeStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    MergeCompleted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    EntryRetried {
        entry_id: String,
        retry_count: u32,
        timestamp: DateTime<Utc>,
    },
    EntryCancelled {
        entry_id: String,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    EntryFailed {
        entry_id: String,
        error: String,
        retryable: bool,
        timestamp: DateTime<Utc>,
    },
}

impl QueueEvent {
    pub fn entry_enqueued(entry_id: String, session_id: String, priority: u8) -> Self {
        Self::EntryEnqueued {
            entry_id,
            session_id,
            priority,
            timestamp: Utc::now(),
        }
    }

    pub fn entry_claimed(entry_id: String, agent_id: String) -> Self {
        Self::EntryClaimed {
            entry_id,
            agent_id,
            timestamp: Utc::now(),
        }
    }

    pub fn rebase_started(entry_id: String) -> Self {
        Self::RebaseStarted {
            entry_id,
            timestamp: Utc::now(),
        }
    }

    pub fn testing_completed(entry_id: String, success: bool) -> Self {
        Self::TestingCompleted {
            entry_id,
            success,
            timestamp: Utc::now(),
        }
    }

    pub fn entry_failed(entry_id: String, error: String, retryable: bool) -> Self {
        Self::EntryFailed {
            entry_id,
            error,
            retryable,
            timestamp: Utc::now(),
        }
    }
}

use std::marker::PhantomData;

use crate::domain::queue::status::QueueStatus;
use crate::error::QueueError;
use chrono::Utc;

use super::entity::{QueueEntry, Claimed, Rebasing, Testing, ReadyToMerge, Merging, Merged, FailedRetryable, FailedTerminal, Cancelled};

impl QueueEntry<Claimed> {
    pub fn start_rebase(self) -> Result<QueueEntry<Rebasing>, QueueError> {
        self.transition_impl(QueueStatus::Rebasing)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<Rebasing> {
    pub fn start_testing(self) -> Result<QueueEntry<Testing>, QueueError> {
        self.transition_impl(QueueStatus::Testing)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<Testing> {
    pub fn mark_ready_to_merge(self) -> Result<QueueEntry<ReadyToMerge>, QueueError> {
        self.transition_impl(QueueStatus::ReadyToMerge)
    }

    pub fn mark_failed_retryable(
        self,
        error: String,
    ) -> Result<QueueEntry<FailedRetryable>, QueueError> {
        let entry: QueueEntry<FailedRetryable> =
            self.transition_impl(QueueStatus::FailedRetryable)?;
        Ok(QueueEntry {
            id: entry.id,
            session_id: entry.session_id,
            bead_id: entry.bead_id,
            priority: entry.priority,
            position: entry.position,
            status: entry.status,
            enqueued_at: entry.enqueued_at,
            updated_at: entry.updated_at,
            retry_count: entry.retry_count + 1,
            error_message: Some(error),
            _state: PhantomData,
        })
    }

    pub fn mark_failed_terminal(
        self,
        error: String,
    ) -> Result<QueueEntry<FailedTerminal>, QueueError> {
        let entry: QueueEntry<FailedTerminal> =
            self.transition_impl(QueueStatus::FailedTerminal)?;
        Ok(QueueEntry {
            id: entry.id,
            session_id: entry.session_id,
            bead_id: entry.bead_id,
            priority: entry.priority,
            position: entry.position,
            status: entry.status,
            enqueued_at: entry.enqueued_at,
            updated_at: entry.updated_at,
            retry_count: entry.retry_count,
            error_message: Some(error),
            _state: PhantomData,
        })
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<ReadyToMerge> {
    pub fn start_merging(self) -> Result<QueueEntry<Merging>, QueueError> {
        self.transition_impl(QueueStatus::Merging)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<Merging> {
    pub fn mark_merged(self) -> Result<QueueEntry<Merged>, QueueError> {
        self.transition_impl(QueueStatus::Merged)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<Merged> {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl QueueEntry<FailedRetryable> {
    #[must_use]
    pub fn can_retry(&self) -> bool {
        self.retry_count < 3
    }

    pub fn claim(self) -> Result<QueueEntry<Claimed>, QueueError> {
        self.transition_impl(QueueStatus::Claimed)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl QueueEntry<FailedTerminal> {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl QueueEntry<Cancelled> {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        true
    }
}

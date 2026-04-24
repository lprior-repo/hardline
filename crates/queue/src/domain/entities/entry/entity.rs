#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::identifiers::QueueEntryId;
use crate::domain::value_objects::{Priority, QueuePosition};
use crate::error::QueueError;

pub use crate::domain::queue::status::QueueStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pending;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Claimed;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rebasing;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Testing;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadyToMerge;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Merging;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Merged;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FailedRetryable;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FailedTerminal;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cancelled;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry<S = Pending> {
    pub(crate) id: QueueEntryId,
    pub(crate) session_id: String,
    pub(crate) bead_id: Option<String>,
    pub(crate) priority: Priority,
    pub(crate) position: QueuePosition,
    pub(crate) status: QueueStatus,
    pub(crate) enqueued_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) retry_count: u32,
    pub(crate) error_message: Option<String>,
    pub(crate) _state: PhantomData<S>,
}

impl QueueEntry<Pending> {
    pub fn enqueue(
        session_id: String,
        bead_id: Option<String>,
        priority: Priority,
    ) -> Result<Self, QueueError> {
        let trimmed = session_id.trim().to_string();
        if trimmed.is_empty() {
            return Err(QueueError::InvalidQueueEntryId("empty id".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: QueueEntryId::generate(),
            session_id: trimmed,
            bead_id,
            priority,
            position: QueuePosition::default(),
            status: QueueStatus::Pending,
            enqueued_at: now,
            updated_at: now,
            retry_count: 0,
            error_message: None,
            _state: PhantomData,
        })
    }

    pub fn claim(self) -> Result<QueueEntry<Claimed>, QueueError> {
        self.transition_impl(QueueStatus::Claimed)
    }

    pub fn cancel(self) -> Result<QueueEntry<Cancelled>, QueueError> {
        self.transition_impl(QueueStatus::Cancelled)
    }
}

impl<S> QueueEntry<S> {
    #[must_use]
    pub fn into_erased(self) -> QueueEntry {
        QueueEntry {
            id: self.id,
            session_id: self.session_id,
            bead_id: self.bead_id,
            priority: self.priority,
            position: self.position,
            status: self.status,
            enqueued_at: self.enqueued_at,
            updated_at: self.updated_at,
            retry_count: self.retry_count,
            error_message: self.error_message,
            _state: PhantomData,
        }
    }

    pub fn complete_success(self) -> Result<QueueEntry, QueueError> {
        match self.status {
            QueueStatus::Testing
            | QueueStatus::ReadyToMerge
            | QueueStatus::Merging => self.transition_impl(QueueStatus::Merged),
            status => Err(QueueError::InvalidStateTransition {
                from: format!("{status:?}"),
                to: "Merged".to_string(),
            }),
        }
    }

    pub fn complete_failure(self, error: String) -> Result<QueueEntry, QueueError> {
        match self.status {
            QueueStatus::Testing => Ok(QueueEntry {
                id: self.id,
                session_id: self.session_id,
                bead_id: self.bead_id,
                priority: self.priority,
                position: self.position,
                status: QueueStatus::FailedRetryable,
                enqueued_at: self.enqueued_at,
                updated_at: Utc::now(),
                retry_count: self.retry_count + 1,
                error_message: Some(error),
                _state: PhantomData,
            }),
            status => Err(QueueError::InvalidStateTransition {
                from: format!("{status:?}"),
                to: "FailedRetryable".to_string(),
            }),
        }
    }

    #[must_use]
    pub fn id(&self) -> &QueueEntryId {
        &self.id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn bead_id(&self) -> Option<&str> {
        self.bead_id.as_deref()
    }

    #[must_use]
    pub fn priority(&self) -> &Priority {
        &self.priority
    }

    #[must_use]
    pub fn position(&self) -> &QueuePosition {
        &self.position
    }

    #[must_use]
    pub fn status(&self) -> QueueStatus {
        self.status
    }

    #[must_use]
    pub fn enqueued_at(&self) -> DateTime<Utc> {
        self.enqueued_at
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[must_use]
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    fn transition_impl<T>(self, new_status: QueueStatus) -> Result<QueueEntry<T>, QueueError> {
        Ok(QueueEntry {
            id: self.id,
            session_id: self.session_id,
            bead_id: self.bead_id,
            priority: self.priority,
            position: self.position,
            status: new_status,
            enqueued_at: self.enqueued_at,
            updated_at: Utc::now(),
            retry_count: self.retry_count,
            error_message: self.error_message,
            _state: PhantomData,
        })
    }
}

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{Priority, QueuePosition};
use crate::error::QueueError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Pending,
    Claimed,
    Rebasing,
    Testing,
    ReadyToMerge,
    Merging,
    Merged,
    FailedRetryable,
    FailedTerminal,
    Cancelled,
}

impl QueueStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }
}

impl Default for QueueStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    pub fn generate() -> Self {
        Self(format!("queue-{}", uuid::Uuid::new_v4()))
    }

    pub fn parse(id: String) -> Result<Self, QueueError> {
        if id.is_empty() {
            return Err(QueueError::InvalidQueueEntryId("empty id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for QueueEntryId {
    fn default() -> Self {
        Self::generate()
    }
}

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
    pub fn id(&self) -> &QueueEntryId {
        &self.id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn bead_id(&self) -> Option<&str> {
        self.bead_id.as_deref()
    }

    pub fn priority(&self) -> &Priority {
        &self.priority
    }

    pub fn position(&self) -> &QueuePosition {
        &self.position
    }

    pub fn status(&self) -> QueueStatus {
        self.status
    }

    pub fn enqueued_at(&self) -> DateTime<Utc> {
        self.enqueued_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

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
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl QueueEntry<FailedRetryable> {
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
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl QueueEntry<Cancelled> {
    pub fn is_terminal(&self) -> bool {
        true
    }
}

pub trait QueueDsl {
    fn enqueue_session(&mut self, session_name: &str) -> &mut Self;
    fn with_high_priority(&mut self) -> &mut Self;
    fn with_low_priority(&mut self) -> &mut Self;
    fn with_critical_priority(&mut self) -> &mut Self;
    fn execute(&mut self) -> Result<QueueEntry<Pending>, QueueError>;
}

pub struct QueueEntryBuilder {
    session_name: Option<String>,
    bead_id: Option<String>,
    priority: Priority,
}

impl QueueEntryBuilder {
    pub fn new() -> Self {
        Self {
            session_name: None,
            bead_id: None,
            priority: Priority::default(),
        }
    }

    pub fn with_session(mut self, session: &str) -> Self {
        self.session_name = Some(session.to_string());
        self
    }

    pub fn with_bead(mut self, bead_id: &str) -> Self {
        self.bead_id = Some(bead_id.to_string());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_high_priority(mut self) -> Self {
        self.priority = Priority::high();
        self
    }

    pub fn with_low_priority(mut self) -> Self {
        self.priority = Priority::low();
        self
    }

    pub fn with_critical_priority(mut self) -> Self {
        self.priority = Priority::critical();
        self
    }

    pub fn enqueue(self) -> Result<QueueEntry<Pending>, QueueError> {
        let session = self
            .session_name
            .ok_or_else(|| QueueError::InvalidQueueEntryId("session name required".into()))?;
        QueueEntry::enqueue(session, self.bead_id, self.priority)
    }
}

impl Default for QueueEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueDsl for QueueEntryBuilder {
    fn enqueue_session(&mut self, session_name: &str) -> &mut Self {
        self.session_name = Some(session_name.to_string());
        self
    }

    fn with_high_priority(&mut self) -> &mut Self {
        self.priority = Priority::high();
        self
    }

    fn with_low_priority(&mut self) -> &mut Self {
        self.priority = Priority::low();
        self
    }

    fn with_critical_priority(&mut self) -> &mut Self {
        self.priority = Priority::critical();
        self
    }

    fn execute(&mut self) -> Result<QueueEntry<Pending>, QueueError> {
        let session = self
            .session_name
            .take()
            .ok_or_else(|| QueueError::InvalidQueueEntryId("session name required".into()))?;
        QueueEntry::enqueue(session, self.bead_id.clone(), self.priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_entry_when_created_then_has_pending_status() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_given_pending_when_claim_then_has_claimed_status() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let claimed = entry.claim().unwrap();
        assert_eq!(claimed.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_given_merged_when_claim_then_fails() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let merged = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_ready_to_merge())
            .and_then(|e| e.start_merging())
            .and_then(|e| e.mark_merged())
            .unwrap();
        let _claimed: Result<QueueEntry<Claimed>, _> = merged.claim();
        assert!(_claimed.is_err());
    }

    #[test]
    fn queue_entry_can_retry_returns_true_for_failed_retryable_under_limit() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let failed = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_failed_retryable("error".into()));
        assert!(failed.is_ok());
        assert!(failed.unwrap().can_retry());
    }

    #[test]
    fn queue_entry_rejects_empty_session_id() {
        let result = QueueEntry::<Pending>::enqueue("".to_string(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_rejects_whitespace_session_id() {
        let result = QueueEntry::<Pending>::enqueue("   ".to_string(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_builder_works() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_high_priority()
            .enqueue()
            .unwrap();
        assert_eq!(entry.session_id, "test-session");
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_dsl_works() {
        let mut builder = QueueEntryBuilder::new();
        builder
            .enqueue_session("dsl-session")
            .with_critical_priority()
            .execute()
            .unwrap();
    }
}

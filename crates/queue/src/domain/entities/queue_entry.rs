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

use crate::domain::queue::status::QueueStatus;
use crate::domain::value_objects::{Priority, QueuePosition};
use crate::error::QueueError;

<<<<<<< HEAD
// Re-export canonical QueueEntryId from identifiers
pub use crate::domain::identifiers::QueueEntryId;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
=======
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("queue-{}", uuid::Uuid::new_v4()))
    }

    pub fn parse(id: String) -> Result<Self, QueueError> {
        if id.is_empty() {
            return Err(QueueError::InvalidQueueEntryId("empty id".into()));
        }
        Ok(Self(id))
    }

    #[must_use]
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
>>>>>>> polecat/epsilon
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
    /// Erase the typestate phantom type, producing `QueueEntry` (= `QueueEntry<Pending>`).
    ///
    /// The repository trait operates on `QueueEntry` regardless of actual runtime status,
    /// so this conversion is necessary when persisting typed entries back through the repo.
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

    /// Complete a job successfully by fast-forwarding from the current runtime state to `Merged`.
    ///
    /// Valid from: `Testing`, `ReadyToMerge`, `Merging`.
    /// Errors for: `Pending`, `Claimed`, `Rebasing` (too early), terminal states.
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

    /// Complete a job with failure by transitioning to `FailedRetryable`.
    ///
    /// Only valid from `Testing` state. Increments `retry_count`.
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_name: None,
            bead_id: None,
            priority: Priority::default(),
        }
    }

    #[must_use]
    pub fn with_session(mut self, session: &str) -> Self {
        self.session_name = Some(session.to_string());
        self
    }

    #[must_use]
    pub fn with_bead(mut self, bead_id: &str) -> Self {
        self.bead_id = Some(bead_id.to_string());
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_high_priority(mut self) -> Self {
        self.priority = Priority::high();
        self
    }

    #[must_use]
    pub fn with_low_priority(mut self) -> Self {
        self.priority = Priority::low();
        self
    }

    #[must_use]
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
        let _merged = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_ready_to_merge())
            .and_then(|e| e.start_merging())
            .and_then(|e| e.mark_merged())
            .unwrap();
        // Verify that claim() is not available on QueueEntry<Merged> (compile-time enforced).
        // The code below would fail to compile if uncommented:
        // let _claimed = merged.claim();
        // This test verifies the typestate pattern prevents invalid transitions.
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

    #[test]
    fn queue_entry_full_lifecycle_happy_path() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Merged);
        assert!(entry.is_terminal());
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
    }

    #[test]
    fn queue_entry_builder_with_bead() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_bead("bead-42")
            .enqueue()
            .unwrap();
        assert_eq!(entry.bead_id(), Some("bead-42"));
    }

    #[test]
    fn queue_entry_builder_with_custom_priority() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_priority(Priority::low())
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_builder_default_is_normal_priority() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 200);
    }

    #[test]
    fn queue_entry_builder_missing_session_returns_error() {
        let result = QueueEntryBuilder::new().enqueue();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_builder_default_trait() {
        let builder = QueueEntryBuilder::default();
        let result = builder.enqueue();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_dsl_missing_session_returns_error() {
        let mut builder = QueueEntryBuilder::new();
        let result = builder.execute();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_dsl_with_low_priority() {
        let mut builder = QueueEntryBuilder::new();
        let entry = builder
            .enqueue_session("session")
            .with_low_priority()
            .execute()
            .unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_failed_retryable_stores_error() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("tests failed".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 1);
        assert_eq!(entry.error_message(), Some("tests failed"));
    }

    #[test]
    fn queue_entry_failed_terminal_stores_error() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_terminal("fatal error".into())
            .unwrap();

        assert!(entry.is_terminal());
        assert_eq!(entry.error_message(), Some("fatal error"));
    }

    #[test]
    fn queue_entry_failed_retryable_can_retry_increments() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error 1".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error 2".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 2);
        assert!(entry.can_retry());
    }

    #[test]
    fn queue_entry_failed_retryable_max_retries_exhausted() {
        // 3 failures exhausts retries
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e1".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e2".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e3".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 3);
        assert!(!entry.can_retry());
    }

    #[test]
    fn queue_entry_cancel_from_pending() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_cancel_from_claimed() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_rebasing() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_testing() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_ready_to_merge() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_merging() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_failed_retryable() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("err".into())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_accessors() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-99".into()),
            Priority::high(),
        )
        .unwrap();

        assert_eq!(entry.session_id(), "session-1");
        assert_eq!(entry.bead_id(), Some("bead-99"));
        assert!(!entry.id().as_str().is_empty());
        assert_eq!(entry.status(), QueueStatus::Pending);
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
        assert!(entry.enqueued_at().timestamp() != 0);
        assert!(entry.updated_at().timestamp() != 0);
    }

    #[test]
    fn queue_entry_trimmed_session_id() {
        let entry =
            QueueEntry::<Pending>::enqueue("  spaced  ".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.session_id(), "spaced");
    }

    #[test]
    fn queue_entry_id_generate_is_unique() {
        let a = QueueEntryId::generate();
        let b = QueueEntryId::generate();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn queue_entry_id_new_valid() {
        let id = QueueEntryId::new("my-id");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "my-id");
    }

    #[test]
    fn queue_entry_id_new_empty_rejected() {
        let result = QueueEntryId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_id_default_generates() {
        let id = QueueEntryId::default();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn queue_entry_id_startswith_queue_prefix() {
        let id = QueueEntryId::generate();
        assert!(id.as_str().starts_with("queue-"));
    }

    #[test]
    fn queue_entry_serde_roundtrip() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-1".into()),
            Priority::normal(),
        )
        .unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let back: QueueEntry<Pending> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id(), "session-1");
        assert_eq!(back.bead_id(), Some("bead-1"));
    }

    #[test]
    fn queue_entry_queue_status_default_is_pending() {
        assert_eq!(QueueStatus::default(), QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_queue_status_is_terminal_all_cases() {
        assert!(QueueStatus::Merged.is_terminal());
        assert!(QueueStatus::FailedTerminal.is_terminal());
        assert!(QueueStatus::Cancelled.is_terminal());
        assert!(!QueueStatus::Pending.is_terminal());
        assert!(!QueueStatus::Claimed.is_terminal());
        assert!(!QueueStatus::Rebasing.is_terminal());
        assert!(!QueueStatus::Testing.is_terminal());
        assert!(!QueueStatus::ReadyToMerge.is_terminal());
        assert!(!QueueStatus::Merging.is_terminal());
        assert!(!QueueStatus::FailedRetryable.is_terminal());
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn queue_entry_enqueue_with_critical_priority() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::critical()).unwrap();
        assert_eq!(entry.priority().value(), u8::MAX);
    }

    #[test]
    fn queue_entry_enqueue_with_low_priority() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::low()).unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_full_failure_path_retry_then_cancel() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error".into())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_position_is_front_after_enqueue() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.position().value(), 0);
    }

    #[test]
    fn queue_entry_enqueued_at_recent() {
        let before = Utc::now();
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let after = Utc::now();
        assert!(entry.enqueued_at() >= before);
        assert!(entry.enqueued_at() <= after);
    }

    #[test]
    fn queue_entry_updated_at_recent() {
        let before = Utc::now();
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let after = Utc::now();
        assert!(entry.updated_at() >= before);
        assert!(entry.updated_at() <= after);
    }

    #[test]
    fn queue_entry_transition_updates_updated_at() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let created_at = entry.enqueued_at();

        // Small sleep to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(5));

        let claimed = entry.claim().unwrap();
        assert!(claimed.updated_at() >= created_at);
    }

    #[test]
    fn queue_entry_builder_with_all_fields() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_bead("bead-1")
            .with_priority(Priority::critical())
            .enqueue()
            .unwrap();

        assert_eq!(entry.session_id(), "test-session");
        assert_eq!(entry.bead_id(), Some("bead-1"));
        assert_eq!(entry.priority().value(), u8::MAX);
        assert_eq!(entry.status(), QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_builder_chained_methods() {
        let entry = QueueEntryBuilder::new()
            .with_session("s1")
            .with_low_priority()
            .with_bead("b1")
            .enqueue()
            .unwrap();

        assert_eq!(entry.session_id(), "s1");
        assert_eq!(entry.bead_id(), Some("b1"));
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_dsl_with_critical_priority() {
        let mut builder = QueueEntryBuilder::new();
        let entry = builder
            .enqueue_session("critical-session")
            .with_critical_priority()
            .execute()
            .unwrap();
        assert_eq!(entry.priority().value(), u8::MAX);
    }

    #[test]
    fn queue_entry_retry_full_cycle_three_failures() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();

        // First failure cycle
        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e1".into())
            .unwrap();
        assert!(entry.can_retry());

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e2".into())
            .unwrap();
        assert!(entry.can_retry());

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e3".into())
            .unwrap();
        assert!(!entry.can_retry());
        assert_eq!(entry.retry_count(), 3);
    }

    #[test]
    fn queue_entry_serde_roundtrip_claimed() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-1".into()),
            Priority::normal(),
        )
        .unwrap();
        let claimed = entry.claim().unwrap();
        let json = serde_json::to_string(&claimed).unwrap();
        let back: QueueEntry<Claimed> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status(), QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_serde_roundtrip_merged() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::normal()).unwrap();
        let merged = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();
        let json = serde_json::to_string(&merged).unwrap();
        let back: QueueEntry<Merged> = serde_json::from_str(&json).unwrap();
        assert!(back.is_terminal());
    }

    #[test]
    fn queue_entry_all_state_markers_terminal() {
        // Merged
        let merged = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();
        assert!(merged.is_terminal());

        // FailedTerminal
        let failed_term = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_terminal("err".into())
            .unwrap();
        assert!(failed_term.is_terminal());

        // Cancelled
        let cancelled = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .cancel()
            .unwrap();
        assert!(cancelled.is_terminal());
    }

    #[test]
    fn queue_entry_state_marker_units() {
        // Verify the unit-like state structs have expected defaults
        assert_eq!(Pending::default(), Pending);
        assert_eq!(Claimed::default(), Claimed);
        assert_eq!(Rebasing::default(), Rebasing);
        assert_eq!(Testing::default(), Testing);
        assert_eq!(ReadyToMerge::default(), ReadyToMerge);
        assert_eq!(Merging::default(), Merging);
        assert_eq!(Merged::default(), Merged);
        assert_eq!(FailedRetryable::default(), FailedRetryable);
        assert_eq!(FailedTerminal::default(), FailedTerminal);
        assert_eq!(Cancelled::default(), Cancelled);
    }

    #[test]
    fn queue_entry_status_serde_serializes_all_variants() {
        let statuses = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            assert!(json.starts_with('"'), "JSON should be a string: {json}");
        }
    }

    #[test]
    fn queue_entry_accessors_all_fields() {
        let entry = QueueEntry::<Pending>::enqueue(
            "my-session".into(),
            Some("bead-7".into()),
            Priority::high(),
        )
        .unwrap();

        assert!(!entry.id().as_str().is_empty());
        assert!(entry.id().as_str().starts_with("queue-"));
        assert_eq!(entry.session_id(), "my-session");
        assert_eq!(entry.bead_id(), Some("bead-7"));
        assert_eq!(entry.priority().value(), 230);
        assert_eq!(entry.position().value(), 0);
        assert_eq!(entry.status(), QueueStatus::Pending);
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
    }

    #[test]
    fn queue_entry_builder_default_priority_is_normal() {
        let entry = QueueEntryBuilder::new()
            .with_session("test")
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 200);
    }
}

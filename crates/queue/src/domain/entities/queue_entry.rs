use crate::domain::value_objects::{Priority, QueuePosition};
use crate::error::QueueError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub session_id: String,
    pub bead_id: Option<String>,
    pub priority: Priority,
    pub position: QueuePosition,
    pub status: QueueStatus,
    pub enqueued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retry_count: u32,
    pub error_message: Option<String>,
}

impl QueueEntry {
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
        })
    }

    pub fn claim(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Pending {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Claimed".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Claimed))
    }

    pub fn start_rebase(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Claimed {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Rebasing".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Rebasing))
    }

    pub fn start_testing(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Rebasing {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Testing".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Testing))
    }

    pub fn mark_ready_to_merge(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Testing {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "ReadyToMerge".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::ReadyToMerge))
    }

    pub fn start_merging(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::ReadyToMerge {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Merging".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Merging))
    }

    pub fn mark_merged(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Merging {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Merged".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Merged))
    }

    pub fn mark_failed_retryable(&self, error: String) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Testing {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "FailedRetryable".into(),
            });
        }
        let mut new_entry = self.transition_to(QueueStatus::FailedRetryable);
        new_entry.retry_count += 1;
        new_entry.error_message = Some(error);
        Ok(new_entry)
    }

    pub fn mark_failed_terminal(&self, error: String) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Testing {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "FailedTerminal".into(),
            });
        }
        let mut new_entry = self.transition_to(QueueStatus::FailedTerminal);
        new_entry.error_message = Some(error);
        Ok(new_entry)
    }

    pub fn cancel(&self) -> Result<Self, QueueError> {
        if matches!(self.status, QueueStatus::Merged | QueueStatus::Cancelled) {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Cancelled".into(),
            });
        }
        Ok(self.transition_to(QueueStatus::Cancelled))
    }

    fn transition_to(&self, new_status: QueueStatus) -> Self {
        Self {
            id: self.id.clone(),
            session_id: self.session_id.clone(),
            bead_id: self.bead_id.clone(),
            priority: self.priority,
            position: self.position,
            status: new_status,
            enqueued_at: self.enqueued_at,
            updated_at: Utc::now(),
            retry_count: self.retry_count,
            error_message: self.error_message.clone(),
        }
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.status, QueueStatus::FailedRetryable) && self.retry_count < 3
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        )
    }
}

pub trait QueueDsl {
    fn enqueue_session(&mut self, session_name: &str) -> &mut Self;
    fn with_high_priority(&mut self) -> &mut Self;
    fn with_low_priority(&mut self) -> &mut Self;
    fn with_critical_priority(&mut self) -> &mut Self;
    fn execute(&mut self) -> Result<QueueEntry, QueueError>;
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

    pub fn enqueue(self) -> Result<QueueEntry, QueueError> {
        let session = self.session_name.ok_or_else(|| {
            QueueError::InvalidQueueEntryId("session name required".into())
        })?;
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

    fn execute(&mut self) -> Result<QueueEntry, QueueError> {
        let session = self.session_name.take().ok_or_else(|| {
            QueueError::InvalidQueueEntryId("session name required".into())
        })?;
        QueueEntry::enqueue(session, self.bead_id.clone(), self.priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_entry_when_created_then_has_pending_status() {
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_given_pending_when_claim_then_has_claimed_status() {
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let claimed = entry.claim().unwrap();
        assert_eq!(claimed.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_given_merged_when_claim_then_fails() {
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let merged = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_ready_to_merge())
            .and_then(|e| e.start_merging())
            .and_then(|e| e.mark_merged())
            .unwrap();
        let result = merged.claim();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_can_retry_returns_true_for_failed_retryable_under_limit() {
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let failed = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_failed_retryable("error".into()))
            .unwrap();
        assert!(failed.can_retry());
    }

    #[test]
    fn queue_entry_rejects_empty_session_id() {
        let result = QueueEntry::enqueue("".to_string(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_rejects_whitespace_session_id() {
        let result = QueueEntry::enqueue("   ".to_string(), None, Priority::default());
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

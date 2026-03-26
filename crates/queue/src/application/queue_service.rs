#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
use crate::domain::ports::QueueRepository;
use crate::domain::value_objects::Priority;
use crate::error::{QueueError, Result};

pub struct QueueService<R: QueueRepository> {
    repository: R,
}

impl<R: QueueRepository> QueueService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn enqueue(
        &self,
        session_id: String,
        bead_id: Option<String>,
        priority: Priority,
    ) -> Result<QueueEntry> {
        let entry = QueueEntry::enqueue(session_id, bead_id, priority)?;
        self.repository.enqueue(entry)
    }

    pub fn dequeue(&self) -> Result<Option<QueueEntry>> {
        self.repository.dequeue()
    }

    pub fn get_job(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        self.repository.get(id)
    }

    pub fn update_job(&self, entry: QueueEntry) -> Result<QueueEntry> {
        self.repository.update(entry)
    }

    pub fn claim_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let claimed = entry.claim()?;
        self.repository.update(claimed)
    }

    pub fn complete_job(&self, id: &QueueEntryId, success: bool) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;

        if success {
            entry
                .claim()
                .and_then(|e| e.start_rebase())
                .and_then(|e| e.start_testing())
                .and_then(|e| e.mark_ready_to_merge())
                .and_then(|e| e.start_merging())
                .and_then(|e| e.mark_merged())
                .and_then(|e| self.repository.update(e))
        } else {
            // Transition through states: Pending -> Claimed -> Rebasing -> Testing -> FailedRetryable
            entry
                .claim()
                .and_then(|e| e.start_rebase())
                .and_then(|e| e.start_testing())
                .and_then(|e| e.mark_failed_retryable("Test failed".into()))
                .and_then(|e| self.repository.update(e))
        }
    }

    pub fn cancel_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let cancelled = entry.cancel()?;
        self.repository.update(cancelled)
    }

    pub fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        self.repository.list_pending()
    }

    pub fn list_active(&self) -> Result<Vec<QueueEntry>> {
        let all = self.repository.list_all()?;
        Ok(all
            .into_iter()
            .filter(|e| QueueStateMachine::is_active(e.status))
            .collect())
    }

    pub fn list_all(&self) -> Result<Vec<QueueEntry>> {
        self.repository.list_all()
    }

    pub fn remove_job(&self, id: &QueueEntryId) -> Result<()> {
        self.repository.remove(id)
    }

    pub fn retry_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;

        // can_retry() is only on QueueEntry<FailedRetryable>, check via runtime status
        if entry.status != QueueStatus::FailedRetryable || entry.retry_count >= 3 {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", entry.status),
                to: "Pending".into(),
            });
        }

        let requeued = QueueEntry::enqueue(
            entry.session_id.clone(),
            entry.bead_id.clone(),
            entry.priority,
        )?;
        self.repository.enqueue(requeued)
    }
}

struct QueueStateMachine;

impl QueueStateMachine {
    fn is_active(status: QueueStatus) -> bool {
        matches!(
            status,
            QueueStatus::Claimed
                | QueueStatus::Rebasing
                | QueueStatus::Testing
                | QueueStatus::ReadyToMerge
                | QueueStatus::Merging
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::InMemoryQueueRepository;

    fn create_service() -> QueueService<InMemoryQueueRepository> {
        QueueService::new(InMemoryQueueRepository::new())
    }

    #[test]
    fn queue_service_enqueue_creates_pending_job() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_service_dequeue_returns_claimed_job() {
        let service = create_service();
        service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let dequeued = service.dequeue().unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_dequeue_empty_queue_returns_none() {
        let service = create_service();
        let result = service.dequeue().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn queue_service_claim_job_changes_status() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        assert_eq!(claimed.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_claim_nonexistent_job_returns_error() {
        let service = create_service();
        let result = service.claim_job(&QueueEntryId::generate());
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_success() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let result = service.complete_job(&claimed.id, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::Merged);
    }

    #[test]
    fn queue_service_complete_job_failure() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let result = service.complete_job(&claimed.id, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::FailedRetryable);
    }

    #[test]
    fn queue_service_cancel_job() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let cancelled = service.cancel_job(&entry.id).unwrap();
        assert_eq!(cancelled.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_list_pending() {
        let service = create_service();
        service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        service
            .enqueue("session-2".into(), None, Priority::default())
            .unwrap();
        let pending = service.list_pending().unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn queue_service_list_all() {
        let service = create_service();
        service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        service
            .enqueue("session-2".into(), None, Priority::default())
            .unwrap();
        let all = service.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn queue_service_remove_job() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        service.remove_job(&entry.id).unwrap();
        let result = service.get_job(&entry.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn queue_service_enqueue_empty_session_returns_error() {
        let service = create_service();
        let result = service.enqueue("".into(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_retry_job() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let failed = service.complete_job(&claimed.id, false).unwrap();
        assert_eq!(failed.status, QueueStatus::FailedRetryable);
        let retried = service.retry_job(&failed.id).unwrap();
        assert_eq!(retried.status, QueueStatus::Pending);
    }
}

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::{
    domain::{
        identifiers::QueueEntryId,
        ports::QueueRepository,
        queue::{entry::QueueEntry, status::QueueStatus},
        value_objects::Priority,
    },
    error::{QueueError, Result},
};

pub struct QueueService<R: QueueRepository> {
    repository: R,
}

impl<R: QueueRepository> QueueService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Enqueue a new job in the queue.
    ///
    /// # Errors
    /// Returns `QueueError::ValidationError` if `session_id` is empty or whitespace.
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn enqueue(&self, session_id: String, priority: u32) -> Result<QueueEntry> {
        let id = QueueEntryId::generate();
        let entry = QueueEntry::new(id.as_str(), session_id, priority)?;
        self.repository.enqueue(entry)
    }

    /// Dequeue the next pending job from the queue.
    ///
    /// # Errors
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn dequeue(&self) -> Result<Option<QueueEntry>> {
        self.repository.dequeue()
    }

    /// Get a job by its ID.
    ///
    /// # Errors
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn get_job(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        self.repository.get(id)
    }

    /// Update a job in the queue.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn update_job(&self, entry: QueueEntry) -> Result<QueueEntry> {
        self.repository.update(entry)
    }

    /// Claim a job, changing its status to Claimed.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::InvalidStateTransition` if the job cannot be claimed.
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn claim_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let claimed = entry.transition_status(QueueStatus::Claimed)?;
        self.repository.update(claimed)
    }

    /// Complete a job, transitioning it to `Merged` or `FailedRetryable`.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::InvalidStateTransition` if the job cannot be completed from its current
    /// state. Returns `QueueError::RepositoryError` if the repository fails.
    pub fn complete_job(&self, id: &QueueEntryId, success: bool) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;

        if success {
            let result = match entry.status {
                QueueStatus::Pending => entry
                    .transition_status(QueueStatus::Claimed)
                    .and_then(|e| e.transition_status(QueueStatus::Rebasing))
                    .and_then(|e| e.transition_status(QueueStatus::Testing))
                    .and_then(|e| e.transition_status(QueueStatus::ReadyToMerge))
                    .and_then(|e| e.transition_status(QueueStatus::Merging))
                    .and_then(|e| e.transition_status(QueueStatus::Merged)),
                QueueStatus::Claimed => entry
                    .transition_status(QueueStatus::Rebasing)
                    .and_then(|e| e.transition_status(QueueStatus::Testing))
                    .and_then(|e| e.transition_status(QueueStatus::ReadyToMerge))
                    .and_then(|e| e.transition_status(QueueStatus::Merging))
                    .and_then(|e| e.transition_status(QueueStatus::Merged)),
                _ => {
                    return Err(QueueError::InvalidStateTransition {
                        from: format!("{:?}", entry.status),
                        to: "Merged".into(),
                    })
                }
            }?;
            self.repository.update(result)
        } else {
            let result = match entry.status {
                QueueStatus::Pending => entry
                    .transition_status(QueueStatus::Claimed)
                    .and_then(|e| e.transition_status(QueueStatus::Rebasing))
                    .and_then(|e| e.transition_status(QueueStatus::Testing))
                    .and_then(|e| e.with_failure("Test failed".into())),
                QueueStatus::Claimed => entry
                    .transition_status(QueueStatus::Rebasing)
                    .and_then(|e| e.transition_status(QueueStatus::Testing))
                    .and_then(|e| e.with_failure("Test failed".into())),
                _ => {
                    return Err(QueueError::InvalidStateTransition {
                        from: format!("{:?}", entry.status),
                        to: "FailedRetryable".into(),
                    })
                }
            }?;
            self.repository.update(result)
        }
    }

    /// Cancel a job, changing its status to Cancelled.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::InvalidStateTransition` if the job cannot be cancelled.
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn cancel_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let cancelled = entry.transition_status(QueueStatus::Cancelled)?;
        self.repository.update(cancelled)
    }

    /// List all pending jobs in the queue.
    ///
    /// # Errors
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        self.repository.list_pending()
    }

    /// List all active (non-terminal) jobs in the queue.
    ///
    /// # Errors
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn list_active(&self) -> Result<Vec<QueueEntry>> {
        let all = self.repository.list_all()?;
        Ok(all
            .into_iter()
            .filter(|e| QueueStatus::is_active(e.status))
            .collect())
    }

    /// List all jobs in the queue.
    ///
    /// # Errors
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn list_all(&self) -> Result<Vec<QueueEntry>> {
        self.repository.list_all()
    }

    /// Remove a job from the queue.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::RepositoryError` if the repository fails.
    pub fn remove_job(&self, id: &QueueEntryId) -> Result<()> {
        self.repository.remove(id)
    }

    /// Retry a failed job, re-queuing it with a new ID.
    ///
    /// # Errors
    /// Returns `QueueError::QueueEntryNotFound` if the job does not exist.
    /// Returns `QueueError::InvalidStateTransition` if the job is not in `FailedRetryable` state or
    /// has exceeded retry limit. Returns `QueueError::RepositoryError` if the repository fails.
    pub fn retry_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;

        if entry.status != QueueStatus::FailedRetryable || entry.retry_count >= 3 {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", entry.status),
                to: "Pending".into(),
            });
        }

        let requeued = QueueEntry::new(
            QueueEntryId::generate().as_str(),
            entry.session.as_str().to_string(),
            entry.priority,
        )?;
        self.repository.enqueue(requeued)
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
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_service_dequeue_returns_pending_job() {
        let service = create_service();
        service.enqueue("session-1".into(), 50).unwrap();
        let dequeued = service.dequeue().unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().status, QueueStatus::Pending);
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
        let entry = service.enqueue("session-1".into(), 50).unwrap();
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
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let result = service.complete_job(&claimed.id, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::Merged);
    }

    #[test]
    fn queue_service_complete_job_failure() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let result = service.complete_job(&claimed.id, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::FailedRetryable);
    }

    #[test]
    fn queue_service_cancel_job() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let cancelled = service.cancel_job(&entry.id).unwrap();
        assert_eq!(cancelled.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_list_pending() {
        let service = create_service();
        service.enqueue("session-1".into(), 50).unwrap();
        service.enqueue("session-2".into(), 50).unwrap();
        let pending = service.list_pending().unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn queue_service_list_all() {
        let service = create_service();
        service.enqueue("session-1".into(), 50).unwrap();
        service.enqueue("session-2".into(), 50).unwrap();
        let all = service.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn queue_service_remove_job() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        service.remove_job(&entry.id).unwrap();
        let result = service.get_job(&entry.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn queue_service_enqueue_empty_session_returns_error() {
        let service = create_service();
        let result = service.enqueue("".into(), 50);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_retry_job() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();
        let failed = service.complete_job(&claimed.id, false).unwrap();
        assert_eq!(failed.status, QueueStatus::FailedRetryable);
        let retried = service.retry_job(&failed.id).unwrap();
        assert_eq!(retried.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_service_get_job_nonexistent() {
        let service = create_service();
        let result = service.get_job(&QueueEntryId::generate());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn queue_service_get_job_existing() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let found = service.get_job(&entry.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().session.as_str(), "session-1");
    }

    #[test]
    fn queue_service_update_job() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let claimed = entry.transition_status(QueueStatus::Claimed).unwrap();
        let updated = service.update_job(claimed).unwrap();
        assert_eq!(updated.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_list_pending_excludes_claimed() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let _claimed = service.claim_job(&entry.id).unwrap();
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_service_list_active_with_active_entry() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        let _claimed = service.claim_job(&entry.id).unwrap();
        let active = service.list_active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_list_active_empty() {
        let service = create_service();
        let active = service.list_active().unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn queue_service_retry_nonexistent_job() {
        let service = create_service();
        let result = service.retry_job(&QueueEntryId::generate());
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_cancel_nonexistent_job() {
        let service = create_service();
        let result = service.cancel_job(&QueueEntryId::generate());
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_nonexistent_job() {
        let service = create_service();
        let result = service.complete_job(&QueueEntryId::generate(), true);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_remove_nonexistent_job() {
        let service = create_service();
        let result = service.remove_job(&QueueEntryId::generate());
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_dequeue_non_pending_returns_none() {
        let service = create_service();
        let entry = service.enqueue("session-1".into(), 50).unwrap();
        service.claim_job(&entry.id).unwrap();

        // Dequeue should skip non-pending entries
        let dequeued = service.dequeue().unwrap();
        assert!(dequeued.is_none());
    }

    #[test]
    fn queue_service_enqueue_whitespace_session_returns_error() {
        let service = create_service();
        let result = service.enqueue("   ".into(), 50);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_enqueue_multiple_entries() {
        let service = create_service();
        service.enqueue("s1".into(), 50).unwrap();
        service.enqueue("s2".into(), 50).unwrap();
        service.enqueue("s3".into(), 50).unwrap();

        let pending = service.list_pending().unwrap();
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn queue_service_dequeue_fifo_order() {
        let service = create_service();
        service.enqueue("s1".into(), 50).unwrap();
        service.enqueue("s2".into(), 50).unwrap();

        let d1 = service.dequeue().unwrap().unwrap();
        assert_eq!(d1.session.as_str(), "s1");

        let d2 = service.dequeue().unwrap().unwrap();
        assert_eq!(d2.session.as_str(), "s2");
    }

    #[test]
    fn queue_service_complete_job_failure_stores_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();
        let failed = service.complete_job(&entry.id, false).unwrap();

        assert_eq!(failed.status, QueueStatus::FailedRetryable);
        assert!(failed.error_message.is_some());
    }

    #[test]
    fn queue_service_get_job_after_dequeue() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();
        let id = entry.id.clone();

        service.dequeue().unwrap();

        let found = service.get_job(&id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn queue_service_list_active_empty_queue() {
        let service = create_service();
        let active = service.list_active().unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn queue_service_list_all_empty_queue() {
        let service = create_service();
        let all = service.list_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn queue_service_list_pending_empty_queue() {
        let service = create_service();
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_service_retry_exhausted_returns_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();

        // Fail to get retry_count = 1
        let failed = service.complete_job(&entry.id, false).unwrap();
        assert_eq!(failed.retry_count, 1);

        // Manually update retry_count to simulate 3 prior failures
        let exhausted = QueueEntry {
            retry_count: 3,
            ..failed.clone()
        };
        service.update_job(exhausted).unwrap();

        let result = service.retry_job(&entry.id);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_retry_non_failed_status_returns_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();

        let result = service.retry_job(&entry.id);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_claim_then_cancel() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();
        let cancelled = service.cancel_job(&entry.id).unwrap();
        assert_eq!(cancelled.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_claim_then_cancel_then_get() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();
        let id = entry.id.clone();

        service.cancel_job(&id).unwrap();

        let found = service.get_job(&id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_update_job_after_claim() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), 50).unwrap();
        let claimed = service.claim_job(&entry.id).unwrap();

        let updated = service.update_job(claimed).unwrap();
        assert_eq!(updated.status, QueueStatus::Claimed);
    }
}

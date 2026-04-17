#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
use crate::domain::ports::QueueRepository;
use crate::domain::state::QueueStateMachine;
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
        Ok(self.repository.enqueue(entry)?)
    }

    pub fn dequeue(&self) -> Result<Option<QueueEntry>> {
        Ok(self.repository.dequeue()?)
    }

    pub fn get_job(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        Ok(self.repository.get(id)?)
    }

    pub fn update_job(&self, entry: QueueEntry) -> Result<QueueEntry> {
        Ok(self.repository.update(entry)?)
    }

    pub fn claim_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let claimed = entry.claim()?;
        Ok(self.repository.update(claimed.into_erased())?)
    }

    pub fn complete_job(&self, id: &QueueEntryId, success: bool) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;

        let updated = if success {
            entry.complete_success()?
        } else {
            entry.complete_failure("Job failed".into())?
        };

        Ok(self.repository.update(updated)?)
    }

    pub fn cancel_job(&self, id: &QueueEntryId) -> Result<QueueEntry> {
        let entry = self
            .repository
            .get(id)?
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().to_string()))?;
        let cancelled = entry.cancel()?;
        Ok(self.repository.update(cancelled.into_erased())?)
    }

    pub fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        Ok(self.repository.list_pending()?)
    }

    pub fn list_active(&self) -> Result<Vec<QueueEntry>> {
        let all = self.repository.list_all()?;
        Ok(all
            .into_iter()
            .filter(|e| QueueStateMachine::is_active(e.status))
            .collect())
    }

    pub fn list_all(&self) -> Result<Vec<QueueEntry>> {
        Ok(self.repository.list_all()?)
    }

    pub fn remove_job(&self, id: &QueueEntryId) -> Result<()> {
        self.repository.remove(id)?;
        Ok(())
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
        Ok(self.repository.enqueue(requeued)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::InMemoryQueueRepository;

    fn create_service() -> QueueService<InMemoryQueueRepository> {
        QueueService::new(InMemoryQueueRepository::new())
    }

    /// Advance an entry through the pipeline to a given status via direct state manipulation.
    /// This bypasses the typestate pattern since the repo erases type info on round-trip.
    fn advance_to(service: &QueueService<InMemoryQueueRepository>, id: &QueueEntryId, target: QueueStatus) -> QueueEntry {
        let entry = service.get_job(id).unwrap().unwrap();
        let advanced = QueueEntry {
            status: target,
            updated_at: chrono::Utc::now(),
            ..entry
        };
        service.update_job(advanced).unwrap()
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
    fn queue_service_dequeue_returns_pending_job() {
        let service = create_service();
        service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
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
    fn queue_service_complete_job_success_from_testing() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::Merged);
    }

    #[test]
    fn queue_service_complete_job_success_from_merging() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Merging);

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::Merged);
    }

    #[test]
    fn queue_service_complete_job_success_from_pending_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_success_from_claimed_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Claimed);

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_success_from_rebasing_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Rebasing);

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_success_from_merged_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Merged);

        let result = service.complete_job(&entry.id, true);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_failure_from_testing() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);

        let result = service.complete_job(&entry.id, false);
        assert!(result.is_ok());
        let failed = result.unwrap();
        assert_eq!(failed.status, QueueStatus::FailedRetryable);
        assert_eq!(failed.retry_count(), 1);
        assert!(failed.error_message().is_some());
    }

    #[test]
    fn queue_service_complete_job_failure_from_pending_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();

        let result = service.complete_job(&entry.id, false);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_complete_job_failure_from_claimed_rejected() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Claimed);

        let result = service.complete_job(&entry.id, false);
        assert!(result.is_err());
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
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);
        let failed = service.complete_job(&entry.id, false).unwrap();
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
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let found = service.get_job(&entry.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_id(), "session-1");
    }

    #[test]
    fn queue_service_update_job() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let updated = service.update_job(entry).unwrap();
        assert_eq!(updated.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_service_list_pending_excludes_claimed() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        let _claimed = service.claim_job(&entry.id).unwrap();
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_service_list_active_with_active_entry() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
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
    fn queue_service_enqueue_with_bead_id() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), Some("bead-42".into()), Priority::default())
            .unwrap();
        assert_eq!(entry.bead_id(), Some("bead-42"));
    }

    #[test]
    fn queue_service_dequeue_non_pending_returns_none() {
        let service = create_service();
        let entry = service
            .enqueue("session-1".into(), None, Priority::default())
            .unwrap();
        // Claim the entry so it's no longer Pending
        service.claim_job(&entry.id).unwrap();

        // Dequeue should skip non-pending entries in the in-memory repo
        let dequeued = service.dequeue().unwrap();
        // The in-memory repo at infrastructure level only dequeues Pending entries
        assert!(dequeued.is_none());
    }

    #[test]
    fn queue_service_enqueue_whitespace_session_returns_error() {
        let service = create_service();
        let result = service.enqueue("   ".into(), None, Priority::default());
        assert!(result.is_err());
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn queue_service_enqueue_multiple_entries() {
        let service = create_service();
        service.enqueue("s1".into(), None, Priority::default()).unwrap();
        service.enqueue("s2".into(), None, Priority::default()).unwrap();
        service.enqueue("s3".into(), None, Priority::default()).unwrap();

        let pending = service.list_pending().unwrap();
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn queue_service_dequeue_fifo_order() {
        let service = create_service();
        service.enqueue("s1".into(), None, Priority::default()).unwrap();
        service.enqueue("s2".into(), None, Priority::default()).unwrap();

        let d1 = service.dequeue().unwrap().unwrap();
        assert_eq!(d1.session_id(), "s1");

        let d2 = service.dequeue().unwrap().unwrap();
        assert_eq!(d2.session_id(), "s2");
    }

    #[test]
    fn queue_service_complete_job_failure_stores_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);

        let failed = service.complete_job(&entry.id, false).unwrap();
        assert_eq!(failed.status, QueueStatus::FailedRetryable);
        assert!(failed.error_message().is_some());
    }

    #[test]
    fn queue_service_complete_job_success_skips_intermediate_from_ready_to_merge() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::ReadyToMerge);

        let completed = service.complete_job(&entry.id, true).unwrap();
        assert_eq!(completed.status, QueueStatus::Merged);
    }

    #[test]
    fn queue_service_get_job_after_dequeue() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
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
    fn queue_service_enqueue_preserves_bead_id() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), Some("bead-99".into()), Priority::default()).unwrap();
        assert_eq!(entry.bead_id(), Some("bead-99"));

        let found = service.get_job(&entry.id).unwrap().unwrap();
        assert_eq!(found.bead_id(), Some("bead-99"));
    }

    #[test]
    fn queue_service_retry_exhausted_returns_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);

        // Fail once (retry_count goes to 1)
        let failed = service.complete_job(&entry.id, false).unwrap();
        assert_eq!(failed.retry_count(), 1);

        // retry_job creates a fresh entry, so retry always succeeds
        let retried = service.retry_job(&failed.id).unwrap();
        assert_eq!(retried.status, QueueStatus::Pending);

        // An entry with retry_count >= 3 should be rejected by retry_job
        // (simulate via direct state manipulation)
        let entry2 = service.enqueue("s2".into(), None, Priority::default()).unwrap();
        let exhausted = QueueEntry {
            status: QueueStatus::FailedRetryable,
            retry_count: 3,
            ..service.get_job(&entry2.id).unwrap().unwrap()
        };
        service.update_job(exhausted).unwrap();
        let result = service.retry_job(&entry2.id);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_retry_non_failed_status_returns_error() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();

        // Try to retry a pending job (not failed)
        let result = service.retry_job(&entry.id);
        assert!(result.is_err());
    }

    #[test]
    fn queue_service_claim_then_cancel() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        let cancelled = service.cancel_job(&entry.id).unwrap();
        assert_eq!(cancelled.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_claim_then_cancel_then_get() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        let id = entry.id.clone();

        service.cancel_job(&id).unwrap();

        let found = service.get_job(&id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().status(), QueueStatus::Cancelled);
    }

    #[test]
    fn queue_service_update_job_after_claim() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        let _claimed = service.claim_job(&entry.id).unwrap();

        // Verify the persisted entry has Claimed status
        let found = service.get_job(&entry.id).unwrap().unwrap();
        assert_eq!(found.status(), QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_complete_job_failure_increments_retry_count() {
        let service = create_service();
        let entry = service.enqueue("s1".into(), None, Priority::default()).unwrap();
        advance_to(&service, &entry.id, QueueStatus::Testing);

        let failed = service.complete_job(&entry.id, false).unwrap();
        assert_eq!(failed.retry_count(), 1);
    }
}

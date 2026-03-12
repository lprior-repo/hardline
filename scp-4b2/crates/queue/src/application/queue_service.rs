use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
use crate::domain::value_objects::Priority;
use crate::error::{QueueError, Result};

pub struct QueueService;

impl QueueService {
    pub fn enqueue(session_id: String, bead_id: Option<String>, priority: Priority) -> QueueEntry {
        QueueEntry::enqueue(session_id, bead_id, priority)
    }

    pub fn claim(entry: &QueueEntry) -> Result<QueueEntry> {
        entry.claim()
    }

    pub fn process_entry(entry: &QueueEntry) -> Result<QueueEntry> {
        let claimed = entry.claim()?;
        let rebasing = claimed.start_rebase()?;
        rebasing.start_testing()
    }

    pub fn complete_testing(entry: &QueueEntry, success: bool) -> Result<QueueEntry> {
        if success {
            let ready = entry.mark_ready_to_merge()?;
            ready.start_merging()?.mark_merged()
        } else {
            entry.mark_failed_retryable("Test failed".into())
        }
    }

    pub fn retry_entry(entry: &QueueEntry) -> Result<QueueEntry> {
        if !entry.can_retry() {
            return Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", entry.status),
                to: "Pending".into(),
            });
        }
        Ok(QueueEntry::enqueue(
            entry.session_id.clone(),
            entry.bead_id.clone(),
            entry.priority,
        ))
    }

    pub fn cancel_entry(entry: &QueueEntry) -> Result<QueueEntry> {
        entry.cancel()
    }

    pub fn get_pending_entries(entries: &[QueueEntry]) -> Vec<&QueueEntry> {
        entries
            .iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .collect()
    }

    pub fn get_active_entries(entries: &[QueueEntry]) -> Vec<&QueueEntry> {
        entries
            .iter()
            .filter(|e| crate::domain::state::QueueStateMachine::is_active(e.status))
            .collect()
    }

    pub fn get_terminal_entries(entries: &[QueueEntry]) -> Vec<&QueueEntry> {
        entries
            .iter()
            .filter(|e| crate::domain::state::QueueStateMachine::is_terminal(e.status))
            .collect()
    }

    pub fn sort_by_priority(entries: &mut [QueueEntry]) {
        entries.sort_by(|a, b| {
            b.priority
                .value()
                .cmp(&a.priority.value())
                .then_with(|| a.position.value().cmp(&b.position.value()))
        });
    }

    pub fn find_entry(entries: &[QueueEntry], entry_id: &QueueEntryId) -> Option<&QueueEntry> {
        entries.iter().find(|e| &e.id == entry_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_service_enqueue_creates_pending_entry() {
        let entry = QueueService::enqueue("session-1".into(), None, Priority::default());
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_service_claim_changes_status() {
        let entry = QueueService::enqueue("session-1".into(), None, Priority::default());
        let claimed = QueueService::claim(&entry).unwrap();
        assert_eq!(claimed.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_service_get_pending_filters_correctly() {
        let entry1 = QueueService::enqueue("session-1".into(), None, Priority::default());
        let entry2 = QueueService::enqueue("session-2".into(), None, Priority::default());
        let claimed = QueueService::claim(&entry2).unwrap();
        let entries = vec![entry1, claimed];
        let pending = QueueService::get_pending_entries(&entries);
        assert_eq!(pending.len(), 1);
    }
}

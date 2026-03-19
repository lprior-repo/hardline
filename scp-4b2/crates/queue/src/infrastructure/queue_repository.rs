use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
use crate::domain::value_objects::QueuePosition;
use crate::error::{QueueError, Result};
use std::collections::VecDeque;

pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry>;
    fn dequeue(&self) -> Result<Option<QueueEntry>>;
    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>>;
    fn update(&self, entry: QueueEntry) -> Result<QueueEntry>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>>;
    fn list_all(&self) -> Result<Vec<QueueEntry>>;
    fn remove(&self, id: &QueueEntryId) -> Result<()>;
}

// Pure calculation: determine position for new entry
fn calculate_enqueue_position(current_len: usize) -> QueuePosition {
    QueuePosition::new(current_len)
}

// Pure calculation: create entry with assigned position
fn assign_position_to_entry(entry: QueueEntry, position: QueuePosition) -> QueueEntry {
    QueueEntry { position, ..entry }
}

// Pure calculation: check if entry is pending dequeue
fn is_pending_for_dequeue(entry: &QueueEntry) -> bool {
    entry.status == QueueStatus::Pending
}

// Pure calculation: find index of entry by id
fn find_entry_index_by_id(entries: &[QueueEntry], id: &QueueEntryId) -> Option<usize> {
    entries.iter().position(|e| &e.id == id)
}

pub struct InMemoryQueueRepository {
    entries: VecDeque<QueueEntry>,
}

impl InMemoryQueueRepository {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    fn enqueue_impl(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let position = calculate_enqueue_position(self.entries.len());
        let positioned_entry = assign_position_to_entry(entry, position);
        let mut entries = self.entries.clone();
        entries.push_back(positioned_entry.clone());
        Ok(positioned_entry)
    }

    fn dequeue_impl(&self) -> Result<Option<QueueEntry>> {
        let entries_vec: Vec<QueueEntry> = self.entries.iter().cloned().collect();
        let first_pending_index = entries_vec.iter().position(is_pending_for_dequeue);

        match first_pending_index {
            Some(idx) => {
                let mut entries = self.entries.clone();
                entries.remove(idx);
                let entry = entries_vec.get(idx).cloned();
                Ok(entry)
            }
            None => Ok(None),
        }
    }

    fn get_impl(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        Ok(self.entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update_impl(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let entries_slice: Vec<QueueEntry> = self.entries.iter().cloned().collect();
        find_entry_index_by_id(&entries_slice, &entry.id)
            .map(|pos| {
                let mut entries = self.entries.clone();
                entries[pos] = entry.clone();
                entry
            })
            .ok_or_else(|| QueueError::QueueEntryNotFound(entry.id.as_str().into()))
    }

    fn list_pending_impl(&self) -> Result<Vec<QueueEntry>> {
        Ok(self
            .entries
            .iter()
            .filter(|e| is_pending_for_dequeue(e))
            .cloned()
            .collect())
    }

    fn list_all_impl(&self) -> Result<Vec<QueueEntry>> {
        Ok(self.entries.iter().cloned().collect())
    }

    fn remove_impl(&self, id: &QueueEntryId) -> Result<()> {
        let entries_slice: Vec<QueueEntry> = self.entries.iter().cloned().collect();
        find_entry_index_by_id(&entries_slice, id)
            .map(|pos| {
                let mut entries = self.entries.clone();
                entries.remove(pos);
            })
            .ok_or_else(|| QueueError::QueueEntryNotFound(id.as_str().into()))
    }
}

impl Default for InMemoryQueueRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueRepository for InMemoryQueueRepository {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry> {
        self.enqueue_impl(entry)
    }

    fn dequeue(&self) -> Result<Option<QueueEntry>> {
        self.dequeue_impl()
    }

    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        self.get_impl(id)
    }

    fn update(&self, entry: QueueEntry) -> Result<QueueEntry> {
        self.update_impl(entry)
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        self.list_pending_impl()
    }

    fn list_all(&self) -> Result<Vec<QueueEntry>> {
        self.list_all_impl()
    }

    fn remove(&self, id: &QueueEntryId) -> Result<()> {
        self.remove_impl(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Priority;

    #[test]
    fn in_memory_repo_enqueue_and_dequeue() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        let enqueued = repo.enqueue(entry).expect("enqueue should succeed");
        let dequeued = repo.dequeue().expect("dequeue should succeed");
        assert!(dequeued.is_some());
        assert_eq!(dequeued.map(|e| e.id), Some(enqueued.id));
    }

    #[test]
    fn in_memory_repo_get_returns_entry() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        let enqueued = repo.enqueue(entry).expect("enqueue should succeed");
        let found = repo.get(&enqueued.id).expect("get should succeed");
        assert!(found.is_some());
        assert_eq!(found.map(|e| e.id), Some(enqueued.id));
    }

    #[test]
    fn in_memory_repo_remove_deletes_entry() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        let enqueued = repo.enqueue(entry).expect("enqueue should succeed");
        repo.remove(&enqueued.id).expect("remove should succeed");
        let found = repo.get(&enqueued.id).expect("get should succeed");
        assert!(found.is_none());
    }
}

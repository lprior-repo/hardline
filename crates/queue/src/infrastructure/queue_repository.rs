use crate::domain::entities::{QueueEntry, QueueEntryId};
use crate::domain::queue::status::QueueStatus;
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

pub struct InMemoryQueueRepository {
    entries: VecDeque<QueueEntry>,
}

impl InMemoryQueueRepository {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl Default for InMemoryQueueRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueRepository for InMemoryQueueRepository {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let mut entries = self.entries.clone();
        let position = entries.len();
        let entry = QueueEntry {
            position: crate::domain::value_objects::QueuePosition::new(position),
            ..entry
        };
        entries.push_back(entry.clone());
        Ok(entry)
    }

    fn dequeue(&self) -> Result<Option<QueueEntry>> {
        let mut entries = self.entries.clone();
        if let Some(entry) = entries.pop_front() {
            if entry.status == QueueStatus::Pending {
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>> {
        Ok(self.entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let mut entries = self.entries.clone();
        if let Some(pos) = entries.iter().position(|e| e.id == entry.id) {
            entries[pos] = entry.clone();
            Ok(entry)
        } else {
            Err(QueueError::QueueEntryNotFound(entry.id.as_str().into()))
        }
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        Ok(self
            .entries
            .iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .cloned()
            .collect())
    }

    fn list_all(&self) -> Result<Vec<QueueEntry>> {
        Ok(self.entries.iter().cloned().collect())
    }

    fn remove(&self, id: &QueueEntryId) -> Result<()> {
        let mut entries = self.entries.clone();
        if let Some(pos) = entries.iter().position(|e| &e.id == id) {
            entries.remove(pos);
            Ok(())
        } else {
            Err(QueueError::QueueEntryNotFound(id.as_str().into()))
        }
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
        let enqueued = repo.enqueue(entry).unwrap();
        let dequeued = repo.dequeue().unwrap();
        assert!(dequeued.is_some());
    }

    #[test]
    fn in_memory_repo_get_returns_entry() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        let enqueued = repo.enqueue(entry).unwrap();
        let found = repo.get(&enqueued.id).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn in_memory_repo_remove_deletes_entry() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        let enqueued = repo.enqueue(entry).unwrap();
        repo.remove(&enqueued.id).unwrap();
        let found = repo.get(&enqueued.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn in_memory_repo_default_is_empty() {
        let repo = InMemoryQueueRepository::default();
        let all = repo.list_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn in_memory_repo_dequeue_empty_returns_none() {
        let repo = InMemoryQueueRepository::new();
        let result = repo.dequeue().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn in_memory_repo_list_pending_filters() {
        let repo = InMemoryQueueRepository::new();
        let entry1 = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let entry2 = QueueEntry::enqueue("session-2".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap();
        repo.enqueue(entry1).unwrap();
        repo.enqueue(entry2).unwrap();

        let pending = repo.list_pending().unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn in_memory_repo_update_nonexistent_returns_error() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let result = repo.update(entry);
        assert!(result.is_err());
    }

    #[test]
    fn in_memory_repo_remove_nonexistent_returns_error() {
        let repo = InMemoryQueueRepository::new();
        let id = QueueEntryId::generate();
        let result = repo.remove(&id);
        assert!(result.is_err());
    }

    #[test]
    fn in_memory_repo_get_nonexistent_returns_none() {
        let repo = InMemoryQueueRepository::new();
        let id = QueueEntryId::generate();
        let result = repo.get(&id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn in_memory_repo_dequeue_skips_non_pending() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap();
        repo.enqueue(entry).unwrap();

        let result = repo.dequeue().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn in_memory_repo_list_all_returns_all() {
        let repo = InMemoryQueueRepository::new();
        repo.enqueue(QueueEntry::enqueue("s1".into(), None, Priority::default()).unwrap())
            .unwrap();
        repo.enqueue(QueueEntry::enqueue("s2".into(), None, Priority::default()).unwrap())
            .unwrap();
        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn in_memory_repo_enqueue_returns_entry_with_position() {
        let repo = InMemoryQueueRepository::new();
        let entry = QueueEntry::enqueue("s1".into(), None, Priority::default()).unwrap();
        let returned = repo.enqueue(entry).unwrap();
        assert_eq!(returned.position().value(), 0);
    }
}

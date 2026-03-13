use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
use crate::error::{QueueError, Result};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

/// Port (trait) for queue repository - defines the contract for queue persistence.
/// This belongs in the domain layer for dependency inversion.
pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry>;
    fn dequeue(&self) -> Result<Option<QueueEntry>>;
    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>>;
    fn update(&self, entry: QueueEntry) -> Result<QueueEntry>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>>;
    fn list_all(&self) -> Result<Vec<QueueEntry>>;
    fn remove(&self, id: &QueueEntryId) -> Result<()>;
}

/// In-memory queue repository implementation using RefCell for interior mutability.
/// This allows mutation of the internal state while maintaining the `&self` receiver.
pub struct InMemoryQueueRepository {
    entries: Rc<RefCell<VecDeque<QueueEntry>>>,
}

impl InMemoryQueueRepository {
    pub fn new() -> Self {
        Self {
            entries: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    /// Creates a new repository with the given initial entries (for testing).
    #[cfg(test)]
    pub fn with_entries(entries: VecDeque<QueueEntry>) -> Self {
        Self {
            entries: Rc::new(RefCell::new(entries)),
        }
    }
}

impl Default for InMemoryQueueRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryQueueRepository {
    fn clone(&self) -> Self {
        Self {
            entries: Rc::new(RefCell::new(self.entries.borrow().clone())),
        }
    }
}

impl QueueRepository for InMemoryQueueRepository {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let mut entries = self.entries.borrow_mut();
        let position = entries.len();
        let entry = QueueEntry {
            position: crate::domain::value_objects::QueuePosition::new(position),
            ..entry
        };
        entries.push_back(entry.clone());
        Ok(entry)
    }

    fn dequeue(&self) -> Result<Option<QueueEntry>> {
        let mut entries = self.entries.borrow_mut();
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
        let entries = self.entries.borrow();
        Ok(entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update(&self, entry: QueueEntry) -> Result<QueueEntry> {
        let mut entries = self.entries.borrow_mut();
        if let Some(pos) = entries.iter().position(|e| e.id == entry.id) {
            entries[pos] = entry.clone();
            Ok(entry)
        } else {
            Err(QueueError::QueueEntryNotFound(entry.id.as_str().into()))
        }
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>> {
        let entries = self.entries.borrow();
        Ok(entries
            .iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .cloned()
            .collect())
    }

    fn list_all(&self) -> Result<Vec<QueueEntry>> {
        let entries = self.entries.borrow();
        Ok(entries.iter().cloned().collect())
    }

    fn remove(&self, id: &QueueEntryId) -> Result<()> {
        let mut entries = self.entries.borrow_mut();
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

    /// Helper to create a test entry
    fn create_test_entry(session: &str) -> QueueEntry {
        QueueEntry::enqueue(session.into(), None, Priority::default())
    }

    #[test]
    fn in_memory_repo_enqueue_and_dequeue() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry("session-1");
        
        // Act
        let enqueued_result = repo.enqueue(entry);
        
        // Assert - using proper error handling
        assert!(enqueued_result.is_ok(), "Enqueue should succeed");
        let enqueued = enqueued_result.unwrap();
        
        let dequeued_result = repo.dequeue();
        assert!(dequeued_result.is_ok(), "Dequeue should succeed");
        let dequeued = dequeued_result.unwrap();
        assert!(dequeued.is_some(), "Dequeued entry should exist");
    }

    #[test]
    fn in_memory_repo_get_returns_entry() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry("session-1");
        
        // Act
        let enqueued_result = repo.enqueue(entry);
        assert!(enqueued_result.is_ok(), "Enqueue should succeed");
        let enqueued = enqueued_result.unwrap();
        
        let get_result = repo.get(&enqueued.id);
        
        // Assert
        assert!(get_result.is_ok(), "Get should succeed");
        let found = get_result.unwrap();
        assert!(found.is_some(), "Entry should be found");
    }

    #[test]
    fn in_memory_repo_remove_deletes_entry() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry("session-1");
        
        // Act
        let enqueued_result = repo.enqueue(entry);
        assert!(enqueued_result.is_ok(), "Enqueue should succeed");
        let enqueued = enqueued_result.unwrap();
        
        let remove_result = repo.remove(&enqueued.id);
        assert!(remove_result.is_ok(), "Remove should succeed");
        
        // Assert
        let get_result = repo.get(&enqueued.id);
        assert!(get_result.is_ok(), "Get should succeed");
        let found = get_result.unwrap();
        assert!(found.is_none(), "Entry should be removed");
    }

    #[test]
    fn in_memory_repo_update_modifies_entry() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry("session-1");
        
        // Act
        let enqueued_result = repo.enqueue(entry);
        assert!(enqueued_result.is_ok(), "Enqueue should succeed");
        let mut enqueued = enqueued_result.unwrap();
        
        // Update status
        enqueued.status = QueueStatus::Processing;
        
        let update_result = repo.update(enqueued.clone());
        
        // Assert
        assert!(update_result.is_ok(), "Update should succeed");
        let updated = update_result.unwrap();
        assert_eq!(updated.status, QueueStatus::Processing, "Status should be updated");
        
        // Verify persistence
        let get_result = repo.get(&enqueued.id);
        assert!(get_result.is_ok(), "Get should succeed");
        let found = get_result.unwrap();
        assert!(found.is_some(), "Entry should exist");
        assert_eq!(found.unwrap().status, QueueStatus::Processing, "Status should persist");
    }

    #[test]
    fn in_memory_repo_list_pending_filters_correctly() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        
        // Act - add multiple entries
        let entry1 = create_test_entry("session-1");
        let entry2 = create_test_entry("session-2");
        let entry3 = create_test_entry("session-3");
        
        let _enqueued1 = repo.enqueue(entry1).unwrap();
        let enqueued2 = repo.enqueue(entry2).unwrap();
        let _enqueued3 = repo.enqueue(entry3).unwrap();
        
        // Mark one as processing
        let mut updated = enqueued2;
        updated.status = QueueStatus::Processing;
        repo.update(updated).unwrap();
        
        // Assert
        let pending_result = repo.list_pending();
        assert!(pending_result.is_ok(), "List pending should succeed");
        let pending = pending_result.unwrap();
        assert_eq!(pending.len(), 2, "Should have 2 pending entries");
    }

    #[test]
    fn in_memory_repo_dequeue_empty_queue_returns_none() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        
        // Act
        let dequeued_result = repo.dequeue();
        
        // Assert
        assert!(dequeued_result.is_ok(), "Dequeue should succeed");
        let dequeued = dequeued_result.unwrap();
        assert!(dequeued.is_none(), "Empty queue should return None");
    }

    #[test]
    fn in_memory_repo_get_nonexistent_returns_none() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let fake_id = QueueEntryId::parse("nonexistent-id").unwrap();
        
        // Act
        let get_result = repo.get(&fake_id);
        
        // Assert
        assert!(get_result.is_ok(), "Get should succeed");
        let found = get_result.unwrap();
        assert!(found.is_none(), "Nonexistent entry should return None");
    }

    #[test]
    fn in_memory_repo_clone_creates_independent_copy() {
        // Arrange
        let repo = InMemoryQueueRepository::new();
        let entry = create_test_entry("session-1");
        
        // Act
        repo.enqueue(entry).unwrap();
        let cloned_repo = repo.clone();
        
        // Assert - cloned repo should have its own copy
        let list_result = repo.list_all();
        assert!(list_result.is_ok(), "List should succeed");
        assert_eq!(list_result.unwrap().len(), 1, "Original should have 1 entry");
        
        let cloned_list_result = cloned_repo.list_all();
        assert!(cloned_list_result.is_ok(), "Cloned list should succeed");
        assert_eq!(cloned_list_result.unwrap().len(), 1, "Cloned should have 1 entry");
    }
}

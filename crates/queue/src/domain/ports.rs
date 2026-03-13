#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::domain::queue::{QueueEntry, QueueStatus};
use crate::domain::identifiers::QueueEntryId;
use crate::domain::validation::ValidationError;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Port (trait) for queue repository - defines the contract for queue persistence.
/// This belongs in the domain layer for dependency inversion.
pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError>;
    fn dequeue(&self) -> Result<Option<QueueEntry>, ValidationError>;
    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, ValidationError>;
    fn update(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>, ValidationError>;
    fn list_all(&self) -> Result<Vec<QueueEntry>, ValidationError>;
    fn remove(&self, id: &QueueEntryId) -> Result<(), ValidationError>;
}

/// In-memory queue repository implementation using Mutex for interior mutability.
/// This allows mutation of the internal state while maintaining the `&self` receiver
/// and ensuring thread-safety.
pub struct InMemoryQueueRepository {
    entries: Arc<Mutex<VecDeque<QueueEntry>>>,
}

impl InMemoryQueueRepository {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Creates a new repository with the given initial entries (for testing).
    #[cfg(test)]
    pub fn with_entries(entries: VecDeque<QueueEntry>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(entries)),
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
        // Handle potential mutex poisoning gracefully instead of panicking.
        // If the mutex is poisoned, we start with an empty queue - not ideal
        // but better than panicking in a Clone implementation.
        let cloned_entries = self
            .entries
            .lock()
            .ok()
            .map(|guard| guard.clone())
            .unwrap_or_else(VecDeque::new);
        
        Self {
            entries: Arc::new(Mutex::new(cloned_entries)),
        }
    }
}

impl QueueRepository for InMemoryQueueRepository {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError> {
        let mut entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        entries.push_back(entry.clone());
        Ok(entry)
    }

    fn dequeue(&self) -> Result<Option<QueueEntry>, ValidationError> {
        let mut entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
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

    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, ValidationError> {
        let entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        Ok(entries.iter().find(|e| &e.id == id).cloned())
    }

    fn update(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError> {
        let mut entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        if let Some(pos) = entries.iter().position(|e| e.id == entry.id) {
            entries[pos] = entry.clone();
            Ok(entry)
        } else {
            Err(ValidationError::EmptyValue("entry not found".into()))
        }
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>, ValidationError> {
        let entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        Ok(entries
            .iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .cloned()
            .collect())
    }

    fn list_all(&self) -> Result<Vec<QueueEntry>, ValidationError> {
        let entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        Ok(entries.iter().cloned().collect())
    }

    fn remove(&self, id: &QueueEntryId) -> Result<(), ValidationError> {
        let mut entries = self.entries.lock()
            .map_err(|e| ValidationError::EmptyValue(e.to_string()))?;
        if let Some(pos) = entries.iter().position(|e| &e.id == id) {
            entries.remove(pos);
            Ok(())
        } else {
            Err(ValidationError::EmptyValue("entry not found".into()))
        }
    }
}

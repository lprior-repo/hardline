#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::domain::entities::{QueueEntry, QueueEntryId};
use crate::domain::queue::status::QueueStatus;
<<<<<<< HEAD
use crate::error::QueueError;
=======
use crate::domain::validation::ValidationError;
>>>>>>> polecat/kappa
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Port (trait) for queue repository - defines the contract for queue persistence.
/// This belongs in the domain layer for dependency inversion.
pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry, QueueError>;
    fn dequeue(&self) -> Result<Option<QueueEntry>, QueueError>;
    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, QueueError>;
    fn update(&self, entry: QueueEntry) -> Result<QueueEntry, QueueError>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>, QueueError>;
    fn list_all(&self) -> Result<Vec<QueueEntry>, QueueError>;
    fn remove(&self, id: &QueueEntryId) -> Result<(), QueueError>;
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
            .unwrap_or_default();

        Self {
            entries: Arc::new(Mutex::new(cloned_entries)),
        }
    }
}

impl QueueRepository for InMemoryQueueRepository {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry, QueueError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        entries.push_back(entry.clone());
        Ok(entry)
    }

    fn dequeue(&self) -> Result<Option<QueueEntry>, QueueError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        if let Some(entry) = entries.pop_front() {
            if entry.status() == QueueStatus::Pending {
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, QueueError> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        Ok(entries.iter().find(|e| e.id() == id).cloned())
    }

    fn update(&self, entry: QueueEntry) -> Result<QueueEntry, QueueError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        if let Some(pos) = entries.iter().position(|e| e.id() == entry.id()) {
            entries[pos] = entry.clone();
            Ok(entry)
        } else {
            Err(QueueError::QueueEntryNotFound("entry not found".into()))
        }
    }

    fn list_pending(&self) -> Result<Vec<QueueEntry>, QueueError> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        Ok(entries
            .iter()
            .filter(|e| e.status() == QueueStatus::Pending)
            .cloned()
            .collect())
    }

    fn list_all(&self) -> Result<Vec<QueueEntry>, QueueError> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        Ok(entries.iter().cloned().collect())
    }

    fn remove(&self, id: &QueueEntryId) -> Result<(), QueueError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| QueueError::RepositoryError(e.to_string()))?;
        if let Some(pos) = entries.iter().position(|e| e.id() == id) {
            entries.remove(pos);
            Ok(())
        } else {
            Err(QueueError::QueueEntryNotFound("entry not found".into()))
        }
    }
}

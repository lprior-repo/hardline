//! In-memory queue implementation and QueueManager trait.

use crate::error::Result;
use crate::error_queue::QueueErrorKind;
use crate::lock::LockManager;
use std::sync::Arc;

use super::errors::lock_failed;
use super::types::{Priority, QueueItem, QueueStatus};

/// Queue manager trait
pub trait QueueManager: Send + Sync {
    /// Add item to queue
    fn enqueue(&self, item: QueueItem) -> Result<()>;

    /// Remove and return front item
    fn dequeue(&self) -> Result<Option<QueueItem>>;

    /// Get item by ID
    fn get(&self, id: &str) -> Result<Option<QueueItem>>;

    /// Remove item by ID
    fn remove(&self, id: &str) -> Result<QueueItem>;

    /// List all items
    fn list(&self) -> Result<Vec<QueueItem>>;

    /// List pending items (sorted by priority)
    fn list_pending(&self) -> Result<Vec<QueueItem>>;

    /// Get queue length
    fn len(&self) -> Result<usize>;

    /// Check if queue is empty
    fn is_empty(&self) -> Result<bool>;

    /// Update item status
    fn update(&self, item: QueueItem) -> Result<()>;

    /// Clear completed/failed items
    fn clear_completed(&self) -> Result<usize>;

    /// Insert item at a specific position (overrides priority ordering)
    fn insert_at(&self, position: usize, item: QueueItem) -> Result<()>;
}

/// In-memory queue implementation
pub struct MemQueue {
    items: std::sync::RwLock<Vec<QueueItem>>,
    #[allow(dead_code)]
    lock_manager: Arc<dyn LockManager>,
}

impl std::fmt::Debug for MemQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items = self.items.read().map_err(|_| std::fmt::Error {})?;
        f.debug_struct("MemQueue")
            .field("items", &*items)
            .field("lock_manager", &"<dyn LockManager>")
            .finish()
    }
}

impl MemQueue {
    pub fn new(lock_manager: Arc<dyn LockManager>) -> Self {
        Self {
            items: std::sync::RwLock::new(Vec::new()),
            lock_manager,
        }
    }
}

impl QueueManager for MemQueue {
    fn enqueue(&self, item: QueueItem) -> Result<()> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire lock", e))?;

        let pos = items
            .iter()
            .position(|i| i.priority > item.priority)
            .unwrap_or(items.len());

        let mut item = item;
        item.created_at = chrono::Utc::now();
        item.updated_at = chrono::Utc::now();

        items.insert(pos, item);
        Ok(())
    }

    fn dequeue(&self) -> Result<Option<QueueItem>> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire lock", e))?;

        if let Some(pos) = items.iter().position(|i| i.status == QueueStatus::Pending) {
            let mut item = items.remove(pos);
            item.start_processing();
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    fn get(&self, id: &str) -> Result<Option<QueueItem>> {
        let items = self.items.read().map_err(|e| lock_failed("Failed to acquire lock", e))?;
        Ok(items.iter().find(|i| i.id == id).cloned())
    }

    fn remove(&self, id: &str) -> Result<QueueItem> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire write lock", e))?;
        let pos = items
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| crate::error::Error::from(QueueErrorKind::ItemNotFound(id.to_string())))?;
        Ok(items.remove(pos))
    }

    fn list(&self) -> Result<Vec<QueueItem>> {
        let items = self.items.read().map_err(|e| lock_failed("Failed to acquire lock", e))?;
        Ok(items.clone())
    }

    fn list_pending(&self) -> Result<Vec<QueueItem>> {
        let items = self.items.read().map_err(|e| lock_failed("Failed to acquire lock", e))?;
        let mut pending: Vec<_> = items
            .iter()
            .filter(|i| i.status == QueueStatus::Pending)
            .cloned()
            .collect();
        pending.sort_by_key(|a| a.priority);
        Ok(pending)
    }

    fn len(&self) -> Result<usize> {
        let items = self.items.read().map_err(|e| lock_failed("Failed to acquire lock", e))?;
        Ok(items.len())
    }

    fn is_empty(&self) -> Result<bool> {
        let items = self.items.read().map_err(|e| lock_failed("Failed to acquire lock", e))?;
        Ok(items.is_empty())
    }

    fn update(&self, item: QueueItem) -> Result<()> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire write lock", e))?;
        if let Some(pos) = items.iter().position(|i| i.id == item.id) {
            items[pos] = item;
            Ok(())
        } else {
            Err(QueueErrorKind::ItemNotFound(item.id).into())
        }
    }

    fn clear_completed(&self) -> Result<usize> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire write lock", e))?;
        let len_before = items.len();
        items.retain(|i| {
            i.status != QueueStatus::Completed
                && i.status != QueueStatus::Failed
                && i.status != QueueStatus::Cancelled
        });
        Ok(len_before - items.len())
    }

    fn insert_at(&self, position: usize, mut item: QueueItem) -> Result<()> {
        let mut items = self.items.write().map_err(|e| lock_failed("Failed to acquire write lock", e))?;
        item.created_at = chrono::Utc::now();
        item.updated_at = chrono::Utc::now();
        let pos = position.min(items.len());
        items.insert(pos, item);
        Ok(())
    }
}

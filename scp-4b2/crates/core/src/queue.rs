//! Unified queue management for Source Control Plane.
//!
//! Combines Stak's queue with Isolate workspace support.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use crate::lock::LockManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Priority levels for queue items
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 3,
    Normal = 2,
    High = 1,
    Critical = 0,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Status of a queue item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    /// Item is waiting in queue
    Pending,
    /// Item is being processed
    Processing,
    /// Item completed successfully
    Completed,
    /// Item failed processing
    Failed,
    /// Item was cancelled
    Cancelled,
}

/// Source of queue item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueSource {
    /// From isolate workspace
    Workspace(String),
    /// Directly enqueued
    Direct,
}

/// A queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub branch: String,
    pub source: QueueSource,
    pub priority: Priority,
    pub status: QueueStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempt_count: u32,
    pub last_error: Option<String>,
}

impl QueueItem {
    /// Create a new queue item
    pub fn new(branch: impl Into<String>, source: QueueSource) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            branch: branch.into(),
            source,
            priority: Priority::default(),
            status: QueueStatus::Pending,
            created_at: now,
            updated_at: now,
            attempt_count: 0,
            last_error: None,
        }
    }

    /// Create from workspace
    pub fn from_workspace(workspace: impl Into<String>, branch: impl Into<String>) -> Self {
        Self::new(branch, QueueSource::Workspace(workspace.into()))
    }

    /// Create direct enqueue
    pub fn direct(branch: impl Into<String>) -> Self {
        Self::new(branch, QueueSource::Direct)
    }

    /// Mark as processing
    pub fn start_processing(&mut self) {
        self.status = QueueStatus::Processing;
        self.updated_at = Utc::now();
        self.attempt_count += 1;
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = QueueStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = QueueStatus::Failed;
        self.last_error = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Mark as cancelled
    pub fn cancel(&mut self) {
        self.status = QueueStatus::Cancelled;
        self.updated_at = Utc::now();
    }
}

/// Result of processing a queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub item_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub processed_at: DateTime<Utc>,
}

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
        let mut items = self
            .items
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let pos = items
            .iter()
            .position(|i| i.priority > item.priority)
            .unwrap_or(items.len());

        let mut item = item;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();

        items.insert(pos, item);
        Ok(())
    }

    fn dequeue(&self) -> Result<Option<QueueItem>> {
        let mut items = self
            .items
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        if let Some(pos) = items.iter().position(|i| i.status == QueueStatus::Pending) {
            let mut item = items.remove(pos);
            item.start_processing();
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    fn get(&self, id: &str) -> Result<Option<QueueItem>> {
        let items = self
            .items
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(items.iter().find(|i| i.id == id).cloned())
    }

    fn remove(&self, id: &str) -> Result<QueueItem> {
        let mut items = self
            .items
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let pos = items
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| Error::QueueItemNotFound(id.to_string()))?;
        Ok(items.remove(pos))
    }

    fn list(&self) -> Result<Vec<QueueItem>> {
        let items = self
            .items
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(items.clone())
    }

    fn list_pending(&self) -> Result<Vec<QueueItem>> {
        let items = self
            .items
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut pending: Vec<_> = items
            .iter()
            .filter(|i| i.status == QueueStatus::Pending)
            .cloned()
            .collect();
        pending.sort_by(|a, b| a.priority.cmp(&b.priority));
        Ok(pending)
    }

    fn len(&self) -> Result<usize> {
        let items = self
            .items
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(items.len())
    }

    fn is_empty(&self) -> Result<bool> {
        let items = self
            .items
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(items.is_empty())
    }

    fn update(&self, item: QueueItem) -> Result<()> {
        let mut items = self
            .items
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        if let Some(pos) = items.iter().position(|i| i.id == item.id) {
            items[pos] = item;
            Ok(())
        } else {
            Err(Error::QueueItemNotFound(item.id))
        }
    }

    fn clear_completed(&self) -> Result<usize> {
        let mut items = self
            .items
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let len_before = items.len();
        items.retain(|i| {
            i.status != QueueStatus::Completed
                && i.status != QueueStatus::Failed
                && i.status != QueueStatus::Cancelled
        });
        Ok(len_before - items.len())
    }
}

// UUID generation (simplified - in real code use uuid crate)
mod uuid {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct Uuid([u8; 16]);

    impl Uuid {
        pub fn new_v4() -> Self {
            let mut bytes = [0u8; 16];
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u128)
                .unwrap_or(0);

            if now == 0 {
                return Self(bytes);
            }

            // Simple UUID v4-like generation
            bytes[0..8].copy_from_slice(&(now as u64).to_le_bytes());
            bytes[8..16].copy_from_slice(&(!(now as u64)).to_le_bytes());

            // Set version (4) and variant
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;

            Self(bytes)
        }

        pub fn to_string(&self) -> String {
            format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0[0], self.0[1], self.0[2], self.0[3],
                self.0[4], self.0[5],
                self.0[6], self.0[7],
                self.0[8], self.0[9],
                self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::MemLockManager;

    #[test]
    fn test_queue_enqueue_dequeue() -> Result<()> {
        let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
        let queue = MemQueue::new(lock);

        // Enqueue items
        queue.enqueue(QueueItem::direct("branch-1"))?;
        queue.enqueue(QueueItem::direct("branch-2"))?;

        assert_eq!(queue.len()?, 2);

        // Dequeue
        let item = queue.dequeue()?;
        assert!(item.is_some());
        assert_eq!(item.unwrap().branch, "branch-1");

        Ok(())
    }

    #[test]
    fn test_priority_ordering() -> Result<()> {
        let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
        let queue = MemQueue::new(lock);

        let mut low = QueueItem::direct("low");
        low.priority = Priority::Low;

        let mut high = QueueItem::direct("high");
        high.priority = Priority::High;

        queue.enqueue(low)?;
        queue.enqueue(high)?;

        // High priority should come first
        let item = queue.dequeue()?.unwrap();
        assert_eq!(item.branch, "high");

        Ok(())
    }
}

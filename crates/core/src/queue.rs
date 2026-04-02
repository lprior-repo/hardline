//! Unified queue management for Source Control Plane.
//!
//! Combines Stak's queue with Isolate workspace support.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::Result;
use crate::error_queue::QueueErrorKind;
use crate::lock::LockManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Priority levels for queue items
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    Low = 3,
    #[default]
    Normal = 2,
    High = 1,
    Critical = 0,
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
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;

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
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;

        if let Some(pos) = items.iter().position(|i| i.status == QueueStatus::Pending) {
            let mut item = items.remove(pos);
            item.start_processing();
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    fn get(&self, id: &str) -> Result<Option<QueueItem>> {
        let items = self.items.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(items.iter().find(|i| i.id == id).cloned())
    }

    fn remove(&self, id: &str) -> Result<QueueItem> {
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;
        let pos = items
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| -> crate::error::Error {
                QueueErrorKind::ItemNotFound(id.to_string()).into()
            })?;
        Ok(items.remove(pos))
    }

    fn list(&self) -> Result<Vec<QueueItem>> {
        let items = self.items.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(items.clone())
    }

    fn list_pending(&self) -> Result<Vec<QueueItem>> {
        let items = self.items.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;
        let mut pending: Vec<_> = items
            .iter()
            .filter(|i| i.status == QueueStatus::Pending)
            .cloned()
            .collect();
        pending.sort_by_key(|a| a.priority);
        Ok(pending)
    }

    fn len(&self) -> Result<usize> {
        let items = self.items.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(items.len())
    }

    fn is_empty(&self) -> Result<bool> {
        let items = self.items.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(items.is_empty())
    }

    fn update(&self, item: QueueItem) -> Result<()> {
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;
        if let Some(pos) = items.iter().position(|i| i.id == item.id) {
            items[pos] = item;
            Ok(())
        } else {
            Err(QueueErrorKind::ItemNotFound(item.id).into())
        }
    }

    fn clear_completed(&self) -> Result<usize> {
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;
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
                .map(|d| d.as_nanos())
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
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0[0], self.0[1], self.0[2], self.0[3],
                self.0[4], self.0[5],
                self.0[6], self.0[7],
                self.0[8], self.0[9],
                self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::MemLockManager;

    fn make_queue() -> MemQueue {
        let lock = Arc::new(MemLockManager::new()) as Arc<dyn LockManager>;
        MemQueue::new(lock)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Priority enum tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_priority_ordering() {
        // Critical < High < Normal < Low (lower ordinal = higher priority)
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);

        // Chain ordering
        assert!(Priority::Critical < Priority::Normal);
        assert!(Priority::High < Priority::Low);
        assert!(Priority::Critical < Priority::Low);
    }

    #[test]
    fn test_priority_default_is_normal() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_priority_ord_all_variants() {
        let mut sorted = vec![
            Priority::Low,
            Priority::Critical,
            Priority::High,
            Priority::Normal,
        ];
        sorted.sort();
        assert_eq!(
            sorted,
            vec![
                Priority::Critical,
                Priority::High,
                Priority::Normal,
                Priority::Low
            ]
        );
    }

    #[test]
    fn test_priority_eq() {
        assert_eq!(Priority::Critical, Priority::Critical);
        assert_eq!(Priority::High, Priority::High);
        assert_ne!(Priority::Critical, Priority::High);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QueueStatus enum tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_status_all_variants_distinct() {
        let all = [
            QueueStatus::Pending,
            QueueStatus::Processing,
            QueueStatus::Completed,
            QueueStatus::Failed,
            QueueStatus::Cancelled,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QueueSource enum tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_source_variants() {
        let direct = QueueSource::Direct;
        let workspace = QueueSource::Workspace("my-ws".to_string());
        assert_ne!(direct, workspace);
        match &workspace {
            QueueSource::Workspace(name) => assert_eq!(name, "my-ws"),
            QueueSource::Direct => panic!("expected Workspace variant"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QueueItem tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_item_new() {
        let item = QueueItem::new("main", QueueSource::Direct);
        assert_eq!(item.branch, "main");
        assert_eq!(item.source, QueueSource::Direct);
        assert_eq!(item.priority, Priority::Normal);
        assert_eq!(item.status, QueueStatus::Pending);
        assert_eq!(item.attempt_count, 0);
        assert!(item.last_error.is_none());
        assert!(!item.id.is_empty());
    }

    #[test]
    fn test_queue_item_direct() {
        let item = QueueItem::direct("feature-x");
        assert_eq!(item.branch, "feature-x");
        assert_eq!(item.source, QueueSource::Direct);
    }

    #[test]
    fn test_queue_item_from_workspace() {
        let item = QueueItem::from_workspace("ws-123", "fix-y");
        assert_eq!(item.branch, "fix-y");
        assert_eq!(item.source, QueueSource::Workspace("ws-123".to_string()));
    }

    #[test]
    fn test_queue_item_state_transitions() {
        let mut item = QueueItem::direct("test-branch");

        // start_processing
        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        // complete
        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
        assert!(item.last_error.is_none());
    }

    #[test]
    fn test_queue_item_fail() {
        let mut item = QueueItem::direct("fail-branch");
        item.fail("something broke");
        assert_eq!(item.status, QueueStatus::Failed);
        assert_eq!(item.last_error, Some("something broke".to_string()));
    }

    #[test]
    fn test_queue_item_cancel() {
        let mut item = QueueItem::direct("cancel-branch");
        item.cancel();
        assert_eq!(item.status, QueueStatus::Cancelled);
    }

    #[test]
    fn test_queue_item_timestamps_updated_on_state_change() {
        let mut item = QueueItem::direct("ts-branch");
        let created = item.created_at;

        // Small delay to ensure timestamp differs (not guaranteed but likely)
        item.start_processing();
        assert!(item.updated_at >= created);

        item.complete();
        assert!(item.updated_at >= created);
    }

    #[test]
    fn test_queue_item_attempt_count_increments() {
        let mut item = QueueItem::direct("attempts-branch");
        assert_eq!(item.attempt_count, 0);

        item.start_processing();
        assert_eq!(item.attempt_count, 1);

        item.start_processing();
        assert_eq!(item.attempt_count, 2);

        item.start_processing();
        assert_eq!(item.attempt_count, 3);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ProcessResult tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_process_result_success() {
        let result = ProcessResult {
            item_id: "id-1".to_string(),
            success: true,
            error: None,
            processed_at: Utc::now(),
        };
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.item_id, "id-1");
    }

    #[test]
    fn test_process_result_failure() {
        let result = ProcessResult {
            item_id: "id-2".to_string(),
            success: false,
            error: Some("timeout".to_string()),
            processed_at: Utc::now(),
        };
        assert!(!result.success);
        assert_eq!(result.error, Some("timeout".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MemQueue: enqueue / dequeue
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_enqueue_dequeue() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("branch-1"))?;
        queue.enqueue(QueueItem::direct("branch-2"))?;

        assert_eq!(queue.len()?, 2);

        let item = queue.dequeue()?;
        assert!(item.is_some());
        assert_eq!(item.unwrap().branch, "branch-1");

        Ok(())
    }

    #[test]
    fn test_dequeue_empty_queue() -> Result<()> {
        let queue = make_queue();
        let item = queue.dequeue()?;
        assert!(item.is_none());
        Ok(())
    }

    #[test]
    fn test_dequeue_marks_as_processing() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("branch-x"))?;

        let item = queue.dequeue()?.expect("should dequeue");
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MemQueue: priority ordering
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_priority_ordering_enqueue_dequeue() -> Result<()> {
        let queue = make_queue();

        let mut low = QueueItem::direct("low");
        low.priority = Priority::Low;

        let mut high = QueueItem::direct("high");
        high.priority = Priority::High;

        queue.enqueue(low)?;
        queue.enqueue(high)?;

        // High priority should come first
        let item = queue.dequeue()?.unwrap();
        assert_eq!(item.branch, "high");

        let item = queue.dequeue()?.unwrap();
        assert_eq!(item.branch, "low");

        Ok(())
    }

    #[test]
    fn test_all_priority_levels_in_order() -> Result<()> {
        let queue = make_queue();

        let mut items: Vec<QueueItem> = vec![
            ("low", Priority::Low),
            ("normal", Priority::Normal),
            ("critical", Priority::Critical),
            ("high", Priority::High),
        ]
        .into_iter()
        .map(|(branch, priority)| {
            let mut item = QueueItem::direct(branch);
            item.priority = priority;
            item
        })
        .collect();

        // Enqueue in mixed order
        for item in items.drain(..) {
            queue.enqueue(item)?;
        }

        // Dequeue should yield Critical, High, Normal, Low
        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "critical");

        let second = queue.dequeue()?.unwrap();
        assert_eq!(second.branch, "high");

        let third = queue.dequeue()?.unwrap();
        assert_eq!(third.branch, "normal");

        let fourth = queue.dequeue()?.unwrap();
        assert_eq!(fourth.branch, "low");

        Ok(())
    }

    #[test]
    fn test_critical_priority_inserted_before_high() -> Result<()> {
        let queue = make_queue();

        let mut high = QueueItem::direct("high");
        high.priority = Priority::High;
        queue.enqueue(high)?;

        let mut critical = QueueItem::direct("critical");
        critical.priority = Priority::Critical;
        queue.enqueue(critical)?;

        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "critical");

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MemQueue: get, remove, update, list
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_get_by_id() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("find-me");
        let id = item.id.clone();
        queue.enqueue(item)?;

        let found = queue.get(&id)?.expect("should find item");
        assert_eq!(found.branch, "find-me");

        Ok(())
    }

    #[test]
    fn test_queue_get_missing_id() -> Result<()> {
        let queue = make_queue();
        let found = queue.get("nonexistent")?;
        assert!(found.is_none());
        Ok(())
    }

    #[test]
    fn test_queue_remove_by_id() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("remove-me");
        let id = item.id.clone();
        queue.enqueue(item)?;

        let removed = queue.remove(&id)?;
        assert_eq!(removed.branch, "remove-me");
        assert_eq!(queue.len()?, 0);

        Ok(())
    }

    #[test]
    fn test_queue_remove_missing_id_fails() {
        let queue = make_queue();
        let result = queue.remove("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_queue_list() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("a"))?;
        queue.enqueue(QueueItem::direct("b"))?;
        queue.enqueue(QueueItem::direct("c"))?;

        let all = queue.list()?;
        assert_eq!(all.len(), 3);

        Ok(())
    }

    #[test]
    fn test_queue_list_empty() -> Result<()> {
        let queue = make_queue();
        let all = queue.list()?;
        assert!(all.is_empty());
        Ok(())
    }

    #[test]
    fn test_queue_update() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("update-me");
        let id = item.id.clone();
        queue.enqueue(item)?;

        let mut fetched = queue.get(&id)?.expect("should exist");
        fetched.status = QueueStatus::Completed;
        queue.update(fetched)?;

        let updated = queue.get(&id)?.expect("should still exist");
        assert_eq!(updated.status, QueueStatus::Completed);

        Ok(())
    }

    #[test]
    fn test_queue_update_missing_id_fails() {
        let queue = make_queue();
        let mut item = QueueItem::direct("ghost");
        item.id = "nonexistent-id".to_string();
        let result = queue.update(item);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MemQueue: list_pending, is_empty, clear_completed
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_list_pending() -> Result<()> {
        let queue = make_queue();

        let item1 = QueueItem::direct("pending-1");
        queue.enqueue(item1)?;

        let mut item2 = QueueItem::direct("completed-1");
        item2.status = QueueStatus::Completed;
        queue.enqueue(item2)?;

        let item3 = QueueItem::direct("pending-2");
        queue.enqueue(item3)?;

        let pending = queue.list_pending()?;
        assert_eq!(pending.len(), 2);

        Ok(())
    }

    #[test]
    fn test_queue_list_pending_sorted_by_priority() -> Result<()> {
        let queue = make_queue();

        let mut low = QueueItem::direct("low-pend");
        low.priority = Priority::Low;
        queue.enqueue(low)?;

        let mut crit = QueueItem::direct("crit-pend");
        crit.priority = Priority::Critical;
        queue.enqueue(crit)?;

        let pending = queue.list_pending()?;
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].branch, "crit-pend");
        assert_eq!(pending[1].branch, "low-pend");

        Ok(())
    }

    #[test]
    fn test_queue_is_empty() -> Result<()> {
        let queue = make_queue();
        assert!(queue.is_empty()?);

        queue.enqueue(QueueItem::direct("x"))?;
        assert!(!queue.is_empty()?);

        Ok(())
    }

    #[test]
    fn test_queue_len() -> Result<()> {
        let queue = make_queue();
        assert_eq!(queue.len()?, 0);

        queue.enqueue(QueueItem::direct("a"))?;
        queue.enqueue(QueueItem::direct("b"))?;
        queue.enqueue(QueueItem::direct("c"))?;
        assert_eq!(queue.len()?, 3);

        Ok(())
    }

    #[test]
    fn test_queue_clear_completed() -> Result<()> {
        let queue = make_queue();

        let pending = QueueItem::direct("still-pending");
        queue.enqueue(pending)?;

        let mut completed = QueueItem::direct("done");
        completed.status = QueueStatus::Completed;
        queue.enqueue(completed)?;

        let mut failed = QueueItem::direct("oops");
        failed.status = QueueStatus::Failed;
        queue.enqueue(failed)?;

        let mut cancelled = QueueItem::direct("nvm");
        cancelled.status = QueueStatus::Cancelled;
        queue.enqueue(cancelled)?;

        let removed = queue.clear_completed()?;
        assert_eq!(removed, 3);
        assert_eq!(queue.len()?, 1);

        let remaining = queue.list()?;
        assert_eq!(remaining[0].branch, "still-pending");

        Ok(())
    }

    #[test]
    fn test_queue_clear_completed_empty() -> Result<()> {
        let queue = make_queue();
        let removed = queue.clear_completed()?;
        assert_eq!(removed, 0);
        Ok(())
    }

    #[test]
    fn test_dequeue_skips_non_pending() -> Result<()> {
        let queue = make_queue();

        let mut completed = QueueItem::direct("already-done");
        completed.status = QueueStatus::Completed;
        queue.enqueue(completed)?;

        // Should skip completed items and return None
        let item = queue.dequeue()?;
        assert!(item.is_none());

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_priority_serde_roundtrip_all_variants() {
        for priority in [
            Priority::Critical,
            Priority::High,
            Priority::Normal,
            Priority::Low,
        ] {
            let json = serde_json::to_string(&priority).expect("serialize ok");
            let deserialized: Priority = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(priority, deserialized, "Roundtrip failed for {priority:?}");
        }
    }

    #[test]
    fn test_queue_status_serde_roundtrip_all_variants() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Processing,
            QueueStatus::Completed,
            QueueStatus::Failed,
            QueueStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: QueueStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized, "Roundtrip failed for {status:?}");
        }
    }

    #[test]
    fn test_queue_source_serde_roundtrip_all_variants() {
        let direct = QueueSource::Direct;
        let workspace = QueueSource::Workspace("my-ws".to_string());
        for source in [&direct, &workspace] {
            let json = serde_json::to_string(source).expect("serialize ok");
            let deserialized: QueueSource = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(*source, deserialized);
        }
    }

    #[test]
    fn test_queue_item_serde_roundtrip() {
        let item = QueueItem::new("main", QueueSource::Direct);
        let json = serde_json::to_string(&item).expect("serialize ok");
        let deserialized: QueueItem = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(item.branch, deserialized.branch);
        assert_eq!(item.source, deserialized.source);
        assert_eq!(item.priority, deserialized.priority);
        assert_eq!(item.status, deserialized.status);
        assert!(deserialized.last_error.is_none());
    }

    #[test]
    fn test_queue_item_serde_with_error() {
        let mut item = QueueItem::new("feature", QueueSource::Workspace("ws".to_string()));
        item.last_error = Some("connection timeout".to_string());
        let json = serde_json::to_string(&item).expect("serialize ok");
        let deserialized: QueueItem = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(
            deserialized.last_error,
            Some("connection timeout".to_string())
        );
    }

    #[test]
    fn test_process_result_serde_roundtrip() {
        let result = ProcessResult {
            item_id: "item-1".to_string(),
            success: true,
            error: None,
            processed_at: Utc::now(),
        };
        let json = serde_json::to_string(&result).expect("serialize ok");
        let deserialized: ProcessResult = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(result.item_id, deserialized.item_id);
        assert!(result.success);
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_process_result_serde_with_error() {
        let result = ProcessResult {
            item_id: "item-2".to_string(),
            success: false,
            error: Some("disk full".to_string()),
            processed_at: Utc::now(),
        };
        let json = serde_json::to_string(&result).expect("serialize ok");
        let deserialized: ProcessResult = serde_json::from_str(&json).expect("deserialize ok");
        assert!(!deserialized.success);
        assert_eq!(deserialized.error, Some("disk full".to_string()));
    }
}

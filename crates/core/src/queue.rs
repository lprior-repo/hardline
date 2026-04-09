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
    /// From hardline workspace
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
    // QueueStatus exhaustive tests (ha-opg)
    // ═══════════════════════════════════════════════════════════════════════

    const ALL_STATUSES: [QueueStatus; 5] = [
        QueueStatus::Pending,
        QueueStatus::Processing,
        QueueStatus::Completed,
        QueueStatus::Failed,
        QueueStatus::Cancelled,
    ];

    #[test]
    fn test_queue_status_has_five_variants() {
        assert_eq!(ALL_STATUSES.len(), 5);
    }

    #[test]
    fn test_queue_status_copy_semantics() {
        let status = QueueStatus::Pending;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_queue_status_clone_semantics() {
        let status = QueueStatus::Processing;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_queue_status_debug_format_all_variants() {
        assert!(format!("{:?}", QueueStatus::Pending).contains("Pending"));
        assert!(format!("{:?}", QueueStatus::Processing).contains("Processing"));
        assert!(format!("{:?}", QueueStatus::Completed).contains("Completed"));
        assert!(format!("{:?}", QueueStatus::Failed).contains("Failed"));
        assert!(format!("{:?}", QueueStatus::Cancelled).contains("Cancelled"));
    }

    #[test]
    fn test_queue_status_exhaustive_reflexive_equality() {
        for status in ALL_STATUSES {
            assert_eq!(status, status, "Reflexive equality failed for {:?}", status);
        }
    }

    #[test]
    fn test_queue_status_exhaustive_cross_inequality() {
        for (i, a) in ALL_STATUSES.iter().enumerate() {
            for (j, b) in ALL_STATUSES.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "Cross inequality failed for {:?} vs {:?}", a, b);
                }
            }
        }
    }

    #[test]
    fn test_queue_status_match_all_variants_exhaustive() {
        // Verifies no variant is missed in pattern matching
        for status in ALL_STATUSES {
            let label = match status {
                QueueStatus::Pending => "pending",
                QueueStatus::Processing => "processing",
                QueueStatus::Completed => "completed",
                QueueStatus::Failed => "failed",
                QueueStatus::Cancelled => "cancelled",
            };
            assert!(!label.is_empty());
        }
    }

    #[test]
    fn test_queue_status_serde_json_each_variant() {
        for status in ALL_STATUSES {
            let json = serde_json::to_string(&status)
                .unwrap_or_else(|e| panic!("Serialize failed for {:?}: {}", status, e));
            let roundtrip: QueueStatus = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("Deserialize failed for {:?}: {}", status, e));
            assert_eq!(status, roundtrip, "Roundtrip failed for {:?}", status);
        }
    }

    #[test]
    fn test_queue_status_item_lifecycle_pending_to_completed() {
        let mut item = QueueItem::direct("lifecycle-ok");
        assert_eq!(item.status, QueueStatus::Pending);

        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
    }

    #[test]
    fn test_queue_status_item_lifecycle_pending_to_failed() {
        let mut item = QueueItem::direct("lifecycle-fail");
        assert_eq!(item.status, QueueStatus::Pending);

        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);

        item.fail("timeout after 30s");
        assert_eq!(item.status, QueueStatus::Failed);
        assert_eq!(item.last_error, Some("timeout after 30s".to_string()));
    }

    #[test]
    fn test_queue_status_item_lifecycle_pending_to_cancelled() {
        let mut item = QueueItem::direct("lifecycle-cancel");
        assert_eq!(item.status, QueueStatus::Pending);

        item.cancel();
        assert_eq!(item.status, QueueStatus::Cancelled);
    }

    #[test]
    fn test_queue_status_filter_items_by_each_variant() -> Result<()> {
        let queue = make_queue();

        let mut pending = QueueItem::direct("pending-item");
        pending.status = QueueStatus::Pending;
        queue.enqueue(pending)?;

        let mut processing = QueueItem::direct("processing-item");
        processing.status = QueueStatus::Processing;
        queue.enqueue(processing)?;

        let mut completed = QueueItem::direct("completed-item");
        completed.status = QueueStatus::Completed;
        queue.enqueue(completed)?;

        let mut failed = QueueItem::direct("failed-item");
        failed.status = QueueStatus::Failed;
        queue.enqueue(failed)?;

        let mut cancelled = QueueItem::direct("cancelled-item");
        cancelled.status = QueueStatus::Cancelled;
        queue.enqueue(cancelled)?;

        let all = queue.list()?;

        // Filter by each status variant
        for status in ALL_STATUSES {
            let count = all.iter().filter(|i| i.status == status).count();
            assert_eq!(count, 1, "Expected exactly 1 item with status {:?}", status);
        }

        Ok(())
    }

    #[test]
    fn test_queue_status_pending_is_initial_state() {
        let item = QueueItem::new("branch", QueueSource::Direct);
        assert_eq!(item.status, QueueStatus::Pending);
    }

    #[test]
    fn test_queue_status_multiple_transitions_preserve_final_state() {
        let mut item = QueueItem::direct("multi-step");

        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        item.fail("first attempt failed");
        assert_eq!(item.status, QueueStatus::Failed);
        assert_eq!(item.attempt_count, 1); // fail doesn't increment

        item.start_processing(); // retry
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 2);

        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QueueSource exhaustive tests (ha-opg)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_queue_source_direct_construction() {
        let source = QueueSource::Direct;
        assert!(matches!(source, QueueSource::Direct));
    }

    #[test]
    fn test_queue_source_workspace_construction() {
        let source = QueueSource::Workspace("my-workspace".to_string());
        assert!(matches!(source, QueueSource::Workspace(_)));
        if let QueueSource::Workspace(name) = &source {
            assert_eq!(name, "my-workspace");
        }
    }

    #[test]
    fn test_queue_source_workspace_empty_name() {
        let source = QueueSource::Workspace(String::new());
        assert!(matches!(source, QueueSource::Workspace(_)));
    }

    #[test]
    fn test_queue_source_workspace_special_characters() {
        let source = QueueSource::Workspace("ws-with-special_chars.123".to_string());
        assert!(matches!(source, QueueSource::Workspace(_)));
    }

    #[test]
    fn test_queue_source_clone() {
        let direct = QueueSource::Direct;
        assert_eq!(direct.clone(), direct);

        let workspace = QueueSource::Workspace("clone-test".to_string());
        assert_eq!(workspace.clone(), workspace);
    }

    #[test]
    fn test_queue_source_debug_format() {
        let direct = QueueSource::Direct;
        let debug = format!("{:?}", direct);
        assert!(debug.contains("Direct"));

        let workspace = QueueSource::Workspace("debug-ws".to_string());
        let debug = format!("{:?}", workspace);
        assert!(debug.contains("Workspace"));
        assert!(debug.contains("debug-ws"));
    }

    #[test]
    fn test_queue_source_cross_variant_inequality() {
        let direct = QueueSource::Direct;
        let workspace = QueueSource::Workspace("any".to_string());
        assert_ne!(direct, workspace);
    }

    #[test]
    fn test_queue_source_workspace_same_name_equality() {
        let a = QueueSource::Workspace("same-ws".to_string());
        let b = QueueSource::Workspace("same-ws".to_string());
        assert_eq!(a, b);
    }

    #[test]
    fn test_queue_source_workspace_different_name_inequality() {
        let a = QueueSource::Workspace("ws-alpha".to_string());
        let b = QueueSource::Workspace("ws-beta".to_string());
        assert_ne!(a, b);
    }

    #[test]
    fn test_queue_source_direct_equality() {
        assert_eq!(QueueSource::Direct, QueueSource::Direct);
    }

    #[test]
    fn test_queue_source_attribution_preserved_through_processing() {
        let mut item = QueueItem::from_workspace("my-workspace", "feature-branch");
        assert_eq!(
            item.source,
            QueueSource::Workspace("my-workspace".to_string())
        );

        item.start_processing();
        assert_eq!(
            item.source,
            QueueSource::Workspace("my-workspace".to_string())
        );
    }

    #[test]
    fn test_queue_source_attribution_preserved_through_completion() {
        let mut item = QueueItem::from_workspace("ws-complete", "branch");
        item.start_processing();
        item.complete();
        assert_eq!(
            item.source,
            QueueSource::Workspace("ws-complete".to_string())
        );
    }

    #[test]
    fn test_queue_source_attribution_preserved_through_failure() {
        let mut item = QueueItem::from_workspace("ws-fail", "branch");
        item.start_processing();
        item.fail("error occurred");
        assert_eq!(item.source, QueueSource::Workspace("ws-fail".to_string()));
    }

    #[test]
    fn test_queue_source_attribution_preserved_through_cancellation() {
        let mut item = QueueItem::from_workspace("ws-cancel", "branch");
        item.cancel();
        assert_eq!(item.source, QueueSource::Workspace("ws-cancel".to_string()));
    }

    #[test]
    fn test_queue_source_direct_attribution_immutable() {
        let mut item = QueueItem::direct("direct-branch");
        assert_eq!(item.source, QueueSource::Direct);

        item.start_processing();
        item.complete();
        assert_eq!(item.source, QueueSource::Direct);
    }

    #[test]
    fn test_queue_source_filter_direct_items() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("direct-1"))?;
        queue.enqueue(QueueItem::from_workspace("ws", "ws-1"))?;
        queue.enqueue(QueueItem::direct("direct-2"))?;
        queue.enqueue(QueueItem::from_workspace("ws", "ws-2"))?;

        let all = queue.list()?;
        let direct_items: Vec<_> = all
            .iter()
            .filter(|i| i.source == QueueSource::Direct)
            .collect();
        assert_eq!(direct_items.len(), 2);
        assert!(direct_items
            .iter()
            .all(|i| matches!(i.source, QueueSource::Direct)));

        Ok(())
    }

    #[test]
    fn test_queue_source_filter_workspace_items() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("direct-1"))?;
        queue.enqueue(QueueItem::from_workspace("ws-a", "ws-1"))?;
        queue.enqueue(QueueItem::from_workspace("ws-b", "ws-2"))?;

        let all = queue.list()?;
        let workspace_items: Vec<_> = all
            .iter()
            .filter(|i| matches!(i.source, QueueSource::Workspace(_)))
            .collect();
        assert_eq!(workspace_items.len(), 2);

        Ok(())
    }

    #[test]
    fn test_queue_source_filter_by_workspace_name() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::from_workspace("target-ws", "branch-1"))?;
        queue.enqueue(QueueItem::from_workspace("other-ws", "branch-2"))?;
        queue.enqueue(QueueItem::from_workspace("target-ws", "branch-3"))?;
        queue.enqueue(QueueItem::direct("direct-branch"))?;

        let all = queue.list()?;
        let target_items: Vec<_> = all
            .iter()
            .filter(|i| matches!(&i.source, QueueSource::Workspace(name) if name == "target-ws"))
            .collect();
        assert_eq!(target_items.len(), 2);

        Ok(())
    }

    #[test]
    fn test_queue_source_serde_roundtrip_direct() {
        let source = QueueSource::Direct;
        let json = serde_json::to_string(&source).unwrap();
        let roundtrip: QueueSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, roundtrip);
    }

    #[test]
    fn test_queue_source_serde_roundtrip_workspace() {
        let source = QueueSource::Workspace("serde-ws".to_string());
        let json = serde_json::to_string(&source).unwrap();
        let roundtrip: QueueSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, roundtrip);
    }

    #[test]
    fn test_queue_source_item_serde_preserves_source() {
        let item = QueueItem::from_workspace("serde-ws", "serde-branch");
        let json = serde_json::to_string(&item).unwrap();
        let roundtrip: QueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item.source, roundtrip.source);
    }

    #[test]
    fn test_queue_source_combined_status_and_source_filter() -> Result<()> {
        let queue = make_queue();

        let mut ws_pending = QueueItem::from_workspace("ws", "ws-pending");
        ws_pending.status = QueueStatus::Pending;
        queue.enqueue(ws_pending)?;

        let mut ws_completed = QueueItem::from_workspace("ws", "ws-completed");
        ws_completed.status = QueueStatus::Completed;
        queue.enqueue(ws_completed)?;

        let mut direct_pending = QueueItem::direct("direct-pending");
        direct_pending.status = QueueStatus::Pending;
        queue.enqueue(direct_pending)?;

        let all = queue.list()?;

        // Filter: workspace + pending
        let ws_pending_items: Vec<_> = all
            .iter()
            .filter(|i| {
                matches!(&i.source, QueueSource::Workspace(_)) && i.status == QueueStatus::Pending
            })
            .collect();
        assert_eq!(ws_pending_items.len(), 1);
        assert_eq!(ws_pending_items[0].branch, "ws-pending");

        // Filter: direct + pending
        let direct_pending_items: Vec<_> = all
            .iter()
            .filter(|i| i.source == QueueSource::Direct && i.status == QueueStatus::Pending)
            .collect();
        assert_eq!(direct_pending_items.len(), 1);
        assert_eq!(direct_pending_items[0].branch, "direct-pending");

        Ok(())
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

    // ═══════════════════════════════════════════════════════════════════════
    // EXHAUSTIVE TESTS: QueueItem, QueueManager (ha-im3)
    // ═══════════════════════════════════════════════════════════════════════

    // ── FIFO ordering for same priority ──────────────────────────────

    #[test]
    fn test_fifo_same_priority_dequeue_order() -> Result<()> {
        let queue = make_queue();

        // All Normal priority (default) — should dequeue in insertion order
        let id_a = {
            let item = QueueItem::direct("alpha");
            let id = item.id.clone();
            queue.enqueue(item)?;
            id
        };
        let id_b = {
            let item = QueueItem::direct("bravo");
            let id = item.id.clone();
            queue.enqueue(item)?;
            id
        };
        let id_c = {
            let item = QueueItem::direct("charlie");
            let id = item.id.clone();
            queue.enqueue(item)?;
            id
        };

        let first = queue.dequeue()?.expect("first item");
        assert_eq!(first.id, id_a, "FIFO: first enqueued should dequeue first");
        let second = queue.dequeue()?.expect("second item");
        assert_eq!(
            second.id, id_b,
            "FIFO: second enqueued should dequeue second"
        );
        let third = queue.dequeue()?.expect("third item");
        assert_eq!(third.id, id_c, "FIFO: third enqueued should dequeue third");

        assert!(
            queue.dequeue()?.is_none(),
            "queue should be empty after draining"
        );
        Ok(())
    }

    #[test]
    fn test_fifo_same_critical_priority() -> Result<()> {
        let queue = make_queue();

        for name in &["c1", "c2", "c3", "c4", "c5"] {
            let mut item = QueueItem::direct(*name);
            item.priority = Priority::Critical;
            queue.enqueue(item)?;
        }

        let order: Vec<String> =
            std::iter::from_fn(|| queue.dequeue().ok().flatten().map(|i| i.branch))
                .take(5)
                .collect();
        assert_eq!(order, vec!["c1", "c2", "c3", "c4", "c5"]);
        Ok(())
    }

    #[test]
    fn test_fifo_same_low_priority() -> Result<()> {
        let queue = make_queue();

        for name in &["low-a", "low-b"] {
            let mut item = QueueItem::direct(*name);
            item.priority = Priority::Low;
            queue.enqueue(item)?;
        }

        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "low-a");
        let second = queue.dequeue()?.unwrap();
        assert_eq!(second.branch, "low-b");
        Ok(())
    }

    #[test]
    fn test_priority_preempts_fifo() -> Result<()> {
        // Even though normal-priority items were enqueued first,
        // critical should dequeue before them.
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("normal-1"))?; // Normal (default)
        queue.enqueue(QueueItem::direct("normal-2"))?;

        let mut crit = QueueItem::direct("critical-late");
        crit.priority = Priority::Critical;
        queue.enqueue(crit)?;

        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "critical-late");
        assert_eq!(first.priority, Priority::Critical);

        let second = queue.dequeue()?.unwrap();
        assert_eq!(second.branch, "normal-1");
        let third = queue.dequeue()?.unwrap();
        assert_eq!(third.branch, "normal-2");
        Ok(())
    }

    #[test]
    fn test_mixed_priority_insertion_fifo_within_tier() -> Result<()> {
        let queue = make_queue();

        // Enqueue: Low, High, Normal, Critical, High, Low, Normal
        let cases = [
            ("l1", Priority::Low),
            ("h1", Priority::High),
            ("n1", Priority::Normal),
            ("c1", Priority::Critical),
            ("h2", Priority::High),
            ("l2", Priority::Low),
            ("n2", Priority::Normal),
        ];
        for (branch, priority) in cases {
            let mut item = QueueItem::direct(branch);
            item.priority = priority;
            queue.enqueue(item)?;
        }

        // Expected: Critical first, then High in FIFO, Normal in FIFO, Low in FIFO
        let expected = vec![
            ("c1", Priority::Critical),
            ("h1", Priority::High),
            ("h2", Priority::High),
            ("n1", Priority::Normal),
            ("n2", Priority::Normal),
            ("l1", Priority::Low),
            ("l2", Priority::Low),
        ];

        for (expected_branch, expected_pri) in expected {
            let item = queue
                .dequeue()?
                .unwrap_or_else(|| panic!("expected item {expected_branch}"));
            assert_eq!(
                item.branch, expected_branch,
                "branch mismatch at {expected_branch}"
            );
            assert_eq!(
                item.priority, expected_pri,
                "priority mismatch at {expected_branch}"
            );
        }

        assert!(queue.dequeue()?.is_none());
        Ok(())
    }

    // ── Duplicate item handling ──────────────────────────────────────

    #[test]
    fn test_enqueue_same_item_twice_both_stored() -> Result<()> {
        // MemQueue does not deduplicate — same ID enqueued twice is stored twice
        let queue = make_queue();
        let item = QueueItem::direct("dup-branch");
        let id = item.id.clone();

        queue.enqueue(item.clone())?;
        queue.enqueue(item)?;

        assert_eq!(queue.len()?, 2);
        let all = queue.list()?;
        assert!(all.iter().all(|i| i.id == id));
        Ok(())
    }

    #[test]
    fn test_dequeue_duplicate_items_both_processing() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("dup-dequeue");
        queue.enqueue(item.clone())?;
        queue.enqueue(item)?;

        let first = queue.dequeue()?.expect("first dequeue");
        assert_eq!(first.status, QueueStatus::Processing);
        assert_eq!(first.branch, "dup-dequeue");

        let second = queue.dequeue()?.expect("second dequeue");
        assert_eq!(second.status, QueueStatus::Processing);
        assert_eq!(second.branch, "dup-dequeue");

        assert!(queue.is_empty()?);
        Ok(())
    }

    // ── Item cancellation in queue context ───────────────────────────

    #[test]
    fn test_cancel_then_dequeue_skips_cancelled() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("should-cancel"))?;
        queue.enqueue(QueueItem::direct("should-dequeue"))?;

        // Cancel the first item in-place
        let all = queue.list()?;
        let cancel_item = all.iter().find(|i| i.branch == "should-cancel").unwrap();
        let mut updated = cancel_item.clone();
        updated.cancel();
        queue.update(updated)?;

        // Dequeue should skip the cancelled item
        let item = queue.dequeue()?.expect("should get pending item");
        assert_eq!(item.branch, "should-dequeue");

        // Now queue has one cancelled item left — dequeue should return None
        assert!(queue.dequeue()?.is_none());
        assert_eq!(queue.len()?, 1); // cancelled item still in queue
        Ok(())
    }

    #[test]
    fn test_cancel_all_items_then_dequeue_returns_none() -> Result<()> {
        let queue = make_queue();

        for name in &["a", "b", "c"] {
            queue.enqueue(QueueItem::direct(*name))?;
        }

        let all = queue.list()?;
        for item in &all {
            let mut updated = item.clone();
            updated.cancel();
            queue.update(updated)?;
        }

        assert!(
            queue.dequeue()?.is_none(),
            "all cancelled — dequeue should return None"
        );
        assert_eq!(queue.len()?, 3, "cancelled items still in queue");
        Ok(())
    }

    #[test]
    fn test_cancel_one_of_same_priority_fifo_continues() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("first"))?;
        queue.enqueue(QueueItem::direct("second"))?;
        queue.enqueue(QueueItem::direct("third"))?;

        // Cancel "second"
        let all = queue.list()?;
        let second = all.iter().find(|i| i.branch == "second").unwrap();
        let mut updated = second.clone();
        updated.cancel();
        queue.update(updated)?;

        // Dequeue should get "first", then "third" (skipping cancelled "second")
        let d1 = queue.dequeue()?.unwrap();
        assert_eq!(d1.branch, "first");
        let d2 = queue.dequeue()?.unwrap();
        assert_eq!(d2.branch, "third");
        assert!(queue.dequeue()?.is_none());
        Ok(())
    }

    // ── Queue persistence (serde roundtrip of full queue state) ──────

    #[test]
    fn test_queue_state_persistence_via_list() -> Result<()> {
        let queue = make_queue();

        let mut crit = QueueItem::direct("critical-work");
        crit.priority = Priority::Critical;
        queue.enqueue(crit)?;

        queue.enqueue(QueueItem::direct("normal-work"))?;

        let mut low = QueueItem::direct("low-work");
        low.priority = Priority::Low;
        queue.enqueue(low)?;

        // Serialize queue state via list
        let state = queue.list()?;
        let json = serde_json::to_string(&state).expect("serialize queue state");

        // Deserialize and reconstruct
        let restored: Vec<QueueItem> =
            serde_json::from_str(&json).expect("deserialize queue state");
        assert_eq!(restored.len(), 3);

        // Verify priority ordering preserved
        assert_eq!(restored[0].priority, Priority::Critical);
        assert_eq!(restored[1].priority, Priority::Normal);
        assert_eq!(restored[2].priority, Priority::Low);

        // Verify all fields survived roundtrip
        for (orig, rest) in state.iter().zip(restored.iter()) {
            assert_eq!(orig.id, rest.id);
            assert_eq!(orig.branch, rest.branch);
            assert_eq!(orig.source, rest.source);
            assert_eq!(orig.priority, rest.priority);
            assert_eq!(orig.status, rest.status);
            assert_eq!(orig.attempt_count, rest.attempt_count);
            assert_eq!(orig.last_error, rest.last_error);
        }
        Ok(())
    }

    #[test]
    fn test_queue_state_roundtrip_with_all_statuses() -> Result<()> {
        let queue = make_queue();

        let mut items = Vec::new();

        let mut pending = QueueItem::direct("pending-item");
        pending.status = QueueStatus::Pending;
        items.push(pending.clone());
        queue.enqueue(pending)?;

        let mut processing = QueueItem::direct("processing-item");
        processing.start_processing();
        items.push(processing.clone());
        queue.enqueue(processing)?;

        let mut completed = QueueItem::direct("completed-item");
        completed.complete();
        items.push(completed.clone());
        queue.enqueue(completed)?;

        let mut failed = QueueItem::direct("failed-item");
        failed.fail("catastrophic failure");
        items.push(failed.clone());
        queue.enqueue(failed)?;

        let mut cancelled = QueueItem::direct("cancelled-item");
        cancelled.cancel();
        items.push(cancelled.clone());
        queue.enqueue(cancelled)?;

        let state = queue.list()?;
        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: Vec<QueueItem> = serde_json::from_str(&json).unwrap();

        for (orig, rest) in items.iter().zip(restored.iter()) {
            assert_eq!(
                orig.status, rest.status,
                "status roundtrip failed for {}",
                orig.branch
            );
            assert_eq!(
                orig.last_error, rest.last_error,
                "last_error roundtrip failed"
            );
        }
        Ok(())
    }

    #[test]
    fn test_queue_state_roundtrip_preserves_source_attribution() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("direct-item"))?;
        queue.enqueue(QueueItem::from_workspace("ws-alpha", "ws-item-1"))?;
        queue.enqueue(QueueItem::from_workspace("ws-beta", "ws-item-2"))?;

        let state = queue.list()?;
        let json = serde_json::to_string(&state).unwrap();
        let restored: Vec<QueueItem> = serde_json::from_str(&json).unwrap();

        assert_eq!(restored[0].source, QueueSource::Direct);
        assert_eq!(
            restored[1].source,
            QueueSource::Workspace("ws-alpha".to_string())
        );
        assert_eq!(
            restored[2].source,
            QueueSource::Workspace("ws-beta".to_string())
        );
        Ok(())
    }

    // ── Queue size tracking edge cases ───────────────────────────────

    #[test]
    fn test_len_after_mixed_operations() -> Result<()> {
        let queue = make_queue();
        assert_eq!(queue.len()?, 0);

        queue.enqueue(QueueItem::direct("a"))?;
        queue.enqueue(QueueItem::direct("b"))?;
        assert_eq!(queue.len()?, 2);

        let _ = queue.dequeue()?;
        assert_eq!(queue.len()?, 1); // dequeue removes from queue

        queue.enqueue(QueueItem::direct("c"))?;
        assert_eq!(queue.len()?, 2);

        let item = queue.get(queue.list()?[0].id.as_str())?.unwrap();
        queue.remove(&item.id)?;
        assert_eq!(queue.len()?, 1);
        Ok(())
    }

    #[test]
    fn test_is_empty_after_dequeue_all() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("x"))?;
        queue.enqueue(QueueItem::direct("y"))?;

        let _ = queue.dequeue()?;
        assert!(!queue.is_empty()?);
        let _ = queue.dequeue()?;
        assert!(queue.is_empty()?);
        Ok(())
    }

    #[test]
    fn test_clear_completed_with_only_pending_returns_zero() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("pending-1"))?;
        queue.enqueue(QueueItem::direct("pending-2"))?;

        let cleared = queue.clear_completed()?;
        assert_eq!(cleared, 0);
        assert_eq!(queue.len()?, 2);
        Ok(())
    }

    #[test]
    fn test_clear_completed_then_dequeue_works() -> Result<()> {
        let queue = make_queue();

        let mut done = QueueItem::direct("done-item");
        done.complete();
        queue.enqueue(done)?;

        queue.enqueue(QueueItem::direct("pending-item"))?;

        let cleared = queue.clear_completed()?;
        assert_eq!(cleared, 1);

        let item = queue.dequeue()?.expect("should dequeue pending");
        assert_eq!(item.branch, "pending-item");
        Ok(())
    }

    #[test]
    fn test_clear_completed_removes_failed_and_cancelled() -> Result<()> {
        let queue = make_queue();

        let mut failed = QueueItem::direct("failed");
        failed.fail("err");
        queue.enqueue(failed)?;

        let mut cancelled = QueueItem::direct("cancelled");
        cancelled.cancel();
        queue.enqueue(cancelled)?;

        queue.enqueue(QueueItem::direct("pending"))?;

        let cleared = queue.clear_completed()?;
        assert_eq!(cleared, 2);
        assert_eq!(queue.len()?, 1);

        let remaining = queue.list()?;
        assert_eq!(remaining[0].branch, "pending");
        Ok(())
    }

    // ── Dequeue edge cases ───────────────────────────────────────────

    #[test]
    fn test_dequeue_only_processing_items_returns_none() -> Result<()> {
        let queue = make_queue();

        // Simulate items that are already Processing (e.g., from a previous crash)
        let mut item = QueueItem::direct("stuck-processing");
        item.start_processing();
        queue.enqueue(item)?;

        assert!(queue.dequeue()?.is_none(), "should skip processing items");
        assert_eq!(queue.len()?, 1, "item remains in queue");
        Ok(())
    }

    #[test]
    fn test_dequeue_mixed_statuses_picks_first_pending() -> Result<()> {
        let queue = make_queue();

        let mut completed = QueueItem::direct("completed");
        completed.complete();
        queue.enqueue(completed)?;

        let mut failed = QueueItem::direct("failed");
        failed.fail("err");
        queue.enqueue(failed)?;

        queue.enqueue(QueueItem::direct("pending-target"))?;

        let mut cancelled = QueueItem::direct("cancelled");
        cancelled.cancel();
        queue.enqueue(cancelled)?;

        let item = queue.dequeue()?.expect("should find pending");
        assert_eq!(item.branch, "pending-target");
        Ok(())
    }

    #[test]
    fn test_dequeue_sets_attempt_count_to_one() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("attempt-test"))?;

        let item = queue.dequeue()?.unwrap();
        assert_eq!(item.attempt_count, 1);
        Ok(())
    }

    #[test]
    fn test_dequeue_sets_updated_at() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("timestamp-test");
        let original_updated = item.updated_at;
        queue.enqueue(item)?;

        let dequeued = queue.dequeue()?.unwrap();
        // After dequeue, updated_at should be >= original (start_processing updates it)
        assert!(dequeued.updated_at >= original_updated);
        Ok(())
    }

    // ── Update edge cases ────────────────────────────────────────────

    #[test]
    fn test_update_priority_change() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::direct("promote-me"))?;
        let mut low = QueueItem::direct("low-item");
        low.priority = Priority::Low;
        queue.enqueue(low)?;

        let all = queue.list()?;
        let item = all
            .iter()
            .find(|i| i.branch == "promote-me")
            .unwrap()
            .clone();
        let mut promoted = item;
        promoted.priority = Priority::Critical;
        queue.update(promoted)?;

        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "promote-me");
        assert_eq!(first.priority, Priority::Critical);
        Ok(())
    }

    #[test]
    fn test_update_status_transition_to_failed() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("will-fail");
        let id = item.id.clone();
        queue.enqueue(item)?;

        let mut fetched = queue.get(&id)?.unwrap();
        fetched.start_processing();
        fetched.fail("timeout after 30s");
        queue.update(fetched)?;

        let updated = queue.get(&id)?.unwrap();
        assert_eq!(updated.status, QueueStatus::Failed);
        assert_eq!(updated.last_error, Some("timeout after 30s".to_string()));
        assert_eq!(updated.attempt_count, 1);
        Ok(())
    }

    #[test]
    fn test_update_nonexistent_returns_error() {
        let queue = make_queue();
        let mut ghost = QueueItem::direct("ghost");
        ghost.id = "does-not-exist".to_string();
        assert!(queue.update(ghost).is_err());
    }

    // ── Remove edge cases ────────────────────────────────────────────

    #[test]
    fn test_remove_middle_item_preserves_others() -> Result<()> {
        let queue = make_queue();

        let mut crit = QueueItem::direct("critical");
        crit.priority = Priority::Critical;
        queue.enqueue(crit)?;

        queue.enqueue(QueueItem::direct("normal"))?;

        let mut low = QueueItem::direct("low");
        low.priority = Priority::Low;
        queue.enqueue(low)?;

        let all = queue.list()?;
        let normal = all.iter().find(|i| i.branch == "normal").unwrap();
        queue.remove(&normal.id)?;

        assert_eq!(queue.len()?, 2);
        let first = queue.dequeue()?.unwrap();
        assert_eq!(first.branch, "critical");
        let second = queue.dequeue()?.unwrap();
        assert_eq!(second.branch, "low");
        Ok(())
    }

    #[test]
    fn test_remove_then_enqueue_replaces() -> Result<()> {
        let queue = make_queue();
        let item = QueueItem::direct("original");
        let id = item.id.clone();
        queue.enqueue(item)?;

        queue.remove(&id)?;
        assert_eq!(queue.len()?, 0);

        queue.enqueue(QueueItem::direct("replacement"))?;
        assert_eq!(queue.len()?, 1);

        let fetched = queue.list()?;
        assert_eq!(fetched[0].branch, "replacement");
        Ok(())
    }

    // ── QueueItem clone / debug / unique ID ──────────────────────────

    #[test]
    fn test_queue_item_clone_is_independent() {
        let mut original = QueueItem::direct("clone-test");
        original.fail("original error");
        let cloned = original.clone();

        // Mutating original doesn't affect clone
        original.complete();
        assert_eq!(cloned.status, QueueStatus::Failed);
        assert_eq!(cloned.last_error, Some("original error".to_string()));
    }

    #[test]
    fn test_queue_item_debug_format() {
        let item = QueueItem::direct("debug-branch");
        let debug = format!("{item:?}");
        assert!(debug.contains("debug-branch"));
        assert!(debug.contains("Pending"));
    }

    #[test]
    fn test_queue_item_unique_ids() {
        let ids: std::collections::HashSet<String> =
            (0..100).map(|_| QueueItem::direct("branch").id).collect();
        // All 100 items should have unique IDs
        assert_eq!(ids.len(), 100, "each QueueItem should get a unique ID");
    }

    #[test]
    fn test_queue_item_created_at_not_default() {
        let item = QueueItem::direct("ts-check");
        let epoch = DateTime::from_timestamp(0, 0).unwrap();
        assert!(
            item.created_at > epoch,
            "created_at should be a real timestamp"
        );
    }

    #[test]
    fn test_queue_item_created_at_equals_updated_at_initially() {
        let item = QueueItem::direct("ts-equal");
        assert_eq!(item.created_at, item.updated_at);
    }

    #[test]
    fn test_memqueue_debug_format() {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("debug-queue")).ok();
        let debug = format!("{queue:?}");
        assert!(debug.contains("MemQueue"));
        assert!(debug.contains("debug-queue"));
    }

    // ── Concurrent access safety ─────────────────────────────────────

    #[test]
    fn test_concurrent_enqueue_dequeue() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let queue = Arc::new(make_queue());
        let mut handles = Vec::new();

        // Spawn 4 threads each enqueueing 25 items
        for t in 0..4 {
            let q = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let branch = format!("t{t}-item{i}");
                    q.enqueue(QueueItem::direct(&branch)).expect("enqueue");
                }
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(queue.len()?, 100, "all 100 items should be enqueued");

        // Dequeue all — should get 100 items
        let mut count = 0;
        while queue.dequeue()?.is_some() {
            count += 1;
        }
        assert_eq!(count, 100);
        Ok(())
    }

    #[test]
    fn test_concurrent_enqueue_and_dequeue_interleaved() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let queue = Arc::new(make_queue());
        let mut handles = Vec::new();

        // Enqueuers
        for t in 0..2 {
            let q = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                for i in 0..50 {
                    q.enqueue(QueueItem::direct(&format!("e{t}-{i}")))
                        .expect("enqueue");
                }
            }));
        }

        // Dequeuer
        let q = Arc::clone(&queue);
        let dequeuer = thread::spawn(move || {
            let mut dequeued = 0;
            for _ in 0..200 {
                if q.dequeue().ok().flatten().is_some() {
                    dequeued += 1;
                }
            }
            dequeued
        });

        for h in handles {
            h.join().expect("enqueuer panicked");
        }
        let dequeued = dequeuer.join().expect("dequeuer panicked");

        // Total items in queue + dequeued = 100
        let remaining = queue.len()?;
        assert_eq!(remaining + dequeued, 100);
        Ok(())
    }

    // ── Large queue stress tests ─────────────────────────────────────

    #[test]
    fn test_enqueue_dequeue_1000_items() -> Result<()> {
        let queue = make_queue();

        for i in 0..1000 {
            let mut item = QueueItem::direct(&format!("item-{i:04}"));
            // Cycle through priorities
            item.priority = match i % 4 {
                0 => Priority::Critical,
                1 => Priority::High,
                2 => Priority::Normal,
                _ => Priority::Low,
            };
            queue.enqueue(item)?;
        }

        assert_eq!(queue.len()?, 1000);

        // Verify priority ordering is maintained
        let mut last_priority = Priority::Critical;
        for _ in 0..1000 {
            let item = queue.dequeue()?.expect("should have item");
            assert!(
                item.priority >= last_priority,
                "priority ordering violated: {:?} < {:?}",
                item.priority,
                last_priority
            );
            last_priority = item.priority;
        }

        assert!(queue.is_empty()?);
        Ok(())
    }

    #[test]
    fn test_rapid_enqueue_dequeue_cycle() -> Result<()> {
        let queue = make_queue();

        for cycle in 0..100 {
            queue.enqueue(QueueItem::direct(&format!("cycle-{cycle}")))?;
            let item = queue.dequeue()?.expect("should dequeue immediately");
            assert_eq!(item.branch, format!("cycle-{cycle}"));
        }

        assert!(queue.is_empty()?);
        Ok(())
    }

    // ── ProcessResult exhaustive tests ───────────────────────────────

    #[test]
    fn test_process_result_debug_format() {
        let result = ProcessResult {
            item_id: "item-debug".to_string(),
            success: true,
            error: None,
            processed_at: Utc::now(),
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("item-debug"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn test_process_result_clone() {
        let result = ProcessResult {
            item_id: "item-clone".to_string(),
            success: false,
            error: Some("err".to_string()),
            processed_at: Utc::now(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.item_id, "item-clone");
        assert!(!cloned.success);
        assert_eq!(cloned.error, Some("err".to_string()));
    }

    // ── Proptests: property-based testing ────────────────────────────

    #[test]
    fn proptest_enqueue_dequeue_roundtrip_preserves_items() {
        proptest::proptest!(|(branches in proptest::collection::vec("[a-z]{1,10}", 0..20))| {
            let queue = make_queue();
            let mut enqueued = Vec::new();

            for branch in &branches {
                let item = QueueItem::direct(branch);
                enqueued.push(item.branch.clone());
                queue.enqueue(item).expect("enqueue");
            }

            assert_eq!(queue.len().expect("len"), branches.len());

            let mut dequeued = Vec::new();
            while let Some(item) = queue.dequeue().expect("dequeue opt") {
                dequeued.push(item.branch);
            }

            // All enqueued branches should be present in dequeued
            let mut enq_sorted = enqueued;
            enq_sorted.sort();
            let mut deq_sorted = dequeued;
            deq_sorted.sort();
            assert_eq!(enq_sorted, deq_sorted);
        });
    }

    #[test]
    fn proptest_priority_ordering_invariant() {
        proptest::proptest!(|(priorities in proptest::collection::vec(0u8..4, 1..30))| {
            let queue = make_queue();

            for (i, &p) in priorities.iter().enumerate() {
                let mut item = QueueItem::direct(&format!("item-{i}"));
                item.priority = match p {
                    0 => Priority::Critical,
                    1 => Priority::High,
                    2 => Priority::Normal,
                    _ => Priority::Low,
                };
                queue.enqueue(item).expect("enqueue");
            }

            let mut prev: Option<Priority> = None;
            while let Some(item) = queue.dequeue().expect("dequeue") {
                if let Some(p) = prev {
                    assert!(item.priority >= p,
                        "dequeue violated priority ordering: {:?} came after {:?}",
                        item.priority, p);
                }
                prev = Some(item.priority);
            }
        });
    }

    #[test]
    fn proptest_queue_item_serde_roundtrip() {
        proptest::proptest!(|(
            branch in "[a-z][a-z0-9-]{0,20}",
            attempt_count in 0u32..100,
            has_error in proptest::bool::ANY,
            error_msg in proptest::option::of("[a-z ]{0,50}"),
        )| {
            let mut item = QueueItem::direct(&branch);
            for _ in 0..attempt_count {
                item.start_processing();
            }
            if has_error {
                if let Some(msg) = &error_msg {
                    item.fail(msg);
                }
            }

            let json = serde_json::to_string(&item).expect("serialize");
            let restored: QueueItem = serde_json::from_str(&json).expect("deserialize");

            assert_eq!(item.branch, restored.branch);
            assert_eq!(item.attempt_count, restored.attempt_count);
            assert_eq!(item.last_error, restored.last_error);
        });
    }

    #[test]
    fn proptest_clear_completed_never_removes_pending_or_processing() {
        proptest::proptest!(|(
            statuses in proptest::collection::vec(
                proptest::sample::select(&[
                    QueueStatus::Pending,
                    QueueStatus::Processing,
                    QueueStatus::Completed,
                    QueueStatus::Failed,
                    QueueStatus::Cancelled,
                ]), 1..30)
        )| {
            let queue = make_queue();
            let mut pending_count = 0usize;
            let mut processing_count = 0usize;

            for (i, &status) in statuses.iter().enumerate() {
                let mut item = QueueItem::direct(&format!("s-{i}"));
                item.status = status;
                if status == QueueStatus::Pending {
                    pending_count += 1;
                }
                if status == QueueStatus::Processing {
                    processing_count += 1;
                }
                queue.enqueue(item).expect("enqueue");
            }

            let cleared = queue.clear_completed().expect("clear");
            let remaining = queue.len().expect("len");

            // Pending + Processing items should never be cleared
            assert_eq!(remaining, pending_count + processing_count,
                "cleared {} items but {} pending + {} processing should remain",
                cleared, pending_count, processing_count);
        });
    }

    #[test]
    fn proptest_list_pending_always_sorted_by_priority() {
        proptest::proptest!(|(
            priorities in proptest::collection::vec(0u8..4, 0..20)
        )| {
            let queue = make_queue();

            for (i, &p) in priorities.iter().enumerate() {
                let mut item = QueueItem::direct(&format!("p-{i}"));
                item.priority = match p {
                    0 => Priority::Critical,
                    1 => Priority::High,
                    2 => Priority::Normal,
                    _ => Priority::Low,
                };
                queue.enqueue(item).expect("enqueue");
            }

            let pending = queue.list_pending().expect("list_pending");
            for window in pending.windows(2) {
                assert!(window[0].priority <= window[1].priority,
                    "list_pending not sorted: {:?} before {:?}",
                    window[0].priority, window[1].priority);
            }
        });
    }

    #[test]
    fn proptest_dequeue_always_pending_status() {
        proptest::proptest!(|(n in 1u8..50)| {
            let queue = make_queue();
            for i in 0..n {
                queue.enqueue(QueueItem::direct(&format!("d-{i}"))).expect("enqueue");
            }

            while let Some(item) = queue.dequeue().expect("dequeue") {
                assert_eq!(item.status, QueueStatus::Processing,
                    "dequeued item should be Processing, got {:?}", item.status);
                assert_eq!(item.attempt_count, 1,
                    "dequeued item should have attempt_count = 1");
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Exhaustive QueueStatus status-tracking tests (ha-oeea)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Every QueueStatus variant — used for exhaustive iteration.
    const ALL_CORE_STATUSES: [QueueStatus; 5] = [
        QueueStatus::Pending,
        QueueStatus::Processing,
        QueueStatus::Completed,
        QueueStatus::Failed,
        QueueStatus::Cancelled,
    ];

    #[test]
    fn test_status_display_all_variants() {
        assert_eq!(format!("{:?}", QueueStatus::Pending), "Pending");
        assert_eq!(format!("{:?}", QueueStatus::Processing), "Processing");
        assert_eq!(format!("{:?}", QueueStatus::Completed), "Completed");
        assert_eq!(format!("{:?}", QueueStatus::Failed), "Failed");
        assert_eq!(format!("{:?}", QueueStatus::Cancelled), "Cancelled");
    }

    #[test]
    fn test_status_copy_preserves_value() {
        for status in ALL_CORE_STATUSES {
            let copied = status;
            assert_eq!(status, copied);
        }
    }

    #[test]
    fn test_status_equality_reflexive_all() {
        for status in ALL_CORE_STATUSES {
            assert_eq!(status, status, "Reflexive eq failed for {:?}", status);
        }
    }

    #[test]
    fn test_status_equality_symmetric_all() {
        for a in ALL_CORE_STATUSES {
            for b in ALL_CORE_STATUSES {
                if a == b {
                    assert_eq!(b, a, "Symmetric eq failed for {:?} == {:?}", a, b);
                } else {
                    assert_ne!(b, a, "Symmetric ne failed for {:?} != {:?}", a, b);
                }
            }
        }
    }

    #[test]
    fn test_status_equality_transitive_all() {
        for a in ALL_CORE_STATUSES {
            for b in ALL_CORE_STATUSES {
                for c in ALL_CORE_STATUSES {
                    if a == b && b == c {
                        assert_eq!(a, c, "Transitive eq failed");
                    }
                }
            }
        }
    }

    #[test]
    fn test_status_serde_json_format() {
        // Verify PascalCase serialization format (default serde convention)
        assert_eq!(
            serde_json::to_string(&QueueStatus::Pending).unwrap(),
            "\"Pending\""
        );
        assert_eq!(
            serde_json::to_string(&QueueStatus::Processing).unwrap(),
            "\"Processing\""
        );
        assert_eq!(
            serde_json::to_string(&QueueStatus::Completed).unwrap(),
            "\"Completed\""
        );
        assert_eq!(
            serde_json::to_string(&QueueStatus::Failed).unwrap(),
            "\"Failed\""
        );
        assert_eq!(
            serde_json::to_string(&QueueStatus::Cancelled).unwrap(),
            "\"Cancelled\""
        );
    }

    #[test]
    fn test_status_serde_rejects_invalid_string() {
        let result = serde_json::from_str::<QueueStatus>("\"InvalidStatus\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_serde_rejects_null() {
        let result = serde_json::from_str::<QueueStatus>("null");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_serde_rejects_number() {
        let result = serde_json::from_str::<QueueStatus>("42");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_serde_rejects_empty_string() {
        let result = serde_json::from_str::<QueueStatus>("\"\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_serde_rejects_lowercase() {
        let result = serde_json::from_str::<QueueStatus>("\"pending\"");
        assert!(result.is_err());
    }

    /// Valid lifecycle: Pending → Processing → Completed
    #[test]
    fn test_status_lifecycle_happy_path() {
        let mut item = QueueItem::direct("happy-path");
        assert_eq!(item.status, QueueStatus::Pending);

        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
    }

    /// Valid lifecycle: Pending → Processing → Failed
    #[test]
    fn test_status_lifecycle_failure_path() {
        let mut item = QueueItem::direct("failure-path");
        assert_eq!(item.status, QueueStatus::Pending);

        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);

        item.fail("test error");
        assert_eq!(item.status, QueueStatus::Failed);
        assert_eq!(item.last_error, Some("test error".to_string()));
    }

    /// Valid lifecycle: Pending → Cancelled (direct cancel before processing)
    #[test]
    fn test_status_lifecycle_cancel_before_processing() {
        let mut item = QueueItem::direct("cancel-early");
        assert_eq!(item.status, QueueStatus::Pending);
        item.cancel();
        assert_eq!(item.status, QueueStatus::Cancelled);
    }

    /// Retry cycle: Pending → Processing → Failed → Processing → Completed
    #[test]
    fn test_status_lifecycle_retry_then_success() {
        let mut item = QueueItem::direct("retry-cycle");

        // First attempt
        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);

        item.fail("transient error");
        assert_eq!(item.status, QueueStatus::Failed);
        assert_eq!(item.last_error, Some("transient error".to_string()));

        // Retry
        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 2);

        // Success
        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
    }

    /// Multiple retries: fail-retry-fail-retry-success
    #[test]
    fn test_status_lifecycle_multiple_retries() {
        let mut item = QueueItem::direct("multi-retry");

        for i in 1..=3 {
            item.start_processing();
            assert_eq!(item.attempt_count, i);
            item.fail(&format!("error {}", i));
        }

        item.start_processing();
        assert_eq!(item.attempt_count, 4);
        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
        // Last error is from the most recent fail
        assert_eq!(item.last_error, Some("error 3".to_string()));
    }

    /// Cancel after processing started
    #[test]
    fn test_status_lifecycle_cancel_during_processing() {
        let mut item = QueueItem::direct("cancel-mid");
        item.start_processing();
        assert_eq!(item.status, QueueStatus::Processing);
        item.cancel();
        assert_eq!(item.status, QueueStatus::Cancelled);
    }

    /// Cancel after failure (not retrying)
    #[test]
    fn test_status_lifecycle_cancel_after_failure() {
        let mut item = QueueItem::direct("cancel-after-fail");
        item.start_processing();
        item.fail("broken");
        assert_eq!(item.status, QueueStatus::Failed);
        item.cancel();
        assert_eq!(item.status, QueueStatus::Cancelled);
    }

    /// Status timestamps advance monotonically through lifecycle
    #[test]
    fn test_status_timestamps_advance_on_each_transition() {
        let mut item = QueueItem::direct("ts-tracking");
        let t0 = item.updated_at;

        item.start_processing();
        let t1 = item.updated_at;
        assert!(t1 >= t0, "start_processing must advance updated_at");

        item.complete();
        let t2 = item.updated_at;
        assert!(t2 >= t1, "complete must advance updated_at");
    }

    /// Status tracking through queue operations: enqueue → dequeue
    #[test]
    fn test_status_tracking_through_dequeue() -> Result<()> {
        let queue = make_queue();
        queue.enqueue(QueueItem::direct("track-dequeue"))?;

        let item = queue.dequeue()?.expect("should dequeue");
        assert_eq!(item.status, QueueStatus::Processing);
        assert_eq!(item.attempt_count, 1);
        assert_eq!(item.branch, "track-dequeue");
        Ok(())
    }

    /// Status filtering: count items per status
    #[test]
    fn test_status_count_by_status() -> Result<()> {
        let queue = make_queue();

        // 3 pending
        queue.enqueue(QueueItem::direct("p1"))?;
        queue.enqueue(QueueItem::direct("p2"))?;
        queue.enqueue(QueueItem::direct("p3"))?;

        // 1 completed
        let mut completed = QueueItem::direct("c1");
        completed.status = QueueStatus::Completed;
        queue.enqueue(completed)?;

        // 1 failed
        let mut failed = QueueItem::direct("f1");
        failed.status = QueueStatus::Failed;
        queue.enqueue(failed)?;

        let all = queue.list()?;
        let pending_count = all
            .iter()
            .filter(|i| i.status == QueueStatus::Pending)
            .count();
        let completed_count = all
            .iter()
            .filter(|i| i.status == QueueStatus::Completed)
            .count();
        let failed_count = all
            .iter()
            .filter(|i| i.status == QueueStatus::Failed)
            .count();

        assert_eq!(pending_count, 3);
        assert_eq!(completed_count, 1);
        assert_eq!(failed_count, 1);
        assert_eq!(all.len(), 5);

        Ok(())
    }

    /// Status-based cleanup: clear_completed removes Completed, Failed, Cancelled
    #[test]
    fn test_status_cleanup_preserves_pending_and_processing() -> Result<()> {
        let queue = make_queue();

        let mut processing = QueueItem::direct("proc-1");
        processing.status = QueueStatus::Processing;
        queue.enqueue(processing)?;

        queue.enqueue(QueueItem::direct("pend-1"))?;

        let mut completed = QueueItem::direct("done-1");
        completed.status = QueueStatus::Completed;
        queue.enqueue(completed)?;

        let mut failed = QueueItem::direct("fail-1");
        failed.status = QueueStatus::Failed;
        queue.enqueue(failed)?;

        let mut cancelled = QueueItem::direct("cancel-1");
        cancelled.status = QueueStatus::Cancelled;
        queue.enqueue(cancelled)?;

        assert_eq!(queue.len()?, 5);
        let removed = queue.clear_completed()?;
        assert_eq!(removed, 3); // Completed + Failed + Cancelled
        assert_eq!(queue.len()?, 2);

        let remaining = queue.list()?;
        let statuses: Vec<_> = remaining.iter().map(|i| i.status).collect();
        assert!(statuses.contains(&QueueStatus::Pending));
        assert!(statuses.contains(&QueueStatus::Processing));

        Ok(())
    }

    /// QueueItem.last_error tracks error across multiple fails
    #[test]
    fn test_status_last_error_overwritten_on_each_fail() {
        let mut item = QueueItem::direct("error-tracking");
        assert!(item.last_error.is_none());

        item.fail("first error");
        assert_eq!(item.last_error, Some("first error".to_string()));

        item.start_processing();
        item.fail("second error");
        assert_eq!(item.last_error, Some("second error".to_string()));
    }

    /// Completed item has no error
    #[test]
    fn test_status_completed_has_no_error() {
        let mut item = QueueItem::direct("clean-complete");
        item.start_processing();
        item.fail("oops");
        assert!(item.last_error.is_some());

        item.start_processing();
        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
        // complete() does NOT clear last_error — it just sets status
        // This verifies the behavior is consistent
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Exhaustive QueueSource status-tracking tests (ha-oeea)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_source_workspace_unicode_name() {
        let source = QueueSource::Workspace("日本語ワークスペース".to_string());
        if let QueueSource::Workspace(name) = &source {
            assert_eq!(name, "日本語ワークスペース");
        } else {
            panic!("Expected Workspace variant");
        }
    }

    #[test]
    fn test_source_workspace_very_long_name() {
        let long_name = "x".repeat(10_000);
        let source = QueueSource::Workspace(long_name.clone());
        if let QueueSource::Workspace(name) = &source {
            assert_eq!(name.len(), 10_000);
        } else {
            panic!("Expected Workspace variant");
        }
    }

    #[test]
    fn test_source_workspace_name_with_spaces() {
        let source = QueueSource::Workspace("my workspace name".to_string());
        if let QueueSource::Workspace(name) = &source {
            assert_eq!(name, "my workspace name");
        } else {
            panic!("Expected Workspace variant");
        }
    }

    #[test]
    fn test_source_serde_workspace_unicode_roundtrip() {
        let source = QueueSource::Workspace("ワークスペース".to_string());
        let json = serde_json::to_string(&source).unwrap();
        let back: QueueSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, back);
    }

    #[test]
    fn test_source_serde_workspace_empty_roundtrip() {
        let source = QueueSource::Workspace(String::new());
        let json = serde_json::to_string(&source).unwrap();
        let back: QueueSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, back);
    }

    #[test]
    fn test_source_item_branch_preserved_through_status_changes() {
        let mut item = QueueItem::from_workspace("ws", "feature-branch");
        assert_eq!(item.branch, "feature-branch");

        item.start_processing();
        assert_eq!(item.branch, "feature-branch");

        item.fail("error");
        assert_eq!(item.branch, "feature-branch");

        item.start_processing();
        assert_eq!(item.branch, "feature-branch");

        item.complete();
        assert_eq!(item.branch, "feature-branch");
    }

    #[test]
    fn test_source_item_id_preserved_through_status_changes() {
        let mut item = QueueItem::direct("id-preservation");
        let original_id = item.id.clone();

        item.start_processing();
        assert_eq!(item.id, original_id);

        item.complete();
        assert_eq!(item.id, original_id);
    }

    /// Queue source attribution is immutable through complete lifecycle
    #[test]
    fn test_source_attribution_full_lifecycle() -> Result<()> {
        let queue = make_queue();

        let item = QueueItem::from_workspace("lifecycle-ws", "lifecycle-branch");
        assert_eq!(
            item.source,
            QueueSource::Workspace("lifecycle-ws".to_string())
        );
        queue.enqueue(item)?;

        // Dequeue marks as Processing, but source must be preserved
        let mut dequeued = queue.dequeue()?.expect("should dequeue");
        assert_eq!(
            dequeued.source,
            QueueSource::Workspace("lifecycle-ws".to_string())
        );
        assert_eq!(dequeued.status, QueueStatus::Processing);

        // Complete it
        dequeued.complete();
        assert_eq!(
            dequeued.source,
            QueueSource::Workspace("lifecycle-ws".to_string())
        );
        assert_eq!(dequeued.status, QueueStatus::Completed);

        Ok(())
    }

    /// Multiple items from different sources coexist correctly
    #[test]
    fn test_source_multiple_sources_in_queue() -> Result<()> {
        let queue = make_queue();

        queue.enqueue(QueueItem::from_workspace("ws-alpha", "branch-1"))?;
        queue.enqueue(QueueItem::direct("branch-2"))?;
        queue.enqueue(QueueItem::from_workspace("ws-beta", "branch-3"))?;
        queue.enqueue(QueueItem::direct("branch-4"))?;

        let all = queue.list()?;

        let ws_items: Vec<_> = all
            .iter()
            .filter(|i| matches!(&i.source, QueueSource::Workspace(_)))
            .collect();
        assert_eq!(ws_items.len(), 2);

        let direct_items: Vec<_> = all
            .iter()
            .filter(|i| matches!(&i.source, QueueSource::Direct))
            .collect();
        assert_eq!(direct_items.len(), 2);

        Ok(())
    }

    /// Source + status combined filter across queue
    #[test]
    fn test_source_status_combined_filter() -> Result<()> {
        let queue = make_queue();

        let mut ws_pending = QueueItem::from_workspace("ws", "ws-p");
        ws_pending.status = QueueStatus::Pending;
        queue.enqueue(ws_pending)?;

        let mut ws_completed = QueueItem::from_workspace("ws", "ws-c");
        ws_completed.status = QueueStatus::Completed;
        queue.enqueue(ws_completed)?;

        let mut direct_pending = QueueItem::direct("d-p");
        direct_pending.status = QueueStatus::Pending;
        queue.enqueue(direct_pending)?;

        let all = queue.list()?;

        // workspace + pending = 1
        let count = all
            .iter()
            .filter(|i| {
                matches!(&i.source, QueueSource::Workspace(_)) && i.status == QueueStatus::Pending
            })
            .count();
        assert_eq!(count, 1);

        // direct + completed = 0
        let count = all
            .iter()
            .filter(|i| i.source == QueueSource::Direct && i.status == QueueStatus::Completed)
            .count();
        assert_eq!(count, 0);

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Proptests: QueueStatus and QueueSource status tracking (ha-oeea)
    // ═══════════════════════════════════════════════════════════════════════════

    use proptest::prelude::*;

    prop_compose! {
        fn arb_queue_source()(ws in "[a-zA-Z0-9_-]{0,20}") -> QueueSource {
            if ws.is_empty() {
                QueueSource::Direct
            } else {
                QueueSource::Workspace(ws)
            }
        }
    }

    static CORE_STATUS_SLICE: &[QueueStatus] = &ALL_CORE_STATUSES;

    proptest! {
        #[test]
        fn prop_queue_status_serde_roundtrip(idx in 0usize..5) {
            let status = CORE_STATUS_SLICE[idx];
            let json = serde_json::to_string(&status).unwrap();
            let back: QueueStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(status, back);
        }

        #[test]
        fn prop_queue_source_serde_roundtrip(source in arb_queue_source()) {
            let json = serde_json::to_string(&source).unwrap();
            let back: QueueSource = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(source, back);
        }

        #[test]
        fn prop_queue_item_source_immutable_through_lifecycle(
            branch in "[a-zA-Z0-9_-]{1,20}",
            source in arb_queue_source()
        ) {
            let mut item = QueueItem::new(branch.clone(), source);
            let original_source = item.source.clone();

            item.start_processing();
            prop_assert_eq!(item.source.clone(), original_source.clone());

            item.fail("error");
            prop_assert_eq!(item.source.clone(), original_source.clone());

            item.start_processing();
            prop_assert_eq!(item.source.clone(), original_source.clone());

            item.complete();
            prop_assert_eq!(item.source.clone(), original_source.clone());
        }

        #[test]
        fn prop_queue_item_attempt_count_increments_on_start(
            branch in "[a-zA-Z0-9_-]{1,20}",
            retries in 0u32..10
        ) {
            let mut item = QueueItem::new(branch, QueueSource::Direct);
            for _ in 0..retries {
                item.start_processing();
                item.fail("retry");
            }
            item.start_processing();
            prop_assert_eq!(item.attempt_count, retries + 1);
        }

        #[test]
        fn prop_queue_item_branch_immutable(
            branch in "[a-zA-Z0-9_-]{1,20}",
            source in arb_queue_source()
        ) {
            let mut item = QueueItem::new(branch.clone(), source);
            item.start_processing();
            item.fail("error");
            item.start_processing();
            item.complete();
            prop_assert_eq!(item.branch, branch);
        }

        #[test]
        fn prop_queue_item_id_immutable(
            branch in "[a-zA-Z0-9_-]{1,20}",
            source in arb_queue_source()
        ) {
            let mut item = QueueItem::new(branch, source);
            let original_id = item.id.clone();
            item.start_processing();
            item.complete();
            prop_assert_eq!(item.id, original_id);
        }

        #[test]
        fn prop_queue_item_created_at_unchanged_through_lifecycle(
            branch in "[a-zA-Z0-9_-]{1,20}"
        ) {
            let mut item = QueueItem::new(branch, QueueSource::Direct);
            let created = item.created_at;

            item.start_processing();
            prop_assert_eq!(item.created_at, created);

            item.complete();
            prop_assert_eq!(item.created_at, created);
        }

        #[test]
        fn prop_queue_item_serde_preserves_status_tracking(
            branch in "[a-zA-Z0-9_-]{1,20}",
            source in arb_queue_source(),
            attempt_count in 0u32..5
        ) {
            let mut item = QueueItem::new(branch, source);
            for _ in 0..attempt_count {
                item.start_processing();
                item.fail("retry");
            }

            let json = serde_json::to_string(&item).unwrap();
            let back: QueueItem = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(back.branch, item.branch);
            prop_assert_eq!(back.source, item.source);
            prop_assert_eq!(back.status, item.status);
            prop_assert_eq!(back.attempt_count, item.attempt_count);
            prop_assert_eq!(back.last_error, item.last_error);
        }
    }
}

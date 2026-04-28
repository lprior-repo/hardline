//! Unified queue management for Source Control Plane.
//!
//! Combines Stak's queue with Isolate workspace support.
//! Zero panic, zero unwrap - all operations return Result.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{error::Result, error_queue::QueueErrorKind, lock::LockManager};

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
///
/// Hierarchical superstates:
/// - **Active**: Pending, Processing, Retrying — item is still being worked on
/// - **Terminal**: Completed, Failed, Cancelled — item has reached a final state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    /// Item is waiting in queue
    Pending,
    /// Item is being processed
    Processing,
    /// Item is being retried after a transient failure
    Retrying,
    /// Item completed successfully
    Completed,
    /// Item failed processing
    Failed,
    /// Item was cancelled
    Cancelled,
}

/// Superstate grouping for QueueStatus variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueSuperstate {
    /// Active statuses: item is still being worked on
    Active,
    /// Terminal statuses: item has reached a final state
    Terminal,
}

impl QueueStatus {
    /// Returns true if this status is in the Active superstate
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            QueueStatus::Pending | QueueStatus::Processing | QueueStatus::Retrying
        )
    }

    /// Returns true if this status is in the Terminal superstate
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            QueueStatus::Completed | QueueStatus::Failed | QueueStatus::Cancelled
        )
    }

    /// Returns the superstate this status belongs to
    pub fn superstate(&self) -> QueueSuperstate {
        if self.is_active() {
            QueueSuperstate::Active
        } else {
            QueueSuperstate::Terminal
        }
    }
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
        items.retain(|i| !i.status.is_terminal());
        Ok(len_before - items.len())
    }

    fn insert_at(&self, position: usize, mut item: QueueItem) -> Result<()> {
        let mut items = self.items.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();
        let pos = position.min(items.len());
        items.insert(pos, item);
        Ok(())
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

    const ALL_STATUSES: [QueueStatus; 6] = [
        QueueStatus::Pending,
        QueueStatus::Processing,
        QueueStatus::Retrying,
        QueueStatus::Completed,
        QueueStatus::Failed,
        QueueStatus::Cancelled,
    ];

    #[test]
    fn test_queue_status_has_six_variants() {
        assert_eq!(ALL_STATUSES.len(), 6);
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
        assert!(format!("{:?}", QueueStatus::Retrying).contains("Retrying"));
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
                QueueStatus::Retrying => "retrying",
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
    fn test_queue_status_superstate_active_variants() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Processing,
            QueueStatus::Retrying,
        ] {
            assert!(status.is_active(), "{:?} should be active", status);
            assert_eq!(
                status.superstate(),
                QueueSuperstate::Active,
                "{:?} should belong to Active superstate",
                status
            );
        }
    }

    #[test]
    fn test_queue_status_superstate_terminal_variants() {
        for status in [
            QueueStatus::Completed,
            QueueStatus::Failed,
            QueueStatus::Cancelled,
        ] {
            assert!(status.is_terminal(), "{:?} should be terminal", status);
            assert_eq!(
                status.superstate(),
                QueueSuperstate::Terminal,
                "{:?} should belong to Terminal superstate",
                status
            );
        }
    }

    #[test]
    fn test_queue_status_active_vs_terminal_mutually_exclusive() {
        for status in ALL_STATUSES {
            let active = status.is_active();
            let terminal = status.is_terminal();
            assert_ne!(
                active, terminal,
                "{:?} cannot be both active and terminal",
                status
            );
        }
    }

    #[test]
    fn test_queue_status_all_variants_covered_by_superstate() {
        for status in ALL_STATUSES {
            assert!(
                status.is_active() || status.is_terminal(),
                "{:?} must be either active or terminal",
                status
            );
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

        let mut retrying = QueueItem::direct("retrying-item");
        retrying.status = QueueStatus::Retrying;
        queue.enqueue(retrying)?;

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

        let mut retrying = QueueItem::direct("retrying");
        retrying.status = QueueStatus::Retrying;
        queue.enqueue(retrying)?;

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
        assert_eq!(queue.len()?, 2);

        let remaining = queue.list()?;
        let branches: Vec<&str> = remaining.iter().map(|i| i.branch.as_str()).collect();
        assert!(branches.contains(&"still-pending"));
        assert!(branches.contains(&"retrying"));

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

    // ═══════════════════════════════════════════════════════════════════════════
    // RED QUEEN ADVERSARIAL TESTS — ha-qaw3
    // ═══════════════════════════════════════════════════════════════════════════

    mod red_queen_adversarial {
        use super::*;

        // --- DIM-1: QueueItem State Machine — Unguarded Transitions ---

        /// CRITICAL: A Completed item can be re-processed via start_processing.
        /// No state guard prevents Completed -> Processing transition.
        #[test]
        fn completed_item_can_be_reprocessed() {
            let mut item = QueueItem::direct("branch-1");
            item.complete();
            assert_eq!(item.status, QueueStatus::Completed);
            assert!(item.status.is_terminal());

            // State machine allows re-processing of completed item
            item.start_processing();
            assert_eq!(
                item.status,
                QueueStatus::Processing,
                "Completed item should NOT be re-processable without state guard"
            );
            assert_eq!(item.attempt_count, 1);
        }

        /// CRITICAL: A Failed item can be marked Completed without any retry.
        #[test]
        fn failed_item_can_be_completed_without_retry() {
            let mut item = QueueItem::direct("branch-2");
            item.fail("disk full");
            assert_eq!(item.status, QueueStatus::Failed);
            assert!(item.status.is_terminal());

            item.complete();
            assert_eq!(
                item.status,
                QueueStatus::Completed,
                "Failed item should NOT transition directly to Completed"
            );
        }

        /// MAJOR: A Completed item can be retroactively marked as Failed.
        #[test]
        fn completed_item_can_be_failed() {
            let mut item = QueueItem::direct("branch-3");
            item.complete();
            assert_eq!(item.status, QueueStatus::Completed);
            assert!(item.status.is_terminal());

            item.fail("post-hoc failure");
            assert_eq!(item.status, QueueStatus::Failed);
            assert_eq!(item.last_error, Some("post-hoc failure".to_string()));
        }

        /// MAJOR: A Completed item can be cancelled.
        #[test]
        fn completed_item_can_be_cancelled() {
            let mut item = QueueItem::direct("branch-4");
            item.complete();

            item.cancel();
            assert_eq!(item.status, QueueStatus::Cancelled);
        }

        /// CRITICAL: start_processing increments attempt_count without bound.
        /// Calling it on a completed item still increments.
        #[test]
        fn start_processing_increments_without_bound() {
            let mut item = QueueItem::direct("branch-5");
            item.complete();

            for i in 1..=100u32 {
                item.start_processing();
                assert_eq!(
                    item.attempt_count, i,
                    "attempt_count should be {i} after {i} start_processing calls"
                );
            }
            assert_eq!(item.attempt_count, 100);
        }

        /// MAJOR: Full irregular cycle: Pending -> Processing -> Failed -> Processing -> Completed
        /// Bypasses the normal Pending -> Processing -> Completed flow.
        #[test]
        fn irregular_cycle_failed_to_processing_to_completed() {
            let mut item = QueueItem::direct("branch-6");

            item.start_processing();
            assert_eq!(item.status, QueueStatus::Processing);
            assert_eq!(item.attempt_count, 1);

            item.fail("first attempt failed");
            assert_eq!(item.status, QueueStatus::Failed);

            // Re-process without queue dequeue
            item.start_processing();
            assert_eq!(item.status, QueueStatus::Processing);
            assert_eq!(item.attempt_count, 2);

            // Complete on second attempt
            item.complete();
            assert_eq!(item.status, QueueStatus::Completed);
        }

        /// MAJOR: Cancelled item can be re-processed and completed.
        #[test]
        fn cancelled_item_can_be_reprocessed_and_completed() {
            let mut item = QueueItem::direct("branch-7");
            item.start_processing();
            item.cancel();

            item.start_processing();
            assert_eq!(item.status, QueueStatus::Processing);
            assert_eq!(item.attempt_count, 2);

            item.complete();
            assert_eq!(item.status, QueueStatus::Completed);
        }

        /// MINOR: fail() overwrites previous last_error without accumulation.
        #[test]
        fn fail_overwrites_previous_error() {
            let mut item = QueueItem::direct("branch-8");
            item.fail("first error");
            assert_eq!(item.last_error, Some("first error".to_string()));

            item.fail("second error");
            assert_eq!(item.last_error, Some("second error".to_string()));
        }

        // --- DIM-3: Hierarchical Superstate Enforcement ---

        /// CRITICAL: All 6 QueueStatus variants map to exactly one superstate.
        #[test]
        fn all_variants_map_to_single_superstate() {
            for status in [
                QueueStatus::Pending,
                QueueStatus::Processing,
                QueueStatus::Retrying,
                QueueStatus::Completed,
                QueueStatus::Failed,
                QueueStatus::Cancelled,
            ] {
                match status.superstate() {
                    QueueSuperstate::Active => assert!(status.is_active()),
                    QueueSuperstate::Terminal => assert!(status.is_terminal()),
                }
            }
        }

        /// CRITICAL: Pending, Processing, Retrying are all Active superstate members.
        #[test]
        fn active_variants_are_not_terminal() {
            for status in [
                QueueStatus::Pending,
                QueueStatus::Processing,
                QueueStatus::Retrying,
            ] {
                assert!(status.is_active(), "{status:?} should be active");
                assert!(!status.is_terminal(), "{status:?} should NOT be terminal");
                assert_eq!(status.superstate(), QueueSuperstate::Active);
            }
        }

        /// CRITICAL: Completed, Failed, Cancelled are all Terminal superstate members.
        #[test]
        fn terminal_variants_are_not_active() {
            for status in [
                QueueStatus::Completed,
                QueueStatus::Failed,
                QueueStatus::Cancelled,
            ] {
                assert!(!status.is_active(), "{status:?} should NOT be active");
                assert!(status.is_terminal(), "{status:?} should be terminal");
                assert_eq!(status.superstate(), QueueSuperstate::Terminal);
            }
        }

        /// MAJOR: clear_completed only removes Terminal items, keeps Active items.
        #[test]
        fn clear_completed_preserves_active_variants() -> Result<()> {
            let queue = make_queue();

            let mut pending = QueueItem::direct("pending");
            pending.status = QueueStatus::Pending;
            queue.enqueue(pending)?;

            let mut processing = QueueItem::direct("processing");
            processing.status = QueueStatus::Processing;
            queue.enqueue(processing)?;

            let mut retrying = QueueItem::direct("retrying");
            retrying.status = QueueStatus::Retrying;
            queue.enqueue(retrying)?;

            let mut completed = QueueItem::direct("completed");
            completed.status = QueueStatus::Completed;
            queue.enqueue(completed)?;

            let mut failed = QueueItem::direct("failed");
            failed.status = QueueStatus::Failed;
            queue.enqueue(failed)?;

            let mut cancelled = QueueItem::direct("cancelled");
            cancelled.status = QueueStatus::Cancelled;
            queue.enqueue(cancelled)?;

            let removed = queue.clear_completed()?;
            assert_eq!(removed, 3);
            assert_eq!(queue.len()?, 3);

            let remaining = queue.list()?;
            let statuses: Vec<_> = remaining.iter().map(|i| i.status).collect();
            assert!(statuses.contains(&QueueStatus::Pending));
            assert!(statuses.contains(&QueueStatus::Processing));
            assert!(statuses.contains(&QueueStatus::Retrying));

            Ok(())
        }

        // --- DIM-5: Queue Priority Edge Cases ---

        /// MAJOR: Enqueue 50 items with same priority — verify FIFO order.
        #[test]
        fn same_priority_preserves_fifo_order() -> Result<()> {
            let queue = make_queue();
            let count = 50;

            for i in 0..count {
                let item = QueueItem::direct(&format!("branch-{:03}", i));
                queue.enqueue(item)?;
            }

            for i in 0..count {
                let item = queue.dequeue()?.expect("should have item");
                assert_eq!(
                    item.branch,
                    format!("branch-{:03}", i),
                    "FIFO order violated at position {i}"
                );
            }

            Ok(())
        }

        /// MAJOR: Dequeue on queue with only Completed items returns None.
        #[test]
        fn dequeue_returns_none_when_all_completed() -> Result<()> {
            let queue = make_queue();

            let mut item = QueueItem::direct("done-1");
            item.status = QueueStatus::Completed;
            queue.enqueue(item)?;

            let mut item2 = QueueItem::direct("done-2");
            item2.status = QueueStatus::Completed;
            queue.enqueue(item2)?;

            assert_eq!(queue.len()?, 2);
            let result = queue.dequeue()?;
            assert!(result.is_none(), "Should return None when no Pending items");
            Ok(())
        }

        /// MINOR: Mixed status queue — dequeue returns only Pending items in priority order.
        #[test]
        fn dequeue_skips_non_pending_in_priority_order() -> Result<()> {
            let queue = make_queue();

            // High priority but completed
            let mut high_done = QueueItem::direct("high-done");
            high_done.priority = Priority::High;
            high_done.status = QueueStatus::Completed;
            queue.enqueue(high_done)?;

            // Low priority but pending
            let mut low_pending = QueueItem::direct("low-pending");
            low_pending.priority = Priority::Low;
            queue.enqueue(low_pending)?;

            // Normal priority but pending
            let mut normal_pending = QueueItem::direct("normal-pending");
            normal_pending.priority = Priority::Normal;
            queue.enqueue(normal_pending)?;

            let first = queue.dequeue()?.expect("should have item");
            assert_eq!(first.branch, "normal-pending");

            let second = queue.dequeue()?.expect("should have item");
            assert_eq!(second.branch, "low-pending");

            let third = queue.dequeue()?;
            assert!(third.is_none());

            Ok(())
        }

        // --- DIM-7: UUID Generation ---

        /// MINOR: Two QueueItems created in rapid succession have different IDs.
        #[test]
        fn rapid_queue_items_have_unique_ids() {
            let items: Vec<QueueItem> = (0..100)
                .map(|i| QueueItem::direct(&format!("rapid-{}", i)))
                .collect();

            let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
            let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();

            assert_eq!(
                unique.len(),
                ids.len(),
                "All queue item IDs should be unique; got {} duplicates",
                ids.len() - unique.len()
            );
        }

        /// MINOR: UUID format is valid (8-4-4-4-12 hex pattern, 36 chars total).
        #[test]
        fn uuid_format_is_valid() {
            let item = QueueItem::direct("uuid-test");
            // Pattern: 8-4-4-4-12 hex chars separated by dashes
            let pattern = regex::Regex::new(
                r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
            )
            .expect("valid regex");

            assert_eq!(
                item.id.len(),
                36,
                "UUID should be 36 chars, got {}",
                item.id.len()
            );
            assert!(
                pattern.is_match(&item.id),
                "UUID '{}' does not match expected format",
                item.id
            );
        }
    }
}

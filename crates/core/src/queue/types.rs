//! Queue domain types: Priority, QueueStatus, QueueSource, QueueItem, ProcessResult.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

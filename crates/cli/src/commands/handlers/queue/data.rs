//! Data types for the queue command handler.
//!
//! This module defines inert, serializable data structures used by the queue handler.
//! All types here are pure data with no behavior.

use serde::{Deserialize, Serialize};

/// Output format for queue display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QueueOutputFormat {
    #[default]
    Table,
    Json,
}

impl QueueOutputFormat {
    /// Create from a JSON flag
    #[must_use]
    pub fn from_json_flag(is_json: bool) -> Self {
        if is_json {
            QueueOutputFormat::Json
        } else {
            QueueOutputFormat::Table
        }
    }

    /// Check if this is JSON format
    #[must_use]
    pub fn is_json(self) -> bool {
        matches!(self, QueueOutputFormat::Json)
    }

    /// Check if this is table format
    #[must_use]
    pub fn is_table(self) -> bool {
        matches!(self, QueueOutputFormat::Table)
    }
}

/// Queue item for list display
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueListItem {
    /// Display index (1-based)
    pub index: usize,
    /// Branch name
    pub branch: String,
    /// Priority as string
    pub priority: String,
    /// Status as string
    pub status: String,
}

/// Queue item for detail view
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItemDetail {
    /// Unique identifier
    pub id: String,
    /// Branch name
    pub branch: String,
    /// Priority level
    pub priority: String,
    /// Status
    pub status: String,
    /// Source (workspace or direct)
    pub source: String,
    /// Attempt count
    pub attempt_count: u32,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Created timestamp
    pub created_at: String,
    /// Updated timestamp
    pub updated_at: String,
}

/// Queue item for human-readable display
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItemDisplay {
    /// Display index (1-based)
    pub index: usize,
    /// Branch name
    pub branch: String,
    /// Priority as string
    pub priority: String,
    /// Status as string
    pub status: String,
}

/// Queue status information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueStatusDisplay {
    /// Total number of items
    pub total_items: usize,
    /// Number of pending items
    pub pending_items: usize,
    /// Next item to process (if any)
    pub next_item: Option<String>,
}

/// Queue subcommands
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueSubcommand {
    /// List queue items
    List,
    /// Add item to queue
    Enqueue {
        /// Branch name
        branch: String,
        /// Priority (optional)
        priority: Option<String>,
    },
    /// Remove front item from queue
    Dequeue,
    /// Process next item
    Process {
        /// Run pre-flight checks
        checks: bool,
    },
    /// Insert at position
    Insert {
        /// Position
        position: usize,
        /// Branch name
        branch: String,
    },
    /// Remove item
    Remove {
        /// Branch name or ID
        branch: String,
    },
    /// Show queue status
    Status,
    /// Clear completed/failed items
    Clear,
    /// Show item detail
    Detail {
        /// Branch or ID to show
        target: String,
    },
}

/// Queue command options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueOptions {
    /// Subcommand to run
    pub subcommand: QueueSubcommand,
    /// Output format
    pub format: QueueOutputFormat,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            subcommand: QueueSubcommand::List,
            format: QueueOutputFormat::default(),
        }
    }
}

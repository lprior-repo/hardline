//! Repair result types
//!
//! Result types for repair operations.

use serde::{Deserialize, Serialize};

use super::RepairStrategy;

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a repair operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    /// Name of the workspace
    pub workspace: String,
    /// Whether repair was successful
    pub success: bool,
    /// Action taken
    pub action: RepairStrategy,
    /// Description of what was done
    pub summary: String,
    /// ID of backup created before repair (if any)
    pub backup_id: Option<String>,
}

impl RepairResult {
    /// Create a successful repair result
    #[must_use]
    pub fn success(
        workspace: impl Into<String>,
        action: RepairStrategy,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            success: true,
            action,
            summary: summary.into(),
            backup_id: None,
        }
    }

    /// Create a failed repair result
    #[must_use]
    pub fn failure(
        workspace: impl Into<String>,
        action: RepairStrategy,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            success: false,
            action,
            summary: summary.into(),
            backup_id: None,
        }
    }

    /// Add a backup ID to the result
    #[must_use]
    pub fn with_backup(mut self, backup_id: impl Into<String>) -> Self {
        self.backup_id = Some(backup_id.into());
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ROLLBACK RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Name of the workspace
    pub workspace: String,
    /// Whether rollback was successful
    pub success: bool,
    /// Description of result
    pub summary: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// BACKUP METADATA
// ═══════════════════════════════════════════════════════════════════════════

/// Metadata for a workspace backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup ID
    pub id: String,
    /// Workspace name
    pub workspace: String,
    /// When backup was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Reason for backup
    pub reason: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum
    pub checksum: Option<String>,
}

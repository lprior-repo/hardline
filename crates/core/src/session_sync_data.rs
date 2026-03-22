//! Session sync data types - immutable domain objects
//!
//! # Architecture
//!
//! - **Data**: `SessionSyncInput`, `SessionSyncResult`, `WorkspaceCleanStatus`, `PreconditionCheck`

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::types::SessionStatus;

// ═══════════════════════════════════════════════════════════════════════════════
// DATA LAYER - Immutable, serializable domain types
// ═══════════════════════════════════════════════════════════════════════════════

/// Input for a session sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSyncInput {
    /// Name of the session to sync
    pub session_name: String,
    /// Path to the workspace
    pub workspace_path: PathBuf,
    /// Main branch to rebase onto
    pub main_branch: String,
    /// Whether to allow dirty workspace (dangerous)
    pub allow_dirty: bool,
}

impl SessionSyncInput {
    /// Create a new sync input with required fields
    #[must_use]
    pub fn new(session_name: String, workspace_path: PathBuf, main_branch: String) -> Self {
        Self {
            session_name,
            workspace_path,
            main_branch,
            allow_dirty: false,
        }
    }

    /// Enable dirty workspace allowance
    #[must_use]
    pub fn with_dirty_allowed(mut self) -> Self {
        self.allow_dirty = true;
        self
    }
}

/// Result of a successful sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSyncResult {
    /// Name of the synced session
    pub session_name: String,
    /// New revision after rebase
    pub new_revision: String,
    /// Whether conflicts were detected
    pub had_conflicts: bool,
    /// Timestamp of sync completion
    pub synced_at: u64,
}

impl SessionSyncResult {
    /// Create a new sync result
    #[must_use]
    pub fn new(session_name: String, new_revision: String, had_conflicts: bool) -> Self {
        let synced_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_name,
            new_revision,
            had_conflicts,
            synced_at,
        }
    }
}

/// Status of the workspace before sync
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceCleanStatus {
    /// Workspace has no uncommitted changes
    Clean,
    /// Workspace has uncommitted changes
    Dirty,
    /// Unable to determine status
    Unknown,
}

/// Precondition check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconditionCheck {
    /// Session exists in database
    pub session_exists: bool,
    /// Current session status
    pub current_status: Option<SessionStatus>,
    /// Workspace clean status
    pub workspace_status: WorkspaceCleanStatus,
}

impl PreconditionCheck {
    /// Check if all preconditions are met
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.session_exists
            && matches!(
                self.current_status,
                Some(SessionStatus::Active | SessionStatus::Failed)
            )
            && matches!(
                self.workspace_status,
                WorkspaceCleanStatus::Clean | WorkspaceCleanStatus::Unknown
            )
    }

    /// Create a valid precondition check
    #[must_use]
    pub fn valid(status: SessionStatus) -> Self {
        Self {
            session_exists: true,
            current_status: Some(status),
            workspace_status: WorkspaceCleanStatus::Clean,
        }
    }
}

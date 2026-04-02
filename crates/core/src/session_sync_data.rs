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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_sync_input_defaults_allow_dirty_false() {
        let input = SessionSyncInput::new(
            "s1".into(),
            PathBuf::from("/w"),
            "main".into(),
        );
        assert!(!input.allow_dirty);
    }

    #[test]
    fn session_sync_input_with_dirty_allowed_returns_true() {
        let input = SessionSyncInput::new(
            "s1".into(),
            PathBuf::from("/w"),
            "main".into(),
        )
        .with_dirty_allowed();
        assert!(input.allow_dirty);
    }

    #[test]
    fn session_sync_input_with_dirty_allowed_does_not_affect_other_fields() {
        let input = SessionSyncInput::new(
            "my-session".into(),
            PathBuf::from("/my/workspace"),
            "develop".into(),
        )
        .with_dirty_allowed();
        assert_eq!(input.session_name, "my-session");
        assert_eq!(input.workspace_path, PathBuf::from("/my/workspace"));
        assert_eq!(input.main_branch, "develop");
    }

    #[test]
    fn session_sync_input_is_clone() {
        let input = SessionSyncInput::new(
            "s1".into(),
            PathBuf::from("/w"),
            "main".into(),
        );
        let cloned = input.clone();
        assert_eq!(cloned.session_name, input.session_name);
    }

    #[test]
    fn session_sync_result_new_sets_had_conflicts_false() {
        let result = SessionSyncResult::new("s1".into(), "rev1".into(), false);
        assert!(!result.had_conflicts);
    }

    #[test]
    fn session_sync_result_new_sets_had_conflicts_true() {
        let result = SessionSyncResult::new("s1".into(), "rev1".into(), true);
        assert!(result.had_conflicts);
    }

    #[test]
    fn session_sync_result_new_sets_synced_at_to_recent_timestamp() {
        let result = SessionSyncResult::new("s1".into(), "rev1".into(), false);
        // synced_at should be a reasonable unix timestamp (after year 2020)
        assert!(result.synced_at > 1_577_836_800, "synced_at should be a recent timestamp");
    }

    #[test]
    fn session_sync_result_is_clone() {
        let result = SessionSyncResult::new("s1".into(), "rev1".into(), true);
        let cloned = result.clone();
        assert_eq!(cloned.session_name, result.session_name);
        assert_eq!(cloned.new_revision, result.new_revision);
        assert_eq!(cloned.had_conflicts, result.had_conflicts);
    }

    #[test]
    fn workspace_clean_status_equality() {
        assert_eq!(WorkspaceCleanStatus::Clean, WorkspaceCleanStatus::Clean);
        assert_eq!(WorkspaceCleanStatus::Dirty, WorkspaceCleanStatus::Dirty);
        assert_eq!(WorkspaceCleanStatus::Unknown, WorkspaceCleanStatus::Unknown);
        assert_ne!(WorkspaceCleanStatus::Clean, WorkspaceCleanStatus::Dirty);
        assert_ne!(WorkspaceCleanStatus::Dirty, WorkspaceCleanStatus::Unknown);
        assert_ne!(WorkspaceCleanStatus::Unknown, WorkspaceCleanStatus::Clean);
    }

    #[test]
    fn workspace_clean_status_is_copy() {
        let status = WorkspaceCleanStatus::Dirty;
        let copied = status;
        assert_eq!(copied, status);
    }

    #[test]
    fn precondition_check_valid_active() {
        let check = PreconditionCheck::valid(SessionStatus::Active);
        assert!(check.session_exists);
        assert_eq!(check.current_status, Some(SessionStatus::Active));
        assert_eq!(check.workspace_status, WorkspaceCleanStatus::Clean);
        assert!(check.is_valid());
    }

    #[test]
    fn precondition_check_valid_failed() {
        let check = PreconditionCheck::valid(SessionStatus::Failed);
        assert!(check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_session_missing() {
        let check = PreconditionCheck {
            session_exists: false,
            current_status: Some(SessionStatus::Active),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_status_creating() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Creating),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_status_paused() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Paused),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_status_completed() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Completed),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_status_none() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: None,
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_invalid_when_workspace_dirty() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Active),
            workspace_status: WorkspaceCleanStatus::Dirty,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn precondition_check_valid_when_workspace_unknown() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Active),
            workspace_status: WorkspaceCleanStatus::Unknown,
        };
        assert!(check.is_valid());
    }

    #[test]
    fn precondition_check_valid_when_status_failed_and_workspace_unknown() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Failed),
            workspace_status: WorkspaceCleanStatus::Unknown,
        };
        assert!(check.is_valid());
    }

    #[test]
    fn precondition_check_is_clone() {
        let check = PreconditionCheck::valid(SessionStatus::Active);
        let cloned = check.clone();
        assert_eq!(cloned.session_exists, check.session_exists);
        assert_eq!(cloned.current_status, check.current_status);
        assert_eq!(cloned.workspace_status, check.workspace_status);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_workspace_clean_status_serde_roundtrip_all_variants() {
        for status in [WorkspaceCleanStatus::Clean, WorkspaceCleanStatus::Dirty, WorkspaceCleanStatus::Unknown] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: WorkspaceCleanStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized, "Roundtrip failed for {status:?}");
        }
    }

    #[test]
    fn test_session_sync_input_serde_roundtrip() {
        let input = SessionSyncInput {
            session_name: "test-session".to_string(),
            workspace_path: PathBuf::from("/tmp/workspace"),
            main_branch: "main".to_string(),
            allow_dirty: false,
        };
        let json = serde_json::to_string(&input).expect("serialize ok");
        let deserialized: SessionSyncInput = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(input, deserialized);
    }

    #[test]
    fn test_session_sync_result_serde_roundtrip() {
        let result = SessionSyncResult {
            session_name: "sess".to_string(),
            new_revision: "abc123".to_string(),
            had_conflicts: false,
            synced_at: 1234567890,
        };
        let json = serde_json::to_string(&result).expect("serialize ok");
        let deserialized: SessionSyncResult = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(result, deserialized);
    }

    #[test]
    fn test_precondition_check_serde_roundtrip() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Active),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        let json = serde_json::to_string(&check).expect("serialize ok");
        let deserialized: PreconditionCheck = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(check, deserialized);
    }

    #[test]
    fn test_precondition_check_serde_with_none_status() {
        let check = PreconditionCheck {
            session_exists: false,
            current_status: None,
            workspace_status: WorkspaceCleanStatus::Unknown,
        };
        let json = serde_json::to_string(&check).expect("serialize ok");
        let deserialized: PreconditionCheck = serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.current_status.is_none());
    }
}

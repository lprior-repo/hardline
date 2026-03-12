//! Session sync domain logic - Data → Calculations → Actions
//!
//! This module implements the sync operation for sessions with:
//! - Preconditions: session exists, status Active/Failed, workspace clean (or allowed)
//! - Postconditions: rebased onto main, conflicts reported, status transitions
//! - Errors: `SessionNotFound`, `InvalidSessionStatus`, `DirtyWorkspace`, `Conflict`,
//!   `RebaseFailure`
//!
//! # Architecture
//!
//! - **Data**: `SessionSyncInput`, `SessionSyncResult`, `SyncError` types
//! - **Calculations**: Pure validation and state transition functions
//! - **Actions**: Async JJ operations wrapped with proper error handling

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::Error as CoreError;
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

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR LAYER - Domain errors using thiserror
// ═══════════════════════════════════════════════════════════════════════════════

/// Domain errors for session sync operations
#[derive(Debug, Clone, Error)]
pub enum SyncError {
    /// Session does not exist in database
    #[error("Session '{0}' not found")]
    SessionNotFound(String),

    /// Session status does not allow sync
    #[error("Invalid session status '{actual}' for sync operation. Expected: Active or Failed")]
    InvalidSessionStatus {
        /// Actual status of the session
        actual: String,
        /// Allowed statuses
        allowed: Vec<String>,
    },

    /// Workspace has uncommitted changes
    #[error("Workspace at '{0}' has uncommitted changes. Use --allow-dirty to sync anyway")]
    DirtyWorkspace(String),

    /// Rebase resulted in conflicts
    #[error("Rebase conflicts in workspace '{workspace}'. Resolve with 'jj resolve' and retry")]
    Conflict {
        /// Workspace path
        workspace: String,
        /// Conflicted files
        conflicted_files: Vec<String>,
    },

    /// Rebase operation failed
    #[error("Rebase failed for workspace '{workspace}': {reason}")]
    RebaseFailure {
        /// Workspace path
        workspace: String,
        /// Underlying error
        reason: String,
    },

    /// JJ command execution failed
    #[error("JJ command failed: {0}")]
    JjCommandError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<SyncError> for CoreError {
    fn from(err: SyncError) -> Self {
        match err {
            SyncError::SessionNotFound(session) => CoreError::SessionNotFound(session),
            SyncError::InvalidSessionStatus { actual, .. } => CoreError::ValidationFieldError {
                message: format!("Invalid session status: {actual}"),
                field: "status".to_string(),
                value: Some(actual),
            },
            SyncError::DirtyWorkspace(path) => CoreError::ValidationFieldError {
                message: format!("Workspace at '{path}' has uncommitted changes"),
                field: "workspace".to_string(),
                value: Some(path),
            },
            SyncError::Conflict {
                workspace,
                conflicted_files,
            } => CoreError::VcsConflict(
                workspace,
                format!("Conflicted files: {}", conflicted_files.join(", ")),
            ),
            SyncError::RebaseFailure {
                workspace: _,
                reason,
            } => CoreError::VcsRebaseFailed(reason),
            SyncError::JjCommandError(msg) => CoreError::VcsConflict("jj".to_string(), msg),
            SyncError::IoError(msg) => CoreError::Io(std::io::Error::other(msg)),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CALCULATIONS LAYER - Pure validation and state transitions
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate preconditions for sync operation
///
/// # Errors
///
/// Returns `SyncError::SessionNotFound` if session doesn't exist
/// Returns `SyncError::InvalidSessionStatus` if status is not Active or Failed
/// Returns `SyncError::DirtyWorkspace` if workspace is dirty and `allow_dirty` is false
pub fn validate_sync_preconditions(
    session_exists: bool,
    current_status: Option<SessionStatus>,
    workspace_status: WorkspaceCleanStatus,
    allow_dirty: bool,
) -> std::result::Result<PreconditionCheck, SyncError> {
    let precheck = PreconditionCheck {
        session_exists,
        current_status,
        workspace_status,
    };

    if !precheck.session_exists {
        return Err(SyncError::SessionNotFound("Unknown session".to_string()));
    }

    let valid_status = matches!(
        precheck.current_status,
        Some(SessionStatus::Active | SessionStatus::Failed)
    );

    if !valid_status {
        let actual = precheck
            .current_status
            .map_or_else(|| "None".to_string(), |s| format!("{s:?}"));

        return Err(SyncError::InvalidSessionStatus {
            actual,
            allowed: vec!["Active".to_string(), "Failed".to_string()],
        });
    }

    let is_dirty = precheck.workspace_status == WorkspaceCleanStatus::Dirty;

    if is_dirty && !allow_dirty {
        return Err(SyncError::DirtyWorkspace("Unknown workspace".to_string()));
    }

    Ok(precheck)
}

/// Parse JJ rebase output to extract revision and conflicts
#[must_use]
pub fn parse_rebase_output(output: &str) -> (Option<String>, Vec<String>) {
    let mut revision = None;
    let mut conflicts = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.len() >= 6
            && trimmed.len() <= 64
            && trimmed.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && !trimmed.contains(':')
            && !trimmed.contains(' ')
        {
            revision = Some(trimmed.to_string());
        }

        let lower = trimmed.to_lowercase();
        if lower.contains("conflict") || lower.contains("conflicted") {
            conflicts.push(trimmed.to_string());
        }
    }

    (revision, conflicts)
}

/// Determine if rebase output indicates conflicts
#[must_use]
pub fn has_conflicts_in_output(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("conflict") || lower.contains("conflicted") || lower.contains("some conflicts")
}

/// Create sync result from rebase output
#[must_use]
pub fn create_sync_result(session_name: String, rebase_output: &str) -> SessionSyncResult {
    let (revision, _conflicts) = parse_rebase_output(rebase_output);
    let had_conflicts = has_conflicts_in_output(rebase_output);

    SessionSyncResult::new(
        session_name,
        revision.unwrap_or_else(|| "unknown".to_string()),
        had_conflicts,
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// CALCULATIONS - Workspace status detection
// ═══════════════════════════════════════════════════════════════════════════════

/// Determine workspace clean status from JJ status output
#[must_use]
pub fn determine_workspace_status(jj_status_output: &str) -> WorkspaceCleanStatus {
    let trimmed = jj_status_output.trim();
    if trimmed.is_empty() {
        return WorkspaceCleanStatus::Clean;
    }

    let has_working_copy = trimmed.contains("Working copy")
        || trimmed.contains("Changes")
        || trimmed.contains("files");

    if has_working_copy && !trimmed.is_empty() {
        WorkspaceCleanStatus::Dirty
    } else if trimmed.is_empty() {
        WorkspaceCleanStatus::Clean
    } else {
        WorkspaceCleanStatus::Unknown
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_sync_input_new() {
        let input = SessionSyncInput::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/workspace"),
            "main".to_string(),
        );

        assert_eq!(input.session_name, "test-session");
        assert_eq!(input.workspace_path, PathBuf::from("/tmp/workspace"));
        assert_eq!(input.main_branch, "main");
        assert!(!input.allow_dirty);
    }

    #[test]
    fn test_session_sync_input_with_dirty_allowed() {
        let input = SessionSyncInput::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/workspace"),
            "main".to_string(),
        )
        .with_dirty_allowed();

        assert!(input.allow_dirty);
    }

    #[test]
    fn test_session_sync_result_creation() {
        let result =
            SessionSyncResult::new("test-session".to_string(), "abc123".to_string(), false);

        assert_eq!(result.session_name, "test-session");
        assert_eq!(result.new_revision, "abc123");
        assert!(!result.had_conflicts);
        assert!(result.synced_at > 0);
    }

    #[test]
    fn test_precondition_check_valid() {
        let check = PreconditionCheck::valid(SessionStatus::Active);
        assert!(check.is_valid());
    }

    #[test]
    fn test_precondition_check_invalid_status() {
        let check = PreconditionCheck {
            session_exists: true,
            current_status: Some(SessionStatus::Creating),
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn test_precondition_check_no_session() {
        let check = PreconditionCheck {
            session_exists: false,
            current_status: None,
            workspace_status: WorkspaceCleanStatus::Clean,
        };
        assert!(!check.is_valid());
    }

    #[test]
    fn test_validate_preconditions_valid() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Clean,
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_failed_status() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Failed),
            WorkspaceCleanStatus::Clean,
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_creating_status() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Creating),
            WorkspaceCleanStatus::Clean,
            false,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SyncError::InvalidSessionStatus { .. }
        ));
    }

    #[test]
    fn test_validate_preconditions_dirty_workspace() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Dirty,
            false,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SyncError::DirtyWorkspace(..)));
    }

    #[test]
    fn test_validate_preconditions_dirty_allowed() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Dirty,
            true,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_unknown_allowed() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Unknown,
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_session_not_found() {
        let result = validate_sync_preconditions(false, None, WorkspaceCleanStatus::Clean, false);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SyncError::SessionNotFound(..)
        ));
    }

    #[test]
    fn test_parse_rebase_output_with_revision() {
        let output = "Rebased 3 commits\nabc123def4567890123456789012345678901\nWorking copy now at: abc123def4567890123456789012345678901";
        let (revision, conflicts) = parse_rebase_output(output);

        assert!(revision.is_some());
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_parse_rebase_output_with_conflicts() {
        let output =
            "Rebase caused conflicts in 2 files:\n  file1.txt\n  file2.txt\nSome conflicts";
        let (revision, conflicts) = parse_rebase_output(output);

        assert!(revision.is_none());
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn test_has_conflicts_in_output() {
        assert!(has_conflicts_in_output("Some conflicts encountered"));
        assert!(has_conflicts_in_output("Conflicted: file.txt"));
        assert!(has_conflicts_in_output("There are 2 conflicts"));
        assert!(!has_conflicts_in_output("Rebased successfully"));
    }

    #[test]
    fn test_determine_workspace_status_clean() {
        let status = determine_workspace_status("");
        assert_eq!(status, WorkspaceCleanStatus::Clean);
    }

    #[test]
    fn test_determine_workspace_status_dirty() {
        let status = determine_workspace_status("Working copy: file.txt\nModified files: 1");
        assert_eq!(status, WorkspaceCleanStatus::Dirty);
    }

    #[test]
    fn test_create_sync_result() {
        let result = create_sync_result("test-session".to_string(), "Rebased successfully\nabc123");

        assert_eq!(result.session_name, "test-session");
        assert_eq!(result.new_revision, "abc123");
    }

    #[test]
    fn test_create_sync_result_with_conflicts() {
        let result = create_sync_result(
            "test-session".to_string(),
            "Conflicted: file.txt\nSome conflicts",
        );

        assert!(result.had_conflicts);
    }

    #[test]
    fn test_sync_error_session_not_found_display() {
        let err = SyncError::SessionNotFound("test-session".to_string());
        assert!(err.to_string().contains("test-session"));
    }

    #[test]
    fn test_sync_error_invalid_status_display() {
        let err = SyncError::InvalidSessionStatus {
            actual: "Creating".to_string(),
            allowed: vec!["Active".to_string(), "Failed".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Creating"));
    }

    #[test]
    fn test_sync_error_dirty_workspace_display() {
        let err = SyncError::DirtyWorkspace("/path/to/workspace".to_string());
        assert!(err.to_string().contains("/path/to/workspace"));
    }

    #[test]
    fn test_sync_error_conflict_display() {
        let err = SyncError::Conflict {
            workspace: "test-workspace".to_string(),
            conflicted_files: vec!["file1.txt".to_string()],
        };
        assert!(err.to_string().contains("test-workspace"));
    }

    #[test]
    fn test_sync_error_rebase_failure_display() {
        let err = SyncError::RebaseFailure {
            workspace: "test-workspace".to_string(),
            reason: "network error".to_string(),
        };
        assert!(err.to_string().contains("test-workspace"));
    }

    #[test]
    fn test_sync_error_jj_command_display() {
        let err = SyncError::JjCommandError("jj not found".to_string());
        assert!(err.to_string().contains("jj"));
    }

    #[test]
    fn test_sync_error_io_display() {
        let err = SyncError::IoError("file not found".to_string());
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_sync_error_to_core_session_not_found() {
        let sync_err = SyncError::SessionNotFound("test".to_string());
        let core_err = CoreError::from(sync_err);

        assert!(matches!(core_err, CoreError::SessionNotFound(s) if s == "test"));
    }

    #[test]
    fn test_sync_error_to_core_conflict() {
        let sync_err = SyncError::Conflict {
            workspace: "test".to_string(),
            conflicted_files: vec![],
        };
        let core_err = CoreError::from(sync_err);

        assert!(matches!(core_err, CoreError::VcsConflict(..)));
    }
}

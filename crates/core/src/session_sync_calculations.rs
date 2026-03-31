//! Session sync calculations - pure validation and state transition functions
//!
//! # Architecture
//!
//! - **Calculations**: Pure validation and state transition functions

use crate::session_sync_data::{PreconditionCheck, SessionSyncResult, WorkspaceCleanStatus};
use crate::session_sync_errors::SyncError;
use crate::types::SessionStatus;

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

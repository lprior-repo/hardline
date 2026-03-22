//! JJ workspace conflict detection and recovery
//!
//! Detects workspace conflicts and provides recovery hints.

use crate::error::JjConflictType;

/// Detect workspace conflict type from error message
#[must_use]
pub fn detect_workspace_conflict(
    stderr: &str,
    _workspace_name: &str,
) -> Option<JjConflictType> {
    stderr.lines().find_map(|line| {
        let line_lower = line.to_lowercase();
        if line_lower.contains("already exists")
            || line_lower.contains("workspace already added")
            || line_lower.contains("already added")
        {
            Some(JjConflictType::AlreadyExists)
        } else if line_lower.contains("concurrent")
            || line_lower.contains("simultaneous")
            || line_lower.contains("locked")
        {
            Some(JjConflictType::ConcurrentModification)
        } else if line_lower.contains("abandoned") {
            Some(JjConflictType::Abandoned)
        } else if line_lower.contains("working copy")
            || line_lower.contains("out of sync")
            || line_lower.contains("stale")
        {
            Some(JjConflictType::Stale)
        } else {
            None
        }
    })
}

/// Generate recovery hint for a conflict type
#[must_use]
pub fn conflict_recovery_hint(
    conflict_type: &JjConflictType,
    workspace_name: &str,
) -> String {
    match conflict_type {
        JjConflictType::AlreadyExists => {
            format!(
                "Recovery options:\n\n 1. Use the existing workspace: jj workspace list\n\n 2. Forget the existing workspace first: jj workspace forget {workspace_name}\n\n 3. Use a different workspace name"
            )
        }
        JjConflictType::ConcurrentModification => {
            "Recovery options:\n\n 1. Wait a moment and retry the operation\n\n 2. Check for other JJ processes: pgrep -fl jj\n\n 3. Verify workspace state: jj workspace list".to_string()
        }
        JjConflictType::Abandoned => {
            format!(
                "Recovery options:\n\n 1. Abandon this workspace: jj workspace forget {workspace_name}\n\n 2. Create a new workspace with a different name\n\n 3. Check repository status: jj status"
            )
        }
        JjConflictType::Stale => {
            "Recovery options:\n\n 1. Update the workspace: jj workspace update-stale\n\n 2. Reload the repository: jj reload\n\n 3. Check for conflicts: jj status".to_string()
        }
    }
}

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

/// Parse rebase output to extract revision and conflicts
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

/// Determine workspace clean status from VCS status output
#[must_use]
pub fn determine_workspace_status(status_output: &str) -> WorkspaceCleanStatus {
    let trimmed = status_output.trim();
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // validate_sync_preconditions tests
    // ========================================================================

    #[test]
    fn validate_preconditions_ok_when_active_clean() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Clean,
            false,
        );
        assert!(result.is_ok());
        let check = result.unwrap();
        assert!(check.session_exists);
        assert_eq!(check.current_status, Some(SessionStatus::Active));
        assert_eq!(check.workspace_status, WorkspaceCleanStatus::Clean);
    }

    #[test]
    fn validate_preconditions_ok_when_failed_clean() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Failed),
            WorkspaceCleanStatus::Clean,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_preconditions_ok_when_active_unknown() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Unknown,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_preconditions_ok_when_dirty_allowed() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Dirty,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_preconditions_err_session_not_found() {
        let result = validate_sync_preconditions(false, None, WorkspaceCleanStatus::Clean, false);
        assert!(matches!(result, Err(SyncError::SessionNotFound(_))));
    }

    #[test]
    fn validate_preconditions_err_creating_status() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Creating),
            WorkspaceCleanStatus::Clean,
            false,
        );
        assert!(matches!(
            result,
            Err(SyncError::InvalidSessionStatus { .. })
        ));
    }

    #[test]
    fn validate_preconditions_err_paused_status() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Paused),
            WorkspaceCleanStatus::Clean,
            false,
        );
        assert!(matches!(
            result,
            Err(SyncError::InvalidSessionStatus { .. })
        ));
    }

    #[test]
    fn validate_preconditions_err_completed_status() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Completed),
            WorkspaceCleanStatus::Clean,
            false,
        );
        assert!(matches!(
            result,
            Err(SyncError::InvalidSessionStatus { .. })
        ));
    }

    #[test]
    fn validate_preconditions_err_no_status() {
        let result = validate_sync_preconditions(true, None, WorkspaceCleanStatus::Clean, false);
        let err = result.unwrap_err();
        assert!(
            matches!(err, SyncError::InvalidSessionStatus { ref actual, .. } if actual == "None")
        );
    }

    #[test]
    fn validate_preconditions_err_dirty_not_allowed() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Active),
            WorkspaceCleanStatus::Dirty,
            false,
        );
        assert!(matches!(result, Err(SyncError::DirtyWorkspace(_))));
    }

    #[test]
    fn validate_preconditions_invalid_status_contains_allowed_list() {
        let result = validate_sync_preconditions(
            true,
            Some(SessionStatus::Paused),
            WorkspaceCleanStatus::Clean,
            false,
        );
        if let Err(SyncError::InvalidSessionStatus { allowed, .. }) = result {
            assert!(allowed.contains(&"Active".to_string()));
            assert!(allowed.contains(&"Failed".to_string()));
        } else {
            panic!("Expected InvalidSessionStatus");
        }
    }

    // ========================================================================
    // parse_rebase_output tests
    // ========================================================================

    #[test]
    fn parse_rebase_output_empty_returns_none() {
        let (rev, conflicts) = parse_rebase_output("");
        assert!(rev.is_none());
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_rebase_output_extracts_hex_revision() {
        // A 12-char hex string looks like a change ID
        let output = "Created new commit\nab12cd34ef56\nDone.";
        let (rev, conflicts) = parse_rebase_output(output);
        assert_eq!(rev.as_deref(), Some("ab12cd34ef56"));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_rebase_output_extracts_long_hex_revision() {
        // A 40-char hex string (SHA-1 length)
        let output = "abc123def456789012345678901234567890abcd";
        let (rev, _conflicts) = parse_rebase_output(output);
        assert_eq!(
            rev.as_deref(),
            Some("abc123def456789012345678901234567890abcd")
        );
    }

    #[test]
    fn parse_rebase_output_skips_too_short_strings() {
        // 5-char hex string should NOT be treated as a revision (< 6 chars)
        let output = "abcde";
        let (rev, _) = parse_rebase_output(output);
        assert!(rev.is_none());
    }

    #[test]
    fn parse_rebase_output_skips_too_long_strings() {
        // 65-char hex string should NOT be treated as a revision (> 64 chars)
        let output = "a".repeat(65);
        let (rev, _) = parse_rebase_output(&output);
        assert!(rev.is_none());
    }

    #[test]
    fn parse_rebase_output_skips_strings_with_colons() {
        // "ab12cd34: extra" should NOT be matched (contains ':')
        let output = "ab12cd34: extra info";
        let (rev, _) = parse_rebase_output(output);
        assert!(rev.is_none());
    }

    #[test]
    fn parse_rebase_output_skips_strings_with_spaces() {
        // "ab12cd34 extra" should NOT be matched (contains space)
        let output = "ab12cd34 extra";
        let (rev, _) = parse_rebase_output(output);
        assert!(rev.is_none());
    }

    #[test]
    fn parse_rebase_output_collects_conflict_lines() {
        let output = "Rebased 1 commit\nConflict in src/main.rs\nAlso conflicted file2.rs";
        let (_rev, conflicts) = parse_rebase_output(output);
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts[0].contains("Conflict"));
        assert!(conflicts[1].contains("conflicted"));
    }

    #[test]
    fn parse_rebase_output_case_insensitive_conflicts() {
        let output = "CONFLICT in file.rs\nAlso Conflicted file2.rs";
        let (_rev, conflicts) = parse_rebase_output(output);
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn parse_rebase_output_returns_last_revision() {
        let output = "ab12cd34ef56\nff88aabb9911\nDone.";
        let (rev, _) = parse_rebase_output(output);
        // Only the last hex-looking line should be captured (if both match)
        assert!(rev.is_some());
    }

    // ========================================================================
    // has_conflicts_in_output tests
    // ========================================================================

    #[test]
    fn has_conflicts_detects_conflict() {
        assert!(has_conflicts_in_output("Conflict detected in 3 files"));
    }

    #[test]
    fn has_conflicts_detects_conflicted() {
        assert!(has_conflicts_in_output("Conflicted: src/lib.rs"));
    }

    #[test]
    fn has_conflicts_detects_some_conflicts() {
        assert!(has_conflicts_in_output(
            "Created 2 new commits. Some conflicts."
        ));
    }

    #[test]
    fn has_conflicts_case_insensitive() {
        assert!(has_conflicts_in_output("CONFLICT IN FILE"));
    }

    #[test]
    fn has_conflicts_returns_false_on_clean_output() {
        assert!(!has_conflicts_in_output("Rebased 3 commits successfully."));
    }

    #[test]
    fn has_conflicts_returns_false_on_empty() {
        assert!(!has_conflicts_in_output(""));
    }

    // ========================================================================
    // create_sync_result tests
    // ========================================================================

    #[test]
    fn create_sync_result_no_conflicts() {
        let result = create_sync_result("s1".into(), "Rebased 3 commits\nabc123def456");
        assert_eq!(result.session_name, "s1");
        assert_eq!(result.new_revision, "abc123def456");
        assert!(!result.had_conflicts);
        assert!(result.synced_at > 0);
    }

    #[test]
    fn create_sync_result_with_conflicts() {
        let result = create_sync_result("s1".into(), "Conflict in file.rs");
        assert!(result.had_conflicts);
    }

    #[test]
    fn create_sync_result_no_revision_uses_unknown() {
        let result = create_sync_result("s1".into(), "Some output with no hex strings");
        assert_eq!(result.new_revision, "unknown");
    }

    // ========================================================================
    // determine_workspace_status tests
    // ========================================================================

    #[test]
    fn workspace_status_empty_is_clean() {
        assert_eq!(determine_workspace_status(""), WorkspaceCleanStatus::Clean);
    }

    #[test]
    fn workspace_status_whitespace_only_is_clean() {
        assert_eq!(
            determine_workspace_status("   \n\t  "),
            WorkspaceCleanStatus::Clean
        );
    }

    #[test]
    fn workspace_status_working_copy_is_dirty() {
        assert_eq!(
            determine_workspace_status("Working copy : file.txt modified"),
            WorkspaceCleanStatus::Dirty
        );
    }

    #[test]
    fn workspace_status_changes_is_dirty() {
        assert_eq!(
            determine_workspace_status("Changes:\n  M file.rs"),
            WorkspaceCleanStatus::Dirty
        );
    }

    #[test]
    fn workspace_status_files_is_dirty() {
        assert_eq!(
            determine_workspace_status("3 files modified"),
            WorkspaceCleanStatus::Dirty
        );
    }

    #[test]
    fn workspace_status_unknown_output_is_unknown() {
        // Something that doesn't match any known pattern
        assert_eq!(
            determine_workspace_status("Some random output"),
            WorkspaceCleanStatus::Unknown
        );
    }
}

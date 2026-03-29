// Session sync tests

use std::path::PathBuf;

use crate::error::Error as CoreError;
use crate::session_sync_calculations::{
    create_sync_result, determine_workspace_status, has_conflicts_in_output, parse_rebase_output,
    validate_sync_preconditions,
};
use crate::session_sync_data::{
    PreconditionCheck, SessionSyncInput, SessionSyncResult, WorkspaceCleanStatus,
};
use crate::session_sync_errors::SyncError;
use crate::types::SessionStatus;

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
    let result = SessionSyncResult::new("test-session".to_string(), "abc123".to_string(), false);

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
    let output = "Rebase caused conflicts in 2 files:\n  file1.txt\n  file2.txt\nSome conflicts";
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

    assert!(matches!(core_err, CoreError::Session(_)));
    assert!(
        core_err.to_string().contains("test"),
        "Expected SessionNotFound with 'test', got: {core_err}"
    );
}

#[test]
fn test_sync_error_to_core_conflict() {
    let sync_err = SyncError::Conflict {
        workspace: "test".to_string(),
        conflicted_files: vec![],
    };
    let core_err = CoreError::from(sync_err);

    assert!(matches!(core_err, CoreError::Vcs(_)));
}

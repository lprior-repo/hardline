//! Snapshot tests for error JSON serialization.
//!
//! These tests verify that the Error type serializes correctly to JSON
//! for CLI output and API responses.

use scp_error::{Error, JjConflictType};

#[test]
fn test_error_workspace_not_found_json() {
    let error = Error::WorkspaceNotFound("test-workspace".into());
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_workspace_not_found", json);
}

#[test]
fn test_error_session_not_found_json() {
    let error = Error::SessionNotFound("test-session".into());
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_session_not_found", json);
}

#[test]
fn test_error_queue_empty_json() {
    let error = Error::QueueEmpty;
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_queue_empty", json);
}

#[test]
fn test_error_vcs_not_initialized_json() {
    let error = Error::VcsNotInitialized;
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_vcs_not_initialized", json);
}

#[test]
fn test_error_jj_command_error_json() {
    let error = Error::JjCommandError {
        operation: "rebase".into(),
        msg: "Source and target are the same".into(),
        is_not_found: false,
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_jj_command", json);
}

#[test]
fn test_error_jj_workspace_conflict_json() {
    let error = Error::JjWorkspaceConflict {
        conflict_type: JjConflictType::AlreadyExists,
        workspace_name: "test-workspace".into(),
        msg: "Workspace already exists".into(),
        recovery_hint: "Remove the existing workspace first".into(),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_jj_workspace_conflict", json);
}

#[test]
fn test_error_validation_field_error_json() {
    let error = Error::ValidationFieldError {
        message: "must not be empty".into(),
        field: "name".into(),
        value: Some("".into()),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_validation_field", json);
}

#[test]
fn test_error_lock_timeout_json() {
    let error = Error::LockTimeout {
        operation: "acquire_lock".into(),
        timeout_ms: 5000,
        retries: 3,
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_lock_timeout", json);
}

#[test]
fn test_error_bead_invalid_state_transition_json() {
    let error = Error::BeadInvalidStateTransition {
        from: "open".into(),
        to: "completed".into(),
    };
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_bead_invalid_transition", json);
}

#[test]
fn test_error_bead_dependency_cycle_json() {
    let error = Error::BeadDependencyCycle("bd-1 -> bd-2 -> bd-3 -> bd-1".into());
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_bead_dependency_cycle", json);
}

#[test]
fn test_error_internal_json() {
    let error = Error::Internal("unexpected null value in cache".into());
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_internal", json);
}

#[test]
fn test_error_database_json() {
    let error = Error::Database("connection refused".into());
    let json = serde_json::to_string(&error).unwrap();
    insta::assert_snapshot!("error_database", json);
}

#[test]
fn test_jj_conflict_type_serialization() {
    let conflict_types = vec![
        (JjConflictType::AlreadyExists, "already_exists"),
        (
            JjConflictType::ConcurrentModification,
            "concurrent_modification",
        ),
        (JjConflictType::Abandoned, "abandoned"),
        (JjConflictType::Stale, "stale"),
    ];

    for (conflict_type, name) in conflict_types {
        let json = serde_json::to_string(&conflict_type).unwrap();
        insta::assert_snapshot!(format!("jj_conflict_type_{}", name.replace(' ', "_")), json);
    }
}

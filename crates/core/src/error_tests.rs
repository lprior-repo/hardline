use crate::error::Error;

#[test]
fn test_error_display() {
    let err = Error::workspace_not_found("test");
    assert!(err.to_string().contains("test"));

    let err = Error::queue_empty();
    assert!(err.to_string().contains("Queue"));
}

#[test]
fn test_error_suggestion_workspace_not_found() {
    let err = Error::workspace_not_found("test");
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("scp"));
}

#[test]
fn test_error_suggestion_session_not_found() {
    let err = Error::session("test");
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
}

#[test]
fn test_error_suggestion_queue_empty() {
    let err = Error::queue_empty();
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("queue"));
}

#[test]
fn test_error_suggestion_workspace_locked() {
    let err = Error::workspace_locked("ws1", "agent1");
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("kill"));
}

#[test]
fn test_error_suggestion_vcs_not_initialized() {
    let err = Error::vcs_not_initialized();
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("init"));
}

#[test]
fn test_error_suggestion_working_copy_dirty() {
    let err = Error::working_copy_dirty();
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    let suggestion_str = suggestion.as_ref().unwrap();
    assert!(suggestion_str.contains("commit") || suggestion_str.contains("stash"));
}

#[test]
fn test_error_no_suggestion() {
    let err = Error::validation_error("test");
    let suggestion = err.suggestion();
    assert!(suggestion.is_none());
}

#[test]
fn test_error_exit_codes_workspace() {
    assert_eq!(Error::workspace_not_found("x").exit_code(), 10);
    assert_eq!(Error::workspace_exists("x").exit_code(), 11);
    assert_eq!(Error::workspace_locked("x", "y").exit_code(), 12);
    assert_eq!(Error::workspace_conflict("x").exit_code(), 13);
}

#[test]
fn test_error_exit_codes_session() {
    assert_eq!(Error::session("x").exit_code(), 14);
    assert_eq!(Error::session_exists("x").exit_code(), 15);
    assert_eq!(Error::session_locked("x", "y").exit_code(), 16);
    assert_eq!(Error::session_invalid_state("x", "y", "z").exit_code(), 17);
}

#[test]
fn test_error_exit_codes_queue() {
    assert_eq!(Error::queue_empty().exit_code(), 20);
    assert_eq!(Error::queue_item_not_found("x").exit_code(), 21);
    assert_eq!(Error::queue_locked("x").exit_code(), 22);
    assert_eq!(Error::queue_processing().exit_code(), 23);
    assert_eq!(Error::queue_invalid_position(1).exit_code(), 24);
    assert_eq!(Error::queue_full(100).exit_code(), 25);
}

#[test]
fn test_error_exit_codes_vcs() {
    assert_eq!(Error::vcs_not_initialized().exit_code(), 30);
    assert_eq!(Error::vcs_conflict("x", "y").exit_code(), 31);
    assert_eq!(Error::vcs_push_failed("x").exit_code(), 32);
    assert_eq!(Error::vcs_pull_failed("x").exit_code(), 33);
    assert_eq!(Error::vcs_rebase_failed("x").exit_code(), 34);
    assert_eq!(Error::branch_not_found("x").exit_code(), 35);
    assert_eq!(Error::branch_exists("x").exit_code(), 36);
    assert_eq!(Error::commit_not_found("x").exit_code(), 37);
    assert_eq!(Error::working_copy_dirty().exit_code(), 38);
}

#[test]
fn test_error_exit_codes_config() {
    assert_eq!(Error::config_not_found("x").exit_code(), 40);
    assert_eq!(Error::config_invalid("x").exit_code(), 41);
    assert_eq!(Error::config_permission("x").exit_code(), 42);
}

#[test]
fn test_error_exit_codes_validation() {
    assert_eq!(Error::validation_error("x").exit_code(), 80);
    assert_eq!(
        Error::validation_field_error("y", "x", None).exit_code(),
        81
    );
    assert_eq!(Error::invalid_identifier("x").exit_code(), 82);
}

#[test]
fn test_error_exit_codes_io() {
    assert_eq!(Error::io_error("test").exit_code(), 60);
    assert_eq!(Error::io_error("invalid json").exit_code(), 64);
}

#[test]
fn test_jj_conflict_type_display() {
    use crate::error::JjConflictType;

    let conflict = JjConflictType::AlreadyExists;
    assert!(format!("{:?}", conflict).contains("AlreadyExists"));

    let conflict = JjConflictType::ConcurrentModification;
    assert!(format!("{:?}", conflict).contains("ConcurrentModification"));

    let conflict = JjConflictType::Abandoned;
    assert!(format!("{:?}", conflict).contains("Abandoned"));

    let conflict = JjConflictType::Stale;
    assert!(format!("{:?}", conflict).contains("Stale"));
}

#[test]
fn test_lock_timeout_error() {
    let err = Error::jj_lock_timeout("test-op", 5000, 3);

    assert!(err.to_string().contains("test-op"));
    assert!(err.to_string().contains("5000"));
    assert!(err.to_string().contains("3"));
    assert_eq!(err.exit_code(), 37);
}

#[test]
fn test_jj_workspace_conflict_error() {
    let err = Error::jj_workspace_conflict(
        crate::error::JjConflictType::AlreadyExists,
        "test-workspace",
        "workspace already exists",
        "use different name",
    );

    assert!(err.to_string().contains("test-workspace"));
    assert!(err.to_string().contains("workspace already exists"));
}

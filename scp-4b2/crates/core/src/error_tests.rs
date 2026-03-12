use crate::error::Error;

#[test]
fn test_error_display() {
    let err = Error::WorkspaceNotFound("test".into());
    assert!(err.to_string().contains("test"));

    let err = Error::QueueEmpty;
    assert!(err.to_string().contains("Queue"));
}

#[test]
fn test_error_suggestion_workspace_not_found() {
    let err = Error::WorkspaceNotFound("test".into());
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("scp"));
}

#[test]
fn test_error_suggestion_session_not_found() {
    let err = Error::SessionNotFound("test".into());
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
}

#[test]
fn test_error_suggestion_queue_empty() {
    let err = Error::QueueEmpty;
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("queue"));
}

#[test]
fn test_error_suggestion_workspace_locked() {
    let err = Error::WorkspaceLocked("ws1".into(), "agent1".into());
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("kill"));
}

#[test]
fn test_error_suggestion_vcs_not_initialized() {
    let err = Error::VcsNotInitialized;
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("init"));
}

#[test]
fn test_error_suggestion_working_copy_dirty() {
    let err = Error::WorkingCopyDirty;
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
    let suggestion_str = suggestion.as_ref().unwrap();
    assert!(suggestion_str.contains("commit") || suggestion_str.contains("stash"));
}

#[test]
fn test_error_no_suggestion() {
    let err = Error::ValidationError("test".into());
    let suggestion = err.suggestion();
    assert!(suggestion.is_none());
}

#[test]
fn test_error_exit_codes_workspace() {
    assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
    assert_eq!(Error::WorkspaceExists("x".into()).exit_code(), 11);
    assert_eq!(
        Error::WorkspaceLocked("x".into(), "y".into()).exit_code(),
        12
    );
    assert_eq!(Error::WorkspaceConflict("x".into()).exit_code(), 13);
}

#[test]
fn test_error_exit_codes_session() {
    assert_eq!(Error::SessionNotFound("x".into()).exit_code(), 14);
    assert_eq!(Error::SessionExists("x".into()).exit_code(), 15);
    assert_eq!(Error::SessionLocked("x".into(), "y".into()).exit_code(), 16);
    assert_eq!(
        Error::SessionInvalidState("x".into(), "y".into(), "z".into()).exit_code(),
        17
    );
}

#[test]
fn test_error_exit_codes_queue() {
    assert_eq!(Error::QueueEmpty.exit_code(), 20);
    assert_eq!(Error::QueueItemNotFound("x".into()).exit_code(), 21);
    assert_eq!(Error::QueueLocked("x".into()).exit_code(), 22);
    assert_eq!(Error::QueueProcessing.exit_code(), 23);
    assert_eq!(Error::QueueInvalidPosition(1).exit_code(), 24);
    assert_eq!(Error::QueueFull(100).exit_code(), 25);
}

#[test]
fn test_error_exit_codes_vcs() {
    assert_eq!(Error::VcsNotInitialized.exit_code(), 30);
    assert_eq!(Error::VcsConflict("x".into(), "y".into()).exit_code(), 31);
    assert_eq!(Error::VcsPushFailed("x".into()).exit_code(), 32);
    assert_eq!(Error::VcsPullFailed("x".into()).exit_code(), 33);
    assert_eq!(Error::VcsRebaseFailed("x".into()).exit_code(), 34);
    assert_eq!(Error::BranchNotFound("x".into()).exit_code(), 35);
    assert_eq!(Error::BranchExists("x".into()).exit_code(), 36);
    assert_eq!(Error::CommitNotFound("x".into()).exit_code(), 37);
    assert_eq!(Error::WorkingCopyDirty.exit_code(), 38);
}

#[test]
fn test_error_exit_codes_config() {
    assert_eq!(Error::ConfigNotFound("x".into()).exit_code(), 40);
    assert_eq!(Error::ConfigInvalid("x".into()).exit_code(), 41);
    assert_eq!(Error::ConfigPermission("x".into()).exit_code(), 42);
}

#[test]
fn test_error_exit_codes_validation() {
    assert_eq!(Error::ValidationError("x".into()).exit_code(), 80);
    assert_eq!(
        Error::ValidationFieldError {
            message: "x".into(),
            field: "y".into(),
            value: None
        }
        .exit_code(),
        81
    );
    assert_eq!(Error::InvalidIdentifier("x".into()).exit_code(), 82);
}

#[test]
fn test_error_exit_codes_io() {
    assert_eq!(
        Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test")).exit_code(),
        60
    );
    assert_eq!(
        Error::JsonParse(serde_json::from_str::<serde_json::Value>("invalid").unwrap_err())
            .exit_code(),
        61
    );
}

#[test]
fn test_error_clone() {
    let err1 = Error::WorkspaceNotFound("test".into());
    let err2 = err1.clone();
    assert_eq!(err1.to_string(), err2.to_string());
}

#[test]
fn test_error_various_clone() {
    let err1 = Error::QueueEmpty;
    let err2 = err1.clone();
    assert_eq!(err1.exit_code(), err2.exit_code());

    let err1 = Error::VcsNotInitialized;
    let err2 = err1.clone();
    assert_eq!(err1.exit_code(), err2.exit_code());

    let err1 = Error::ValidationError("test".into());
    let err2 = err1.clone();
    assert_eq!(err1.exit_code(), err2.exit_code());
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
    let err = Error::LockTimeout {
        operation: "test-op".into(),
        timeout_ms: 5000,
        retries: 3,
    };

    assert!(err.to_string().contains("test-op"));
    assert!(err.to_string().contains("5000"));
    assert!(err.to_string().contains("3"));
    assert_eq!(err.exit_code(), 37);
}

#[test]
fn test_jj_workspace_conflict_error() {
    let err = Error::JjWorkspaceConflict {
        conflict_type: crate::error::JjConflictType::AlreadyExists,
        workspace_name: "test-workspace".into(),
        msg: "workspace already exists".into(),
        recovery_hint: "use different name".into(),
    };

    assert!(err.to_string().contains("test-workspace"));
    assert!(err.to_string().contains("workspace already exists"));
}

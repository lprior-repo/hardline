use crate::error::Error;

#[test]
fn given_session_status_when_serialize_then_lowercase() {
    use crate::types::SessionStatus;

    let state = SessionStatus::Active;
    let serialized = serde_json::to_string(&state).ok();
    assert!(serialized.is_some());

    if let Some(s) = serialized {
        assert!(s.contains("active"));
    }
}

#[test]
fn given_session_status_when_deserialize_then_parses() {
    use crate::types::SessionStatus;

    let json = "\"active\"";
    let deserialized: Result<SessionStatus, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.ok(), Some(SessionStatus::Active));
}

#[test]
fn given_branch_state_when_serialize_then_correct_format() {
    use crate::types::BranchState;

    let detached = BranchState::detached();
    let serialized = serde_json::to_string(&detached).unwrap();
    assert!(serialized.contains("detached"));

    let on_branch = BranchState::on_branch("feature/test");
    let serialized = serde_json::to_string(&on_branch).unwrap();
    assert!(serialized.contains("feature/test"));
}

#[test]
fn given_branch_state_when_deserialize_then_parses() {
    use crate::types::BranchState;

    let json = "\"detached\"";
    let deserialized: Result<BranchState, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());
    assert!(deserialized.unwrap().is_detached());

    let json = "\"main\"";
    let deserialized: Result<BranchState, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.unwrap().branch_name(), Some("main"));
}

#[test]
fn given_file_status_when_serialize_then_single_letter() {
    use crate::types::FileStatus;

    assert_eq!(
        serde_json::to_string(&FileStatus::Modified).unwrap(),
        "\"M\""
    );
    assert_eq!(serde_json::to_string(&FileStatus::Added).unwrap(), "\"A\"");
    assert_eq!(
        serde_json::to_string(&FileStatus::Deleted).unwrap(),
        "\"D\""
    );
    assert_eq!(
        serde_json::to_string(&FileStatus::Renamed).unwrap(),
        "\"R\""
    );
    assert_eq!(
        serde_json::to_string(&FileStatus::Untracked).unwrap(),
        "\"?\""
    );
}

#[test]
fn given_file_status_when_deserialize_then_parses() {
    use crate::types::FileStatus;

    assert_eq!(
        serde_json::from_str::<FileStatus>("\"M\"").unwrap(),
        FileStatus::Modified
    );
    assert_eq!(
        serde_json::from_str::<FileStatus>("\"A\"").unwrap(),
        FileStatus::Added
    );
    assert_eq!(
        serde_json::from_str::<FileStatus>("\"D\"").unwrap(),
        FileStatus::Deleted
    );
}

#[test]
fn given_error_when_serialize_then_includes_message() {
    let err = Error::WorkspaceNotFound("test-workspace".to_string());
    let json = serde_json::to_string(&err).ok();
    assert!(json.is_some());

    if let Some(s) = json {
        assert!(s.contains("test-workspace"));
    }
}

#[test]
fn given_validation_error_when_serialize_then_includes_details() {
    let err = Error::ValidationError("Invalid input".to_string());
    let json = serde_json::to_string(&err).ok();
    assert!(json.is_some());

    if let Some(s) = json {
        assert!(s.contains("Validation"));
    }
}

#[test]
fn given_validation_field_error_when_serialize_then_includes_field() {
    let err = Error::ValidationFieldError {
        message: "must not be empty".to_string(),
        field: "name".to_string(),
        value: Some("".to_string()),
    };
    let json = serde_json::to_string(&err).ok();
    assert!(json.is_some());

    if let Some(s) = json {
        assert!(s.contains("name"));
        assert!(s.contains("must not be empty"));
    }
}

#[test]
fn given_jj_command_error_when_serialize_then_includes_operation() {
    let err = Error::JjCommandError {
        operation: "status".to_string(),
        msg: "failed".to_string(),
        is_not_found: false,
    };
    let json = serde_json::to_string(&err).ok();
    assert!(json.is_some());

    if let Some(s) = json {
        assert!(s.contains("status"));
        assert!(s.contains("failed"));
    }
}

#[test]
fn given_error_with_suggestion() {
    let err = Error::WorkspaceNotFound("test".into());
    let suggestion = err.suggestion();
    assert!(suggestion.is_some());
}

#[test]
fn given_error_exit_codes() {
    assert_eq!(Error::WorkspaceNotFound("x".into()).exit_code(), 10);
    assert_eq!(Error::WorkspaceExists("x".into()).exit_code(), 11);
    assert_eq!(Error::QueueEmpty.exit_code(), 20);
    assert_eq!(Error::VcsNotInitialized.exit_code(), 30);
    assert_eq!(Error::ValidationError("x".into()).exit_code(), 80);
}

#[test]
fn given_changes_summary_when_serialize_then_correct() {
    use crate::types::ChangesSummary;

    let summary = ChangesSummary {
        modified: 5,
        added: 3,
        deleted: 2,
        renamed: 1,
        untracked: 4,
    };

    let json = serde_json::to_string(&summary).ok();
    assert!(json.is_some());

    let deserialized: ChangesSummary = serde_json::from_str(&json.unwrap()).ok().unwrap();
    assert_eq!(deserialized.modified, 5);
    assert_eq!(deserialized.total(), 15);
}

#[test]
fn given_beads_summary_when_serialize_then_correct() {
    use crate::types::BeadsSummary;

    let summary = BeadsSummary {
        open: 3,
        in_progress: 2,
        blocked: 1,
        closed: 5,
    };

    let json = serde_json::to_string(&summary).ok();
    assert!(json.is_some());

    let deserialized: BeadsSummary = serde_json::from_str(&json.unwrap()).ok().unwrap();
    assert_eq!(deserialized.total(), 11);
    assert_eq!(deserialized.active(), 5);
    assert!(deserialized.has_blockers());
}

#[test]
fn given_issue_status_when_serialize_then_lowercase() {
    use crate::types::IssueStatus;

    assert_eq!(
        serde_json::to_string(&IssueStatus::Open).unwrap(),
        "\"open\""
    );
    assert_eq!(
        serde_json::to_string(&IssueStatus::InProgress).unwrap(),
        "\"inprogress\""
    );
    assert_eq!(
        serde_json::to_string(&IssueStatus::Blocked).unwrap(),
        "\"blocked\""
    );
    assert_eq!(
        serde_json::to_string(&IssueStatus::Closed).unwrap(),
        "\"closed\""
    );
}

#[test]
fn given_config_scope_when_serialize_then_correct() {
    use crate::config::ConfigScope;

    assert_eq!(
        serde_json::to_string(&ConfigScope::Global).unwrap(),
        "\"Global\""
    );
    assert_eq!(
        serde_json::to_string(&ConfigScope::Project).unwrap(),
        "\"Project\""
    );
    assert_eq!(serde_json::to_string(&ConfigScope::Env).unwrap(), "\"Env\"");
}

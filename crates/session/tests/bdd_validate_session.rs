//! BDD Validation: scp-session — prove it works before ship
//!
//! Claim Sheet built from types, docs, help text, and error variants.
//! Each claim is tested on the happy path with real terminal output,
//! then adversarial attacks are applied.
//!
//! Categories:
//!   C1-C30:  Happy path claims
//!   ADV1-ADV25: Adversarial attacks

use chrono::Utc;
use scp_session::application::session_service::SessionService;
use scp_session::domain::bead::{Bead, BeadState};
use scp_session::domain::bead_types::{BeadType, Priority};
use scp_session::domain::bead_value::{BeadDescription, BeadId, BeadTitle};
use scp_session::domain::entities::session::{
    BranchState, Created, Session, SessionId, SessionState,
};
use scp_session::domain::events::{
    deserialize_event, serialize_event, SessionCompletedEvent, SessionCreatedEvent,
    SessionEvent, SessionFailedEvent,
};
use scp_session::domain::value_objects::metadata::{
    DependsOn, IssueType, Labels, Priority as MetaPriority, WorkspaceName as MetaWorkspaceName,
};
use scp_session::domain::value_objects::path::AbsolutePath;
use scp_session::domain::value_objects::path::path_validation::find_first_metacharacter;
use scp_session::domain::value_objects::session::{
    BeadId as VoBeadId, SessionName, WorkspaceId as VoWorkspaceId,
};
use scp_session::domain::value_objects::task::{AgentId, Description, TaskId, Title};
use scp_session::domain::workspace::{Workspace, WorkspaceId, WorkspaceName, WorkspacePath};
use scp_session::domain::workspace_state::{WorkspaceState, WorkspaceStateMachine};
use scp_session::error::{SessionError, TaskIdError};
use std::collections::HashSet;

// =========================================================================
// HAPPY PATH CLAIMS (C1-C30)
// =========================================================================

// --- SessionId Claims ---

#[test]
fn c01_session_id_parse_valid_ascii() {
    let id = SessionId::parse("session-abc123").expect("C01: valid ASCII session ID");
    assert_eq!(id.as_str(), "session-abc123");
}

#[test]
fn c02_session_id_generate_has_prefix_and_is_unique() {
    let id = SessionId::generate();
    assert!(id.as_str().starts_with("session-"), "C02: must have session- prefix");
    assert!(!id.as_str().is_empty(), "C02: must be non-empty");
    let id2 = SessionId::generate();
    assert_ne!(id, id2, "C02: must be unique");
}

#[test]
fn c03_session_id_parse_empty_rejects() {
    let result = SessionId::parse("");
    assert!(result.is_err(), "C03: empty must reject");
    assert!(matches!(result.unwrap_err(), SessionError::InvalidIdentifier(_)));
}

#[test]
fn c04_session_id_parse_non_ascii_rejects() {
    let result = SessionId::parse("session-café");
    assert!(result.is_err(), "C04: non-ASCII must reject");
}

// --- SessionName Claims ---

#[test]
fn c05_session_name_parse_valid() {
    let name = SessionName::parse("my-session_name").expect("C05: valid session name");
    assert_eq!(name.as_str(), "my-session_name");
}

#[test]
fn c06_session_name_trims_whitespace() {
    let name = SessionName::parse("  padded  ").expect("C06: trims whitespace");
    assert_eq!(name.as_str(), "padded");
}

#[test]
fn c07_session_name_at_max_length() {
    let max_name = "a".repeat(SessionName::MAX_LENGTH);
    let name = SessionName::parse(&max_name).expect("C07: max length valid");
    assert_eq!(name.as_str().len(), SessionName::MAX_LENGTH);
}

#[test]
fn c08_session_name_exceeds_max_rejects() {
    let too_long = "a".repeat(SessionName::MAX_LENGTH + 1);
    let result = SessionName::parse(&too_long);
    assert!(result.is_err(), "C08: too long must reject");
}

#[test]
fn c09_session_name_empty_rejects() {
    assert!(SessionName::parse("").is_err(), "C09: empty rejects");
    assert!(SessionName::parse("   ").is_err(), "C09: whitespace-only rejects");
}

#[test]
fn c10_session_name_must_start_with_letter() {
    assert!(SessionName::parse("123invalid").is_err(), "C10: starts with number rejects");
    assert!(SessionName::parse("_underscore").is_err(), "C10: starts with underscore rejects");
}

// --- VoBeadId Claims ---

#[test]
fn c11_vo_bead_id_parse_valid() {
    let id = VoBeadId::parse("bd-deadbeef").expect("C11: valid BeadId");
    assert_eq!(id.as_str(), "bd-deadbeef");
}

#[test]
fn c12_vo_bead_id_generate_has_prefix_and_hex() {
    let id = VoBeadId::generate();
    assert!(id.as_str().starts_with("bd-"), "C12: prefix bd-");
    let suffix = &id.as_str()[3..];
    assert!(suffix.len() >= 8, "C12: hex suffix at least 8 chars");
    assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()), "C12: all hex");
}

#[test]
fn c13_vo_bead_id_invalid_formats_reject() {
    assert!(VoBeadId::parse("").is_err(), "C13: empty rejects");
    assert!(VoBeadId::parse("abc123").is_err(), "C13: no prefix rejects");
    assert!(VoBeadId::parse("bd-").is_err(), "C13: empty suffix rejects");
    assert!(VoBeadId::parse("bd-xyz").is_err(), "C13: non-hex rejects");
}

// --- TaskId Claims ---

#[test]
fn c14_task_id_parse_valid() {
    let id = TaskId::parse("bd-abc123").expect("C14: valid TaskId");
    assert_eq!(id.as_str(), "bd-abc123");
}

#[test]
fn c15_task_id_all_error_variants() {
    assert!(matches!(TaskId::parse("").unwrap_err(), TaskIdError::InvalidInput));
    assert!(matches!(TaskId::parse("abc-123").unwrap_err(), TaskIdError::InvalidPrefix));
    assert!(matches!(TaskId::parse("bd-xyz").unwrap_err(), TaskIdError::InvalidHex));
    assert!(matches!(TaskId::parse("bd-").unwrap_err(), TaskIdError::EmptySuffix));
}

// --- Title, Description, AgentId Claims ---

#[test]
fn c16_title_valid_and_trims() {
    let t = Title::new("  Hello World  ").expect("C16: valid");
    assert_eq!(t.as_str(), "Hello World");
}

#[test]
fn c17_title_at_max_boundary() {
    let max = "a".repeat(Title::MAX_LENGTH);
    assert!(Title::new(max).is_ok(), "C17: max length valid");
    assert!(Title::new("a".repeat(Title::MAX_LENGTH + 1)).is_err(), "C17: over max rejects");
}

#[test]
fn c18_description_empty_allowed_and_preserves_whitespace() {
    let d = Description::new("").expect("C18: empty allowed");
    assert_eq!(d.as_str(), "");
    let d2 = Description::new("  spaces  ").expect("C18: whitespace preserved");
    assert_eq!(d2.as_str(), "  spaces  ");
}

#[test]
fn c19_agent_id_non_empty() {
    assert!(AgentId::new("agent-001").is_ok(), "C19: valid agent ID");
    assert!(AgentId::new("").is_err(), "C19: empty rejects");
}

// --- AbsolutePath Claims ---

#[test]
fn c20_absolute_path_valid() {
    let path = AbsolutePath::try_from("/usr/local/bin").expect("C20: valid absolute path");
    assert_eq!(path.to_path_buf(), std::path::PathBuf::from("/usr/local/bin"));
}

#[test]
fn c21_absolute_path_rejects_relative() {
    let result = AbsolutePath::try_from("relative/path");
    assert!(result.is_err(), "C21: relative path rejects");
}

#[test]
fn c22_absolute_path_rejects_metacharacters() {
    assert!(AbsolutePath::try_from("/home/$USER/bin").is_err(), "C22: $ rejects");
    assert!(AbsolutePath::try_from("/tmp/`cmd`").is_err(), "C22: backtick rejects");
    assert!(AbsolutePath::try_from("/tmp/a;b").is_err(), "C22: semicolon rejects");
    assert!(AbsolutePath::try_from("/tmp/a|b").is_err(), "C22: pipe rejects");
    assert!(AbsolutePath::try_from("/tmp/a&b").is_err(), "C22: ampersand rejects");
}

#[test]
fn c23_find_first_metacharacter() {
    let err = find_first_metacharacter("/path/$var").expect("C23: finds $");
    assert!(matches!(err, scp_session::domain::value_objects::path::ShellMetacharacterError::ContainsDollar { .. }));
    let err2 = find_first_metacharacter("/path/`cmd`").expect("C23: finds backtick");
    assert!(matches!(err2, scp_session::domain::value_objects::path::ShellMetacharacterError::ContainsBacktick { .. }));
}

// --- Session Lifecycle Claims ---

#[test]
fn c24_session_create_starts_created() {
    let name = SessionName::parse("test-session").expect("valid");
    let session = Session::<Created>::create(name).expect("C24: created");
    assert_eq!(session.state(), SessionState::Created);
    assert!(matches!(session.branch, BranchState::Detached));
    assert!(session.workspace().is_none());
    assert!(session.bead().is_none());
    assert!(session.last_synced.is_none());
}

#[test]
fn c25_session_full_sync_path() {
    let name = SessionName::parse("sync-path").expect("valid");
    let session = Session::<Created>::create(name).expect("created");
    let active = session.activate().expect("activate");
    assert!(active.is_active());
    let syncing = active.sync().expect("sync");
    assert!(syncing.is_active());
    let synced = syncing.sync_complete().expect("sync_complete");
    assert!(synced.is_active());
    let completed = synced.complete().expect("complete");
    assert!(completed.state().is_terminal());
    assert_eq!(completed.state(), SessionState::Completed);
}

#[test]
fn c26_session_pause_resume_path() {
    let name = SessionName::parse("pause-path").expect("valid");
    let session = Session::<Created>::create(name).expect("created");
    let active = session.activate().expect("activate");
    let paused = active.pause().expect("pause");
    assert_eq!(paused.state(), SessionState::Paused);
    let resumed = paused.resume().expect("resume");
    assert_eq!(resumed.state(), SessionState::Active);
}

#[test]
fn c27_session_fail_from_every_non_terminal_state() {
    // Fail from Created
    let s = Session::<Created>::create(SessionName::parse("f1").expect("v")).unwrap();
    assert_eq!(s.fail().unwrap().state(), SessionState::Failed);
    // Fail from Active
    let s = Session::<Created>::create(SessionName::parse("f2").expect("v")).unwrap().activate().unwrap();
    assert_eq!(s.fail().unwrap().state(), SessionState::Failed);
    // Fail from Syncing
    let s = Session::<Created>::create(SessionName::parse("f3").expect("v")).unwrap().activate().unwrap().sync().unwrap();
    assert_eq!(s.fail().unwrap().state(), SessionState::Failed);
    // Fail from Paused
    let s = Session::<Created>::create(SessionName::parse("f4").expect("v")).unwrap().activate().unwrap().pause().unwrap();
    assert_eq!(s.fail().unwrap().state(), SessionState::Failed);
}

#[test]
fn c28_session_restart_and_retry() {
    let name = SessionName::parse("restart").expect("valid");
    let completed = Session::<Created>::create(name)
        .unwrap()
        .activate()
        .unwrap()
        .complete()
        .unwrap();
    let restarted = completed.restart().expect("restart");
    assert_eq!(restarted.state(), SessionState::Created);

    let failed = Session::<Created>::create(SessionName::parse("retry").expect("valid"))
        .unwrap()
        .fail()
        .unwrap();
    let retried = failed.retry().expect("retry");
    assert_eq!(retried.state(), SessionState::Created);
}

#[test]
fn c29_session_id_name_preserved_through_full_lifecycle() {
    let name = SessionName::parse("persist-test").expect("valid");
    let created = Session::<Created>::create(name).expect("created");
    let id_before = created.id.as_str().to_string();
    let name_before = created.name.as_str().to_string();
    let ts = Utc::now();

    let created = created.transition_branch(BranchState::OnBranch { name: "feature".into() }).expect("branch");
    let created = created.mark_synced(ts).expect("sync");
    let active = created.activate().expect("activate");
    let syncing = active.sync().expect("sync");
    let synced = syncing.sync_complete().expect("sync_complete");
    let completed = synced.complete().expect("complete");

    assert_eq!(completed.id.as_str(), id_before, "C29: ID preserved");
    assert_eq!(completed.name.as_str(), name_before, "C29: name preserved");
    assert_eq!(completed.branch.branch_name(), Some("feature"), "C29: branch preserved");
    assert_eq!(completed.last_synced, Some(ts), "C29: last_synced preserved");
}

#[test]
fn c30_session_branch_transitions() {
    let name = SessionName::parse("branch-test").expect("valid");
    let session = Session::<Created>::create(name).expect("created");
    let on_branch = session.transition_branch(BranchState::OnBranch { name: "main".into() }).expect("C30: to branch");
    assert_eq!(on_branch.branch.branch_name(), Some("main"));
    let detached = on_branch.transition_branch(BranchState::Detached).expect("C30: to detached");
    assert!(detached.branch.is_detached());
    // Detached to detached rejects
    assert!(detached.transition_branch(BranchState::Detached).is_err(), "C30: detached->detached rejects");
}

// --- Bead Aggregate Claims ---

#[test]
fn c31_bead_create_starts_open() {
    let bead = Bead::create(
        BeadId::new("bd-test1").unwrap(),
        BeadTitle::new("Test Bead").unwrap(),
        Some(BeadDescription::new("A test").unwrap()),
    );
    assert_eq!(bead.state(), BeadState::Open);
    assert_eq!(bead.created_at(), bead.updated_at());
    assert!(bead.closed_at().is_none());
    assert!(bead.assignee().is_none());
    assert!(bead.parent().is_none());
    assert!(bead.depends_on().is_empty());
    assert!(bead.blocked_by().is_empty());
}

#[test]
fn c32_bead_full_lifecycle() {
    let bead = Bead::create(
        BeadId::new("bd-lifecycle").unwrap(),
        BeadTitle::new("Lifecycle Bead").unwrap(),
        None,
    );
    assert_eq!(bead.state(), BeadState::Open);

    let in_progress = bead.transition(BeadState::InProgress).expect("C32: to InProgress");
    assert_eq!(in_progress.state(), BeadState::InProgress);

    let blocked = in_progress.transition(BeadState::Blocked).expect("C32: to Blocked");
    assert_eq!(blocked.state(), BeadState::Blocked);

    let resumed = blocked.transition(BeadState::InProgress).expect("C32: back to InProgress");
    let closed = resumed.transition(BeadState::Closed).expect("C32: to Closed");
    assert!(closed.state().is_terminal());
    assert!(closed.closed_at().is_some(), "C32: closed_at set on close");
}

#[test]
fn c33_bead_closed_is_terminal() {
    let bead = Bead::create(
        BeadId::new("bd-terminal").unwrap(),
        BeadTitle::new("Terminal").unwrap(),
        None,
    );
    let closed = bead.transition(BeadState::InProgress).unwrap()
        .transition(BeadState::Closed).unwrap();
    assert!(closed.transition(BeadState::Open).is_err(), "C33: closed->open rejects");
    assert!(closed.transition(BeadState::InProgress).is_err(), "C33: closed->in_progress rejects");
}

#[test]
fn c34_bead_dependencies_no_self_reference() {
    let bead = Bead::create(
        BeadId::new("bd-self").unwrap(),
        BeadTitle::new("Self Ref").unwrap(),
        None,
    );
    let with_dep = bead.add_dependency(BeadId::new("bd-self").unwrap());
    assert_eq!(with_dep.depends_on().len(), 0, "C34: self-reference rejected");
}

#[test]
fn c35_bead_blocked_by_and_is_blocked() {
    let bead = Bead::create(
        BeadId::new("bd-blocked").unwrap(),
        BeadTitle::new("Blocked Bead").unwrap(),
        None,
    );
    assert!(!bead.is_blocked(), "C35: no blockers initially");
    let blocked = bead.add_blocker(BeadId::new("bd-blocker1").unwrap());
    assert!(blocked.is_blocked(), "C35: is_blocked after adding blocker");
    assert_eq!(blocked.blocked_by().len(), 1);
    // Duplicate blocker rejected
    let blocked2 = blocked.add_blocker(BeadId::new("bd-blocker1").unwrap());
    assert_eq!(blocked2.blocked_by().len(), 1, "C35: duplicate blocker rejected");
}

#[test]
fn c36_bead_can_transition_to_always_allows_closed() {
    let bead = Bead::create(
        BeadId::new("bd-ct").unwrap(),
        BeadTitle::new("Can Transition").unwrap(),
        None,
    );
    assert!(bead.can_transition_to(BeadState::Closed), "C36: open->closed allowed");
    let in_progress = bead.transition(BeadState::InProgress).unwrap();
    assert!(in_progress.can_transition_to(BeadState::Closed), "C36: in_progress->closed allowed");
    let closed = in_progress.transition(BeadState::Closed).unwrap();
    assert!(!closed.can_transition_to(BeadState::Open), "C36: closed->open rejected");
}

// --- Workspace Aggregate Claims ---

#[test]
fn c37_workspace_create_starts_created() {
    let ws = Workspace::create(
        WorkspaceName::new("test-ws").unwrap(),
        WorkspacePath::new("/tmp/test").unwrap(),
    ).expect("C37: created");
    assert_eq!(ws.state(), WorkspaceState::Created);
    assert!(ws.id().as_str().starts_with("ws-"));
    assert_eq!(ws.created_at(), ws.updated_at());
}

#[test]
fn c38_workspace_full_happy_path() {
    let ws = Workspace::create(
        WorkspaceName::new("happy-path").unwrap(),
        WorkspacePath::new("/tmp/happy").unwrap(),
    ).expect("created");
    let working = ws.start_working().expect("C38: working");
    assert!(working.is_working());
    let ready = working.mark_ready().expect("C38: ready");
    assert!(ready.is_ready());
    let merged = ready.merge().expect("C38: merged");
    assert!(merged.is_terminal());
    assert_eq!(merged.state(), WorkspaceState::Merged);
}

#[test]
fn c39_workspace_conflict_path() {
    let ws = Workspace::create(
        WorkspaceName::new("conflict-ws").unwrap(),
        WorkspacePath::new("/tmp/conflict").unwrap(),
    ).expect("created");
    let working = ws.start_working().unwrap();
    let ready = working.mark_ready().unwrap();
    let conflict = ready.mark_conflict().expect("C39: conflict");
    assert!(conflict.is_terminal());
}

#[test]
fn c40_workspace_abandon_from_any_non_terminal() {
    // Abandon from Created
    let ws = Workspace::create(WorkspaceName::new("a1").unwrap(), WorkspacePath::new("/tmp/a1").unwrap()).unwrap();
    assert_eq!(ws.abandon().unwrap().state(), WorkspaceState::Abandoned);
    // Abandon from Working
    let ws = Workspace::create(WorkspaceName::new("a2").unwrap(), WorkspacePath::new("/tmp/a2").unwrap()).unwrap();
    assert_eq!(ws.start_working().unwrap().abandon().unwrap().state(), WorkspaceState::Abandoned);
    // Abandon from Ready
    let ws = Workspace::create(WorkspaceName::new("a3").unwrap(), WorkspacePath::new("/tmp/a3").unwrap()).unwrap();
    let ready = ws.start_working().unwrap().mark_ready().unwrap();
    assert_eq!(ready.abandon().unwrap().state(), WorkspaceState::Abandoned);
    // Abandon from terminal (Merged) fails
    let ws = Workspace::create(WorkspaceName::new("a4").unwrap(), WorkspacePath::new("/tmp/a4").unwrap()).unwrap();
    let merged = ws.start_working().unwrap().mark_ready().unwrap().merge().unwrap();
    assert!(merged.abandon().is_err(), "C40: abandon from terminal rejects");
}

// --- WorkspaceState Machine Claims ---

#[test]
fn c41_workspace_state_machine_transitions() {
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Working).is_ok());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Ready).is_ok());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Merged).is_ok());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Conflict).is_ok());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Working, WorkspaceState::Abandoned).is_ok());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Abandoned).is_ok());
    // Invalid transitions
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Ready).is_err());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Merged, WorkspaceState::Working).is_err());
    assert!(WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Created).is_err());
}

// --- BeadState Machine Claims ---

#[test]
fn c42_bead_state_all_transitions() {
    assert!(BeadState::Open.can_transition_to(BeadState::InProgress));
    assert!(BeadState::InProgress.can_transition_to(BeadState::Blocked));
    assert!(BeadState::InProgress.can_transition_to(BeadState::Deferred));
    assert!(BeadState::InProgress.can_transition_to(BeadState::Closed));
    assert!(BeadState::Blocked.can_transition_to(BeadState::InProgress));
    assert!(BeadState::Blocked.can_transition_to(BeadState::Deferred));
    assert!(BeadState::Blocked.can_transition_to(BeadState::Closed));
    assert!(BeadState::Deferred.can_transition_to(BeadState::InProgress));
    assert!(BeadState::Deferred.can_transition_to(BeadState::Closed));
    // Invalid
    assert!(!BeadState::Open.can_transition_to(BeadState::Blocked));
    assert!(!BeadState::Deferred.can_transition_to(BeadState::Blocked));
    assert!(!BeadState::Closed.can_transition_to(BeadState::Open));
    assert!(!BeadState::Open.can_transition_to(BeadState::Open));
}

// --- BeadType and Priority Claims ---

#[test]
fn c43_bead_type_all_variants_and_default() {
    assert_eq!(BeadType::default(), BeadType::Task);
    assert_eq!(BeadType::Bug.as_str(), "bug");
    assert_eq!(BeadType::Feature.as_str(), "feature");
    assert_eq!(BeadType::Task.as_str(), "task");
    assert_eq!(BeadType::Epic.as_str(), "epic");
    assert_eq!(BeadType::Chore.as_str(), "chore");
}

#[test]
fn c44_priority_range_and_default() {
    assert_eq!(Priority::default().as_u8(), 2);
    for p in 0..=4u8 {
        assert!(Priority::new(p).is_ok(), "C44: {p} valid");
    }
    assert!(Priority::new(5).is_err(), "C44: 5 invalid");
    assert!(Priority::new(255).is_err(), "C44: 255 invalid");
}

// --- Metadata Value Objects Claims ---

#[test]
fn c45_labels_validation() {
    assert!(Labels::new(vec![]).is_ok(), "C45: empty labels valid");
    assert!(Labels::new(vec!["a".into(), "b".into()]).is_ok(), "C45: valid labels");
    assert!(Labels::new((0..=Labels::MAX_LABELS).map(|i| format!("l-{i}")).collect()).is_err(), "C45: too many rejects");
    assert!(Labels::new(vec!["dup".into(), "dup".into()]).is_err(), "C45: duplicates reject");
}

#[test]
fn c46_depends_on_validation() {
    assert!(DependsOn::new("bd-abc123").is_ok(), "C46: valid");
    assert!(DependsOn::new("bd-ABCDEF").is_ok(), "C46: uppercase hex");
    assert!(DependsOn::new("").is_err(), "C46: empty rejects");
    assert!(DependsOn::new("abc-123").is_err(), "C46: no prefix rejects");
    assert!(DependsOn::new("bd-").is_err(), "C46: empty suffix rejects");
    assert!(DependsOn::new("bd-xyz").is_err(), "C46: non-hex rejects");
}

#[test]
fn c47_issue_type_validation() {
    for valid in ["bug", "feature", "task", "epic", "chore"] {
        assert!(IssueType::new(valid).is_ok(), "C47: {valid} valid");
    }
    assert!(IssueType::new("invalid").is_err(), "C47: invalid rejects");
    assert!(IssueType::new("").is_err(), "C47: empty rejects");
    assert!(IssueType::new("Bug").is_err(), "C47: case sensitive");
}

#[test]
fn c48_meta_priority_range() {
    for p in 0..=4u8 { assert!(MetaPriority::new(p).is_ok()); }
    assert!(MetaPriority::new(5).is_err(), "C48: 5 rejects");
    assert!(MetaPriority::new(255).is_err(), "C48: 255 rejects");
}

#[test]
fn c49_meta_workspace_name_validation() {
    assert!(MetaWorkspaceName::new("my-ws").is_ok(), "C49: valid");
    assert!(MetaWorkspaceName::new("  padded  ").is_ok(), "C49: trims");
    assert!(MetaWorkspaceName::new("").is_err(), "C49: empty rejects");
    assert!(MetaWorkspaceName::new("   ").is_err(), "C49: whitespace rejects");
    assert!(MetaWorkspaceName::new("a".repeat(MetaWorkspaceName::MAX_LENGTH + 1)).is_err(), "C49: too long rejects");
}

// --- Event Claims ---

#[test]
fn c50_event_serialize_deserialize_roundtrip() {
    let events = [
        SessionEvent::Activated,
        SessionEvent::Syncing,
        SessionEvent::Synced,
        SessionEvent::Paused,
        SessionEvent::Completed,
        SessionEvent::Failed,
    ];
    for event in &events {
        let json = serialize_event(event).expect("C50: serialize");
        let parsed = deserialize_event(&json).expect("C50: deserialize");
        assert_eq!(event, &parsed);
    }
}

#[test]
fn c51_event_structs_timestamp_within_bounds() {
    let before = Utc::now();
    let name = SessionName::parse("bounds").unwrap();
    let created = SessionCreatedEvent::new("s-1".into(), name.clone());
    let completed = SessionCompletedEvent::new("s-2".into(), name.clone());
    let failed = SessionFailedEvent::new("s-3".into(), name, "err".into());
    let after = Utc::now();
    assert!(created.timestamp >= before && created.timestamp <= after, "C51: created timestamp");
    assert!(completed.timestamp >= before && completed.timestamp <= after, "C51: completed timestamp");
    assert!(failed.timestamp >= before && failed.timestamp <= after, "C51: failed timestamp");
}

// --- SessionService Claims ---

#[test]
fn c52_service_full_lifecycle() {
    let created = SessionService::create_session(SessionName::parse("svc-test").unwrap()).unwrap();
    let active = SessionService::activate_session(created).unwrap();
    let completed = SessionService::complete_session(active).unwrap();
    assert!(completed.state().is_terminal());
}

#[test]
fn c53_service_list_returns_empty() {
    assert!(SessionService::list_sessions().unwrap().is_empty(), "C53: stub returns empty");
}

#[test]
fn c54_service_get_session_returns_not_found() {
    let result = SessionService::get_session(SessionId::parse("nonexistent").unwrap());
    assert!(matches!(result, Err(SessionError::NotFound(_))), "C54: returns NotFound");
}

// --- Serde Roundtrip Claims ---

#[test]
fn c55_session_state_serde_all_variants() {
    for state in [SessionState::Created, SessionState::Active, SessionState::Syncing,
                  SessionState::Synced, SessionState::Paused, SessionState::Completed, SessionState::Failed] {
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed, "C55: roundtrip {:?}", state);
    }
}

#[test]
fn c56_bead_serde_roundtrip() {
    let bead = Bead::create(
        BeadId::new("bd-serde").unwrap(),
        BeadTitle::new("Serde Bead").unwrap(),
        Some(BeadDescription::new("desc").unwrap()),
    ).with_priority(Priority::new(0).unwrap())
     .with_type(BeadType::Bug)
     .with_assignee("alice");
    let json = serde_json::to_string(&bead).unwrap();
    let parsed: Bead = serde_json::from_str(&json).unwrap();
    assert_eq!(bead.id().as_str(), parsed.id().as_str());
    assert_eq!(bead.state(), parsed.state());
    assert_eq!(bead.bead_type(), parsed.bead_type());
}

// =========================================================================
// ADVERSARIAL ATTACKS (ADV1-ADV25)
// =========================================================================

#[test]
fn adv01_session_name_null_bytes() {
    let result = SessionName::parse("test\0name");
    assert!(result.is_err(), "ADV01: null byte in session name rejects");
}

#[test]
fn adv02_session_name_unicode() {
    let result = SessionName::parse("café-session");
    assert!(result.is_err(), "ADV02: non-ASCII unicode rejects");
}

#[test]
fn adv03_session_name_special_chars() {
    for name in ["test@session", "test.session", "test session", "test/session"] {
        assert!(SessionName::parse(name).is_err(), "ADV03: '{name}' rejects");
    }
}

#[test]
fn adv04_task_id_mixed_invalid_chars() {
    // Uppercase hex IS valid (A-F are hex digits)
    assert!(TaskId::parse("bd-ABCDEF").is_ok(), "ADV04: uppercase A-F hex valid");
    assert!(TaskId::parse("bd-abcdef").is_ok(), "ADV04: lowercase a-f hex valid");
    // G is NOT a hex digit
    assert!(TaskId::parse("bd-12345G").is_err(), "ADV04: G not hex");
    assert!(TaskId::parse("bd-ghijkl").is_err(), "ADV04: g-l not hex");
    // TaskId requires lowercase 'bd-' prefix (starts_with check)
    assert!(TaskId::parse("BD-abc123").is_err(), "ADV04: uppercase BD- prefix rejects");
}

#[test]
fn adv05_bead_id_value_special_chars() {
    for id in ["bd 123", "bd.123", "bd@123", "bd/123", "bd:123"] {
        assert!(BeadId::new(id).is_err(), "ADV05: '{id}' rejects");
    }
}

#[test]
fn adv06_absolute_path_traversal() {
    assert!(AbsolutePath::try_from("/safe/path").is_ok(), "ADV06: normal path ok");
    // These should pass path validation (they're absolute and no metacharacters)
    // but the crate doesn't specifically block traversal sequences
    let result = AbsolutePath::try_from("/tmp/../etc/passwd");
    assert!(result.is_ok(), "ADV06: traversal is absolute and shell-safe (no metacharacters)");
}

#[test]
fn adv07_absolute_path_shell_injection() {
    assert!(AbsolutePath::try_from("/$(whoami)").is_err(), "ADV07: $() injection rejects");
    assert!(AbsolutePath::try_from("/`whoami`").is_err(), "ADV07: backtick injection rejects");
    assert!(AbsolutePath::try_from("/tmp;rm -rf /").is_err(), "ADV07: semicolon injection rejects");
    assert!(AbsolutePath::try_from("/tmp|cat /etc/passwd").is_err(), "ADV07: pipe injection rejects");
    assert!(AbsolutePath::try_from("/tmp&&echo pwned").is_err(), "ADV07: ampersand injection rejects");
}

#[test]
fn adv08_workspace_path_validation() {
    assert!(WorkspacePath::new("").is_err(), "ADV08: empty rejects");
    assert!(WorkspacePath::new("bare-name").is_err(), "ADV08: bare name rejects");
    assert!(WorkspacePath::new("/valid/path").is_ok(), "ADV08: absolute ok");
    assert!(WorkspacePath::new("./relative").is_ok(), "ADV08: relative with dot ok");
}

#[test]
fn adv09_bead_title_boundary() {
    assert!(BeadTitle::new("").is_err(), "ADV09: empty rejects");
    assert!(BeadTitle::new("   ").is_err(), "ADV09: whitespace-only rejects");
    let max = "x".repeat(BeadTitle::MAX_LENGTH);
    assert!(BeadTitle::new(max).is_ok(), "ADV09: max length ok");
    assert!(BeadTitle::new("x".repeat(BeadTitle::MAX_LENGTH + 1)).is_err(), "ADV09: over max rejects");
}

#[test]
fn adv10_bead_description_none_behavior() {
    let desc = BeadDescription::new("");
    assert!(desc.unwrap().is_empty(), "ADV10: empty -> None -> is_empty");
    let desc = BeadDescription::new("   ");
    assert!(desc.unwrap().is_empty(), "ADV10: whitespace -> None -> is_empty");
    let desc = BeadDescription::new("content");
    assert!(!desc.unwrap().is_empty(), "ADV10: content -> Some -> not is_empty");
}

#[test]
fn adv11_workspace_state_invalid_transitions() {
    // Cannot go backwards
    assert!(!WorkspaceState::Working.can_transition_to(WorkspaceState::Created));
    assert!(!WorkspaceState::Ready.can_transition_to(WorkspaceState::Created));
    assert!(!WorkspaceState::Ready.can_transition_to(WorkspaceState::Working));
    assert!(!WorkspaceState::Merged.can_transition_to(WorkspaceState::Ready));
    assert!(!WorkspaceState::Conflict.can_transition_to(WorkspaceState::Ready));
    // Terminal states cannot transition anywhere
    for terminal in [WorkspaceState::Merged, WorkspaceState::Conflict, WorkspaceState::Abandoned] {
        for target in WorkspaceState::all() {
            assert!(!terminal.can_transition_to(target), "ADV11: {:?} -> {:?} must be invalid", terminal, target);
        }
    }
}

#[test]
fn adv12_bead_state_invalid_transitions() {
    // Cannot skip states
    assert!(!BeadState::Open.can_transition_to(BeadState::Closed), "ADV12: open->closed (via state machine, but Bead allows it via can_transition_to override)");
    // NOTE: Bead::can_transition_to always returns true for Closed target (Q16)
    // So this is actually valid per the Bead aggregate's design
    // Let's verify the Bead transition works
    let bead = Bead::create(BeadId::new("bd-direct").unwrap(), BeadTitle::new("Direct").unwrap(), None);
    let result = bead.transition(BeadState::Closed);
    assert!(result.is_ok(), "ADV12: open->closed is allowed by Bead (Q16)");

    // But closed->anything fails
    let closed = result.unwrap();
    assert!(!closed.can_transition_to(BeadState::Open));
    assert!(!closed.can_transition_to(BeadState::InProgress));
    assert!(!closed.can_transition_to(BeadState::Blocked));
    assert!(!closed.can_transition_to(BeadState::Deferred));
}

#[test]
fn adv13_session_from_parts_with_all_fields() {
    let id = SessionId::parse("preset-id").unwrap();
    let name = SessionName::parse("preset-name").unwrap();
    let ws = VoWorkspaceId::parse("ws-test").unwrap();
    let bd = VoBeadId::parse("bd-abc123").unwrap();
    let branch = BranchState::OnBranch { name: "dev".into() };
    let ts = Utc::now();

    let session = Session::from_parts(
        id.clone(), name.clone(), Some(ws.clone()), Some(bd.clone()),
        branch.clone(), Some(ts), Utc::now(),
    );

    assert_eq!(session.id.as_str(), "preset-id");
    assert_eq!(session.name.as_str(), "preset-name");
    assert_eq!(session.workspace().map(|w| w.as_str()), Some("ws-test"));
    assert_eq!(session.bead().map(|b| b.as_str()), Some("bd-abc123"));
    assert_eq!(session.branch.branch_name(), Some("dev"));
    assert_eq!(session.last_synced, Some(ts));
}

#[test]
fn adv14_bead_add_dependency_duplicate_rejected() {
    let bead = Bead::create(
        BeadId::new("bd-dep").unwrap(),
        BeadTitle::new("Dep Test").unwrap(),
        None,
    );
    let dep_id = BeadId::new("bd-dep1").unwrap();
    let with_dep = bead.add_dependency(dep_id.clone());
    assert_eq!(with_dep.depends_on().len(), 1);
    let with_dup = with_dep.add_dependency(dep_id);
    assert_eq!(with_dup.depends_on().len(), 1, "ADV14: duplicate dependency rejected");
}

#[test]
fn adv15_bead_add_blocker_self_reference_rejected() {
    let bead = Bead::create(
        BeadId::new("bd-self-block").unwrap(),
        BeadTitle::new("Self Block").unwrap(),
        None,
    );
    let with_self = bead.add_blocker(BeadId::new("bd-self-block").unwrap());
    assert_eq!(with_self.blocked_by().len(), 0, "ADV15: self-blocker rejected");
}

#[test]
fn adv16_session_mark_synced_with_edge_timestamps() {
    let name = SessionName::parse("sync-edges").unwrap();
    let session = Session::<Created>::create(name).unwrap();

    let past = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00+00:00").unwrap().with_timezone(&Utc);
    let synced = session.mark_synced(past).unwrap();
    assert_eq!(synced.last_synced, Some(past), "ADV16: past timestamp works");

    let future = Utc::now() + chrono::Duration::days(365);
    let synced2 = synced.mark_synced(future).unwrap();
    assert_eq!(synced2.last_synced, Some(future), "ADV16: future timestamp works");
}

#[test]
fn adv17_labels_boundary_max() {
    let labels_vec: Vec<String> = (0..Labels::MAX_LABELS).map(|i| format!("label-{i}")).collect();
    let labels = Labels::new(labels_vec).expect("ADV17: at max boundary");
    assert_eq!(labels.as_slice().len(), Labels::MAX_LABELS);
}

#[test]
fn adv18_issue_type_case_sensitivity() {
    assert!(IssueType::new("BUG").is_err(), "ADV18: uppercase BUG rejects");
    assert!(IssueType::new("Feature").is_err(), "ADV18: capitalized rejects");
    assert!(IssueType::new("TASK").is_err(), "ADV18: uppercase TASK rejects");
}

#[test]
fn adv19_workspace_name_max_boundary() {
    let max = "w".repeat(WorkspaceName::MAX_LENGTH);
    assert!(WorkspaceName::new(max).is_ok(), "ADV19: max workspace name ok");
    assert!(WorkspaceName::new("w".repeat(WorkspaceName::MAX_LENGTH + 1)).is_err(), "ADV19: over max rejects");
}

#[test]
fn adv20_workspace_id_empty_rejects() {
    assert!(WorkspaceId::new("").is_err(), "ADV20: empty workspace ID rejects");
}

#[test]
fn adv21_description_max_boundary() {
    let max = "x".repeat(Description::MAX_LENGTH);
    assert!(Description::new(max).is_ok(), "ADV21: at max ok");
    assert!(Description::new("x".repeat(Description::MAX_LENGTH + 1)).is_err(), "ADV21: over max rejects");
}

#[test]
fn adv22_session_state_is_terminal_only_completed_and_failed() {
    for state in [SessionState::Created, SessionState::Active, SessionState::Syncing,
                  SessionState::Synced, SessionState::Paused] {
        assert!(!state.is_terminal(), "ADV22: {:?} not terminal", state);
    }
    assert!(SessionState::Completed.is_terminal());
    assert!(SessionState::Failed.is_terminal());
}

#[test]
fn adv23_bead_state_all_returns_exactly_five() {
    let all = BeadState::all();
    assert_eq!(all.len(), 5, "ADV23: exactly 5 bead states");
    let unique: HashSet<_> = all.iter().collect();
    assert_eq!(unique.len(), 5, "ADV23: all unique");
}

#[test]
fn adv24_workspace_state_all_returns_exactly_six() {
    let all = WorkspaceState::all();
    assert_eq!(all.len(), 6, "ADV24: exactly 6 workspace states");
    let unique: HashSet<_> = all.iter().collect();
    assert_eq!(unique.len(), 6, "ADV24: all unique");
}

#[test]
fn adv25_event_deserialize_invalid_json_fails() {
    let result: Result<SessionEvent, _> = serde_json::from_str("\"invalid\"");
    assert!(result.is_err(), "ADV25: invalid event JSON rejects");
    let result2: Result<SessionEvent, _> = serde_json::from_str("not even json");
    assert!(result2.is_err(), "ADV25: malformed JSON rejects");
}

// =========================================================================
// CROSS-CUTTING: Serde consistency for all enums
// =========================================================================

#[test]
fn xcut01_bead_state_serde_roundtrip() {
    for state in BeadState::all() {
        let json = serde_json::to_string(&state).unwrap();
        let parsed: BeadState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed, "XCUT01: roundtrip {:?}", state);
    }
}

#[test]
fn xcut02_bead_type_serde_roundtrip() {
    for bt in [BeadType::Bug, BeadType::Feature, BeadType::Task, BeadType::Epic, BeadType::Chore] {
        let json = serde_json::to_string(&bt).unwrap();
        let parsed: BeadType = serde_json::from_str(&json).unwrap();
        assert_eq!(bt, parsed, "XCUT02: roundtrip {:?}", bt);
    }
}

#[test]
fn xcut03_workspace_state_serde_roundtrip() {
    for state in WorkspaceState::all() {
        let json = serde_json::to_string(&state).unwrap();
        let parsed: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed, "XCUT03: roundtrip {:?}", state);
    }
}

#[test]
fn xcut04_branch_state_serde_roundtrip() {
    for bs in [BranchState::Detached, BranchState::OnBranch { name: "main".into() }] {
        let json = serde_json::to_string(&bs).unwrap();
        let parsed: BranchState = serde_json::from_str(&json).unwrap();
        assert_eq!(bs, parsed, "XCUT04: roundtrip {:?}", bs);
    }
}

#[test]
fn xcut05_workspace_serde_roundtrip_through_lifecycle() {
    let ws = Workspace::create(
        WorkspaceName::new("serde-ws").unwrap(),
        WorkspacePath::new("/tmp/serde").unwrap(),
    ).unwrap();
    let working = ws.start_working().unwrap();
    let ready = working.mark_ready().unwrap();
    let merged = ready.merge().unwrap();
    let json = serde_json::to_string(&merged).unwrap();
    let parsed: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(merged.id(), parsed.id());
    assert_eq!(merged.state(), parsed.state());
}

// =========================================================================
// STRESS: Bulk operations
// =========================================================================

#[test]
fn stress01_bulk_session_creation() {
    let mut ids = HashSet::new();
    for i in 0..100 {
        let name = SessionName::parse(&format!("stress-{i}")).unwrap();
        let session = Session::<Created>::create(name).unwrap();
        assert!(ids.insert(session.id.as_str().to_string()), "STRESS01: unique ID at iteration {i}");
    }
}

#[test]
fn stress02_bulk_bead_creation_and_lifecycle() {
    for i in 0..50 {
        let bead = Bead::create(
            BeadId::new(&format!("bd-stress-{i}")).unwrap(),
            BeadTitle::new(&format!("Stress Bead {i}")).unwrap(),
            None,
        );
        let in_progress = bead.transition(BeadState::InProgress).unwrap();
        let closed = in_progress.transition(BeadState::Closed).unwrap();
        assert!(closed.state().is_terminal());
    }
}

#[test]
fn stress03_bulk_bead_id_generate_uniqueness() {
    let mut ids = HashSet::new();
    for _ in 0..200 {
        let id = VoBeadId::generate();
        assert!(ids.insert(id.as_str().to_string()), "STRESS03: unique generated ID");
    }
}

#[test]
fn stress04_bulk_labels_at_boundary() {
    let labels_vec: Vec<String> = (0..Labels::MAX_LABELS).map(|i| format!("stress-label-{i}")).collect();
    let labels = Labels::new(labels_vec).unwrap();
    assert_eq!(labels.as_slice().len(), Labels::MAX_LABELS);
    // Verify all unique
    let unique: HashSet<_> = labels.as_slice().iter().collect();
    assert_eq!(unique.len(), Labels::MAX_LABELS);
}

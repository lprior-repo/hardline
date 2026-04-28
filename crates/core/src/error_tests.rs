use proptest::{prelude::*, prop_assert};

use crate::error::Error;

// ========================================================================
// Exhaustive Error Variant Generator
// ========================================================================
//
// Instead of a single monolithic strategy, we build a `Vec<Error>` that
// covers every leaf variant in the hierarchy. Each helper constructs one
// Error::Variant(SubKind::Leaf) with sensible defaults so the proptests
// below can verify invariants across the *entire* error surface.

/// Returns an Error for every leaf variant in the hierarchy.
///
/// This is intentionally not a proptest `Arbitrary` impl because the Error
/// enum wraps private inner types (`#[from] inner: XErrorKind`) that we
/// cannot construct outside their defining modules. Instead we use the
/// public constructor methods on `Error` and the `From<XErrorKind> for Error`
/// conversions that are available via `use`.
fn all_error_variants() -> Vec<Error> {
    use crate::{
        coordination::locks::errors::LockErrorKind,
        error_agent::AgentErrorKind,
        error_config::ConfigErrorKind,
        error_internal::InternalErrorKind,
        error_io::IoErrorKind,
        error_queue::QueueErrorKind,
        error_state::StateErrorKind,
        error_task::TaskErrorKind,
        error_vcs::VcsErrorKind,
        error_wait::WaitErrorKind,
        error_workspace::{SessionErrorKind, WorkspaceErrorKind},
    };

    let s = |i: i32| format!("s{i}");
    let mut errors = Vec::new();

    // -- Workspace (4 kinds) --
    errors.push(Error::from(WorkspaceErrorKind::NotFound(s(0))));
    errors.push(Error::from(WorkspaceErrorKind::Exists(s(1))));
    errors.push(Error::from(WorkspaceErrorKind::Locked(s(2), s(3))));
    errors.push(Error::from(WorkspaceErrorKind::Conflict(s(4))));

    // -- Session (5 kinds) --
    errors.push(Error::from(SessionErrorKind::NotFound(s(10))));
    errors.push(Error::from(SessionErrorKind::Exists(s(11))));
    errors.push(Error::from(SessionErrorKind::Locked(s(12), s(13))));
    errors.push(Error::from(SessionErrorKind::NotLockHolder(s(14), s(15))));
    errors.push(Error::from(SessionErrorKind::InvalidState(
        s(16),
        s(17),
        s(18),
    )));

    // -- Queue (6 kinds) --
    errors.push(Error::from(QueueErrorKind::Empty));
    errors.push(Error::from(QueueErrorKind::ItemNotFound(s(20))));
    errors.push(Error::from(QueueErrorKind::Locked(s(21))));
    errors.push(Error::from(QueueErrorKind::Processing));
    errors.push(Error::from(QueueErrorKind::InvalidPosition(42)));
    errors.push(Error::from(QueueErrorKind::Full(100)));

    // -- VCS (14 kinds) --
    errors.push(Error::from(VcsErrorKind::NotInitialized));
    errors.push(Error::from(VcsErrorKind::Conflict(s(30), s(31))));
    errors.push(Error::from(VcsErrorKind::PushFailed(s(32))));
    errors.push(Error::from(VcsErrorKind::PullFailed(s(33))));
    errors.push(Error::from(VcsErrorKind::RebaseFailed(s(34))));
    errors.push(Error::from(VcsErrorKind::BranchNotFound(s(35))));
    errors.push(Error::from(VcsErrorKind::BranchExists(s(36))));
    errors.push(Error::from(VcsErrorKind::CommitNotFound(s(37))));
    errors.push(Error::from(VcsErrorKind::WorkingCopyDirty));
    errors.push(Error::from(VcsErrorKind::CommitFailed(s(38))));
    errors.push(Error::from(VcsErrorKind::CheckoutFailed(s(39))));
    errors.push(Error::from(VcsErrorKind::DiffFailed(s(40))));
    errors.push(Error::from(VcsErrorKind::MergeNoCommitId));
    errors.push(Error::from(VcsErrorKind::InitFailed {
        vcs_type: s(41),
        directory: s(42),
        reason: s(43),
    }));

    // -- Config (11 kinds) --
    errors.push(Error::from(ConfigErrorKind::ConfigKeyNotFound(s(50))));
    errors.push(Error::from(ConfigErrorKind::ConfigParseError(s(51))));
    errors.push(Error::from(ConfigErrorKind::ConfigWriteError(s(52))));
    errors.push(Error::from(ConfigErrorKind::ConfigScopeError(s(53))));
    errors.push(Error::from(ConfigErrorKind::ConfigLockError(s(54))));
    errors.push(Error::from(ConfigErrorKind::NotFound(s(55))));
    errors.push(Error::from(ConfigErrorKind::Invalid(s(56))));
    errors.push(Error::from(ConfigErrorKind::Permission(s(57))));
    errors.push(Error::from(ConfigErrorKind::SecuritySymlink(s(58))));
    errors.push(Error::from(ConfigErrorKind::FileTooLarge(s(59))));
    errors.push(Error::from(ConfigErrorKind::WatcherError(s(60))));
    errors.push(Error::from(ConfigErrorKind::DeadSymlink(s(61))));

    // -- Agent (3 kinds) --
    errors.push(Error::from(AgentErrorKind::NotFound(s(70))));
    errors.push(Error::from(AgentErrorKind::Exists(s(71))));
    errors.push(Error::from(AgentErrorKind::Timeout(s(72))));

    // -- IO (5 kinds) --
    errors.push(Error::from(IoErrorKind::IoError(s(80))));
    errors.push(Error::from(IoErrorKind::Database(s(81))));
    errors.push(Error::from(IoErrorKind::JsonParse(
        serde_json::from_str::<serde_json::Value>("invalid").expect_err("should fail to parse"),
    )));
    errors.push(Error::from(IoErrorKind::YamlParse(
        serde_yaml::from_str::<serde_yaml::Value>(": invalid").expect_err("should fail to parse"),
    )));
    errors.push(Error::from(IoErrorKind::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file missing",
    ))));

    // -- State (5 kinds) --
    errors.push(Error::from(StateErrorKind::InvalidState(s(90))));
    errors.push(Error::from(StateErrorKind::NotFound(s(91))));
    errors.push(Error::from(StateErrorKind::ValidationError(s(92))));
    errors.push(Error::from(StateErrorKind::ValidationFieldError {
        field: s(93),
        message: s(94),
        value: None,
    }));
    errors.push(Error::from(StateErrorKind::ValidationFieldError {
        field: s(95),
        message: s(96),
        value: Some(s(97)),
    }));
    errors.push(Error::from(StateErrorKind::InvalidIdentifier(s(98))));

    // -- Internal (7 kinds) --
    errors.push(Error::from(InternalErrorKind::Internal(s(100))));
    errors.push(Error::from(InternalErrorKind::Unimplemented(s(101))));
    errors.push(Error::from(InternalErrorKind::InvalidConfig(s(102))));
    errors.push(Error::from(InternalErrorKind::CloneFailed(s(103))));
    errors.push(Error::from(InternalErrorKind::RecordFailed(s(104))));
    errors.push(Error::from(InternalErrorKind::InvalidRepoUrl(s(105))));
    errors.push(Error::from(InternalErrorKind::InvalidOperation(s(106))));

    // -- Task (6 kinds) --
    errors.push(Error::from(TaskErrorKind::NotFound(s(120))));
    errors.push(Error::from(TaskErrorKind::AlreadyClaimed(s(121), s(122))));
    errors.push(Error::from(TaskErrorKind::NotClaimed(s(123))));
    errors.push(Error::from(TaskErrorKind::Locked(s(124))));
    errors.push(Error::from(TaskErrorKind::InvalidId(s(125))));
    errors.push(Error::from(TaskErrorKind::InvalidStateTransition(
        s(126),
        s(127),
    )));

    // -- Wait (8 kinds) --
    errors.push(Error::from(WaitErrorKind::Timeout(s(130), s(131))));
    errors.push(Error::from(WaitErrorKind::InvalidWaitMode(s(132))));
    errors.push(Error::from(WaitErrorKind::InvalidSessionName(s(133))));
    errors.push(Error::from(WaitErrorKind::BatchEmpty));
    errors.push(Error::from(WaitErrorKind::BatchCommandFailed(s(134))));
    errors.push(Error::from(WaitErrorKind::BatchRollbackFailed(s(135))));
    errors.push(Error::from(WaitErrorKind::CheckpointError(s(136))));
    errors.push(Error::from(WaitErrorKind::BatchSizeExceeded(999)));

    // -- Lock (12 kinds) --
    errors.push(Error::from(LockErrorKind::SessionNotFound {
        session: s(140),
    }));
    errors.push(Error::from(LockErrorKind::SessionLocked {
        session: s(141),
        holder: s(142),
    }));
    errors.push(Error::from(LockErrorKind::NotLockHolder {
        session: s(143),
        agent_id: s(144),
    }));
    errors.push(Error::from(LockErrorKind::NotFound(s(145))));
    errors.push(Error::from(LockErrorKind::DatabaseError(s(146))));
    errors.push(Error::from(LockErrorKind::ParseError(s(147))));
    errors.push(Error::from(LockErrorKind::Unknown(s(148))));
    errors.push(Error::from(LockErrorKind::TtlOutOfRange(s(149))));
    errors.push(Error::from(LockErrorKind::EmptySessionName(s(150))));
    errors.push(Error::from(LockErrorKind::EmptyAgentId(s(151))));
    errors.push(Error::from(LockErrorKind::TtlOverflow(s(152))));
    errors.push(Error::from(LockErrorKind::SessionNameTooLong(s(153))));
    errors.push(Error::from(LockErrorKind::InvalidSessionName(s(154))));

    errors
}

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
    assert_eq!(Error::session_invalid_state("x", "y", "z").exit_code(), 18);
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
    assert_eq!(Error::io_error("test").exit_code(), 64);
    assert_eq!(Error::io_error("invalid json").exit_code(), 64);
}

// ========================================================================
// Property-Based Tests: Exhaustive Error Hierarchy Invariants
// ========================================================================

proptest! {
    // Invariant 1: Every Error variant has a non-zero exit_code().
    //
    // We use prop_oneof to pick a random leaf variant from the exhaustive
    // list so that each proptest run focuses on a different variant while
    // still covering all of them over many iterations.
    #[test]
    fn prop_exit_code_is_nonzero(idx in 0..200usize) {
        let errors = all_error_variants();
        // Wrap around so any index is valid
        let err = &errors[idx % errors.len()];
        prop_assert!(err.exit_code() != 0,
            "exit_code() returned 0 for: {err}");
    }

    // Invariant 2: Every Error variant has a Display impl that produces
    // a non-empty string.
    #[test]
    fn prop_display_is_nonempty(idx in 0..200usize) {
        let errors = all_error_variants();
        let err = &errors[idx % errors.len()];
        let display = err.to_string();
        prop_assert!(!display.is_empty(),
            "Display produced empty string for: {err:?}");
    }

    // Invariant 3: Error::code() returns SCREAMING_SNAKE_CASE for all
    // variants. We check: non-empty, only uppercase/underscores.
    // Single-word codes like UNIMPLEMENTED are valid SCREAMING_SNAKE_CASE.
    #[test]
    fn prop_code_is_screaming_snake_case(idx in 0..200usize) {
        let errors = all_error_variants();
        let err = &errors[idx % errors.len()];
        let code = err.code();
        prop_assert!(!code.is_empty(),
            "code() returned empty for: {err:?}");
        prop_assert!(code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "code() is not SCREAMING_SNAKE_CASE: {code} (from {err:?})");
    }

    // Invariant 4: suggestion() never panics for any variant.
    // This is a "doesn't panic" property -- the fact that proptest
    // reaches the assertion proves no panic occurred.
    #[test]
    fn prop_suggestion_no_panic(idx in 0..200usize) {
        let errors = all_error_variants();
        let err = &errors[idx % errors.len()];
        // Just call it -- if it panics, the test fails.
        let _ = err.suggestion();
        // If we reach here, no panic occurred.
        prop_assert!(true);
    }

    // Invariant 5: context_map() never panics for any variant.
    #[test]
    fn prop_context_map_no_panic(idx in 0..200usize) {
        let errors = all_error_variants();
        let err = &errors[idx % errors.len()];
        let _ = err.context_map();
        prop_assert!(true);
    }

    // Invariant 6: from() conversions are lossless.
    //
    // For errors constructed via `From<XErrorKind> for Error`, the Display
    // output of the original XErrorKind must be preserved through the
    // conversion. We verify this by checking that the Display of the
    // top-level Error contains the same content as the Display of the
    // source error kind.
    #[test]
    fn prop_from_conversion_preserves_display(msg in "[a-zA-Z ]{1,100}") {
        use crate::error_agent::AgentErrorKind;
        use crate::error_internal::InternalErrorKind;
        use crate::error_io::IoErrorKind;
        use crate::error_state::StateErrorKind;
        use crate::error_workspace::{SessionErrorKind, WorkspaceErrorKind};

        // Pick a few representative conversions and verify display preservation.

        // WorkspaceErrorKind::NotFound
        let kind = WorkspaceErrorKind::NotFound(msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));

        // SessionErrorKind::Locked
        let kind = SessionErrorKind::Locked(msg.clone(), msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));

        // AgentErrorKind::Timeout
        let kind = AgentErrorKind::Timeout(msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));

        // IoErrorKind::IoError
        let kind = IoErrorKind::IoError(msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));

        // StateErrorKind::ValidationError
        let kind = StateErrorKind::ValidationError(msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));

        // InternalErrorKind::Unimplemented
        let kind = InternalErrorKind::Unimplemented(msg.clone());
        let kind_display = kind.to_string();
        let err: Error = kind.into();
        prop_assert!(err.to_string().contains(&kind_display));
    }
}

// ========================================================================
// Exhaustive (non-proptest) coverage checks
// ========================================================================

#[test]
fn exhaustive_all_variants_have_nonzero_exit_code() {
    let errors = all_error_variants();
    let total = errors.len();
    for (i, err) in errors.iter().enumerate() {
        assert_ne!(
            err.exit_code(),
            0,
            "exit_code() == 0 for variant #{i}: {err:?}"
        );
    }
    // Sanity: we actually tested a substantial number of variants.
    assert!(
        total >= 75,
        "expected >= 75 leaf error variants, got {total}"
    );
}

#[test]
fn exhaustive_all_variants_have_nonempty_display() {
    let errors = all_error_variants();
    for (i, err) in errors.iter().enumerate() {
        let display = err.to_string();
        assert!(
            !display.is_empty(),
            "Display is empty for variant #{i}: {err:?}"
        );
    }
}

#[test]
fn exhaustive_all_codes_are_screaming_snake_case() {
    let errors = all_error_variants();
    for (i, err) in errors.iter().enumerate() {
        let code = err.code();
        assert!(
            !code.is_empty(),
            "code() is empty for variant #{i}: {err:?}"
        );
        assert!(
            code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "code() is not SCREAMING_SNAKE_CASE: '{code}' for variant #{i}: {err:?}"
        );
    }
}

#[test]
fn exhaustive_suggestion_never_panics() {
    let errors = all_error_variants();
    for (i, err) in errors.iter().enumerate() {
        let _suggestion = err.suggestion();
        // If we get here, no panic.
        assert!(true, "suggestion() panicked for variant #{i}: {err:?}");
    }
}

#[test]
fn exhaustive_context_map_never_panics() {
    let errors = all_error_variants();
    for (i, err) in errors.iter().enumerate() {
        let _ctx = err.context_map();
        // If we get here, no panic.
        assert!(true, "context_map() panicked for variant #{i}: {err:?}");
    }
}

#[test]
fn exhaustive_from_conversion_display_preserved() {
    use crate::{
        coordination::locks::errors::LockErrorKind,
        error_agent::AgentErrorKind,
        error_config::ConfigErrorKind,
        error_internal::InternalErrorKind,
        error_io::IoErrorKind,
        error_queue::QueueErrorKind,
        error_state::StateErrorKind,
        error_task::TaskErrorKind,
        error_vcs::VcsErrorKind,
        error_wait::WaitErrorKind,
        error_workspace::{SessionErrorKind, WorkspaceErrorKind},
    };

    let msg = "lossless-display-test-payload";

    // Workspace
    let kind = WorkspaceErrorKind::Conflict(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Session
    let kind = SessionErrorKind::InvalidState(msg.into(), msg.into(), msg.into());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Queue
    let kind = QueueErrorKind::ItemNotFound(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // VCS
    let kind = VcsErrorKind::PushFailed(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Config
    let kind = ConfigErrorKind::ConfigKeyNotFound(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Agent
    let kind = AgentErrorKind::Exists(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // IO
    let kind = IoErrorKind::Database(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // State
    let kind = StateErrorKind::InvalidIdentifier(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Internal
    let kind = InternalErrorKind::RecordFailed(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Task
    let kind = TaskErrorKind::AlreadyClaimed(msg.into(), msg.into());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Wait
    let kind = WaitErrorKind::BatchCommandFailed(msg.to_string());
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));

    // Lock
    let kind = LockErrorKind::SessionLocked {
        session: msg.into(),
        holder: msg.into(),
    };
    let display = kind.to_string();
    let err: Error = kind.into();
    assert!(err.to_string().contains(&display));
}

//! Error classification and mapping logic
//!
//! Converts hierarchical `crate::error::Error` variants into semantic exit codes
//! and machine-readable `ErrorCode` values for JSON output.

use crate::error::Error;

use super::error_code::ErrorCode;

/// Classify an error into a semantic exit code.
///
/// Exit codes follow this semantic mapping:
/// - 1: Usage/validation errors (invalid config, parse errors, validation failures)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors (hooks, etc.)
/// - 5: Lock contention errors
pub fn classify_exit_code(error: &crate::error::Error) -> i32 {
    match error {
        // Usage/validation errors: exit code 1
        Error::Config(_) => 1,
        // State errors: depends on kind
        Error::State(e) => {
            use crate::error_state::StateErrorKind;
            match e.kind() {
                StateErrorKind::NotFound(_) => 2,
                _ => 1,
            }
        }
        // Not found errors: exit code 2
        Error::Session(e) => {
            let code = e.exit_code();
            if matches!(code, 14 | 15) { 2 } else { code }
        }
        Error::Workspace(e) => {
            let code = e.exit_code();
            if matches!(code, 10 | 11) { 2 } else { code }
        }
        Error::Vcs(e) => {
            use crate::error_vcs::VcsErrorKind;
            match e.kind() {
                VcsErrorKind::NotInitialized => 1,
                VcsErrorKind::BranchNotFound(_) | VcsErrorKind::CommitNotFound(_) => 2,
                VcsErrorKind::WorkingCopyDirty => 1,
                _ => e.exit_code(),
            }
        }
        // System errors: exit code 3
        Error::Io(_) | Error::Agent(_) | Error::Queue(_) | Error::Task(_) | Error::Wait(_) => 3,
        // Lock contention errors: exit code 5
        Error::Lock(_) => 5,
        // Internal errors: exit code 4 (treated as external/unknown)
        Error::Internal(_) => 4,
    }
}

/// Map a `crate::Error` to (`ErrorCode`, message, optional suggestion)
pub fn map_error_to_parts(err: &crate::error::Error) -> (ErrorCode, String, Option<String>) {
    let message = err.to_string();
    let suggestion = err.suggestion();

    match err {
        // Workspace errors
        Error::Workspace(e) => {
            let msg = e.to_string();
            match e.exit_code() {
                10 => (ErrorCode::WorkspaceNotFound, msg, suggestion),
                _ => (ErrorCode::Unknown, msg, suggestion),
            }
        }
        // Session errors
        Error::Session(e) => {
            let msg = e.to_string();
            match e.exit_code() {
                14 => (ErrorCode::SessionNotFound, msg, suggestion),
                _ => (ErrorCode::Unknown, msg, suggestion),
            }
        }
        // Queue errors
        Error::Queue(e) => (ErrorCode::Unknown, e.to_string(), suggestion),
        // VCS errors
        Error::Vcs(e) => map_vcs_error(e),
        // Config errors
        Error::Config(_) => (
            ErrorCode::ConfigParseError,
            format!("Invalid configuration: {message}"),
            Some("Check your configuration file for errors".to_string()),
        ),
        // Agent errors
        Error::Agent(_) => (ErrorCode::Unknown, message, suggestion),
        // IO errors
        Error::Io(_) => (ErrorCode::Unknown, message, suggestion),
        // State errors (validation, not-found, invalid-state, etc.)
        Error::State(_) => (ErrorCode::InvalidArgument, message, suggestion),
        // Internal errors
        Error::Internal(_) => (ErrorCode::Unknown, message, suggestion),
        // Task errors
        Error::Task(_) => (ErrorCode::Unknown, message, suggestion),
        // Wait/Batch errors
        Error::Wait(_) => (ErrorCode::Unknown, message, suggestion),
        // Lock errors
        Error::Lock(_) => (ErrorCode::Unknown, message, suggestion),
    }
}

/// Map a VCS error to its error code, message, and suggestion.
fn map_vcs_error(e: &crate::error_vcs::VcsError) -> (ErrorCode, String, Option<String>) {
    let msg = e.to_string();
    let suggestion = e.suggestion();
    use crate::error_vcs::VcsErrorKind;
    match e.kind() {
        VcsErrorKind::NotInitialized => (
            ErrorCode::NotGitRepository,
            "VCS not initialized".to_string(),
            Some("Run 'scp init' to initialize VCS".to_string()),
        ),
        VcsErrorKind::Conflict(_, _) => (
            ErrorCode::Unknown,
            msg,
            Some("Resolve conflicts before continuing".to_string()),
        ),
        VcsErrorKind::PushFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::PullFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::RebaseFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::BranchNotFound(_) => (ErrorCode::SpawnBeadNotFound, msg, None),
        VcsErrorKind::BranchExists(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::CommitNotFound(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::WorkingCopyDirty => (
            ErrorCode::Unknown,
            msg,
            Some("Commit or stash your changes before continuing".to_string()),
        ),
        VcsErrorKind::CommitFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::CheckoutFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::DiffFailed(_) => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::MergeNoCommitId => (ErrorCode::Unknown, msg, suggestion),
        VcsErrorKind::InitFailed { .. } => (ErrorCode::Unknown, msg, suggestion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_state::StateErrorKind;
    use crate::error_vcs::VcsErrorKind;
    use crate::error_workspace::{SessionErrorKind, WorkspaceErrorKind};
    use crate::error_config::ConfigErrorKind;
    use crate::error_agent::AgentErrorKind;
    use crate::error_queue::QueueErrorKind;
    use crate::error_task::TaskErrorKind;
    use crate::error_wait::WaitErrorKind;
    use crate::error_internal::InternalErrorKind;
    use crate::error_io::IoErrorKind;
    use crate::coordination::locks::errors::LockErrorKind;

    // ========================================================================
    // classify_exit_code tests
    // ========================================================================

    // -- Exit code 1: Usage/validation errors --

    #[test]
    fn classify_exit_code_config_returns_1() {
        let err: crate::error::Error = ConfigErrorKind::ConfigParseError("bad key".into()).into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_state_invalid_state_returns_1() {
        let err: crate::error::Error = StateErrorKind::InvalidState("bad".into()).into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_state_validation_error_returns_1() {
        let err: crate::error::Error = StateErrorKind::ValidationError("bad".into()).into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_state_validation_field_error_returns_1() {
        let err: crate::error::Error = StateErrorKind::ValidationFieldError {
            message: "bad".into(),
            field: "name".into(),
            value: None,
        }
        .into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_state_invalid_identifier_returns_1() {
        let err: crate::error::Error = StateErrorKind::InvalidIdentifier("bad!".into()).into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_vcs_not_initialized_returns_1() {
        let err: crate::error::Error = VcsErrorKind::NotInitialized.into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_vcs_working_copy_dirty_returns_1() {
        let err: crate::error::Error = VcsErrorKind::WorkingCopyDirty.into();
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn classify_exit_code_internal_returns_4() {
        let err: crate::error::Error = InternalErrorKind::Internal("bug".into()).into();
        assert_eq!(classify_exit_code(&err), 4);
    }

    #[test]
    fn classify_exit_code_internal_unimplemented_returns_4() {
        let err: crate::error::Error = InternalErrorKind::Unimplemented("todo".into()).into();
        assert_eq!(classify_exit_code(&err), 4);
    }

    // -- Exit code 2: Not found errors --

    #[test]
    fn classify_exit_code_state_not_found_returns_2() {
        let err: crate::error::Error = StateErrorKind::NotFound("missing".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_session_not_found_returns_2() {
        // Session NotFound has exit_code 14, which matches the `14 | 15` -> 2 branch
        let err: crate::error::Error = SessionErrorKind::NotFound("s1".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_session_exists_returns_2() {
        // Session Exists has exit_code 15, which matches the `14 | 15` -> 2 branch
        let err: crate::error::Error = SessionErrorKind::Exists("s1".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_session_locked_returns_passthrough_code() {
        // Session Locked has exit_code 16, not in {14, 15}, so passthrough
        let err: crate::error::Error = SessionErrorKind::Locked("s1".into(), "a1".into()).into();
        assert_eq!(classify_exit_code(&err), 16);
    }

    #[test]
    fn classify_exit_code_session_not_lock_holder_returns_passthrough_code() {
        let err: crate::error::Error =
            SessionErrorKind::NotLockHolder("s1".into(), "a2".into()).into();
        assert_eq!(classify_exit_code(&err), 17);
    }

    #[test]
    fn classify_exit_code_session_invalid_state_returns_passthrough_code() {
        let err: crate::error::Error =
            SessionErrorKind::InvalidState("s1".into(), "active".into(), "paused".into()).into();
        assert_eq!(classify_exit_code(&err), 18);
    }

    #[test]
    fn classify_exit_code_workspace_not_found_returns_2() {
        // Workspace NotFound has exit_code 10, matches {10, 11} -> 2 branch
        let err: crate::error::Error = WorkspaceErrorKind::NotFound("w1".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_workspace_exists_returns_2() {
        // Workspace Exists has exit_code 11, matches {10, 11} -> 2 branch
        let err: crate::error::Error = WorkspaceErrorKind::Exists("w1".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_workspace_locked_returns_passthrough_code() {
        // Workspace Locked has exit_code 12, not in {10, 11}, so passthrough
        let err: crate::error::Error =
            WorkspaceErrorKind::Locked("w1".into(), "a1".into()).into();
        assert_eq!(classify_exit_code(&err), 12);
    }

    #[test]
    fn classify_exit_code_workspace_conflict_returns_passthrough_code() {
        // Workspace Conflict has exit_code 13, not in {10, 11}, so passthrough
        let err: crate::error::Error = WorkspaceErrorKind::Conflict("race".into()).into();
        assert_eq!(classify_exit_code(&err), 13);
    }

    #[test]
    fn classify_exit_code_vcs_branch_not_found_returns_2() {
        let err: crate::error::Error = VcsErrorKind::BranchNotFound("main".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    #[test]
    fn classify_exit_code_vcs_commit_not_found_returns_2() {
        let err: crate::error::Error = VcsErrorKind::CommitNotFound("abc123".into()).into();
        assert_eq!(classify_exit_code(&err), 2);
    }

    // -- Exit code 3: System errors --

    #[test]
    fn classify_exit_code_io_returns_3() {
        let err: crate::error::Error = IoErrorKind::IoError("file missing".into()).into();
        assert_eq!(classify_exit_code(&err), 3);
    }

    #[test]
    fn classify_exit_code_agent_returns_3() {
        let err: crate::error::Error = AgentErrorKind::NotFound("a1".into()).into();
        assert_eq!(classify_exit_code(&err), 3);
    }

    #[test]
    fn classify_exit_code_queue_returns_3() {
        let err: crate::error::Error = QueueErrorKind::Empty.into();
        assert_eq!(classify_exit_code(&err), 3);
    }

    #[test]
    fn classify_exit_code_task_returns_3() {
        let err: crate::error::Error = TaskErrorKind::NotFound("t1".into()).into();
        assert_eq!(classify_exit_code(&err), 3);
    }

    #[test]
    fn classify_exit_code_wait_returns_3() {
        let err: crate::error::Error =
            WaitErrorKind::Timeout("s1".into(), "idle".into()).into();
        assert_eq!(classify_exit_code(&err), 3);
    }

    // -- Exit code 5: Lock contention errors --

    #[test]
    fn classify_exit_code_lock_returns_5() {
        let err: crate::error::Error = LockErrorKind::SessionLocked {
            session: "s1".into(),
            holder: "a1".into(),
        }
        .into();
        assert_eq!(classify_exit_code(&err), 5);
    }

    #[test]
    fn classify_exit_code_lock_session_not_found_returns_5() {
        let err: crate::error::Error = LockErrorKind::SessionNotFound {
            session: "s1".into(),
        }
        .into();
        assert_eq!(classify_exit_code(&err), 5);
    }

    #[test]
    fn classify_exit_code_lock_ttl_out_of_range_returns_5() {
        let err: crate::error::Error =
            LockErrorKind::TtlOutOfRange("TTL must be >= 0".into()).into();
        assert_eq!(classify_exit_code(&err), 5);
    }

    // -- VCS passthrough exit codes --

    #[test]
    fn classify_exit_code_vcs_conflict_returns_passthrough_code() {
        // VcsErrorKind::Conflict exit_code is 31, not a special case, so passthrough
        let err: crate::error::Error =
            VcsErrorKind::Conflict("file.rs".into(), "content".into()).into();
        assert_eq!(classify_exit_code(&err), 31);
    }

    #[test]
    fn classify_exit_code_vcs_push_failed_returns_passthrough_code() {
        let err: crate::error::Error = VcsErrorKind::PushFailed("network".into()).into();
        assert_eq!(classify_exit_code(&err), 32);
    }

    #[test]
    fn classify_exit_code_vcs_branch_exists_returns_passthrough_code() {
        let err: crate::error::Error = VcsErrorKind::BranchExists("main".into()).into();
        assert_eq!(classify_exit_code(&err), 36);
    }

    // ========================================================================
    // map_error_to_parts tests
    // ========================================================================

    // -- Workspace error mappings --

    #[test]
    fn map_error_to_parts_workspace_not_found() {
        let err: crate::error::Error = WorkspaceErrorKind::NotFound("w1".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::WorkspaceNotFound);
        assert!(msg.contains("w1"));
    }

    #[test]
    fn map_error_to_parts_workspace_exists_maps_to_unknown() {
        let err: crate::error::Error = WorkspaceErrorKind::Exists("w1".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("w1"));
    }

    // -- Session error mappings --

    #[test]
    fn map_error_to_parts_session_not_found() {
        let err: crate::error::Error = SessionErrorKind::NotFound("s1".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::SessionNotFound);
        assert!(msg.contains("s1"));
    }

    #[test]
    fn map_error_to_parts_session_exists_maps_to_unknown() {
        let err: crate::error::Error = SessionErrorKind::Exists("s1".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("s1"));
    }

    // -- Queue error mappings --

    #[test]
    fn map_error_to_parts_queue_empty_maps_to_unknown() {
        let err: crate::error::Error = QueueErrorKind::Empty.into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("empty") || msg.contains("Empty"));
    }

    #[test]
    fn map_error_to_parts_queue_item_not_found_maps_to_unknown() {
        let err: crate::error::Error = QueueErrorKind::ItemNotFound("q1".into()).into();
        let (code, _msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
    }

    // -- VCS error mappings --

    #[test]
    fn map_error_to_parts_vcs_not_initialized() {
        let err: crate::error::Error = VcsErrorKind::NotInitialized.into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::NotGitRepository);
        assert_eq!(msg, "VCS not initialized");
        assert_eq!(
            suggestion,
            Some("Run 'scp init' to initialize VCS".to_string())
        );
    }

    #[test]
    fn map_error_to_parts_vcs_conflict() {
        let err: crate::error::Error =
            VcsErrorKind::Conflict("file.rs".into(), "merge conflict".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("file.rs"));
        assert_eq!(
            suggestion,
            Some("Resolve conflicts before continuing".to_string())
        );
    }

    #[test]
    fn map_error_to_parts_vcs_branch_not_found() {
        let err: crate::error::Error = VcsErrorKind::BranchNotFound("feature".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::SpawnBeadNotFound);
        assert!(msg.contains("feature"));
        assert!(suggestion.is_none());
    }

    #[test]
    fn map_error_to_parts_vcs_working_copy_dirty() {
        let err: crate::error::Error = VcsErrorKind::WorkingCopyDirty.into();
        let (code, _msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert_eq!(
            suggestion,
            Some("Commit or stash your changes before continuing".to_string())
        );
    }

    #[test]
    fn map_error_to_parts_vcs_push_failed() {
        let err: crate::error::Error = VcsErrorKind::PushFailed("network".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("network"));
    }

    #[test]
    fn map_error_to_parts_vcs_pull_failed() {
        let err: crate::error::Error = VcsErrorKind::PullFailed("timeout".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn map_error_to_parts_vcs_rebase_failed() {
        let err: crate::error::Error = VcsErrorKind::RebaseFailed("conflict".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("conflict"));
    }

    #[test]
    fn map_error_to_parts_vcs_branch_exists() {
        let err: crate::error::Error = VcsErrorKind::BranchExists("main".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("main"));
    }

    #[test]
    fn map_error_to_parts_vcs_commit_not_found() {
        let err: crate::error::Error = VcsErrorKind::CommitNotFound("abc".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("abc"));
    }

    #[test]
    fn map_error_to_parts_vcs_commit_failed() {
        let err: crate::error::Error = VcsErrorKind::CommitFailed("disk full".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn map_error_to_parts_vcs_checkout_failed() {
        let err: crate::error::Error = VcsErrorKind::CheckoutFailed("race".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("race"));
    }

    #[test]
    fn map_error_to_parts_vcs_diff_failed() {
        let err: crate::error::Error = VcsErrorKind::DiffFailed("binary".into()).into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("binary"));
    }

    #[test]
    fn map_error_to_parts_vcs_merge_no_commit_id() {
        let err: crate::error::Error = VcsErrorKind::MergeNoCommitId.into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("no commit ID"));
    }

    #[test]
    fn map_error_to_parts_vcs_init_failed() {
        let err: crate::error::Error = VcsErrorKind::InitFailed {
            vcs_type: "git".into(),
            directory: "/tmp".into(),
            reason: "not found".into(),
        }
        .into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("git"));
    }

    // -- Config error mapping --

    #[test]
    fn map_error_to_parts_config() {
        let err: crate::error::Error = ConfigErrorKind::ConfigParseError("bad TOML".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::ConfigParseError);
        assert!(msg.starts_with("Invalid configuration:"));
        assert!(msg.contains("bad TOML"));
        assert_eq!(
            suggestion,
            Some("Check your configuration file for errors".to_string())
        );
    }

    // -- State error mapping --

    #[test]
    fn map_error_to_parts_state_not_found() {
        let err: crate::error::Error = StateErrorKind::NotFound("resource".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::InvalidArgument);
        assert!(msg.contains("resource"));
        // StateError::NotFound has a suggestion
        assert!(suggestion.is_some());
    }

    #[test]
    fn map_error_to_parts_state_validation() {
        let err: crate::error::Error =
            StateErrorKind::ValidationFieldError {
                message: "too short".into(),
                field: "name".into(),
                value: Some("a".into()),
            }
            .into();
        let (code, msg, _suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::InvalidArgument);
        assert!(msg.contains("name"));
    }

    // -- Agent error mapping --

    #[test]
    fn map_error_to_parts_agent() {
        let err: crate::error::Error = AgentErrorKind::NotFound("a1".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("a1"));
        assert!(suggestion.is_none());
    }

    // -- IO error mapping --

    #[test]
    fn map_error_to_parts_io() {
        let err: crate::error::Error = IoErrorKind::IoError("disk full".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("disk full"));
        assert!(suggestion.is_none());
    }

    // -- Internal error mapping --

    #[test]
    fn map_error_to_parts_internal() {
        let err: crate::error::Error = InternalErrorKind::Internal("invariant violated".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("invariant violated"));
        assert!(suggestion.is_none());
    }

    // -- Task error mapping --

    #[test]
    fn map_error_to_parts_task() {
        let err: crate::error::Error = TaskErrorKind::NotFound("t1".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("t1"));
        assert!(suggestion.is_none());
    }

    // -- Wait error mapping --

    #[test]
    fn map_error_to_parts_wait() {
        let err: crate::error::Error =
            WaitErrorKind::Timeout("s1".into(), "idle".into()).into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("s1"));
        assert!(suggestion.is_none());
    }

    // -- Lock error mapping --

    #[test]
    fn map_error_to_parts_lock() {
        let err: crate::error::Error = LockErrorKind::SessionLocked {
            session: "s1".into(),
            holder: "a1".into(),
        }
        .into();
        let (code, msg, suggestion) = map_error_to_parts(&err);
        assert_eq!(code, ErrorCode::Unknown);
        assert!(msg.contains("s1"));
        // LockError::SessionLocked has a suggestion
        assert!(suggestion.is_some());
        assert!(suggestion.as_ref().is_some_and(|s| s.contains("agent kill")));
    }

    // ========================================================================
    // Exhaustiveness / consistency checks
    // ========================================================================

    #[test]
    fn classify_exit_code_never_returns_zero() {
        // Every Error variant should produce a non-zero exit code
        let errors: Vec<crate::error::Error> = vec![
            ConfigErrorKind::ConfigParseError("x".into()).into(),
            StateErrorKind::InvalidState("x".into()).into(),
            StateErrorKind::NotFound("x".into()).into(),
            StateErrorKind::ValidationError("x".into()).into(),
            StateErrorKind::ValidationFieldError {
                message: "x".into(),
                field: "f".into(),
                value: None,
            }
            .into(),
            StateErrorKind::InvalidIdentifier("x".into()).into(),
            SessionErrorKind::NotFound("x".into()).into(),
            SessionErrorKind::Exists("x".into()).into(),
            SessionErrorKind::Locked("x".into(), "y".into()).into(),
            SessionErrorKind::NotLockHolder("x".into(), "y".into()).into(),
            SessionErrorKind::InvalidState("x".into(), "y".into(), "z".into()).into(),
            WorkspaceErrorKind::NotFound("x".into()).into(),
            WorkspaceErrorKind::Exists("x".into()).into(),
            WorkspaceErrorKind::Locked("x".into(), "y".into()).into(),
            WorkspaceErrorKind::Conflict("x".into()).into(),
            VcsErrorKind::NotInitialized.into(),
            VcsErrorKind::Conflict("a".into(), "b".into()).into(),
            VcsErrorKind::PushFailed("x".into()).into(),
            VcsErrorKind::PullFailed("x".into()).into(),
            VcsErrorKind::RebaseFailed("x".into()).into(),
            VcsErrorKind::BranchNotFound("x".into()).into(),
            VcsErrorKind::BranchExists("x".into()).into(),
            VcsErrorKind::CommitNotFound("x".into()).into(),
            VcsErrorKind::WorkingCopyDirty.into(),
            VcsErrorKind::CommitFailed("x".into()).into(),
            VcsErrorKind::CheckoutFailed("x".into()).into(),
            VcsErrorKind::DiffFailed("x".into()).into(),
            VcsErrorKind::MergeNoCommitId.into(),
            VcsErrorKind::InitFailed {
                vcs_type: "git".into(),
                directory: "/tmp".into(),
                reason: "x".into(),
            }
            .into(),
            IoErrorKind::IoError("x".into()).into(),
            AgentErrorKind::NotFound("x".into()).into(),
            AgentErrorKind::Exists("x".into()).into(),
            AgentErrorKind::Timeout("x".into()).into(),
            QueueErrorKind::Empty.into(),
            QueueErrorKind::ItemNotFound("x".into()).into(),
            QueueErrorKind::Locked("x".into()).into(),
            QueueErrorKind::Processing.into(),
            QueueErrorKind::InvalidPosition(0).into(),
            QueueErrorKind::Full(10).into(),
            InternalErrorKind::Internal("x".into()).into(),
            InternalErrorKind::Unimplemented("x".into()).into(),
            TaskErrorKind::NotFound("x".into()).into(),
            TaskErrorKind::AlreadyClaimed("x".into(), "y".into()).into(),
            TaskErrorKind::NotClaimed("x".into()).into(),
            TaskErrorKind::Locked("x".into()).into(),
            TaskErrorKind::InvalidId("x".into()).into(),
            TaskErrorKind::InvalidStateTransition("x".into(), "y".into()).into(),
            WaitErrorKind::Timeout("x".into(), "y".into()).into(),
            WaitErrorKind::InvalidWaitMode("x".into()).into(),
            WaitErrorKind::InvalidSessionName("x".into()).into(),
            WaitErrorKind::BatchEmpty.into(),
            WaitErrorKind::BatchCommandFailed("x".into()).into(),
            WaitErrorKind::BatchRollbackFailed("x".into()).into(),
            WaitErrorKind::CheckpointError("x".into()).into(),
            WaitErrorKind::BatchSizeExceeded(100).into(),
            LockErrorKind::SessionLocked {
                session: "x".into(),
                holder: "y".into(),
            }
            .into(),
            LockErrorKind::NotLockHolder {
                session: "x".into(),
                agent_id: "y".into(),
            }
            .into(),
            LockErrorKind::NotFound("x".into()).into(),
            LockErrorKind::DatabaseError("x".into()).into(),
            LockErrorKind::TtlOutOfRange("x".into()).into(),
        ];
        for err in &errors {
            let code = classify_exit_code(err);
            assert!(
                code > 0,
                "classify_exit_code returned 0 for {err:?}"
            );
        }
    }

    #[test]
    fn classify_exit_code_semantic_range() {
        // Verify semantic exit codes fall into documented ranges
        let err: crate::error::Error = ConfigErrorKind::ConfigParseError("x".into()).into();
        let code = classify_exit_code(&err);
        assert!(
            (1..=5).contains(&code),
            "Expected semantic exit code 1-5, got {code}"
        );
    }

    #[test]
    fn map_error_to_parts_always_returns_non_empty_message() {
        let errors: Vec<crate::error::Error> = vec![
            ConfigErrorKind::ConfigParseError("x".into()).into(),
            StateErrorKind::NotFound("x".into()).into(),
            SessionErrorKind::NotFound("x".into()).into(),
            WorkspaceErrorKind::NotFound("x".into()).into(),
            VcsErrorKind::NotInitialized.into(),
            IoErrorKind::IoError("x".into()).into(),
            AgentErrorKind::NotFound("x".into()).into(),
            QueueErrorKind::Empty.into(),
            InternalErrorKind::Internal("x".into()).into(),
            TaskErrorKind::NotFound("x".into()).into(),
            WaitErrorKind::Timeout("x".into(), "y".into()).into(),
            LockErrorKind::SessionLocked {
                session: "x".into(),
                holder: "y".into(),
            }
            .into(),
        ];
        for err in &errors {
            let (code, msg, _suggestion) = map_error_to_parts(err);
            assert!(
                !msg.is_empty(),
                "map_error_to_parts returned empty message for {code:?}"
            );
        }
    }
}

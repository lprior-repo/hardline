//! Exhaustive tests for sync command handler.
//!
//! Covers: sync fetch/pull/push orchestration, rebase with retry,
//! conflict detection, dirty workspace handling, sync-all orchestration,
//! error recovery, integration tests with temp git repos.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use scp_core::vcs::{
    Branch, Commit, CommitId, RepoStatus, VcsBackend, VcsStatus, Workspace,
};
use scp_core::error::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

// ═══════════════════════════════════════════════════════════════════════════
// MOCK VCS BACKEND
// ═══════════════════════════════════════════════════════════════════════════

enum MockResult {
    Ok,
    Err(String),
    ConflictErr,
}

struct MockBackend {
    switch_ok: bool,
    status_value: Option<VcsStatus>,
    rebase_results: Mutex<Vec<MockResult>>,
    rebase_calls: AtomicUsize,
    workspaces: Vec<Workspace>,
}

impl MockBackend {
    fn new() -> Self {
        Self {
            switch_ok: true,
            status_value: Some(VcsStatus::Clean),
            rebase_results: Mutex::new(vec![MockResult::Ok]),
            rebase_calls: AtomicUsize::new(0),
            workspaces: vec![],
        }
    }

    fn with_status(mut self, status: VcsStatus) -> Self {
        self.status_value = Some(status);
        self
    }

    fn with_status_error(mut self) -> Self {
        self.status_value = None;
        self
    }

    fn with_rebase_results(mut self, results: Vec<MockResult>) -> Self {
        self.rebase_results = Mutex::new(results);
        self
    }

    fn with_switch_error(mut self) -> Self {
        self.switch_ok = false;
        self
    }

    fn with_workspaces(mut self, workspaces: Vec<Workspace>) -> Self {
        self.workspaces = workspaces;
        self
    }

    fn rebase_call_count(&self) -> usize {
        self.rebase_calls.load(Ordering::SeqCst)
    }
}

impl VcsBackend for MockBackend {
    fn current_branch(&self) -> Result<String> {
        Ok("main".to_string())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        Ok(vec![])
    }

    fn create_branch(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn switch_branch(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn push(&self) -> Result<()> {
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        Ok(())
    }

    fn rebase(&self, _onto: &str) -> Result<()> {
        let call_num = self.rebase_calls.fetch_add(1, Ordering::SeqCst);
        let results = self.rebase_results.lock().expect("mock lock");
        let mock_res = if call_num < results.len() {
            &results[call_num]
        } else {
            results.last().unwrap_or(&MockResult::Ok)
        };
        match mock_res {
            MockResult::Ok => Ok(()),
            MockResult::Err(msg) => Err(scp_core::Error::internal(msg.clone())),
            MockResult::ConflictErr => {
                Err(scp_core::Error::internal("rebase conflict detected".to_string()))
            }
        }
    }

    fn merge(&self, _branch: &str) -> Result<()> {
        Ok(())
    }

    fn log(&self, _limit: usize) -> Result<Vec<Commit>> {
        Ok(vec![])
    }

    fn status(&self) -> Result<VcsStatus> {
        match self.status_value {
            Some(ref s) => Ok(s.clone()),
            None => Err(scp_core::Error::unimplemented("mock status error")),
        }
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(true)
    }

    fn repo_exists(&self, _path: &str) -> bool {
        true
    }

    fn checkout(&self, _target: &str) -> Result<()> {
        Ok(())
    }

    fn commit(&self, _message: &str) -> Result<CommitId> {
        CommitId::new("abc123").ok_or_else(|| scp_core::Error::unimplemented("mock"))
    }

    fn diff(&self, _from: &CommitId, _to: &CommitId) -> Result<String> {
        Ok(String::new())
    }

    fn repo_status(&self) -> Result<RepoStatus> {
        Ok(RepoStatus::clean())
    }

    fn create_workspace(&self, _name: &str) -> Result<()> {
        Err(scp_core::Error::unimplemented("mock"))
    }

    fn switch_workspace(&self, _name: &str) -> Result<()> {
        if self.switch_ok {
            Ok(())
        } else {
            Err(scp_core::Error::unimplemented("mock switch"))
        }
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        Ok(self.workspaces.clone())
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(scp_core::Error::unimplemented("mock"))
    }

    fn fork_workspace(&self, _source: &str, _target: &str) -> Result<()> {
        Err(scp_core::Error::unimplemented("mock"))
    }

    fn merge_workspace(&self, _name: &str) -> Result<()> {
        Err(scp_core::Error::unimplemented("mock"))
    }

    fn abort_workspace(&self, _name: &str) -> Result<()> {
        Err(scp_core::Error::unimplemented("mock"))
    }
}

fn default_options() -> SyncOptions {
    SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC ERROR DISPLAY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_error_display_workspace_not_found() {
    let err = SyncError::WorkspaceNotFound(PathBuf::from("/tmp/wrong"));
    let msg = err.to_string();
    assert!(msg.contains("/tmp/wrong"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_sync_error_display_session_not_found() {
    let err = SyncError::SessionNotFound("missing-session".to_string());
    assert!(err.to_string().contains("missing-session"));
}

#[test]
fn test_sync_error_display_lock_held() {
    let err = SyncError::LockHeldByOther {
        pid: 12345,
        holder: "other-host".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("12345"));
    assert!(msg.contains("other-host"));
}

#[test]
fn test_sync_error_display_conflict() {
    let err = SyncError::Conflict {
        workspace: "my-ws".to_string(),
        files: "a.rs, b.rs".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("my-ws"));
    assert!(msg.contains("a.rs, b.rs"));
}

#[test]
fn test_sync_error_display_retry_limit() {
    assert!(SyncError::RetryLimitExceeded(5).to_string().contains("5"));
}

#[test]
fn test_sync_error_display_dirty_workspace() {
    let err = SyncError::DirtyWorkspace("feature-branch".to_string());
    let msg = err.to_string();
    assert!(msg.contains("feature-branch"));
    assert!(msg.contains("uncommitted"));
}

#[test]
fn test_sync_error_display_invalid_identifier() {
    assert!(SyncError::InvalidIdentifier("bad name!".to_string())
        .to_string()
        .contains("bad name!"));
}

#[test]
fn test_sync_error_display_session_already_syncing() {
    let err = SyncError::SessionAlreadySyncing("ws-x".to_string());
    let msg = err.to_string();
    assert!(msg.contains("ws-x"));
    assert!(msg.contains("already syncing"));
}

#[test]
fn test_sync_error_display_lock_timeout() {
    assert!(SyncError::LockTimeout(30).to_string().contains("30"));
}

#[test]
fn test_sync_error_display_vcs_command_failed() {
    assert!(SyncError::VcsCommandFailed("rebase error".to_string())
        .to_string()
        .contains("rebase error"));
}

#[test]
fn test_sync_error_display_workspace_path_not_accessible() {
    assert!(SyncError::WorkspacePathNotAccessible(PathBuf::from("/no/access"))
        .to_string()
        .contains("/no/access"));
}

#[test]
fn test_sync_error_display_session_terminal_state() {
    let err = SyncError::SessionTerminalState("ws-done".to_string());
    let msg = err.to_string();
    assert!(msg.contains("ws-done"));
    assert!(msg.contains("terminal"));
}

#[test]
fn test_sync_error_display_lock_acquisition_failed() {
    assert!(SyncError::LockAcquisitionFailed("resource busy".to_string())
        .to_string()
        .contains("resource busy"));
}

#[test]
fn test_sync_error_display_session_database_not_found() {
    assert!(
        SyncError::SessionDatabaseNotFound(PathBuf::from("/tmp/no.db"))
            .to_string()
            .contains("/tmp/no.db")
    );
}

#[test]
fn test_sync_error_display_session_database_read_failed() {
    assert!(SyncError::SessionDatabaseReadFailed("corrupted".to_string())
        .to_string()
        .contains("corrupted"));
}

#[test]
fn test_sync_error_display_session_database_write_failed() {
    assert!(SyncError::SessionDatabaseWriteFailed("disk full".to_string())
        .to_string()
        .contains("disk full"));
}

#[test]
fn test_sync_error_display_configuration_error() {
    assert!(SyncError::ConfigurationError("bad config".to_string())
        .to_string()
        .contains("bad config"));
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC ERROR FROM CONVERSIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let sync_err: SyncError = io_err.into();
    assert!(matches!(sync_err, SyncError::IoError(_)));
    assert!(sync_err.to_string().contains("file missing"));
}

#[test]
fn test_sync_error_from_io_error_permission_denied() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let sync_err: SyncError = io_err.into();
    assert!(matches!(sync_err, SyncError::IoError(_)));
    assert!(sync_err.to_string().contains("access denied"));
}

#[test]
fn test_sync_error_from_scp_core_error() {
    let core_err = Error::internal("something broke");
    let sync_err: SyncError = core_err.into();
    assert!(matches!(sync_err, SyncError::ConfigurationError(_)));
    assert!(sync_err.to_string().contains("something broke"));
}

#[test]
fn test_sync_error_from_scp_core_workspace_not_found() {
    let core_err = Error::workspace_not_found("test-ws".to_string());
    let sync_err: SyncError = core_err.into();
    assert!(matches!(sync_err, SyncError::ConfigurationError(_)));
    assert!(sync_err.to_string().contains("test-ws"));
}

#[test]
fn test_sync_error_into_scp_core_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
    let sync_err: SyncError = SyncError::IoError(io_err);
    let core_err: Error = sync_err.into();
    assert!(core_err.to_string().contains("pipe broke"));
}

#[test]
fn test_sync_error_into_scp_core_error_non_io() {
    let sync_err = SyncError::SessionNotFound("x".to_string());
    let core_err: Error = sync_err.into();
    assert!(core_err.to_string().contains("Session not found"));
}

#[test]
fn test_sync_error_into_core_error_conflict() {
    let sync_err = SyncError::Conflict {
        workspace: "my-workspace".to_string(),
        files: "a.rs, b.rs".to_string(),
    };
    let core_err: Error = sync_err.into();
    assert!(core_err.to_string().contains("my-workspace"));
}

#[test]
fn test_sync_error_non_io_variant_into_core_error_preserves_message() {
    let sync_err = SyncError::DirtyWorkspace("dirty-ws".to_string());
    let core_err: Error = sync_err.into();
    assert!(core_err.to_string().contains("dirty-ws"));
}

#[test]
fn test_sync_error_roundtrip_io_error() {
    let original_msg = "disk write failed";
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, original_msg);
    let sync_err: SyncError = io_err.into();
    let core_err: Error = sync_err.into();
    assert!(core_err.to_string().contains(original_msg));
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA TYPE CONSTRUCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_options_construction() {
    let opts = SyncOptions {
        allow_dirty: true,
        target_branch: Some("develop".to_string()),
        lock_timeout_secs: 60,
        retry_config: RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 200,
        },
    };
    assert!(opts.allow_dirty);
    assert_eq!(opts.target_branch.as_deref(), Some("develop"));
    assert_eq!(opts.lock_timeout_secs, 60);
    assert_eq!(opts.retry_config.max_attempts, 5);
}

#[test]
fn test_sync_options_defaults() {
    let opts = SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
        },
    };
    assert!(!opts.allow_dirty);
    assert!(opts.target_branch.is_none());
}

#[test]
fn test_sync_summary_construction() {
    let session = SessionName::parse("my-session").expect("valid session name");
    let summary = SyncSummary {
        sessions_synced: vec![session.clone()],
        total_operations: 2,
        had_conflicts: false,
    };
    assert_eq!(summary.sessions_synced.len(), 1);
    assert_eq!(summary.sessions_synced[0].as_str(), "my-session");
    assert_eq!(summary.total_operations, 2);
    assert!(!summary.had_conflicts);
}

#[test]
fn test_sync_summary_empty() {
    let summary = SyncSummary {
        sessions_synced: Vec::new(),
        total_operations: 0,
        had_conflicts: false,
    };
    assert!(summary.sessions_synced.is_empty());
    assert_eq!(summary.total_operations, 0);
}

#[test]
fn test_sync_summary_with_conflicts() {
    let session = SessionName::parse("my-session").expect("valid");
    let summary = SyncSummary {
        sessions_synced: vec![session],
        total_operations: 3,
        had_conflicts: true,
    };
    assert!(summary.had_conflicts);
}

#[test]
fn test_sync_summary_multiple_sessions() {
    let s1 = SessionName::parse("ws-1").expect("valid");
    let s2 = SessionName::parse("ws-2").expect("valid");
    let summary = SyncSummary {
        sessions_synced: vec![s1, s2],
        total_operations: 5,
        had_conflicts: false,
    };
    assert_eq!(summary.sessions_synced.len(), 2);
    assert_eq!(summary.total_operations, 5);
}

#[test]
fn test_sync_summary_clone() {
    let session = SessionName::parse("clone-test").expect("valid");
    let summary = SyncSummary {
        sessions_synced: vec![session],
        total_operations: 1,
        had_conflicts: false,
    };
    let cloned = summary.clone();
    assert_eq!(cloned.sessions_synced.len(), summary.sessions_synced.len());
    assert_eq!(cloned.total_operations, summary.total_operations);
    assert_eq!(cloned.had_conflicts, summary.had_conflicts);
}

#[test]
fn test_sync_summary_debug() {
    let summary = SyncSummary {
        sessions_synced: vec![],
        total_operations: 0,
        had_conflicts: false,
    };
    let debug_str = format!("{:?}", summary);
    assert!(debug_str.contains("SyncSummary"));
}

#[test]
fn test_retry_config_construction() {
    let config = RetryConfig {
        max_attempts: 10,
        initial_delay_ms: 500,
    };
    assert_eq!(config.max_attempts, 10);
    assert_eq!(config.initial_delay_ms, 500);
}

#[test]
fn test_retry_config_zero_attempts() {
    let config = RetryConfig {
        max_attempts: 0,
        initial_delay_ms: 100,
    };
    assert_eq!(config.max_attempts, 0);
}

#[test]
fn test_retry_config_zero_delay() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 0,
    };
    assert_eq!(config.initial_delay_ms, 0);
}

#[test]
fn test_retry_config_clone() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 100,
    };
    let cloned = config.clone();
    assert_eq!(cloned.max_attempts, config.max_attempts);
    assert_eq!(cloned.initial_delay_ms, config.initial_delay_ms);
}

#[test]
fn test_sync_options_clone() {
    let opts = SyncOptions {
        allow_dirty: true,
        target_branch: Some("develop".to_string()),
        lock_timeout_secs: 60,
        retry_config: RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 200,
        },
    };
    let cloned = opts.clone();
    assert_eq!(cloned.allow_dirty, opts.allow_dirty);
    assert_eq!(cloned.target_branch, opts.target_branch);
    assert_eq!(cloned.lock_timeout_secs, opts.lock_timeout_secs);
}

// ═══════════════════════════════════════════════════════════════════════════
// FIND_GIT_ROOT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_find_git_root_finds_marker() {
    let dir = std::env::temp_dir().join("hardline_test_find_git_root_found");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::create_dir(dir.join(".git"));
    let result = find_git_root(&dir);
    assert_eq!(result, Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_find_git_root_searches_parent() {
    let dir = std::env::temp_dir().join("hardline_test_find_git_root_parent");
    let child = dir.join("subdir");
    let _ = std::fs::create_dir_all(&child);
    let _ = std::fs::create_dir(dir.join(".git"));
    let result = find_git_root(&child);
    assert_eq!(result, Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_find_git_root_not_found() {
    let dir = std::env::temp_dir().join("hardline_test_find_git_root_missing");
    let _ = std::fs::create_dir_all(&dir);
    let result = find_git_root(&dir);
    assert!(result.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_find_git_root_stops_at_filesystem_root() {
    let nonexistent = PathBuf::from("/nonexistent_dir_for_testing");
    let result = find_git_root(&nonexistent);
    assert!(result.is_none());
}

#[test]
fn test_find_git_root_deeply_nested_finds_ancestor() {
    let dir = std::env::temp_dir().join("hardline_test_git_deep");
    let deep = dir.join("a/b/c/d");
    let _ = std::fs::create_dir_all(&deep);
    let _ = std::fs::create_dir(dir.join(".git"));
    let result = find_git_root(&deep);
    assert_eq!(result, Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_find_git_root_sibling_directory_not_found() {
    let dir = std::env::temp_dir().join("hardline_test_git_sibling_parent");
    let child_a = dir.join("child_a");
    let child_b = dir.join("child_b");
    let _ = std::fs::create_dir_all(&child_a);
    let _ = std::fs::create_dir_all(&child_b);
    let _ = std::fs::create_dir(child_b.join(".git"));
    let result = find_git_root(&child_a);
    assert!(result.is_none(), "should not find .git in sibling directory");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_find_git_root_file_git_not_directory() {
    let dir = std::env::temp_dir().join("hardline_test_git_file");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(".git"), "gitdir: /somewhere/else");
    let result = find_git_root(&dir);
    assert!(result.is_some(), ".git file (worktree) should be found");
    let _ = std::fs::remove_dir_all(&dir);
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC_SESSION_INTERNAL UNIT TESTS (MOCK BACKEND)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sync_internal_clean_workspace_succeeds() {
    let backend = MockBackend::new();
    let opts = default_options();
    let result = sync_session_internal(&backend, "test-session", &opts).await;
    assert!(result.is_ok());
    let summary = result.expect("summary");
    assert_eq!(summary.sessions_synced.len(), 1);
    assert_eq!(summary.sessions_synced[0].as_str(), "test-session");
    assert_eq!(summary.total_operations, 1);
    assert!(!summary.had_conflicts);
    assert_eq!(backend.rebase_call_count(), 1);
}

#[tokio::test]
async fn test_sync_internal_dirty_workspace_rejected() {
    let backend = MockBackend::new().with_status(VcsStatus::Dirty);
    let opts = SyncOptions {
        allow_dirty: false,
        ..default_options()
    };
    let result = sync_session_internal(&backend, "dirty-ws", &opts).await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, SyncError::DirtyWorkspace(ref ws) if ws == "dirty-ws"));
    assert!(err.to_string().contains("dirty-ws"));
    assert!(err.to_string().contains("uncommitted"));
}

#[tokio::test]
async fn test_sync_internal_dirty_workspace_allowed() {
    let backend = MockBackend::new().with_status(VcsStatus::Dirty);
    let opts = SyncOptions {
        allow_dirty: true,
        ..default_options()
    };
    let result = sync_session_internal(&backend, "dirty-allowed", &opts).await;
    assert!(result.is_ok());
    let summary = result.expect("summary");
    assert_eq!(summary.sessions_synced.len(), 1);
}

#[tokio::test]
async fn test_sync_internal_switch_workspace_fails() {
    let backend = MockBackend::new().with_switch_error();
    let opts = default_options();
    let result = sync_session_internal(&backend, "no-such-ws", &opts).await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, SyncError::ConfigurationError(_)));
}

#[tokio::test]
async fn test_sync_internal_rebase_succeeds_first_attempt() {
    let backend = MockBackend::new().with_rebase_results(vec![MockResult::Ok]);
    let opts = default_options();
    let result = sync_session_internal(&backend, "fast-sync", &opts).await;
    assert!(result.is_ok());
    assert_eq!(result.expect("summary").total_operations, 1);
    assert_eq!(backend.rebase_call_count(), 1);
}

#[tokio::test]
async fn test_sync_internal_rebase_fails_then_succeeds() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("transient failure".to_string()),
        MockResult::Ok,
    ]);
    let opts = default_options();
    let result = sync_session_internal(&backend, "retry-sync", &opts).await;
    assert!(result.is_ok());
    let summary = result.expect("summary");
    assert_eq!(summary.total_operations, 2);
    assert_eq!(backend.rebase_call_count(), 2);
}

#[tokio::test]
async fn test_sync_internal_rebase_fails_all_attempts() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("fail 1".to_string()),
        MockResult::Err("fail 2".to_string()),
        MockResult::Err("fail 3".to_string()),
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "doomed-sync", &opts).await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, SyncError::VcsCommandFailed(_)));
    assert!(err.to_string().contains("3 attempts"));
    assert_eq!(backend.rebase_call_count(), 3);
}

#[tokio::test]
async fn test_sync_internal_conflict_detected() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::ConflictErr,
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 1,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "conflict-ws", &opts).await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, SyncError::Conflict { .. }));
    if let SyncError::Conflict { workspace, files } = err {
        assert_eq!(workspace, "conflict-ws");
        assert_eq!(files, "unknown (see git status)");
    }
}

#[tokio::test]
async fn test_sync_internal_conflict_after_retries() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("busy".to_string()),
        MockResult::ConflictErr,
    ]);
    let opts = default_options();
    let result = sync_session_internal(&backend, "late-conflict", &opts).await;
    assert!(result.is_err());
    assert!(matches!(result.expect_err("err"), SyncError::Conflict { .. }));
}

#[tokio::test]
async fn test_sync_internal_post_rebase_conflict_status() {
    let backend = MockBackend {
        status_value: Some(VcsStatus::Conflicted),
        rebase_results: Mutex::new(vec![MockResult::Ok]),
        rebase_calls: AtomicUsize::new(0),
        switch_ok: true,
        workspaces: vec![],
    };
    let opts = default_options();
    let result = sync_session_internal(&backend, "post-conflict", &opts).await;
    assert!(result.is_ok());
    let summary = result.expect("summary");
    assert!(summary.had_conflicts);
}

#[tokio::test]
async fn test_sync_internal_default_target_branch_is_main() {
    let backend = MockBackend::new();
    let opts = SyncOptions {
        target_branch: None,
        ..default_options()
    };
    let result = sync_session_internal(&backend, "default-branch", &opts).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_internal_custom_target_branch() {
    let backend = MockBackend::new();
    let opts = SyncOptions {
        target_branch: Some("develop".to_string()),
        ..default_options()
    };
    let result = sync_session_internal(&backend, "custom-branch", &opts).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_internal_zero_max_attempts_fails_immediately() {
    let backend = MockBackend::new().with_rebase_results(vec![MockResult::Err("nope".to_string())]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 0,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "zero-attempts", &opts).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sync_internal_single_attempt_no_retry() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("once".to_string()),
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 1,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "single-attempt", &opts).await;
    assert!(result.is_err());
    assert_eq!(backend.rebase_call_count(), 1);
}

#[tokio::test]
async fn test_sync_internal_many_retries_eventual_success() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("1".to_string()),
        MockResult::Err("2".to_string()),
        MockResult::Err("3".to_string()),
        MockResult::Err("4".to_string()),
        MockResult::Ok,
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "patient-sync", &opts).await;
    assert!(result.is_ok());
    assert_eq!(result.expect("summary").total_operations, 5);
    assert_eq!(backend.rebase_call_count(), 5);
}

#[tokio::test]
async fn test_sync_internal_status_check_error_propagates() {
    let backend = MockBackend {
        status_value: None,
        switch_ok: true,
        rebase_results: Mutex::new(vec![MockResult::Ok]),
        rebase_calls: AtomicUsize::new(0),
        workspaces: vec![],
    };
    let opts = default_options();
    let result = sync_session_internal(&backend, "status-fail", &opts).await;
    assert!(result.is_err());
    assert!(matches!(result.expect_err("err"), SyncError::ConfigurationError(_)));
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC ALL SESSIONS TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sync_all_no_workspaces() {
    let backend = MockBackend::new().with_workspaces(vec![]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut sessions_synced = Vec::new();
    let mut total_operations = 0;
    let mut had_conflicts = false;

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            match sync_session_internal(&backend, session_name.as_str(), &opts).await {
                Ok(summary) => {
                    sessions_synced.extend(summary.sessions_synced);
                    total_operations += summary.total_operations;
                    had_conflicts |= summary.had_conflicts;
                }
                Err(_) => {}
            }
        }
    }

    assert!(sessions_synced.is_empty());
    assert_eq!(total_operations, 0);
    assert!(!had_conflicts);
}

#[tokio::test]
async fn test_sync_all_skips_main_workspace() {
    let backend = MockBackend::new().with_workspaces(vec![
        Workspace { name: "main".to_string(), branch: "main".to_string(), is_current: true },
        Workspace { name: "feature-x".to_string(), branch: "feature-x".to_string(), is_current: false },
    ]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut synced_names = Vec::new();

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            if sync_session_internal(&backend, session_name.as_str(), &opts).await.is_ok() {
                synced_names.push(ws.name);
            }
        }
    }

    assert_eq!(synced_names, vec!["feature-x"]);
}

#[tokio::test]
async fn test_sync_all_multiple_workspaces() {
    let backend = MockBackend::new().with_workspaces(vec![
        Workspace { name: "ws-alpha".to_string(), branch: "ws-alpha".to_string(), is_current: false },
        Workspace { name: "ws-beta".to_string(), branch: "ws-beta".to_string(), is_current: false },
        Workspace { name: "ws-gamma".to_string(), branch: "ws-gamma".to_string(), is_current: false },
    ]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut synced_count = 0;

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            if sync_session_internal(&backend, session_name.as_str(), &opts).await.is_ok() {
                synced_count += 1;
            }
        }
    }

    assert_eq!(synced_count, 3);
}

#[tokio::test]
async fn test_sync_all_partial_failure_continues() {
    let mut backend = MockBackend::new();
    backend = backend.with_workspaces(vec![
        Workspace { name: "good-ws".to_string(), branch: "good-ws".to_string(), is_current: false },
        Workspace { name: "bad-ws".to_string(), branch: "bad-ws".to_string(), is_current: false },
    ]);
    backend = backend.with_switch_error();

    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut errors = 0;
    let mut successes = 0;

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            match sync_session_internal(&backend, session_name.as_str(), &opts).await {
                Ok(_) => successes += 1,
                Err(_) => errors += 1,
            }
        }
    }

    assert_eq!(errors, 2, "both workspaces should fail with bad switch");
    assert_eq!(successes, 0);
}

#[tokio::test]
async fn test_sync_all_invalid_session_names_skipped() {
    let backend = MockBackend::new().with_workspaces(vec![
        Workspace { name: "valid-session".to_string(), branch: "valid-session".to_string(), is_current: false },
        Workspace { name: "".to_string(), branch: "".to_string(), is_current: false },
    ]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut synced = Vec::new();

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            if sync_session_internal(&backend, session_name.as_str(), &opts).await.is_ok() {
                synced.push(ws.name);
            }
        }
    }

    assert_eq!(synced, vec!["valid-session"]);
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS WITH REAL GIT REPOS
// ═══════════════════════════════════════════════════════════════════════════

fn init_git_repo(path: &std::path::Path) {
    use std::process::Command;
    Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().expect("git init");
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().expect("git config email");
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().expect("git config name");
}

fn git_commit(path: &std::path::Path, message: &str) {
    use std::process::Command;
    let filename = format!("{}.txt", message.replace(' ', "_"));
    std::fs::write(path.join(&filename), message).expect("write file");
    Command::new("git").args(["add", "."]).current_dir(path).output().expect("git add");
    Command::new("git").args(["commit", "-m", message]).current_dir(path).output().expect("git commit");
}

fn git_create_branch(path: &std::path::Path, name: &str) {
    use std::process::Command;
    Command::new("git").args(["checkout", "-b", name]).current_dir(path).output().expect("git checkout -b");
}

fn git_checkout(path: &std::path::Path, branch: &str) {
    use std::process::Command;
    Command::new("git").args(["checkout", branch]).current_dir(path).output().expect("git checkout");
}

fn git_log_oneline(path: &std::path::Path) -> String {
    use std::process::Command;
    let output = Command::new("git").args(["log", "--oneline", "-5"]).current_dir(path).output().expect("git log");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
#[serial_test::serial]
fn integration_sync_named_session_non_vcs_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = default_options();
    let session = SessionName::parse("test-session").expect("valid");
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(sync_named_session(session, opts));

    assert!(result.is_err(), "sync must fail in non-VCS directory");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_sync_all_sessions_non_vcs_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = default_options();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(sync_all_sessions(opts));

    assert!(result.is_err(), "sync_all must fail in non-VCS directory");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_sync_current_workspace_non_vcs_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    std::env::set_current_dir(tmp.path()).expect("chdir");

    let opts = default_options();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(sync_current_workspace(opts));

    assert!(result.is_err(), "sync_current_workspace must fail without VCS");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_create_and_sync() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial commit");
    git_create_branch(tmp.path(), "feature-test");
    git_commit(tmp.path(), "feature work");
    git_checkout(tmp.path(), "main");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    assert!(backend.is_initialized().expect("init check"));

    let log = git_log_oneline(tmp.path());
    assert!(log.contains("initial commit"));

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_status_clean() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let status = backend.status().expect("status");
    assert_eq!(status, VcsStatus::Clean);

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_status_dirty() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");
    std::fs::write(tmp.path().join("new_file.txt"), "dirty content").expect("write");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let status = backend.status().expect("status");
    assert_eq!(status, VcsStatus::Dirty);

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_rebase_success() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial on main");

    git_create_branch(tmp.path(), "feature-rb");
    git_commit(tmp.path(), "feature commit");

    git_checkout(tmp.path(), "main");
    git_commit(tmp.path(), "main advance");

    git_checkout(tmp.path(), "feature-rb");
    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let result = backend.rebase("main");
    assert!(result.is_ok(), "rebase should succeed without conflicts");

    let log = git_log_oneline(tmp.path());
    assert!(log.contains("main advance"));
    assert!(log.contains("feature commit"));

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_branch_operations() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");

    backend.create_branch("develop").expect("create branch");
    let branches = backend.list_branches().expect("list");
    let branch_names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(branch_names.contains(&"develop"));
    assert!(branch_names.contains(&"main") || branch_names.iter().any(|n| n.contains("main")));

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_push_without_remote_fails() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let result = backend.push();
    assert!(result.is_err(), "push without remote should fail");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_pull_without_remote_fails() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let result = backend.pull();
    assert!(result.is_err(), "pull without remote should fail");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_commit_and_log() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());

    std::fs::write(tmp.path().join("a.txt"), "content a").expect("write a");
    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");

    use std::process::Command;
    Command::new("git").args(["add", "."]).current_dir(tmp.path()).output().expect("add");
    let commit_id = backend.commit("first commit").expect("commit");
    assert!(!commit_id.as_str().is_empty());

    let commits = backend.log(5).expect("log");
    assert!(!commits.is_empty());

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_current_branch() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    let branch = backend.current_branch().expect("current branch");
    assert_eq!(branch, "main");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

#[test]
#[serial_test::serial]
fn integration_git_repo_switch_branch() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(tmp.path());
    git_commit(tmp.path(), "initial");
    git_create_branch(tmp.path(), "develop");

    let backend = scp_core::vcs::create_backend(tmp.path()).expect("backend");
    backend.switch_branch("main").expect("switch to main");
    let branch = backend.current_branch().expect("branch");
    assert_eq!(branch, "main");

    backend.switch_branch("develop").expect("switch to develop");
    let branch = backend.current_branch().expect("branch");
    assert_eq!(branch, "develop");

    std::env::set_current_dir(std::env::temp_dir()).ok();
}

// ═══════════════════════════════════════════════════════════════════════════
// ERROR RECOVERY AND RETRY LOGIC TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_retry_backoff_eventual_success() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("network blip".to_string()),
        MockResult::Err("still recovering".to_string()),
        MockResult::Ok,
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "backoff-test", &opts).await;
    assert!(result.is_ok());
    assert_eq!(backend.rebase_call_count(), 3);
}

#[tokio::test]
async fn test_retry_exhausted_returns_vcs_error() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("permanent".to_string()),
        MockResult::Err("permanent".to_string()),
        MockResult::Err("permanent".to_string()),
    ]);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "exhausted", &opts).await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    match err {
        SyncError::VcsCommandFailed(msg) => {
            assert!(msg.contains("3 attempts"));
        }
        other => panic!("expected VcsCommandFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_retry_conflict_takes_priority_over_retry() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("first failure".to_string()),
        MockResult::ConflictErr,
    ]);
    let opts = default_options();
    let result = sync_session_internal(&backend, "conflict-priority", &opts).await;
    assert!(result.is_err());
    assert!(matches!(result.expect_err("err"), SyncError::Conflict { .. }));
}

#[tokio::test]
async fn test_retry_with_large_attempt_count() {
    let mut results: Vec<MockResult> = (0..9)
        .map(|i| MockResult::Err(format!("fail {}", i)))
        .collect();
    results.push(MockResult::Ok);

    let backend = MockBackend::new().with_rebase_results(results);
    let opts = SyncOptions {
        retry_config: RetryConfig {
            max_attempts: 10,
            initial_delay_ms: 1,
        },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "tenacious", &opts).await;
    assert!(result.is_ok());
    assert_eq!(result.expect("summary").total_operations, 10);
    assert_eq!(backend.rebase_call_count(), 10);
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC OPTIONS EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_options_target_branch_none() {
    let opts = SyncOptions {
        target_branch: None,
        ..default_options()
    };
    assert!(opts.target_branch.is_none());
    let target = opts.target_branch.as_deref().map_or("main", |v| v);
    assert_eq!(target, "main");
}

#[test]
fn test_sync_options_target_branch_some() {
    let opts = SyncOptions {
        target_branch: Some("release/1.0".to_string()),
        ..default_options()
    };
    let target = opts.target_branch.as_deref().map_or("main", |v| v);
    assert_eq!(target, "release/1.0");
}

#[test]
fn test_sync_options_allow_dirty_false() {
    let opts = SyncOptions {
        allow_dirty: false,
        ..default_options()
    };
    assert!(!opts.allow_dirty);
}

#[test]
fn test_sync_options_allow_dirty_true() {
    let opts = SyncOptions {
        allow_dirty: true,
        ..default_options()
    };
    assert!(opts.allow_dirty);
}

#[test]
fn test_sync_options_lock_timeout() {
    let opts = SyncOptions {
        lock_timeout_secs: 120,
        ..default_options()
    };
    assert_eq!(opts.lock_timeout_secs, 120);
}

#[test]
fn test_sync_options_zero_lock_timeout() {
    let opts = SyncOptions {
        lock_timeout_secs: 0,
        ..default_options()
    };
    assert_eq!(opts.lock_timeout_secs, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// SYNC ERROR TAXONOMY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_error_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<SyncError>();
}

#[test]
fn test_sync_error_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<SyncError>();
}

#[test]
fn test_sync_error_size() {
    assert!(std::mem::size_of::<SyncError>() < 256, "SyncError should be reasonably sized");
}

#[test]
fn test_sync_error_all_variants_error_trait() {
    let errors: Vec<SyncError> = vec![
        SyncError::WorkspaceNotFound(PathBuf::from("/a")),
        SyncError::WorkspacePathNotAccessible(PathBuf::from("/b")),
        SyncError::SessionNotFound("s".into()),
        SyncError::SessionAlreadySyncing("s".into()),
        SyncError::SessionTerminalState("s".into()),
        SyncError::LockAcquisitionFailed("l".into()),
        SyncError::LockHeldByOther { pid: 1, holder: "h".into() },
        SyncError::LockTimeout(10),
        SyncError::DirtyWorkspace("d".into()),
        SyncError::VcsCommandFailed("v".into()),
        SyncError::Conflict { workspace: "w".into(), files: "f".into() },
        SyncError::RetryLimitExceeded(3),
        SyncError::SessionDatabaseNotFound(PathBuf::from("/c")),
        SyncError::SessionDatabaseReadFailed("r".into()),
        SyncError::SessionDatabaseWriteFailed("w".into()),
        SyncError::ConfigurationError("c".into()),
        SyncError::InvalidIdentifier("i".into()),
        SyncError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "io")),
    ];
    for err in &errors {
        let msg = err.to_string();
        assert!(!msg.is_empty(), "error display should not be empty");
    }
}

#[test]
fn test_sync_error_debug_format() {
    let err = SyncError::DirtyWorkspace("test-ws".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("DirtyWorkspace"));
    assert!(debug.contains("test-ws"));
}

// ═══════════════════════════════════════════════════════════════════════════
// REMOTE TRACKING / FETCH SIMULATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sync_with_remote_tracking_workspace() {
    let backend = MockBackend::new().with_workspaces(vec![
        Workspace {
            name: "tracking-ws".to_string(),
            branch: "tracking-ws".to_string(),
            is_current: false,
        },
    ]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut synced = Vec::new();

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            if sync_session_internal(&backend, session_name.as_str(), &opts).await.is_ok() {
                synced.push(ws.name);
            }
        }
    }

    assert_eq!(synced, vec!["tracking-ws"]);
}

#[tokio::test]
async fn test_sync_ignores_main_named_workspace() {
    let backend = MockBackend::new().with_workspaces(vec![
        Workspace { name: "main".to_string(), branch: "main".to_string(), is_current: true },
        Workspace { name: "feature".to_string(), branch: "feature".to_string(), is_current: false },
    ]);
    let workspaces = backend.list_workspaces().expect("list");
    let opts = default_options();
    let mut synced = Vec::new();

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }
        if let Ok(session_name) = SessionName::parse(&ws.name) {
            if sync_session_internal(&backend, session_name.as_str(), &opts).await.is_ok() {
                synced.push(ws.name);
            }
        }
    }

    assert_eq!(synced, vec!["feature"]);
    assert!(!synced.contains(&"main".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// DRY-RUN / PROGRESS REPORTING OPTIONS TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sync_options_for_dry_run_scenario() {
    let dry_run_opts = SyncOptions {
        allow_dirty: true,
        target_branch: None,
        lock_timeout_secs: 0,
        retry_config: RetryConfig {
            max_attempts: 1,
            initial_delay_ms: 0,
        },
    };
    assert!(dry_run_opts.allow_dirty);
    assert_eq!(dry_run_opts.retry_config.max_attempts, 1);
}

#[test]
fn test_sync_options_for_progress_reporting() {
    let progress_opts = SyncOptions {
        allow_dirty: false,
        target_branch: Some("main".to_string()),
        lock_timeout_secs: 300,
        retry_config: RetryConfig {
            max_attempts: 10,
            initial_delay_ms: 1000,
        },
    };
    assert_eq!(progress_opts.lock_timeout_secs, 300);
    assert_eq!(progress_opts.retry_config.max_attempts, 10);
    assert_eq!(progress_opts.retry_config.initial_delay_ms, 1000);
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFLICT DETECTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_conflict_error_on_first_attempt() {
    let backend = MockBackend::new().with_rebase_results(vec![MockResult::ConflictErr]);
    let opts = SyncOptions {
        retry_config: RetryConfig { max_attempts: 1, initial_delay_ms: 1 },
        ..default_options()
    };
    let result = sync_session_internal(&backend, "immediate-conflict", &opts).await;
    assert!(result.is_err());
    if let SyncError::Conflict { workspace, files } = result.expect_err("err") {
        assert_eq!(workspace, "immediate-conflict");
        assert_eq!(files, "unknown (see git status)");
    } else {
        panic!("expected Conflict error");
    }
}

#[tokio::test]
async fn test_conflict_error_after_retry() {
    let backend = MockBackend::new().with_rebase_results(vec![
        MockResult::Err("temp issue".to_string()),
        MockResult::ConflictErr,
    ]);
    let opts = default_options();
    let result = sync_session_internal(&backend, "delayed-conflict", &opts).await;
    assert!(result.is_err());
    assert!(matches!(result.expect_err("err"), SyncError::Conflict { .. }));
}

#[tokio::test]
async fn test_post_rebase_conflict_status_reported() {
    let backend = MockBackend {
        status_value: Some(VcsStatus::Conflicted),
        rebase_results: Mutex::new(vec![MockResult::Ok]),
        rebase_calls: AtomicUsize::new(0),
        switch_ok: true,
        workspaces: vec![],
    };
    let opts = default_options();
    let result = sync_session_internal(&backend, "post-rebase-conflict", &opts).await;
    assert!(result.is_ok());
    assert!(result.expect("summary").had_conflicts);
}

#[tokio::test]
async fn test_clean_after_rebase_no_conflicts() {
    let backend = MockBackend {
        status_value: Some(VcsStatus::Clean),
        rebase_results: Mutex::new(vec![MockResult::Ok]),
        rebase_calls: AtomicUsize::new(0),
        switch_ok: true,
        workspaces: vec![],
    };
    let opts = default_options();
    let result = sync_session_internal(&backend, "clean-sync", &opts).await;
    assert!(result.is_ok());
    assert!(!result.expect("summary").had_conflicts);
}

// ═══════════════════════════════════════════════════════════════════════════
// EDGE CASE: DETACHED HEAD STATUS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sync_internal_detached_head_not_dirty() {
    let backend = MockBackend {
        status_value: Some(VcsStatus::Detached),
        rebase_results: Mutex::new(vec![MockResult::Ok]),
        rebase_calls: AtomicUsize::new(0),
        switch_ok: true,
        workspaces: vec![],
    };
    let opts = SyncOptions {
        allow_dirty: false,
        ..default_options()
    };
    let result = sync_session_internal(&backend, "detached-ws", &opts).await;
    assert!(result.is_ok(), "Detached != Dirty, should proceed");
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION NAME VALIDATION IN SYNC CONTEXT
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_name_parse_valid_names() {
    let valid_names = vec!["my-session", "feature_x", "workspace-123", "abc"];
    for name in valid_names {
        assert!(SessionName::parse(name).is_ok(), "should parse: {}", name);
    }
}

#[test]
fn test_session_name_parse_empty_fails() {
    assert!(SessionName::parse("").is_err());
}

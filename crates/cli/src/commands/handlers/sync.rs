//! Sync command handler for Port CLI
//!
//! Implementation of the sync command ported from isolate.

use scp_core::domain::SessionName;
use scp_core::jj_operation_sync::acquire_cross_process_lock;
use scp_core::output_jsonl::{
    emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
    IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput,
};
use scp_core::vcs::{create_backend, VcsBackend, VcsStatus};
use scp_core::Error;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

/// Options for sync operation
#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub allow_dirty: bool,
    pub target_branch: Option<String>,
    pub lock_timeout_secs: u64,
    pub retry_config: RetryConfig,
}

/// Configuration for retries with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
}

/// Summary of a sync operation
#[derive(Debug, Clone)]
pub struct SyncSummary {
    pub sessions_synced: Vec<SessionName>,
    pub total_operations: u32,
    pub had_conflicts: bool,
}

/// Error taxonomy for sync operations
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Identifier error: {0}")]
    IdentifierError(#[from] scp_core::domain::IdentifierError),
    #[error("Output line error: {0}")]
    OutputLineError(#[from] scp_core::output_jsonl::OutputLineError),
    #[error("Workspace not found at {0}")]
    WorkspaceNotFound(PathBuf),
    #[error("Workspace path not accessible: {0}")]
    WorkspacePathNotAccessible(PathBuf),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session {0} is already syncing")]
    SessionAlreadySyncing(String),
    #[error("Session {0} is in terminal state")]
    SessionTerminalState(String),
    #[error("Failed to acquire sync lock: {0}")]
    LockAcquisitionFailed(String),
    #[error("Sync lock held by another process (PID: {pid}, holder: {holder})")]
    LockHeldByOther { pid: u32, holder: String },
    #[error("Timed out waiting for sync lock after {0} seconds")]
    LockTimeout(u64),
    #[error("Workspace has uncommitted changes: {0}")]
    DirtyWorkspace(String),
    #[error("JJ command failed: {0}")]
    JjCommandFailed(String),
    #[error("Rebase resulted in conflicts in workspace {workspace}: {files}")]
    Conflict { workspace: String, files: String },
    #[error("Retry limit exceeded after {0} attempts")]
    RetryLimitExceeded(u32),
    #[error("Session database not found at {0}")]
    SessionDatabaseNotFound(PathBuf),
    #[error("Failed to read session database: {0}")]
    SessionDatabaseReadFailed(String),
    #[error("Failed to write to session database: {0}")]
    SessionDatabaseWriteFailed(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<Error> for SyncError {
    fn from(err: Error) -> Self {
        SyncError::ConfigurationError(err.to_string())
    }
}

impl From<SyncError> for Error {
    fn from(err: SyncError) -> Self {
        match err {
            SyncError::IoError(e) => Error::from(e),
            _ => Error::internal(err.to_string()),
        }
    }
}

/// Sync a specific session by name.
pub async fn sync_named_session(
    session_name: SessionName,
    options: SyncOptions,
) -> std::result::Result<SyncSummary, SyncError> {
    let cwd = std::env::current_dir().map_err(SyncError::IoError)?;

    // 1. Emit Action: Syncing
    emit_action("sync", "session", ActionStatus::InProgress, None)?;

    // 2. Create backend and get root
    let backend = create_backend(&cwd)?;

    // We need the root for the lock file
    let repo_root = find_jj_root(&cwd)
        .ok_or_else(|| SyncError::JjCommandFailed("Not in a JJ repo".to_string()))?;

    // 3. Acquire lock
    let _lock = acquire_cross_process_lock(&repo_root)
        .await
        .map_err(|e| SyncError::LockAcquisitionFailed(e.to_string()))?;

    // 4. Perform sync
    let result = sync_session_internal(backend.as_ref(), session_name.as_str(), &options).await;

    // 5. Emit result and return
    match result {
        Ok(summary) => {
            emit_action("sync", "session", ActionStatus::Completed, Some("success"))?;
            emit_result(
                true,
                ResultKind::Operation,
                &format!("Successfully synced {}", session_name),
            )?;
            Ok(summary)
        }
        Err(e) => {
            emit_action(
                "sync",
                "session",
                ActionStatus::Failed,
                Some(&e.to_string()),
            )?;
            emit_issue(
                "sync-failed",
                e.to_string(),
                IssueKind::External,
                IssueSeverity::Error,
                Some(session_name.as_str()),
                None,
            )?;
            Err(e)
        }
    }
}

/// Sync all eligible sessions.
pub async fn sync_all_sessions(
    options: SyncOptions,
) -> std::result::Result<SyncSummary, SyncError> {
    let cwd = std::env::current_dir().map_err(SyncError::IoError)?;
    let backend = create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;

    let mut sessions_synced = Vec::new();
    let mut total_operations = 0;
    let mut had_conflicts = false;

    for ws in workspaces {
        if ws.name == "main" {
            continue;
        }

        if let Ok(session_name) = SessionName::parse(&ws.name) {
            match sync_session_internal(backend.as_ref(), session_name.as_str(), &options).await {
                Ok(summary) => {
                    sessions_synced.extend(summary.sessions_synced);
                    total_operations += summary.total_operations;
                    had_conflicts |= summary.had_conflicts;
                }
                Err(e) => {
                    tracing::error!("Failed to sync session {}: {}", ws.name, e);
                }
            }
        }
    }

    Ok(SyncSummary {
        sessions_synced,
        total_operations,
        had_conflicts,
    })
}

/// Sync the session associated with the current workspace.
pub async fn sync_current_workspace(
    options: SyncOptions,
) -> std::result::Result<SyncSummary, SyncError> {
    let cwd = std::env::current_dir().map_err(SyncError::IoError)?;
    let backend = create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let current_ws = workspaces.iter().find(|w| w.is_current).ok_or_else(|| {
        SyncError::ConfigurationError("No current workspace detected".to_string())
    })?;

    let session_name = SessionName::parse(&current_ws.name)?;
    sync_named_session(session_name, options).await
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

async fn sync_session_internal(
    backend: &dyn VcsBackend,
    name: &str,
    options: &SyncOptions,
) -> std::result::Result<SyncSummary, SyncError> {
    // Switch to workspace
    backend.switch_workspace(name)?;

    // Check status
    let status = backend.status()?;
    if status == VcsStatus::Dirty && !options.allow_dirty {
        return Err(SyncError::DirtyWorkspace(name.to_string()));
    }

    let target = options.target_branch.as_deref().map_or("main", |v| v);

    // Rebase with retries
    let mut attempts = 0;
    let max_attempts = options.retry_config.max_attempts;
    let mut delay = Duration::from_millis(options.retry_config.initial_delay_ms);

    loop {
        attempts += 1;
        match backend.rebase(target) {
            Ok(()) => break,
            Err(e) if attempts < max_attempts => {
                sleep(delay).await;
                delay *= 2;
                tracing::warn!(
                    "Rebase failed for {}, retrying (attempt {}/{}): {}",
                    name,
                    attempts,
                    max_attempts,
                    e
                );
            }
            Err(e) => {
                if e.to_string().contains("conflict") {
                    return Err(SyncError::Conflict {
                        workspace: name.to_string(),
                        files: "unknown (see jj status)".to_string(),
                    });
                }
                return Err(SyncError::JjCommandFailed(format!(
                    "Rebase failed after {} attempts: {}",
                    attempts, e
                )));
            }
        }
    }

    let final_status = backend.status()?;
    let had_conflicts = final_status == VcsStatus::Conflicted;

    Ok(SyncSummary {
        sessions_synced: vec![
            SessionName::parse(name).map_err(|e| SyncError::ConfigurationError(e.to_string()))?
        ],
        total_operations: attempts,
        had_conflicts,
    })
}

fn find_jj_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.join(".jj").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// JSONL EMIT HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn emit_action(
    verb_str: &str,
    target_str: &str,
    status: ActionStatus,
    result_str: Option<&str>,
) -> std::result::Result<(), SyncError> {
    let verb = ActionVerb::new(verb_str)?;
    let target = ActionTarget::new(target_str)?;
    let mut action = Action::new(verb, target, status);
    if let Some(r) = result_str {
        action = action.with_result(r.to_string());
    }
    emit_stdout(&OutputLine::Action(action)).map_err(SyncError::IoError)
}

fn emit_result(
    success: bool,
    kind: ResultKind,
    message_str: &str,
) -> std::result::Result<(), SyncError> {
    let message = Message::new(message_str)?;
    let result = if success {
        ResultOutput::success(kind, message)
    } else {
        ResultOutput::failure(kind, message)
    }?;
    emit_stdout(&OutputLine::Result(result)).map_err(SyncError::IoError)
}

fn emit_issue(
    id_str: &str,
    title_str: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> std::result::Result<(), SyncError> {
    let id = IssueId::new(id_str)?;
    let title = IssueTitle::new(title_str)?;
    let mut issue = Issue::new(id, title, kind, severity)?;
    if let Some(s) = session {
        issue = issue.with_session(
            SessionName::parse(s).map_err(|e| SyncError::ConfigurationError(e.to_string()))?,
        );
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }
    emit_stdout(&OutputLine::Issue(issue)).map_err(SyncError::IoError)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SyncError Display
    // -----------------------------------------------------------------------

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
        let msg = err.to_string();
        assert!(msg.contains("missing-session"));
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
        let err = SyncError::RetryLimitExceeded(5);
        let msg = err.to_string();
        assert!(msg.contains("5"));
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
        let err = SyncError::InvalidIdentifier("bad name!".to_string());
        let msg = err.to_string();
        assert!(msg.contains("bad name!"));
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
        let err = SyncError::LockTimeout(30);
        let msg = err.to_string();
        assert!(msg.contains("30"));
    }

    #[test]
    fn test_sync_error_display_jj_command_failed() {
        let err = SyncError::JjCommandFailed("rebase error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("rebase error"));
    }

    #[test]
    fn test_sync_error_display_retry_limit_exceeded() {
        let err = SyncError::RetryLimitExceeded(7);
        let msg = err.to_string();
        assert!(msg.contains("7"));
    }

    #[test]
    fn test_sync_error_display_workspace_path_not_accessible() {
        let err = SyncError::WorkspacePathNotAccessible(PathBuf::from("/no/access"));
        let msg = err.to_string();
        assert!(msg.contains("/no/access"));
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
        let err = SyncError::LockAcquisitionFailed("resource busy".to_string());
        let msg = err.to_string();
        assert!(msg.contains("resource busy"));
    }

    #[test]
    fn test_sync_error_display_session_database_not_found() {
        let err = SyncError::SessionDatabaseNotFound(PathBuf::from("/tmp/no.db"));
        let msg = err.to_string();
        assert!(msg.contains("/tmp/no.db"));
    }

    #[test]
    fn test_sync_error_display_session_database_read_failed() {
        let err = SyncError::SessionDatabaseReadFailed("corrupted".to_string());
        let msg = err.to_string();
        assert!(msg.contains("corrupted"));
    }

    #[test]
    fn test_sync_error_display_session_database_write_failed() {
        let err = SyncError::SessionDatabaseWriteFailed("disk full".to_string());
        let msg = err.to_string();
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn test_sync_error_display_configuration_error() {
        let err = SyncError::ConfigurationError("bad config".to_string());
        let msg = err.to_string();
        assert!(msg.contains("bad config"));
    }

    // -----------------------------------------------------------------------
    // SyncSummary edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_sync_summary_with_conflicts() {
        let session = SessionName::parse("my-session").expect("valid");
        let summary = SyncSummary {
            sessions_synced: vec![session],
            total_operations: 3,
            had_conflicts: true,
        };
        assert!(summary.had_conflicts);
        assert_eq!(summary.sessions_synced.len(), 1);
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

    // -----------------------------------------------------------------------
    // RetryConfig edge cases
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // find_jj_root additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_jj_root_deeply_nested_finds_ancestor() {
        let dir = std::env::temp_dir().join("hardline_test_jj_deep");
        let deep = dir.join("a/b/c/d");
        let _ = std::fs::create_dir_all(&deep);
        let _ = std::fs::create_dir(dir.join(".jj"));

        let result = find_jj_root(&deep);
        assert_eq!(result, Some(dir.clone()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_jj_root_sibling_directory_not_found() {
        // .jj is in a sibling, not ancestor, so it should not be found
        let dir = std::env::temp_dir().join("hardline_test_jj_sibling_parent");
        let child_a = dir.join("child_a");
        let child_b = dir.join("child_b");
        let _ = std::fs::create_dir_all(&child_a);
        let _ = std::fs::create_dir_all(&child_b);
        let _ = std::fs::create_dir(child_b.join(".jj"));

        let result = find_jj_root(&child_a);
        assert!(result.is_none(), "should not find .jj in sibling directory");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // SyncError From conversions edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_sync_error_from_scp_core_error_variants() {
        let core_err = Error::workspace_not_found("test-ws".to_string());
        let sync_err: SyncError = core_err.into();
        assert!(matches!(sync_err, SyncError::ConfigurationError(_)));
        assert!(sync_err.to_string().contains("test-ws"));
    }

    #[test]
    fn test_sync_error_non_io_variant_into_core_error_preserves_message() {
        let sync_err = SyncError::DirtyWorkspace("dirty-ws".to_string());
        let core_err: Error = sync_err.into();
        assert!(core_err.to_string().contains("dirty-ws"));
    }

    #[test]
    fn test_sync_error_into_core_error_conflict_preserves_workspace() {
        let sync_err = SyncError::Conflict {
            workspace: "my-workspace".to_string(),
            files: "a.rs, b.rs".to_string(),
        };
        let core_err: Error = sync_err.into();
        let msg = core_err.to_string();
        assert!(msg.contains("my-workspace"));
    }

    // -----------------------------------------------------------------------
    // SyncError From<Error> and From<std::io::Error>
    // -----------------------------------------------------------------------

    #[test]
    fn test_sync_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let sync_err: SyncError = io_err.into();
        assert!(matches!(sync_err, SyncError::IoError(_)));
        let msg = sync_err.to_string();
        assert!(msg.contains("file missing"));
    }

    #[test]
    fn test_sync_error_from_scp_core_error() {
        let core_err = Error::internal("something broke");
        let sync_err: SyncError = core_err.into();
        assert!(matches!(sync_err, SyncError::ConfigurationError(_)));
        let msg = sync_err.to_string();
        assert!(msg.contains("something broke"));
    }

    #[test]
    fn test_sync_error_from_io_error_kind_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let sync_err: SyncError = io_err.into();
        assert!(matches!(sync_err, SyncError::IoError(_)));
        let msg = sync_err.to_string();
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn test_sync_error_into_scp_core_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
        let sync_err: SyncError = SyncError::IoError(io_err);
        let core_err: Error = sync_err.into();
        // IoError variant maps to Error::from(io::Error)
        let msg = core_err.to_string();
        assert!(msg.contains("pipe broke"));
    }

    #[test]
    fn test_sync_error_non_io_into_core_error() {
        let sync_err = SyncError::SessionNotFound("x".to_string());
        let core_err: Error = sync_err.into();
        let msg = core_err.to_string();
        assert!(msg.contains("Session not found"));
    }

    // -----------------------------------------------------------------------
    // SyncOptions and SyncSummary construction
    // -----------------------------------------------------------------------

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
        assert_eq!(opts.retry_config.initial_delay_ms, 200);
    }

    #[test]
    fn test_sync_options_defaults() {
        // SyncOptions does not derive Default, so construct manually with
        // sensible defaults.
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
        assert_eq!(opts.lock_timeout_secs, 30);
        assert_eq!(opts.retry_config.max_attempts, 3);
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
        assert!(!summary.had_conflicts);
    }

    // -----------------------------------------------------------------------
    // RetryConfig
    // -----------------------------------------------------------------------

    #[test]
    fn test_retry_config_construction() {
        let config = RetryConfig {
            max_attempts: 10,
            initial_delay_ms: 500,
        };
        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay_ms, 500);
    }

    // -----------------------------------------------------------------------
    // find_jj_root
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_jj_root_finds_marker() {
        let dir = std::env::temp_dir().join("hardline_test_find_jj_root_found");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir(dir.join(".jj"));

        let result = find_jj_root(&dir);
        assert_eq!(result, Some(dir.clone()));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_jj_root_searches_parent() {
        let dir = std::env::temp_dir().join("hardline_test_find_jj_root_parent");
        let child = dir.join("subdir");
        let _ = std::fs::create_dir_all(&child);
        let _ = std::fs::create_dir(dir.join(".jj"));

        let result = find_jj_root(&child);
        assert_eq!(result, Some(dir.clone()));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_jj_root_not_found() {
        let dir = std::env::temp_dir().join("hardline_test_find_jj_root_missing");
        let _ = std::fs::create_dir_all(&dir);

        let result = find_jj_root(&dir);
        assert!(result.is_none());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_jj_root_stops_at_filesystem_root() {
        // /tmp should not have a .jj dir, so searching from a non-existent
        // nested path under /tmp should return None.
        // Use a non-existent directory to test the pop-until-empty behavior.
        let nonexistent = PathBuf::from("/nonexistent_dir_for_testing");
        // The directory won't exist, but find_jj_root only checks .jj existence
        // via .join().exists(), not that the current dir itself exists.
        let result = find_jj_root(&nonexistent);
        assert!(result.is_none());
    }
}

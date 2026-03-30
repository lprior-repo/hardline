//! Sync command handler for Port CLI
//!
//! Implementation of the sync command ported from isolate.

use scp_core::domain::SessionName;
use scp_core::jj_operation_sync::acquire_cross_process_lock;
use scp_core::output_jsonl::{
    emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
    IssueSeverity, IssueTitle, Message, Outcome, OutputLine, ResultKind, ResultOutput, Summary,
    SummaryType,
};
use scp_core::vcs::{create_backend, VcsBackend, VcsStatus};
use scp_core::{Error, Result};
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
    emit_stdout(&OutputLine::Action(action)).map_err(|e| SyncError::IoError(e))
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
    emit_stdout(&OutputLine::Result(result)).map_err(|e| SyncError::IoError(e))
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
    emit_stdout(&OutputLine::Issue(issue)).map_err(|e| SyncError::IoError(e))
}

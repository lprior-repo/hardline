//! Error classification and mapping logic

use crate::error::Error;

use super::error_code::ErrorCode;

/// Classify an error into a semantic exit code.
///
/// Exit codes follow this semantic mapping:
/// - 1: Usage/validation errors (invalid config, parse errors, validation failures)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors (JJ, hooks, etc.)
/// - 5: Lock contention errors
/// - 130: Operation cancelled (SIGINT)
pub const fn classify_exit_code(error: &crate::error::Error) -> i32 {
    match error {
        // Usage/validation errors: exit code 1
        Error::InvalidConfig(_)
        | Error::ConfigInvalid(_)
        | Error::ValidationError(_)
        | Error::ValidationFieldError { .. }
        | Error::InvalidIdentifier(_) => 1,
        // Not found errors: exit code 2
        Error::NotFound(_) | Error::SessionNotFound(_) | Error::WorkspaceNotFound(_) => 2,
        // System errors: exit code 3
        Error::Io(_) | Error::IoError(_) | Error::Database(_) => 3,
        // External command errors: exit code 4
        Error::JjCommandError { .. }
        | Error::JjWorkspaceConflict { .. }
        | Error::VcsNotInitialized
        | Error::VcsConflict(_, _)
        | Error::VcsPushFailed(_)
        | Error::VcsPullFailed(_)
        | Error::VcsRebaseFailed(_) => 4,
        // Lock contention errors: exit code 5
        Error::SessionLocked(_, _) | Error::WorkspaceLocked(_, _) | Error::LockTimeout { .. } => 5,
        // New error types
        Error::InvalidState(_) => 1,
        Error::QueueEmpty
        | Error::QueueItemNotFound(_)
        | Error::QueueLocked(_)
        | Error::QueueProcessing { .. } => 3,
        Error::AgentNotFound(_) | Error::AgentExists(_) | Error::AgentTimeout(_) => 3,
    }
}

/// Map a `crate::Error` to (`ErrorCode`, message, optional suggestion)
#[allow(clippy::too_many_lines)]
pub fn map_error_to_parts(err: &crate::error::Error) -> (ErrorCode, String, Option<String>) {
    match err {
        Error::InvalidConfig(msg) | Error::ConfigInvalid(msg) => (
            ErrorCode::ConfigParseError,
            format!("Invalid configuration: {msg}"),
            Some("Check your configuration file for errors".to_string()),
        ),
        Error::IoError(msg) | Error::Io(msg) => {
            (ErrorCode::Unknown, format!("IO error: {msg}"), None)
        }
        Error::JsonParse(msg) => (
            ErrorCode::ConfigParseError,
            format!("Parse error: {msg}"),
            None,
        ),
        Error::ValidationError(msg) => (
            ErrorCode::InvalidArgument,
            format!("Validation error: {msg}"),
            None,
        ),
        Error::ValidationFieldError { message, field, value } => {
            let full_message = match (field, value) {
                (Some(f), Some(v)) => {
                    format!("Validation error: {message} (field: {f}, value: {v})")
                }
                (Some(f), None) => format!("Validation error: {message} (field: {f})"),
                (None, Some(v)) => format!("Validation error: {message} (value: {v})"),
                (None, None) => format!("Validation error: {message}"),
            };
            (ErrorCode::InvalidArgument, full_message, None)
        }
        Error::NotFound(_) => (
            ErrorCode::SessionNotFound,
            "Not found".to_string(),
            Some("Use 'scp session list' to see available sessions".to_string()),
        ),
        Error::SessionNotFound { .. } => (
            ErrorCode::SessionNotFound,
            err.to_string(),
            Some("Use 'scp session list' to see available sessions".to_string()),
        ),
        Error::WorkspaceNotFound(_) => (
            ErrorCode::WorkspaceNotFound,
            err.to_string(),
            Some("Use 'scp workspace list' to see available workspaces".to_string()),
        ),
        Error::Database(msg) => (
            ErrorCode::StateDbCorrupted,
            format!("Database error: {msg}"),
            Some("Try running 'scp doctor --fix' to repair the database".to_string()),
        ),
        Error::JjCommandError {
            operation,
            msg,
            is_not_found,
        } => {
            if *is_not_found {
                (
                    ErrorCode::JjNotInstalled,
                    format!("Failed to {operation}: JJ is not installed or not in PATH"),
                    Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
                )
            } else {
                (
                    ErrorCode::JjCommandFailed,
                    format!("Failed to {operation}: {msg}"),
                    None,
                )
            }
        }
        Error::JjWorkspaceConflict {
            conflict_type,
            workspace_name,
            msg,
            recovery_hint,
        } => (
            ErrorCode::JjCommandFailed,
            format!(
                "JJ workspace conflict: {conflict_type:?}\nWorkspace: {workspace_name}\n{recovery_hint}\nJJ error: {msg}"
            ),
            Some("Follow the recovery hints in the error message".to_string()),
        ),
        Error::SessionLocked { session, holder } => (
            ErrorCode::Unknown,
            format!("Session '{session}' is locked by agent '{holder}'"),
            Some("Wait for the other agent to finish or check lock status".to_string()),
        ),
        Error::WorkspaceLocked { .. } => (ErrorCode::Unknown, err.to_string(), None),
        Error::LockTimeout {
            operation,
            timeout_ms,
            retries,
        } => (
            ErrorCode::Unknown,
            format!(
                "Lock acquisition timeout for '{operation}' after {retries} retries (timeout: {timeout_ms}ms per attempt)"
            ),
            Some("System is under heavy load. Wait a few moments and retry".to_string()),
        ),
        Error::InvalidState(msg) => (
            ErrorCode::InvalidArgument,
            format!("Invalid state: {msg}"),
            None,
        ),
        Error::VcsNotInitialized => (
            ErrorCode::NotJjRepository,
            "VCS not initialized".to_string(),
            Some("Run 'scp init' to initialize VCS".to_string()),
        ),
        Error::VcsConflict(_, msg) => (
            ErrorCode::JjCommandFailed,
            format!("VCS conflict: {msg}"),
            None,
        ),
        Error::BranchNotFound(branch) => (
            ErrorCode::SpawnBeadNotFound,
            format!("Branch not found: {branch}"),
            None,
        ),
        Error::WorkingCopyDirty => (
            ErrorCode::JjCommandFailed,
            "Working copy has uncommitted changes".to_string(),
            Some("Commit or stash your changes before continuing".to_string()),
        ),
        _ => (ErrorCode::Unknown, err.to_string(), err.suggestion()),
    }
}

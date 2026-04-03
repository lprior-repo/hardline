//! JSON error types and basic error structures

use serde::{Deserialize, Serialize};

use super::error_mapping::{classify_exit_code, map_error_to_parts};

/// Standard JSON success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSuccess<T> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> JsonSuccess<T> {
    /// Create a new success response
    pub const fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard JSON error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub success: bool,
    pub error: ErrorDetail,
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "UNKNOWN".to_string(),
                message: "An unknown error occurred".to_string(),
                exit_code: 4,
                details: None,
                suggestion: None,
            },
        }
    }
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code (`SCREAMING_SNAKE_CASE`)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Semantic exit code (1-4)
    pub exit_code: i32,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl JsonError {
    /// Create a new JSON error with just a code and message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                exit_code: 4, // Default to unknown/external error
                details: None,
                suggestion: None,
            },
        }
    }

    /// Add details to the error
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code: self.error.exit_code,
                details: Some(details),
                suggestion: self.error.suggestion,
            },
        }
    }

    /// Add a suggestion to the error
    #[must_use]
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code: self.error.exit_code,
                details: self.error.details,
                suggestion: Some(suggestion.into()),
            },
        }
    }

    /// Set exit code for this error
    #[must_use]
    pub fn with_exit_code(self, exit_code: i32) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: self.error.code,
                message: self.error.message,
                exit_code,
                details: self.error.details,
                suggestion: self.error.suggestion,
            },
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::Error::io_error(format!("Failed to serialize JSON: {e}")))
    }
}

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    /// Includes structured context from the error's `context_map()` in the
    /// `details` field.
    #[must_use]
    pub fn from_error(error: &crate::error::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: error.context_map(),
            suggestion: error.suggestion(),
        }
    }
}

impl From<&crate::error::Error> for JsonError {
    fn from(err: &crate::error::Error) -> Self {
        let (code, message, suggestion) = map_error_to_parts(err);

        let json_error = Self::new(code, message);
        let json_error = match suggestion {
            Some(sugg) => json_error.with_suggestion(sugg),
            None => json_error,
        };
        // Override exit code to match the error classification
        let json_error = json_error.with_exit_code(classify_exit_code(err));
        // Include error context details from context_map()
        let details = err.context_map();
        match details {
            Some(d) => json_error.with_details(d),
            None => json_error,
        }
    }
}

impl From<crate::error::Error> for JsonError {
    fn from(err: crate::error::Error) -> Self {
        Self::from(&err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ErrorDetail::from_error() for each Error variant ─────────────

    #[test]
    fn test_error_detail_from_workspace_not_found() {
        let err = crate::error::Error::workspace_not_found("my-workspace");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WORKSPACE_NOT_FOUND");
        assert!(detail.message.contains("my-workspace"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        assert!(detail.suggestion.is_some());
    }

    #[test]
    fn test_error_detail_from_session_not_found() {
        let err = crate::error::Error::session("my-session");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "SESSION_NOT_FOUND");
        assert!(detail.message.contains("my-session"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        assert!(detail.suggestion.is_some());
    }

    #[test]
    fn test_error_detail_from_queue_empty() {
        let err = crate::error::Error::queue_empty();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_EMPTY");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        assert!(detail.suggestion.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_not_initialized() {
        let err = crate::error::Error::vcs_not_initialized();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_NOT_INITIALIZED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        assert!(detail.suggestion.is_some());
    }

    #[test]
    fn test_error_detail_from_config_not_found() {
        let err = crate::error::Error::config_not_found("file.toml");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "CONFIG_NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        // Config errors have no suggestion on the Error itself
    }

    #[test]
    fn test_error_detail_from_agent_not_found() {
        let err = crate::error::Error::agent_not_found("agent-1");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "AGENT_NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_io_error() {
        let err = crate::error::Error::io_error("disk full");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "IO_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_state_not_found() {
        let err = crate::error::Error::not_found("resource-xyz");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        assert!(detail.suggestion.is_some());
    }

    #[test]
    fn test_error_detail_from_state_validation_error() {
        let err = crate::error::Error::validation_error("bad input");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VALIDATION_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_internal_error() {
        let err = crate::error::Error::internal("invariant violated");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "INTERNAL_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // -- VCS error variants --

    #[test]
    fn test_error_detail_from_database_error() {
        let err = crate::error::Error::database("corruption detected");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "DATABASE_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_lock_session_locked() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::SessionLocked {
            session: "s1".to_string(),
            holder: "agent-1".to_string(),
        }
        .into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "SESSION_LOCKED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let details = detail.details.unwrap();
        assert_eq!(details["session"], "s1");
        assert_eq!(details["holder"], "agent-1");
    }

    #[test]
    fn test_error_detail_from_batch_empty() {
        let err = crate::error::Error::batch_empty();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BATCH_EMPTY");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── Workspace error variants ─────────────────────────────────────

    #[test]
    fn test_error_detail_from_workspace_exists() {
        let err = crate::error::Error::workspace_exists("my-workspace");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WORKSPACE_EXISTS");
        assert!(detail.message.contains("my-workspace"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_workspace_locked() {
        let err = crate::error::Error::workspace_locked("ws-1", "agent-1");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WORKSPACE_LOCKED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["workspace_name"], "ws-1");
        assert_eq!(ctx["holder"], "agent-1");
    }

    #[test]
    fn test_error_detail_from_workspace_conflict() {
        let err = crate::error::Error::workspace_conflict("concurrent write");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WORKSPACE_CONFLICT");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["message"], "concurrent write");
    }

    // ── Session error variants ───────────────────────────────────────

    #[test]
    fn test_error_detail_from_session_exists() {
        let err = crate::error::Error::session_exists("my-session");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "SESSION_EXISTS");
        assert!(detail.message.contains("my-session"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_not_lock_holder() {
        let err = crate::error::Error::not_lock_holder("s1", "agent-2");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "NOT_LOCK_HOLDER");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["agent_id"], "agent-2");
    }

    #[test]
    fn test_error_detail_from_session_invalid_state() {
        let err = crate::error::Error::session_invalid_state("s1", "active", "paused");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "SESSION_INVALID_STATE");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["actual_state"], "active");
        assert_eq!(ctx["expected_state"], "paused");
    }

    // ── Queue error variants ─────────────────────────────────────────

    #[test]
    fn test_error_detail_from_queue_item_not_found() {
        let err = crate::error::Error::queue_item_not_found("item-42");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_ITEM_NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["item"], "item-42");
    }

    #[test]
    fn test_error_detail_from_queue_locked() {
        let err = crate::error::Error::queue_locked("agent-1");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_LOCKED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["holder"], "agent-1");
    }

    #[test]
    fn test_error_detail_from_queue_processing() {
        let err = crate::error::Error::queue_processing();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_PROCESSING");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_queue_invalid_position() {
        let err = crate::error::Error::queue_invalid_position(99);
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_INVALID_POSITION");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["position"], 99);
    }

    #[test]
    fn test_error_detail_from_queue_full() {
        let err = crate::error::Error::queue_full(100);
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "QUEUE_FULL");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["max_size"], 100);
    }

    // ── Config error variants ────────────────────────────────────────

    #[test]
    fn test_error_detail_from_config_invalid() {
        let err = crate::error::Error::config_invalid("missing key");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "CONFIG_INVALID");
        assert!(detail.message.contains("missing key"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        // Config errors have no suggestion on the Error itself
        assert!(detail.suggestion.is_none());
    }

    #[test]
    fn test_error_detail_from_config_permission() {
        let err = crate::error::Error::config_permission("read access denied");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "CONFIG_PERMISSION");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── Agent error variants ─────────────────────────────────────────

    #[test]
    fn test_error_detail_from_agent_exists() {
        let err = crate::error::Error::agent_exists("agent-1");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "AGENT_EXISTS");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["agent_id"], "agent-1");
    }

    #[test]
    fn test_error_detail_from_agent_timeout() {
        let err: crate::error::Error =
            crate::error_agent::AgentErrorKind::Timeout("agent-1".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "AGENT_TIMEOUT");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["agent_id"], "agent-1");
    }

    // ── State error variants ─────────────────────────────────────────

    #[test]
    fn test_error_detail_from_invalid_state() {
        let err = crate::error::Error::invalid_state("expected active");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "INVALID_STATE");
        assert!(detail.message.contains("expected active"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_invalid_identifier() {
        let err = crate::error::Error::invalid_identifier("bad name!");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "INVALID_IDENTIFIER");
        assert!(detail.message.contains("bad name!"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── Internal error variants ──────────────────────────────────────

    #[test]
    fn test_error_detail_from_unimplemented() {
        let err = crate::error::Error::unimplemented("future feature");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "UNIMPLEMENTED");
        assert!(detail.message.contains("future feature"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── VCS error variants ───────────────────────────────────────────

    #[test]
    fn test_error_detail_from_branch_not_found() {
        let err = crate::error::Error::branch_not_found("feature-xyz");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BRANCH_NOT_FOUND");
        assert!(detail.message.contains("feature-xyz"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_branch_exists() {
        let err = crate::error::Error::branch_exists("main");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BRANCH_EXISTS");
        assert!(detail.message.contains("main"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_commit_not_found() {
        let err = crate::error::Error::commit_not_found("abc123");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "COMMIT_NOT_FOUND");
        assert!(detail.message.contains("abc123"));
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_working_copy_dirty() {
        let err = crate::error::Error::working_copy_dirty();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WORKING_COPY_DIRTY");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_conflict() {
        let err = crate::error::Error::vcs_conflict("my-repo", "merge conflict");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_CONFLICT");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["repo"], "my-repo");
        assert_eq!(ctx["message"], "merge conflict");
    }

    #[test]
    fn test_error_detail_from_vcs_push_failed() {
        let err = crate::error::Error::vcs_push_failed("network error");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_PUSH_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_pull_failed() {
        let err = crate::error::Error::vcs_pull_failed("timeout");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_PULL_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_rebase_failed() {
        let err = crate::error::Error::vcs_rebase_failed("conflict");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_REBASE_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_commit_failed() {
        let err = crate::error::Error::vcs_commit_failed("disk full");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_COMMIT_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_checkout_failed() {
        let err = crate::error::Error::vcs_checkout_failed("race condition");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_CHECKOUT_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_diff_failed() {
        let err = crate::error::Error::vcs_diff_failed("binary file");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_DIFF_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_vcs_init_failed() {
        let err = crate::error::Error::vcs_init_failed("jj", "/tmp/test", "not found");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "VCS_INIT_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["vcs_type"], "jj");
        assert_eq!(ctx["directory"], "/tmp/test");
        assert_eq!(ctx["reason"], "not found");
    }

    // ── Wait/Batch error variants ────────────────────────────────────

    #[test]
    fn test_error_detail_from_batch_command_failed() {
        let err = crate::error::Error::batch_command_failed("exit code 1");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BATCH_COMMAND_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_batch_rollback_failed() {
        let err = crate::error::Error::batch_rollback_failed("state corrupted");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BATCH_ROLLBACK_FAILED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_batch_size_exceeded() {
        let err = crate::error::Error::batch_size_exceeded(50);
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "BATCH_SIZE_EXCEEDED");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["max_size"], 50);
    }

    #[test]
    fn test_error_detail_from_checkpoint_error() {
        let err = crate::error::Error::checkpoint_error("write failed");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "CHECKPOINT_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_wait_timeout() {
        let err: crate::error::Error =
            crate::error_wait::WaitErrorKind::Timeout("s1".into(), "idle".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "WAIT_TIMEOUT");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["waiting_for"], "idle");
    }

    #[test]
    fn test_error_detail_from_invalid_wait_mode() {
        let err: crate::error::Error =
            crate::error_wait::WaitErrorKind::InvalidWaitMode("bad-mode".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "INVALID_WAIT_MODE");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_invalid_session_name() {
        let err: crate::error::Error =
            crate::error_wait::WaitErrorKind::InvalidSessionName("bad!".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "INVALID_SESSION_NAME");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── Lock error variants ──────────────────────────────────────────

    #[test]
    fn test_error_detail_from_lock_session_not_found() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::SessionNotFound {
            session: "ghost-session".to_string(),
        }
        .into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "SESSION_NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_lock_not_lock_holder() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::NotLockHolder {
            session: "s1".to_string(),
            agent_id: "imposter".to_string(),
        }
        .into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "NOT_LOCK_HOLDER");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["agent_id"], "imposter");
    }

    #[test]
    fn test_error_detail_from_lock_not_found() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::NotFound("lock file missing".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "NOT_FOUND");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_lock_database_error() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error =
            LockErrorKind::DatabaseError("corruption".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "DATABASE_ERROR");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_from_lock_ttl_out_of_range() {
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error =
            LockErrorKind::TtlOutOfRange("TTL must be >= 0".into()).into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.code, "TTL_OUT_OF_RANGE");
        assert!(detail.exit_code > 0);
        assert!(detail.details.is_some());
    }

    // ── Fields populated correctly ───────────────────────────────────

    #[test]
    fn test_error_detail_context_fields_workspace() {
        let err = crate::error::Error::workspace_not_found("my-workspace");
        let detail = ErrorDetail::from_error(&err);
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["resource_type"], "workspace");
        assert_eq!(ctx["workspace_name"], "my-workspace");
        assert_eq!(ctx["searched_in"], "database");
    }

    #[test]
    fn test_error_detail_context_fields_session_locked() {
        let err = crate::error::Error::session_locked("s1", "agent-1");
        let detail = ErrorDetail::from_error(&err);
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["session"], "s1");
        assert_eq!(ctx["holder"], "agent-1");
    }

    #[test]
    fn test_error_detail_context_fields_validation_field() {
        let err = crate::error::Error::validation_field_error("name", "too short", Some("ab".into()));
        let detail = ErrorDetail::from_error(&err);
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["field"], "name");
        assert_eq!(ctx["message"], "too short");
        assert_eq!(ctx["value"], "ab");
    }

    #[test]
    fn test_error_detail_context_fields_validation_field_no_value() {
        let err = crate::error::Error::validation_field_error("name", "required", None);
        let detail = ErrorDetail::from_error(&err);
        let ctx = detail.details.unwrap();
        assert_eq!(ctx["field"], "name");
        assert!(!ctx.as_object().map_or(true, |o| o.contains_key("value")));
    }

    #[test]
    fn test_error_detail_exit_code_classification() {
        // Config -> 1 (usage/validation)
        let err = crate::error::Error::config_invalid("bad");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.exit_code, 1);

        // Session not found -> 2 (not found)
        let err = crate::error::Error::session("s");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.exit_code, 2);

        // IO -> 3 (system)
        let err = crate::error::Error::io_error("disk full");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.exit_code, 3);

        // Internal -> 4 (internal error)
        let err = crate::error::Error::internal("invariant violated");
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.exit_code, 4);

        // Lock -> 5 (lock contention)
        use crate::coordination::locks::errors::LockErrorKind;
        let err: crate::error::Error = LockErrorKind::SessionLocked {
            session: "s".into(),
            holder: "h".into(),
        }
        .into();
        let detail = ErrorDetail::from_error(&err);
        assert_eq!(detail.exit_code, 5);
    }

    // ── JsonError construction and builder pattern ───────────────────

    #[test]
    fn test_json_error_new() {
        let err = JsonError::new("TEST_CODE", "test message");
        assert_eq!(err.success, false);
        assert_eq!(err.error.code, "TEST_CODE");
        assert_eq!(err.error.message, "test message");
        assert_eq!(err.error.exit_code, 4); // default
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_builder_pattern() {
        let err = JsonError::new("TEST_CODE", "test message")
            .with_suggestion("try again")
            .with_exit_code(2)
            .with_details(serde_json::json!({"key": "value"}));

        assert_eq!(err.error.suggestion.as_deref(), Some("try again"));
        assert_eq!(err.error.exit_code, 2);
        let details = err.error.details.unwrap();
        assert_eq!(details["key"], "value");
    }

    #[test]
    fn test_json_error_default() {
        let err = JsonError::default();
        assert_eq!(err.success, false);
        assert_eq!(err.error.code, "UNKNOWN");
        assert_eq!(err.error.message, "An unknown error occurred");
        assert_eq!(err.error.exit_code, 4);
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_from_error_ref() {
        let err = crate::error::Error::workspace_not_found("ws");
        let json_err = JsonError::from(&err);
        assert_eq!(json_err.success, false);
        assert_eq!(json_err.error.code, "WORKSPACE_NOT_FOUND");
    }

    #[test]
    fn test_json_error_from_error_owned() {
        let err = crate::error::Error::session("s");
        let json_err = JsonError::from(err);
        assert_eq!(json_err.success, false);
        assert_eq!(json_err.error.code, "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_json_error_to_json() {
        let err = JsonError::new("TEST", "test message");
        let json_str = err.to_json().expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should parse");
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "TEST");
        assert_eq!(parsed["error"]["message"], "test message");
    }

    #[test]
    fn test_json_error_to_json_includes_suggestion_and_details() {
        let err = JsonError::new("TEST", "msg")
            .with_suggestion("fix it")
            .with_details(serde_json::json!({"info": "here"}));
        let json_str = err.to_json().expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should parse");
        assert_eq!(parsed["error"]["suggestion"], "fix it");
        assert_eq!(parsed["error"]["details"]["info"], "here");
    }

    #[test]
    fn test_json_error_to_json_omits_none_fields() {
        let err = JsonError::new("TEST", "msg");
        let json_str = err.to_json().expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should parse");
        assert!(parsed["error"].get("suggestion").is_none());
        assert!(parsed["error"].get("details").is_none());
    }

    // ── JsonSuccess construction ──────────────────────────────────────

    #[test]
    fn test_json_success_new() {
        let success = JsonSuccess::new(serde_json::json!({"key": "value"}));
        assert!(success.success);
        assert_eq!(success.data["key"], "value");
    }
}

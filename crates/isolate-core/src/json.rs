//! JSON output structures for AI-first CLI design
//!
//! This module provides consistent JSON output formats across all commands.

use serde::{Deserialize, Serialize};

use crate::{fix::Fix, hints::NextAction};

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA REGISTRY - Single Source of Truth for Schema IDs
// ═══════════════════════════════════════════════════════════════════════════

/// Schema name constants for all CLI JSON output schemas.
///
/// These constants ensure that:
/// 1. Contract documentation and runtime code use the same schema IDs
/// 2. Schema IDs are compile-time checked
/// 3. No drift between documentation and implementation
///
/// # Conformance
///
/// - Commands MUST use these constants when creating `SchemaEnvelope`
/// - Contract.rs MUST reference these for `output_schema` documentation
/// - Tests verify both contract and runtime use the same values
pub mod schemas {
    /// Schema version for all responses
    pub const SCHEMA_VERSION: &str = "1.0";

    /// Base URI for all schemas
    pub const BASE_URI: &str = "isolate://";

    // Command response schemas
    pub const INIT_RESPONSE: &str = "init-response";
    pub const ADD_RESPONSE: &str = "add-response";
    pub const LIST_RESPONSE: &str = "list-response";
    pub const REMOVE_RESPONSE: &str = "remove-response";
    pub const FOCUS_RESPONSE: &str = "focus-response";
    pub const STATUS_RESPONSE: &str = "status-response";
    pub const SYNC_RESPONSE: &str = "sync-response";
    pub const DONE_RESPONSE: &str = "done-response";
    pub const UNDO_RESPONSE: &str = "undo-response";
    pub const REVERT_RESPONSE: &str = "revert-response";
    pub const WORK_RESPONSE: &str = "work-response";
    pub const ABORT_RESPONSE: &str = "abort-response";
    pub const SPAWN_RESPONSE: &str = "spawn-response";
    pub const WHEREAMI_RESPONSE: &str = "whereami-response";
    pub const WHOAMI_RESPONSE: &str = "whoami-response";
    pub const DOCTOR_RESPONSE: &str = "doctor-response";
    pub const CLEAN_RESPONSE: &str = "clean-response";
    pub const CONTEXT_RESPONSE: &str = "context-response";
    pub const INTROSPECT_RESPONSE: &str = "introspect-response";
    pub const CHECKPOINT_RESPONSE: &str = "checkpoint-response";
    pub const CONTRACT_RESPONSE: &str = "contract-response";
    pub const CONTRACTS_RESPONSE: &str = "contracts-response";
    pub const SUBMIT_RESPONSE: &str = "submit-response";
    pub const EXPORT_RESPONSE: &str = "export-response";
    pub const IMPORT_RESPONSE: &str = "import-response";
    pub const CLI_DISPLAY_RESPONSE: &str = "cli-display-response";

    // Diff schemas
    pub const DIFF_RESPONSE: &str = "diff-response";
    pub const DIFF_STAT_RESPONSE: &str = "diff-stat-response";

    // Query schemas
    pub const QUERY_SESSION_EXISTS: &str = "query-session-exists";
    pub const QUERY_CAN_RUN: &str = "query-can-run";
    pub const QUERY_SUGGEST_NAME: &str = "query-suggest-name";
    pub const QUERY_LOCK_STATUS: &str = "query-lock-status";
    pub const QUERY_CAN_SPAWN: &str = "query-can-spawn";
    pub const QUERY_PENDING_MERGES: &str = "query-pending-merges";
    pub const QUERY_LOCATION: &str = "query-location";

    // Error schema
    pub const ERROR_RESPONSE: &str = "error-response";

    /// Build a full schema URI from a schema name
    #[must_use]
    pub fn uri(schema_name: &str) -> String {
        format!("{BASE_URI}{schema_name}/v1")
    }

    /// Get all valid schema names for validation
    ///
    /// # Returns
    ///
    /// Returns a vector of all valid schema names. The result should be used
    /// for validation or schema discovery purposes.
    #[must_use]
    pub fn all_valid_schemas() -> Vec<&'static str> {
        vec![
            INIT_RESPONSE,
            ADD_RESPONSE,
            LIST_RESPONSE,
            REMOVE_RESPONSE,
            FOCUS_RESPONSE,
            STATUS_RESPONSE,
            SYNC_RESPONSE,
            DONE_RESPONSE,
            UNDO_RESPONSE,
            REVERT_RESPONSE,
            WORK_RESPONSE,
            ABORT_RESPONSE,
            SPAWN_RESPONSE,
            WHEREAMI_RESPONSE,
            WHOAMI_RESPONSE,
            DOCTOR_RESPONSE,
            CLEAN_RESPONSE,
            CONTEXT_RESPONSE,
            INTROSPECT_RESPONSE,
            CHECKPOINT_RESPONSE,
            CONTRACT_RESPONSE,
            CONTRACTS_RESPONSE,
            SUBMIT_RESPONSE,
            EXPORT_RESPONSE,
            IMPORT_RESPONSE,
            CLI_DISPLAY_RESPONSE,
            DIFF_RESPONSE,
            DIFF_STAT_RESPONSE,
            QUERY_SESSION_EXISTS,
            QUERY_CAN_RUN,
            QUERY_SUGGEST_NAME,
            QUERY_LOCK_STATUS,
            QUERY_CAN_SPAWN,
            QUERY_PENDING_MERGES,
            QUERY_LOCATION,
            ERROR_RESPONSE,
        ]
    }

    /// Check if a schema name is valid
    #[must_use]
    pub fn is_valid_schema(schema_name: &str) -> bool {
        all_valid_schemas().contains(&schema_name)
    }
}

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
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }

    /// Add a suggestion to the error
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error.suggestion = Some(suggestion.into());
        self
    }

    /// Set exit code for this error
    #[must_use]
    pub const fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.error.exit_code = exit_code;
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize error: {e}")))
    }
}

/// Error codes for machine-readable errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Session errors
    SessionNotFound,
    SessionAlreadyExists,
    SessionNameInvalid,

    // Workspace errors
    WorkspaceCreationFailed,
    WorkspaceNotFound,

    // JJ errors
    JjNotInstalled,
    JjCommandFailed,
    NotJjRepository,

    // Zellij errors
    ZellijNotRunning,
    ZellijCommandFailed,

    // Config errors
    ConfigNotFound,
    ConfigParseError,
    ConfigKeyNotFound,

    // Hook errors
    HookFailed,
    HookExecutionError,

    // State errors
    StateDbCorrupted,
    StateDbLocked,

    // Undo errors
    ReadUndoLogFailed,
    WriteUndoLogFailed,

    // Spawn errors
    SpawnNotOnMain,
    SpawnInvalidBeadStatus,
    SpawnBeadNotFound,
    SpawnWorkspaceCreationFailed,
    SpawnAgentSpawnFailed,
    SpawnTimeout,
    SpawnMergeFailed,
    SpawnCleanupFailed,
    SpawnDatabaseError,
    SpawnJjCommandFailed,

    // Generic errors
    InvalidArgument,
    Unknown,
}

impl ErrorCode {
    /// Get the string representation of the error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::ReadUndoLogFailed => "READ_UNDO_LOG_FAILED",
            Self::WriteUndoLogFailed => "WRITE_UNDO_LOG_FAILED",
            Self::SpawnNotOnMain => "SPAWN_NOT_ON_MAIN",
            Self::SpawnInvalidBeadStatus => "SPAWN_INVALID_BEAD_STATUS",
            Self::SpawnBeadNotFound => "SPAWN_BEAD_NOT_FOUND",
            Self::SpawnWorkspaceCreationFailed => "SPAWN_WORKSPACE_CREATION_FAILED",
            Self::SpawnAgentSpawnFailed => "SPAWN_AGENT_SPAWN_FAILED",
            Self::SpawnTimeout => "SPAWN_TIMEOUT",
            Self::SpawnMergeFailed => "SPAWN_MERGE_FAILED",
            Self::SpawnCleanupFailed => "SPAWN_CLEANUP_FAILED",
            Self::SpawnDatabaseError => "SPAWN_DATABASE_ERROR",
            Self::SpawnJjCommandFailed => "SPAWN_JJ_COMMAND_FAILED",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

/// Classify an error into a semantic exit code.
///
/// Exit codes follow this semantic mapping:
/// - 1: Usage/validation errors (invalid config, parse errors, validation failures)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors (JJ, hooks, etc.)
/// - 5: Lock contention errors
/// - 130: Operation cancelled (SIGINT)
const fn classify_exit_code(error: &crate::Error) -> i32 {
    use crate::Error;
    match error {
        // Usage/validation errors: exit code 1
        Error::InvalidConfig(_)
        | Error::ParseError(_)
        | Error::DedupeKeyConflict { .. }
        | Error::ValidationError { .. } => 1,
        // Not found errors: exit code 2
        Error::NotFound(_) | Error::SessionNotFound { .. } => 2,
        // System errors: exit code 3
        Error::IoError(_) | Error::DatabaseError(_) | Error::Serialization(_) | Error::Io(_) => 3,
        // External command errors: exit code 4
        Error::Command(_)
        | Error::JjCommandError { .. }
        | Error::JjWorkspaceConflict { .. }
        | Error::HookFailed { .. }
        | Error::HookExecutionFailed { .. }
        | Error::Unknown(_) => 4,
        // Lock contention errors: exit code 5
        Error::SessionLocked { .. } | Error::NotLockHolder { .. } | Error::LockTimeout { .. } => 5,
        // Operation cancelled: exit code 130
        Error::OperationCancelled(_) => 130,
        // New error types
        Error::InvalidId(_) | Error::InvalidInput(_) | Error::InvalidState(_) => 1,
        Error::QueueError(_) | Error::AgentError(_) | Error::DagError(_) | Error::VcsError(_) => 3,
    }
}

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    #[must_use]
    pub fn from_error(error: &crate::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: error.context_map(),
            suggestion: error.suggestion(),
        }
    }
}

/// Map a `crate::Error` to (`ErrorCode`, message, optional suggestion)
#[allow(clippy::too_many_lines)]
fn map_error_to_parts(err: &crate::Error) -> (ErrorCode, String, Option<String>) {
    use crate::Error;

    match err {
        Error::InvalidConfig(msg) => (
            ErrorCode::ConfigParseError,
            format!("Invalid configuration: {msg}"),
            Some("Check your configuration file for errors".to_string()),
        ),
        Error::IoError(msg) => (ErrorCode::Unknown, format!("IO error: {msg}"), None),
        Error::ParseError(msg) => (
            ErrorCode::ConfigParseError,
            format!("Parse error: {msg}"),
            None,
        ),
        Error::ValidationError { message, field, value, constraints } => {
            let full_message = match (field, value) {
                (Some(f), Some(v)) => format!("Validation error: {message} (field: {f}, value: {v})"),
                (Some(f), None) => format!("Validation error: {message} (field: {f})"),
                (None, Some(v)) => format!("Validation error: {message} (value: {v})"),
                (None, None) => format!("Validation error: {message}"),
            };
            let suggestion = if constraints.is_empty() {
                None
            } else {
                Some(format!("Valid values: {}", constraints.join(", ")))
            };
            (ErrorCode::InvalidArgument, full_message, suggestion)
        },
        Error::NotFound(_) => (
            ErrorCode::SessionNotFound,
            "Not found".to_string(),
            Some("Use 'isolate list' to see available sessions".to_string()),
        ),
        Error::SessionNotFound { session } => (
            ErrorCode::SessionNotFound,
            format!("Session '{session}' not found"),
            Some("Use 'isolate list' to see available sessions".to_string()),
        ),
        Error::DatabaseError(msg) => (
            ErrorCode::StateDbCorrupted,
            format!("Database error: {msg}"),
            Some("Try running 'isolate doctor --fix' to repair the database".to_string()),
        ),
        Error::Command(msg) => (ErrorCode::Unknown, format!("Command error: {msg}"), None),
        Error::HookFailed {
            hook_type,
            command,
            exit_code,
            stdout: _,
            stderr,
        } => (
            ErrorCode::HookFailed,
            format!(
                "Hook '{hook_type}' failed: {command}\nExit code: {exit_code:?}\nStderr: {stderr}"
            ),
            Some("Check your hook configuration and ensure the command is correct".to_string()),
        ),
        Error::HookExecutionFailed { command, source } => (
            ErrorCode::HookExecutionError,
            format!("Failed to execute hook '{command}': {source}"),
            Some("Ensure the hook command exists and is executable".to_string()),
        ),
        Error::JjCommandError {
            operation,
            source,
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
                    format!("Failed to {operation}: {source}"),
                    None,
                )
            }
        }
        Error::JjWorkspaceConflict {
            conflict_type,
            workspace_name,
            source,
            recovery_hint,
        } => (
            ErrorCode::JjCommandFailed,
            format!("JJ workspace conflict: {conflict_type}\nWorkspace: {workspace_name}\n{recovery_hint}\nJJ error: {source}"),
            Some("Follow the recovery hints in the error message".to_string()),
        ),
        Error::Unknown(msg) => (ErrorCode::Unknown, format!("Unknown error: {msg}"), None),
        Error::SessionLocked { session, holder } => (
            ErrorCode::Unknown,
            format!("Session '{session}' is locked by agent '{holder}'"),
            Some("Wait for the other agent to finish or check lock status".to_string()),
        ),
        Error::NotLockHolder { session, agent_id } => (
            ErrorCode::Unknown,
            format!("Agent '{agent_id}' does not hold the lock for session '{session}'"),
            None,
        ),
        Error::LockTimeout { operation, timeout_ms, retries } => (
            ErrorCode::Unknown,
            format!("Lock acquisition timeout for '{operation}' after {retries} retries (timeout: {timeout_ms}ms per attempt)"),
            Some("System is under heavy load. Wait a few moments and retry".to_string()),
        ),
        Error::OperationCancelled(reason) => (
            ErrorCode::Unknown,
            format!("Operation cancelled: {reason}"),
            Some("Operation was interrupted by shutdown signal".to_string()),
        ),
        Error::Serialization(msg) => (
            ErrorCode::Unknown,
            format!("Serialization error: {msg}"),
            Some("Data could not be serialized. Check for invalid characters or types.".to_string()),
        ),
        Error::Io(msg) => (
            ErrorCode::Unknown,
            format!("IO error: {msg}"),
            Some("An I/O operation failed. Check file permissions and disk space.".to_string()),
        ),
        Error::DedupeKeyConflict {
            dedupe_key,
            existing_workspace,
            provided_workspace,
        } => (
            ErrorCode::InvalidArgument,
            format!(
                "Dedupe key conflict: '{dedupe_key}' already used by workspace '{existing_workspace}', cannot be used by '{provided_workspace}'"
            ),
            Some("Use a different dedupe_key or wait for the existing entry to complete".to_string()),
        ),
        Error::InvalidId(msg) => (
            ErrorCode::InvalidArgument,
            format!("Invalid ID: {msg}"),
            None,
        ),
        Error::InvalidInput(msg) => (
            ErrorCode::InvalidArgument,
            format!("Invalid input: {msg}"),
            None,
        ),
        Error::InvalidState(msg) => (
            ErrorCode::InvalidArgument,
            format!("Invalid state: {msg}"),
            None,
        ),
        Error::QueueError(msg) => (
            ErrorCode::Unknown,
            format!("Queue error: {msg}"),
            None,
        ),
        Error::AgentError(msg) => (
            ErrorCode::Unknown,
            format!("Agent error: {msg}"),
            None,
        ),
        Error::DagError(msg) => (
            ErrorCode::Unknown,
            format!("DAG error: {msg}"),
            None,
        ),
        Error::VcsError(msg) => (
            ErrorCode::Unknown,
            format!("VCS error: {msg}"),
            None,
        ),
    }
}

impl From<&crate::Error> for JsonError {
    fn from(err: &crate::Error) -> Self {
        let (code, message, suggestion) = map_error_to_parts(err);

        let mut json_error = Self::new(code, message);
        if let Some(sugg) = suggestion {
            json_error = json_error.with_suggestion(sugg);
        }
        // Override exit code to match the error classification
        json_error.error.exit_code = classify_exit_code(err);
        // Include error context details for programmatic access
        json_error.error.details = err.context_map();
        json_error
    }
}

impl From<crate::Error> for JsonError {
    fn from(err: crate::Error) -> Self {
        Self::from(&err)
    }
}

// Note: from_anyhow method removed as isolate-core doesn't depend on anyhow
// If needed, implement this in the isolate crate instead

/// Trait for types that can be serialized to JSON
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string
    fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize to JSON: {e}")))
    }
}

// Implement for all Serialize types
impl<T: Serialize> JsonSerializable for T {}

/// HATEOAS-style link for API discoverability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HateoasLink {
    /// Link relation type (e.g., "self", "next", "parent")
    pub rel: String,
    /// The command or action to take
    pub href: String,
    /// HTTP-like method hint ("GET" for read, "POST" for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl HateoasLink {
    /// Create a self-reference link
    #[must_use]
    pub fn self_link(command: impl Into<String>) -> Self {
        Self {
            rel: "self".to_string(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create a related resource link
    #[must_use]
    pub fn related(rel: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create an action link (mutating operation)
    #[must_use]
    pub fn action(
        rel: impl Into<String>,
        command: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("POST".to_string()),
            title: Some(title.into()),
        }
    }

    /// Add a title to this link
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Related resource information for cross-referencing
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedResources {
    /// Related sessions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sessions: Vec<String>,
    /// Related beads/issues
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub beads: Vec<String>,
    /// Related workspaces
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub workspaces: Vec<String>,
    /// Related commits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub commits: Vec<String>,
    /// Parent resource (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Child resources
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<String>,
}

impl RelatedResources {
    /// Check if there are any related resources
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sessions.is_empty()
            && self.beads.is_empty()
            && self.workspaces.is_empty()
            && self.commits.is_empty()
            && self.parent.is_none()
            && self.children.is_empty()
    }
}

/// Response metadata for debugging and tracing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseMeta {
    /// Command that generated this response
    pub command: String,
    /// Timestamp of response generation (ISO 8601)
    pub timestamp: String,
    /// Duration of command execution in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether this was a dry-run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    /// Whether the operation is reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    /// Command to undo this operation (if reversible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Agent ID if executed by an agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl ResponseMeta {
    /// Create new metadata for a command
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
            dry_run: None,
            reversible: None,
            undo_command: None,
            request_id: None,
            agent_id: None,
        }
    }

    /// Set duration
    #[must_use]
    pub const fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Mark as dry run
    #[must_use]
    pub const fn as_dry_run(mut self) -> Self {
        self.dry_run = Some(true);
        self
    }

    /// Mark as reversible with undo command
    #[must_use]
    pub fn with_undo(mut self, undo_cmd: impl Into<String>) -> Self {
        self.reversible = Some(true);
        self.undo_command = Some(undo_cmd.into());
        self
    }

    /// Set agent ID
    #[must_use]
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set request ID
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

/// Generic schema envelope for protocol-compliant JSON responses
///
/// Wraps response data with schema metadata (`$schema`, `_schema_version`) for AI-first CLI design.
/// All JSON outputs should be wrapped with this envelope to conform to `ResponseEnvelope` pattern.
///
/// Includes HATEOAS-style navigation with `_links`, `_related`, and `_meta` blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelope<T> {
    /// JSON Schema reference (e.g., `isolate://status-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("single" for objects, "array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Response data (flattened into envelope at JSON level)
    #[serde(flatten)]
    pub data: T,
    /// Suggested next actions for AI agents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub next: Vec<NextAction>,
    /// Available fixes for errors (empty for success responses)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<Fix>,
    /// HATEOAS-style navigation links
    #[serde(rename = "_links", skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<HateoasLink>,
    /// Related resources for cross-referencing
    #[serde(rename = "_related", skip_serializing_if = "Option::is_none")]
    pub related: Option<RelatedResources>,
    /// Response metadata for debugging and tracing
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SchemaEnvelope<T> {
    /// Create a new schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "status-response") Should use a constant from
    ///   `schemas` module for conformance
    /// * `schema_type` - Response shape ("single" or "array")
    /// * `data` - The response data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// use isolate_core::json::schemas;
    /// let envelope = SchemaEnvelope::new(schemas::STATUS_RESPONSE, "single", data);
    /// ```
    pub fn new(schema_name: &str, schema_type: &str, data: T) -> Self {
        Self {
            schema: schemas::uri(schema_name),
            schema_version: schemas::SCHEMA_VERSION.to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Create a schema envelope with next actions
    pub fn with_next(schema_name: &str, schema_type: &str, data: T, next: Vec<NextAction>) -> Self {
        Self {
            schema: format!("isolate://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next,
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links to envelope
    #[must_use]
    pub fn with_links(mut self, links: Vec<HateoasLink>) -> Self {
        self.links = links;
        self
    }

    /// Add a single link
    #[must_use]
    pub fn add_link(mut self, link: HateoasLink) -> Self {
        self.links.push(link);
        self
    }

    /// Add related resources
    #[must_use]
    pub fn with_related(mut self, related: RelatedResources) -> Self {
        if !related.is_empty() {
            self.related = Some(related);
        }
        self
    }

    /// Add response metadata
    #[must_use]
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add fixes to envelope
    #[must_use]
    pub fn with_fixes(mut self, fixes: Vec<Fix>) -> Self {
        self.fixes = fixes;
        self
    }

    /// Mark as failed response
    #[must_use]
    pub const fn as_error(mut self) -> Self {
        self.success = false;
        self
    }
}

/// Schema envelope for array responses
///
/// Unlike `SchemaEnvelope` which uses flatten for single objects,
/// `SchemaEnvelopeArray` explicitly wraps array data because serde flatten
/// cannot serialize sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelopeArray<T> {
    /// JSON Schema reference (e.g., `isolate://list-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Array data (cannot be flattened, so stored as explicit field)
    pub data: Vec<T>,
    /// Suggested next actions for AI agents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub next: Vec<NextAction>,
    /// Available fixes for errors (empty for success responses)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<Fix>,
    /// HATEOAS-style navigation links
    #[serde(rename = "_links", skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<HateoasLink>,
    /// Related resources for cross-referencing
    #[serde(rename = "_related", skip_serializing_if = "Option::is_none")]
    pub related: Option<RelatedResources>,
    /// Response metadata for debugging and tracing
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SchemaEnvelopeArray<T> {
    /// Create a new array schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "list-response")
    /// * `data` - The array data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// let envelope = SchemaEnvelopeArray::new("list-response", items);
    /// ```
    /// Create a new schema envelope array.
    ///
    /// # Returns
    ///
    /// Returns a new envelope instance. The result must be used as this
    /// creates a structured response object.
    #[must_use]
    pub fn new(schema_name: &str, data: Vec<T>) -> Self {
        Self {
            schema: format!("isolate://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: "array".to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links
    #[must_use]
    pub fn with_links(mut self, links: Vec<HateoasLink>) -> Self {
        self.links = links;
        self
    }

    /// Add related resources
    #[must_use]
    pub fn with_related(mut self, related: RelatedResources) -> Self {
        if !related.is_empty() {
            self.related = Some(related);
        }
        self
    }

    /// Add response metadata
    #[must_use]
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add next actions
    #[must_use]
    pub fn with_next(mut self, next: Vec<NextAction>) -> Self {
        self.next = next;
        self
    }
}

/// Helper to create error details with available sessions
pub fn error_with_available_sessions(
    code: ErrorCode,
    message: impl Into<String>,
    session_name: impl Into<String>,
    available: &[String],
) -> JsonError {
    let details = serde_json::json!({
        "session_name": session_name.into(),
        "available_sessions": available,
    });

    JsonError::new(code, message)
        .with_details(details)
        .with_suggestion("Use 'isolate list' to see available sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_basic() {
        let err = JsonError::new("TEST_ERROR", "Test error message");
        assert_eq!(err.error.code, "TEST_ERROR");
        assert_eq!(err.error.message, "Test error message");
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_with_details() {
        let details = serde_json::json!({"key": "value"});
        let err = JsonError::new("TEST_ERROR", "Test").with_details(details.clone());

        assert!(err.error.details.is_some());
        assert_eq!(err.error.details, Some(details));
    }

    #[test]
    fn test_json_error_with_suggestion() {
        let err = JsonError::new("TEST_ERROR", "Test").with_suggestion("Try this instead");

        assert_eq!(err.error.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
    }

    #[test]
    fn test_error_code_to_string() {
        let code: String = ErrorCode::SessionNotFound.into();
        assert_eq!(code, "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_json_error_serialization() -> crate::Result<()> {
        let err = JsonError::new("TEST_ERROR", "Test message");
        let json = err.to_json()?;

        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));

        Ok(())
    }

    #[test]
    fn test_error_with_available_sessions() {
        let available = vec!["session1".to_string(), "session2".to_string()];
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session 'foo' not found",
            "foo",
            &available,
        );

        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.details.is_some());
        assert!(err.error.suggestion.is_some());
    }

    #[test]
    fn test_json_serializable_trait() -> crate::Result<()> {
        #[derive(Serialize)]
        struct TestStruct {
            field: String,
        }

        let test = TestStruct {
            field: "value".to_string(),
        };

        let json = test.to_json()?;
        assert!(json.contains("\"field\""));
        assert!(json.contains("\"value\""));

        Ok(())
    }

    #[test]
    fn test_json_success_wrapper() -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
            count: usize,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        let success = JsonSuccess {
            success: true,
            data,
        };
        let json = success.to_json()?;

        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
        assert!(json.contains("\"count\""));
        assert!(json.contains("42"));

        Ok(())
    }

    #[test]
    fn test_error_detail_skip_none() -> crate::Result<()> {
        let err = JsonError::new("TEST", "message");
        let json = err.to_json()?;

        // Should not contain "details" or "suggestion" fields when they're None
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"suggestion\""));

        Ok(())
    }

    // Tests for ErrorDetail::from_error() constructor (isolate-lgkf Phase 4 - RED)
    #[test]
    fn test_error_detail_from_validation_error() {
        let err = crate::Error::ValidationError {
            message: "invalid session name".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "VALIDATION_ERROR");
        assert!(detail.message.contains("Validation error"));
        assert_eq!(detail.exit_code, 1);
    }

    #[test]
    fn test_error_detail_from_io_error() {
        let err = crate::Error::IoError("file not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "IO_ERROR");
        assert!(detail.message.contains("IO error"));
        assert_eq!(detail.exit_code, 3);
    }

    #[test]
    fn test_error_detail_from_not_found_error() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "NOT_FOUND");
        assert!(detail.message.contains("Not found"));
        assert_eq!(detail.exit_code, 2);
    }

    #[test]
    fn test_error_detail_preserves_context() {
        let err = crate::Error::ValidationError {
            message: "invalid input".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let detail = ErrorDetail::from_error(&err);

        // Should have context map populated
        assert_eq!(detail.code, "VALIDATION_ERROR");
        assert!(detail.message.contains("Validation error"));
        assert_eq!(detail.exit_code, 1);
    }

    #[test]
    fn test_error_detail_includes_suggestion() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have suggestion populated
        assert!(detail.suggestion.is_some());
        if let Some(sugg) = detail.suggestion {
            assert!(sugg.contains("list"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // HATEOAS LINK TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_hateoas_link_self() {
        let link = HateoasLink::self_link("isolate status test");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "isolate status test");
        assert_eq!(link.method, Some("GET".to_string()));
        assert!(link.title.is_none());
    }

    #[test]
    fn test_hateoas_link_related() {
        let link = HateoasLink::related("parent", "isolate list");
        assert_eq!(link.rel, "parent");
        assert_eq!(link.href, "isolate list");
        assert_eq!(link.method, Some("GET".to_string()));
    }

    #[test]
    fn test_hateoas_link_action() {
        let link = HateoasLink::action("remove", "isolate remove test", "Delete session");
        assert_eq!(link.rel, "remove");
        assert_eq!(link.href, "isolate remove test");
        assert_eq!(link.method, Some("POST".to_string()));
        assert_eq!(link.title, Some("Delete session".to_string()));
    }

    #[test]
    fn test_hateoas_link_with_title() {
        let link = HateoasLink::self_link("isolate status").with_title("Get current status");
        assert_eq!(link.title, Some("Get current status".to_string()));
    }

    #[test]
    fn test_hateoas_link_serialization() -> crate::Result<()> {
        let link = HateoasLink::action("sync", "isolate sync test", "Sync session");
        let json =
            serde_json::to_string(&link).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"rel\":\"sync\""));
        assert!(json.contains("\"href\":\"isolate sync test\""));
        assert!(json.contains("\"method\":\"POST\""));
        assert!(json.contains("\"title\":\"Sync session\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RELATED RESOURCES TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_related_resources_empty() {
        let related = RelatedResources::default();
        assert!(related.is_empty());
    }

    #[test]
    fn test_related_resources_with_sessions() {
        let related = RelatedResources {
            sessions: vec!["session-1".to_string(), "session-2".to_string()],
            ..Default::default()
        };
        assert!(!related.is_empty());
        assert_eq!(related.sessions.len(), 2);
    }

    #[test]
    fn test_related_resources_with_parent() {
        let related = RelatedResources {
            parent: Some("main".to_string()),
            ..Default::default()
        };
        assert!(!related.is_empty());
    }

    #[test]
    fn test_related_resources_serialization() -> crate::Result<()> {
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["isolate-1234".to_string()],
            commits: vec!["abc123".to_string()],
            ..Default::default()
        };
        let json =
            serde_json::to_string(&related).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"sessions\":[\"s1\"]"));
        assert!(json.contains("\"beads\":[\"isolate-1234\"]"));
        assert!(json.contains("\"commits\":[\"abc123\"]"));
        // Empty fields should be omitted
        assert!(!json.contains("\"workspaces\""));
        assert!(!json.contains("\"parent\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RESPONSE META TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_response_meta_new() {
        let meta = ResponseMeta::new("status");
        assert_eq!(meta.command, "status");
        assert!(!meta.timestamp.is_empty());
        assert!(meta.duration_ms.is_none());
        assert!(meta.dry_run.is_none());
        assert!(meta.reversible.is_none());
        assert!(meta.undo_command.is_none());
    }

    #[test]
    fn test_response_meta_with_duration() {
        let meta = ResponseMeta::new("add").with_duration(150);
        assert_eq!(meta.duration_ms, Some(150));
    }

    #[test]
    fn test_response_meta_as_dry_run() {
        let meta = ResponseMeta::new("remove").as_dry_run();
        assert_eq!(meta.dry_run, Some(true));
    }

    #[test]
    fn test_response_meta_with_undo() {
        let meta = ResponseMeta::new("remove test").with_undo("isolate undo");
        assert_eq!(meta.reversible, Some(true));
        assert_eq!(meta.undo_command, Some("isolate undo".to_string()));
    }

    #[test]
    fn test_response_meta_with_agent() {
        let meta = ResponseMeta::new("work").with_agent("agent-001");
        assert_eq!(meta.agent_id, Some("agent-001".to_string()));
    }

    #[test]
    fn test_response_meta_with_request_id() {
        let meta = ResponseMeta::new("status").with_request_id("req-123");
        assert_eq!(meta.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_response_meta_serialization() -> crate::Result<()> {
        let meta = ResponseMeta::new("add test")
            .with_duration(50)
            .with_undo("isolate undo")
            .with_agent("agent-x");
        let json =
            serde_json::to_string(&meta).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"command\":\"add test\""));
        assert!(json.contains("\"duration_ms\":50"));
        assert!(json.contains("\"reversible\":true"));
        assert!(json.contains("\"undo_command\":\"isolate undo\""));
        assert!(json.contains("\"agent_id\":\"agent-x\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE WITH HATEOAS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_envelope_with_links() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test".to_string(),
        };
        let envelope = SchemaEnvelope::new("test-response", "single", data)
            .add_link(HateoasLink::self_link("isolate status test"))
            .add_link(HateoasLink::related("list", "isolate list"));

        assert_eq!(envelope.links.len(), 2);
        assert_eq!(
            envelope.links.first().map(|l| &l.rel),
            Some(&"self".to_string())
        );
        assert_eq!(
            envelope.links.get(1).map(|l| &l.rel),
            Some(&"list".to_string())
        );
    }

    #[test]
    fn test_schema_envelope_with_related() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            id: String,
        }

        let data = TestData {
            id: "abc".to_string(),
        };
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["isolate-001".to_string()],
            ..Default::default()
        };
        let envelope = SchemaEnvelope::new("test-response", "single", data).with_related(related);

        assert!(envelope.related.is_some());
        if let Some(rel) = envelope.related.as_ref() {
            assert_eq!(rel.sessions.len(), 1);
            assert_eq!(rel.beads.len(), 1);
        }
    }

    #[test]
    fn test_schema_envelope_with_meta() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            value: i32,
        }

        let data = TestData { value: 42 };
        let meta = ResponseMeta::new("test").with_duration(100);
        let envelope = SchemaEnvelope::new("test-response", "single", data).with_meta(meta);

        assert!(envelope.meta.is_some());
        if let Some(m) = envelope.meta {
            assert_eq!(m.command, "test");
            assert_eq!(m.duration_ms, Some(100));
        }
    }

    #[test]
    fn test_schema_envelope_as_error() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            error: String,
        }

        let data = TestData {
            error: "failed".to_string(),
        };
        let envelope = SchemaEnvelope::new("error-response", "single", data).as_error();

        assert!(!envelope.success);
    }

    #[test]
    fn test_schema_envelope_with_fixes() {
        use crate::fix::Fix;

        #[derive(Serialize, Deserialize)]
        struct TestData {
            status: String,
        }

        let data = TestData {
            status: "error".to_string(),
        };
        let fixes = vec![Fix::safe("Try again", vec!["isolate retry".to_string()])];
        let envelope = SchemaEnvelope::new("error-response", "single", data).with_fixes(fixes);

        assert_eq!(envelope.fixes.len(), 1);
    }

    #[test]
    fn test_schema_envelope_full_serialization() -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test-session".to_string(),
        };
        let envelope = SchemaEnvelope::new("session-response", "single", data)
            .add_link(HateoasLink::self_link("isolate status test-session"))
            .with_related(RelatedResources {
                beads: vec!["isolate-1".to_string()],
                ..Default::default()
            })
            .with_meta(ResponseMeta::new("status test-session").with_duration(25));

        // Functional approach: Serialize → Parse → Validate structure
        let result = serde_json::to_string_pretty(&envelope)
            .map_err(|e| crate::Error::ParseError(e.to_string()))
            .and_then(|json_str| {
                // Parse JSON to verify structure (type-safe, not fragile string matching)
                serde_json::from_str::<serde_json::Value>(&json_str)
                    .map(|value| (json_str, value))
                    .map_err(|e| crate::Error::ParseError(format!("Failed to parse JSON: {e}")))
            });

        let (json_str, parsed) = result?;

        // Validate structure via parsed JSON (immutable, composable checks)
        let checks = [
            (parsed.get("$schema").is_some(), "$schema field missing"),
            (
                parsed.get("_schema_version").is_some(),
                "_schema_version field missing",
            ),
            (
                parsed.get("_links").and_then(|v| v.as_array()).is_some(),
                "_links should be array",
            ),
            (parsed.get("_related").is_some(), "_related field missing"),
            (parsed.get("_meta").is_some(), "_meta field missing"),
            (
                parsed.get("name").and_then(|v| v.as_str()) == Some("test-session"),
                "name field should be 'test-session'",
            ),
        ];

        // Functional validation: all checks must pass (Railway pattern)
        for (passed, msg) in &checks {
            assert!(
                passed,
                "Schema validation failed: {msg}\n\nJSON:\n{json_str}"
            );
        }

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE ARRAY WITH HATEOAS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_envelope_array_with_links() {
        let data = vec!["item1".to_string(), "item2".to_string()];
        let envelope = SchemaEnvelopeArray::new("list-response", data)
            .with_links(vec![HateoasLink::self_link("isolate list")]);

        assert_eq!(envelope.links.len(), 1);
        assert_eq!(envelope.data.len(), 2);
    }

    #[test]
    fn test_schema_envelope_array_with_meta() {
        let data = vec![1, 2, 3];
        let meta = ResponseMeta::new("list").with_duration(10);
        let envelope = SchemaEnvelopeArray::new("numbers-response", data).with_meta(meta);

        assert!(envelope.meta.is_some());
        assert_eq!(envelope.data.len(), 3);
    }

    #[test]
    fn test_schema_envelope_array_with_next() {
        use crate::hints::{ActionRisk, NextAction};

        let data: Vec<String> = vec![];
        let next = vec![NextAction {
            action: "Create first item".to_string(),
            commands: vec!["isolate add item".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        }];
        let envelope = SchemaEnvelopeArray::new("empty-list", data).with_next(next);

        assert_eq!(envelope.next.len(), 1);
        assert!(envelope.data.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA REGISTRY TESTS (bd-2nv: cli-contracts)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_registry_uri_format() {
        assert_eq!(
            schemas::uri(schemas::ADD_RESPONSE),
            "isolate://add-response/v1"
        );
        assert_eq!(
            schemas::uri(schemas::DONE_RESPONSE),
            "isolate://done-response/v1"
        );
        assert_eq!(
            schemas::uri(schemas::CONTEXT_RESPONSE),
            "isolate://context-response/v1"
        );
    }

    #[test]
    fn test_schema_registry_all_schemas_are_valid() {
        for schema in schemas::all_valid_schemas() {
            assert!(!schema.is_empty(), "Schema name should not be empty");
            assert!(
                schema.contains('-') || schema.contains('_'),
                "Schema '{schema}' should use kebab-case or snake_case"
            );
        }
    }

    #[test]
    fn test_schema_registry_validation() {
        assert!(schemas::is_valid_schema(schemas::ADD_RESPONSE));
        assert!(schemas::is_valid_schema(schemas::DONE_RESPONSE));
        assert!(schemas::is_valid_schema(schemas::DIFF_RESPONSE));
        assert!(!schemas::is_valid_schema("invalid-schema"));
        assert!(!schemas::is_valid_schema(""));
    }

    #[test]
    fn test_schema_envelope_uses_registry() {
        // Verify that SchemaEnvelope::new uses the registry format
        let data = serde_json::json!({"test": "data"});
        let envelope = SchemaEnvelope::new(schemas::ADD_RESPONSE, "single", data);

        assert_eq!(envelope.schema, "isolate://add-response/v1");
        assert_eq!(envelope.schema_version, schemas::SCHEMA_VERSION);
        assert_eq!(envelope.schema_type, "single");
    }

    #[test]
    fn test_all_contract_schemas_exist_in_registry() {
        // This test verifies that all schemas used by contract.rs are in the registry
        let contract_schemas = [
            "init-response",
            "add-response",
            "list-response",
            "remove-response",
            "focus-response",
            "status-response",
            "sync-response",
            "done-response",
            "undo-response",
            "revert-response",
            "work-response",
            "abort-response",
            "spawn-response",
            "whereami-response",
            "whoami-response",
            "doctor-response",
            "clean-response",
            "context-response",
            "introspect-response",
            "checkpoint-response",
            "contract-response",
            "contracts-response",
        ];

        for schema in contract_schemas {
            assert!(
                schemas::is_valid_schema(schema),
                "Contract schema '{schema}' not found in registry"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ERROR CODE AND CLASSIFICATION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn given_all_error_codes_when_converted_to_string_then_screaming_snake_case() {
        let codes = [
            ErrorCode::SessionNotFound,
            ErrorCode::SessionAlreadyExists,
            ErrorCode::SessionNameInvalid,
            ErrorCode::WorkspaceCreationFailed,
            ErrorCode::WorkspaceNotFound,
            ErrorCode::JjNotInstalled,
            ErrorCode::JjCommandFailed,
            ErrorCode::NotJjRepository,
            ErrorCode::ZellijNotRunning,
            ErrorCode::ZellijCommandFailed,
            ErrorCode::ConfigNotFound,
            ErrorCode::ConfigParseError,
            ErrorCode::ConfigKeyNotFound,
            ErrorCode::HookFailed,
            ErrorCode::HookExecutionError,
            ErrorCode::StateDbCorrupted,
            ErrorCode::StateDbLocked,
            ErrorCode::SpawnNotOnMain,
            ErrorCode::SpawnInvalidBeadStatus,
            ErrorCode::SpawnBeadNotFound,
            ErrorCode::SpawnWorkspaceCreationFailed,
            ErrorCode::SpawnAgentSpawnFailed,
            ErrorCode::SpawnTimeout,
            ErrorCode::SpawnMergeFailed,
            ErrorCode::SpawnCleanupFailed,
            ErrorCode::SpawnDatabaseError,
            ErrorCode::SpawnJjCommandFailed,
            ErrorCode::InvalidArgument,
            ErrorCode::Unknown,
        ];

        for code in codes {
            let s = code.as_str();
            assert!(
                s.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "Error code '{s}' is not SCREAMING_SNAKE_CASE"
            );
            assert!(!s.is_empty(), "Error code string should not be empty");
        }
    }

    #[test]
    fn given_validation_error_when_classified_then_exit_code_1() {
        let err = crate::Error::ValidationError {
            message: "test".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(classify_exit_code(&err), 1);
    }

    #[test]
    fn given_not_found_error_when_classified_then_exit_code_2() {
        let err = crate::Error::NotFound("test".into());
        assert_eq!(classify_exit_code(&err), 2);

        let err2 = crate::Error::SessionNotFound {
            session: "test".into(),
        };
        assert_eq!(classify_exit_code(&err2), 2);
    }

    #[test]
    fn given_io_error_when_classified_then_exit_code_3() {
        let err = crate::Error::IoError("test".into());
        assert_eq!(classify_exit_code(&err), 3);

        let err2 = crate::Error::DatabaseError("test".into());
        assert_eq!(classify_exit_code(&err2), 3);
    }

    #[test]
    fn given_command_error_when_classified_then_exit_code_4() {
        let err = crate::Error::Command("test".into());
        assert_eq!(classify_exit_code(&err), 4);

        let err2 = crate::Error::JjCommandError {
            operation: "test".into(),
            source: "err".into(),
            is_not_found: false,
        };
        assert_eq!(classify_exit_code(&err2), 4);
    }

    #[test]
    fn given_lock_error_when_classified_then_exit_code_5() {
        let err = crate::Error::SessionLocked {
            session: "test".into(),
            holder: "agent1".into(),
        };
        assert_eq!(classify_exit_code(&err), 5);

        let err2 = crate::Error::LockTimeout {
            operation: "test".into(),
            timeout_ms: 100,
            retries: 3,
        };
        assert_eq!(classify_exit_code(&err2), 5);
    }

    #[test]
    fn given_operation_cancelled_when_classified_then_exit_code_130() {
        let err = crate::Error::OperationCancelled("user interrupt".into());
        assert_eq!(classify_exit_code(&err), 130);
    }

    #[test]
    fn given_all_error_variants_when_mapped_then_have_suggestions() {
        let test_cases = vec![
            (
                crate::Error::InvalidConfig("test".into()),
                Some("configuration"),
            ),
            (crate::Error::NotFound("test".into()), Some("isolate list")),
            (
                crate::Error::SessionNotFound {
                    session: "test".into(),
                },
                Some("isolate list"),
            ),
            (
                crate::Error::DatabaseError("test".into()),
                Some("isolate doctor"),
            ),
        ];

        for (err, expected_substring) in test_cases {
            let json_err = JsonError::from(&err);
            if let Some(expected) = expected_substring {
                assert!(
                    json_err.error.suggestion.is_some(),
                    "Expected suggestion for error: {:?}",
                    err
                );
                let suggestion = json_err.error.suggestion.unwrap_or_default();
                assert!(
                    suggestion.contains(expected),
                    "Expected suggestion to contain '{}', got: {}",
                    expected,
                    suggestion
                );
            }
        }
    }

    #[test]
    fn given_hook_failed_error_when_converted_then_includes_all_details() {
        let err = crate::Error::HookFailed {
            hook_type: "pre-commit".into(),
            command: "lint".into(),
            exit_code: Some(1),
            stdout: "output".into(),
            stderr: "error details".into(),
        };
        let (code, message, _) = map_error_to_parts(&err);
        assert_eq!(code.as_str(), "HOOK_FAILED");
        assert!(message.contains("pre-commit"));
        assert!(message.contains("lint"));
        assert!(message.contains("error details"));
    }

    #[test]
    fn given_jj_not_found_error_when_converted_then_suggests_installation() {
        let err = crate::Error::JjCommandError {
            operation: "test".into(),
            source: "not found".into(),
            is_not_found: true,
        };
        let (code, message, suggestion) = map_error_to_parts(&err);
        assert_eq!(code.as_str(), "JJ_NOT_INSTALLED");
        assert!(message.contains("JJ is not installed"));
        assert!(suggestion.is_some());
        if let Some(sugg) = suggestion {
            assert!(sugg.contains("Install JJ"));
        }
    }

    #[test]
    fn given_jj_workspace_conflict_when_converted_then_includes_recovery_hint() {
        let err = crate::Error::JjWorkspaceConflict {
            conflict_type: crate::error::JjConflictType::Stale,
            workspace_name: "test-ws".into(),
            source: "operation mismatch".into(),
            recovery_hint: "Run jj workspace forget".into(),
        };
        let (code, message, _) = map_error_to_parts(&err);
        assert_eq!(code.as_str(), "JJ_COMMAND_FAILED");
        assert!(message.contains("test-ws"));
        assert!(message.contains("Run jj workspace forget"));
    }

    #[test]
    fn given_lock_timeout_error_when_converted_then_includes_retry_details() {
        let err = crate::Error::LockTimeout {
            operation: "workspace creation".into(),
            timeout_ms: 100,
            retries: 5,
        };
        let (code, message, suggestion) = map_error_to_parts(&err);
        assert_eq!(code.as_str(), "UNKNOWN");
        assert!(message.contains("workspace creation"));
        assert!(message.contains("5 retries"));
        assert!(suggestion.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JSON SUCCESS AND ERROR WRAPPER TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn given_json_success_when_created_then_success_is_true() {
        #[derive(Serialize)]
        struct TestData {
            value: i32,
        }
        let data = TestData { value: 42 };
        let success = JsonSuccess::new(data);
        assert!(success.success);
        assert_eq!(success.data.value, 42);
    }

    #[test]
    fn given_json_error_default_when_created_then_has_unknown_code() {
        let err = JsonError::default();
        assert!(!err.success);
        assert_eq!(err.error.code, "UNKNOWN");
        assert_eq!(err.error.exit_code, 4);
    }

    #[test]
    fn given_json_error_when_chained_with_builders_then_all_fields_set() {
        let err = JsonError::new("TEST", "message")
            .with_details(serde_json::json!({"key": "value"}))
            .with_suggestion("try this")
            .with_exit_code(2);

        assert_eq!(err.error.code, "TEST");
        assert_eq!(err.error.message, "message");
        assert_eq!(err.error.exit_code, 2);
        assert!(err.error.details.is_some());
        assert_eq!(err.error.suggestion, Some("try this".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE BUILDER PATTERN TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn given_schema_envelope_when_built_with_all_fields_then_serializes_correctly(
    ) -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            id: String,
        }

        let data = TestData { id: "test".into() };
        let envelope = SchemaEnvelope::new(schemas::STATUS_RESPONSE, "single", data)
            .add_link(HateoasLink::self_link("isolate status"))
            .add_link(HateoasLink::action("remove", "isolate remove", "Delete"))
            .with_related(RelatedResources {
                sessions: vec!["s1".into()],
                ..Default::default()
            })
            .with_meta(ResponseMeta::new("status").with_duration(50))
            .with_fixes(vec![]);

        let json = serde_json::to_string(&envelope)
            .map_err(|e| crate::Error::ParseError(e.to_string()))?;
        assert!(json.contains("$schema"));
        assert!(json.contains("_links"));
        assert!(json.contains("_related"));
        assert!(json.contains("_meta"));
        Ok(())
    }

    #[test]
    fn given_schema_envelope_when_marked_as_error_then_success_is_false() {
        #[derive(Serialize)]
        struct ErrorData {
            error: String,
        }
        let data = ErrorData {
            error: "failed".into(),
        };
        let envelope = SchemaEnvelope::new(schemas::ERROR_RESPONSE, "single", data).as_error();
        assert!(!envelope.success);
    }

    #[test]
    fn given_schema_envelope_with_next_when_created_then_includes_next_actions() {
        use crate::hints::{ActionRisk, NextAction};

        #[derive(Serialize)]
        struct TestData {
            status: String,
        }
        let data = TestData {
            status: "waiting".into(),
        };
        let next = vec![NextAction {
            action: "Continue".into(),
            commands: vec!["isolate work".into()],
            risk: ActionRisk::Safe,
            description: None,
        }];
        let envelope = SchemaEnvelope::with_next(schemas::STATUS_RESPONSE, "single", data, next);
        assert_eq!(envelope.next.len(), 1);
        assert_eq!(envelope.next[0].action, "Continue");
    }

    #[test]
    fn given_related_resources_empty_when_added_to_envelope_then_not_included() {
        #[derive(Serialize)]
        struct TestData {
            id: String,
        }
        let data = TestData { id: "test".into() };
        let empty = RelatedResources::default();
        let envelope =
            SchemaEnvelope::new(schemas::STATUS_RESPONSE, "single", data).with_related(empty);
        // Empty related resources should not be set
        assert!(envelope.related.is_none());
    }

    #[test]
    fn given_related_resources_with_data_when_added_then_included() {
        #[derive(Serialize)]
        struct TestData {
            id: String,
        }
        let data = TestData { id: "test".into() };
        let related = RelatedResources {
            beads: vec!["bead-1".into()],
            ..Default::default()
        };
        let envelope =
            SchemaEnvelope::new(schemas::STATUS_RESPONSE, "single", data).with_related(related);
        assert!(envelope.related.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE ARRAY TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn given_schema_envelope_array_when_created_then_has_array_type() {
        let data: Vec<String> = vec!["item1".into(), "item2".into()];
        let envelope = SchemaEnvelopeArray::new(schemas::LIST_RESPONSE, data);
        assert_eq!(envelope.schema_type, "array");
        assert_eq!(envelope.data.len(), 2);
        assert!(envelope.success);
    }

    #[test]
    fn given_schema_envelope_array_when_serialized_then_data_is_array_field() -> crate::Result<()> {
        let data = vec![1, 2, 3];
        let envelope = SchemaEnvelopeArray::new(schemas::LIST_RESPONSE, data);
        let json = serde_json::to_string(&envelope)
            .map_err(|e| crate::Error::ParseError(e.to_string()))?;
        assert!(json.contains("\"data\":[1,2,3]"));
        Ok(())
    }

    #[test]
    fn given_empty_array_envelope_when_with_next_then_suggests_actions() {
        use crate::hints::{ActionRisk, NextAction};

        let data: Vec<String> = vec![];
        let next = vec![NextAction {
            action: "Add first item".into(),
            commands: vec!["isolate add".into()],
            risk: ActionRisk::Safe,
            description: Some("Create your first session".into()),
        }];
        let envelope = SchemaEnvelopeArray::new(schemas::LIST_RESPONSE, data).with_next(next);
        assert_eq!(envelope.next.len(), 1);
        assert!(envelope.data.is_empty());
    }

    #[test]
    fn given_error_with_available_sessions_when_created_then_includes_details() {
        let available = vec!["session1".into(), "session2".into()];
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session not found",
            "missing",
            &available,
        );

        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.details.is_some());
        assert!(err.error.suggestion.is_some());

        if let Some(details) = err.error.details {
            assert!(details.get("session_name").is_some());
            assert!(details.get("available_sessions").is_some());
        }
    }
}

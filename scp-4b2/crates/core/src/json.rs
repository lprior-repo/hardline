//! JSON output structures for AI-first CLI design
//!
//! This module provides consistent JSON output formats across all commands.

use serde::{Deserialize, Serialize};

use crate::fix::Fix;
use crate::hints::NextAction;

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
    pub const BASE_URI: &str = "scp://";

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
    pub fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::Error::JsonParse(e))
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
const fn classify_exit_code(error: &crate::error::Error) -> i32 {
    use crate::error::Error;
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

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    #[must_use]
    pub fn from_error(error: &crate::error::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: None,
            suggestion: error.suggestion(),
        }
    }
}

/// Map a `crate::Error` to (`ErrorCode`, message, optional suggestion)
#[allow(clippy::too_many_lines)]
fn map_error_to_parts(err: &crate::error::Error) -> (ErrorCode, String, Option<String>) {
    use crate::error::Error;

    match err {
        Error::InvalidConfig(msg) | Error::ConfigInvalid(msg) => (
            ErrorCode::ConfigParseError,
            format!("Invalid configuration: {msg}"),
            Some("Check your configuration file for errors".to_string()),
        ),
        Error::IoError(msg) | Error::Io(msg) => (ErrorCode::Unknown, format!("IO error: {msg}"), None),
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
                (Some(f), Some(v)) => format!("Validation error: {message} (field: {f}, value: {v})"),
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
            format!("JJ workspace conflict: {conflict_type:?}\nWorkspace: {workspace_name}\n{recovery_hint}\nJJ error: {msg}"),
            Some("Follow the recovery hints in the error message".to_string()),
        ),
        Error::SessionLocked { session, holder } => (
            ErrorCode::Unknown,
            format!("Session '{session}' is locked by agent '{holder}'"),
            Some("Wait for the other agent to finish or check lock status".to_string()),
        ),
        Error::WorkspaceLocked { .. } => (
            ErrorCode::Unknown,
            err.to_string(),
            None,
        ),
        Error::LockTimeout { operation, timeout_ms, retries } => (
            ErrorCode::Unknown,
            format!("Lock acquisition timeout for '{operation}' after {retries} retries (timeout: {timeout_ms}ms per attempt)"),
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

impl From<&crate::error::Error> for JsonError {
    fn from(err: &crate::error::Error) -> Self {
        let (code, message, suggestion) = map_error_to_parts(err);

        let mut json_error = Self::new(code, message);
        if let Some(sugg) = suggestion {
            json_error = json_error.with_suggestion(sugg);
        }
        // Override exit code to match the error classification
        json_error.error.exit_code = classify_exit_code(err);
        json_error
    }
}

impl From<crate::error::Error> for JsonError {
    fn from(err: crate::error::Error) -> Self {
        Self::from(&err)
    }
}

/// Trait for types that can be serialized to JSON
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string
    fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::Error::JsonParse(e))
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
    /// JSON Schema reference (e.g., `scp://status-response/v1`)
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
    /// use scp_core::json::schemas;
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
            schema: format!("scp://{schema_name}/v1"),
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
    /// JSON Schema reference (e.g., `scp://list-response/v1`)
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
            schema: format!("scp://{schema_name}/v1"),
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
        .with_suggestion("Use 'scp session list' to see available sessions")
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
    fn test_json_error_serialization() -> crate::error::Result<()> {
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
    fn test_json_serializable_trait() -> crate::error::Result<()> {
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
    fn test_json_success_wrapper() -> crate::error::Result<()> {
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
    fn test_error_detail_skip_none() -> crate::error::Result<()> {
        let err = JsonError::new("TEST", "message");
        let json = err.to_json()?;

        // Should not contain "details" or "suggestion" fields when they're None
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"suggestion\""));

        Ok(())
    }

    #[test]
    fn test_error_detail_from_validation_error() {
        let err = crate::error::Error::ValidationError("invalid session name".into());
        let detail = ErrorDetail::from_error(&err);

        assert!(detail.code.contains("VALIDATION") || detail.code.contains("INVALID"));
        assert!(detail.message.contains("Validation"));
    }

    #[test]
    fn test_error_detail_from_io_error() {
        let err = crate::error::Error::IoError("file not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert!(detail.code.contains("UNKNOWN") || detail.message.contains("IO"));
        assert!(detail.message.contains("file not found"));
        assert_eq!(detail.exit_code, 3);
    }

    #[test]
    fn test_error_detail_from_not_found_error() {
        let err = crate::error::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert!(detail.code.contains("NOT_FOUND") || detail.code.contains("SESSION"));
        assert!(detail.message.contains("Not found"));
        assert_eq!(detail.exit_code, 2);
    }

    #[test]
    fn test_error_detail_includes_suggestion() {
        let err = crate::error::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have suggestion populated
        assert!(detail.suggestion.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HATEOAS LINK TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_hateoas_link_self() {
        let link = HateoasLink::self_link("scp status test");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "scp status test");
        assert_eq!(link.method, Some("GET".to_string()));
        assert!(link.title.is_none());
    }

    #[test]
    fn test_hateoas_link_related() {
        let link = HateoasLink::related("parent", "scp list");
        assert_eq!(link.rel, "parent");
        assert_eq!(link.href, "scp list");
        assert_eq!(link.method, Some("GET".to_string()));
    }

    #[test]
    fn test_hateoas_link_action() {
        let link = HateoasLink::action("remove", "scp remove test", "Delete session");
        assert_eq!(link.rel, "remove");
        assert_eq!(link.href, "scp remove test");
        assert_eq!(link.method, Some("POST".to_string()));
        assert_eq!(link.title, Some("Delete session".to_string()));
    }

    #[test]
    fn test_hateoas_link_with_title() {
        let link = HateoasLink::self_link("scp status").with_title("Get current status");
        assert_eq!(link.title, Some("Get current status".to_string()));
    }

    #[test]
    fn test_hateoas_link_serialization() -> crate::error::Result<()> {
        let link = HateoasLink::action("sync", "scp sync test", "Sync session");
        let json = serde_json::to_string(&link).map_err(|e| crate::error::Error::JsonParse(e))?;

        assert!(json.contains("\"rel\":\"sync\""));
        assert!(json.contains("\"href\":\"scp sync test\""));
        assert!(json.contains("\"method\":\"POST\""));
        assert!(json.contains("\"title\":\"Sync session\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RELATED RESOURCES TESTS
    // ═══════════════════════════════════════════════════════════════════════════

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
    fn test_related_resources_serialization() -> crate::error::Result<()> {
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["scp-1234".to_string()],
            commits: vec!["abc123".to_string()],
            ..Default::default()
        };
        let json =
            serde_json::to_string(&related).map_err(|e| crate::error::Error::JsonParse(e))?;

        assert!(json.contains("\"sessions\":[\"s1\"]"));
        assert!(json.contains("\"beads\":[\"scp-1234\"]"));
        assert!(json.contains("\"commits\":[\"abc123\"]"));
        // Empty fields should be omitted
        assert!(!json.contains("\"workspaces\""));
        assert!(!json.contains("\"parent\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RESPONSE META TESTS
    // ═══════════════════════════════════════════════════════════════════════════

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
        let meta = ResponseMeta::new("remove test").with_undo("scp undo");
        assert_eq!(meta.reversible, Some(true));
        assert_eq!(meta.undo_command, Some("scp undo".to_string()));
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
    fn test_response_meta_serialization() -> crate::error::Result<()> {
        let meta = ResponseMeta::new("add test")
            .with_duration(50)
            .with_undo("scp undo")
            .with_agent("agent-x");
        let json = serde_json::to_string(&meta).map_err(|e| crate::error::Error::JsonParse(e))?;

        assert!(json.contains("\"command\":\"add test\""));
        assert!(json.contains("\"duration_ms\":50"));
        assert!(json.contains("\"reversible\":true"));
        assert!(json.contains("\"undo_command\":\"scp undo\""));
        assert!(json.contains("\"agent_id\":\"agent-x\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE WITH HATEOAS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

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
            .add_link(HateoasLink::self_link("scp status test"))
            .add_link(HateoasLink::related("list", "scp list"));

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
            beads: vec!["scp-001".to_string()],
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
        #[derive(Serialize, Deserialize)]
        struct TestData {
            status: String,
        }

        let data = TestData {
            status: "error".to_string(),
        };
        let fixes = vec![Fix::safe("Try again", vec!["scp retry".to_string()])];
        let envelope = SchemaEnvelope::new("error-response", "single", data).with_fixes(fixes);

        assert_eq!(envelope.fixes.len(), 1);
    }

    #[test]
    fn test_schema_envelope_full_serialization() -> crate::error::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test-session".to_string(),
        };
        let envelope = SchemaEnvelope::new("session-response", "single", data)
            .add_link(HateoasLink::self_link("scp status test-session"))
            .add_link(HateoasLink::related("list", "scp list"));

        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|e| crate::error::Error::JsonParse(e))?;

        assert!(json.contains("$schema"));
        assert!(json.contains("test-session"));
        assert!(json.contains("_links"));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE ARRAY TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_envelope_array_new() {
        let envelope = SchemaEnvelopeArray::new("list-response", vec!["a", "b", "c"]);
        assert_eq!(envelope.data.len(), 3);
        assert_eq!(envelope.schema_type, "array");
        assert!(envelope.success);
    }

    #[test]
    fn test_schema_envelope_array_with_meta() {
        let data = vec![1, 2, 3];
        let meta = ResponseMeta::new("list").with_duration(50);
        let envelope = SchemaEnvelopeArray::new("list-response", data).with_meta(meta);

        assert!(envelope.meta.is_some());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SCHEMA URI TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_uri() {
        let uri = schemas::uri("status-response");
        assert!(uri.starts_with("scp://"));
        assert!(uri.contains("status-response"));
        assert!(uri.ends_with("/v1"));
    }

    #[test]
    fn test_all_valid_schemas() {
        let schemas = schemas::all_valid_schemas();
        assert!(!schemas.is_empty());
        assert!(schemas.contains(&schemas::STATUS_RESPONSE));
    }

    #[test]
    fn test_is_valid_schema() {
        assert!(schemas::is_valid_schema("status-response"));
        assert!(!schemas::is_valid_schema("invalid-schema"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // JSON ERROR FROM ERROR TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_json_error_from_session_not_found() {
        let err = crate::error::Error::SessionNotFound("test".to_string());
        let json_error = JsonError::from(&err);

        assert!(!json_error.success);
        assert!(json_error.error.code.contains("SESSION"));
    }

    #[test]
    fn test_json_error_from_workspace_not_found() {
        let err = crate::error::Error::WorkspaceNotFound("test".to_string());
        let json_error = JsonError::from(&err);

        assert!(!json_error.success);
        assert!(
            json_error.error.code.contains("WORKSPACE")
                || json_error.error.code.contains("NOT_FOUND")
        );
    }

    #[test]
    fn test_json_error_from_validation_error() {
        let err = crate::error::Error::ValidationError("Invalid input".to_string());
        let json_error = JsonError::from(&err);

        assert!(!json_error.success);
        assert_eq!(json_error.error.exit_code, 1);
    }

    #[test]
    fn test_json_error_from_io_error() {
        let err = crate::error::Error::IoError("disk full".to_string());
        let json_error = JsonError::from(&err);

        assert!(!json_error.success);
        assert_eq!(json_error.error.exit_code, 3);
    }

    #[test]
    fn test_json_error_from_jj_not_found() {
        let err = crate::error::Error::JjCommandError {
            operation: "test".to_string(),
            msg: "jj not found".to_string(),
            is_not_found: true,
        };
        let json_error = JsonError::from(&err);

        assert!(!json_error.success);
        assert!(json_error.error.code.contains("JJ"));
        assert!(json_error.error.suggestion.is_some());
    }
}

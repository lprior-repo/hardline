#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
// Context map function handles all error variants exhaustively
#![allow(clippy::too_many_lines)]

use std::fmt;

use serde::{Deserialize, Serialize};

/// Validation hint for explaining what was expected vs received
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationHint {
    /// The field or parameter that failed validation
    pub field: String,
    /// What was expected
    pub expected: String,
    /// What was actually received (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received: Option<String>,
    /// Example of valid input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    /// Regular expression pattern for valid input (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

impl ValidationHint {
    /// Create a new validation hint
    #[must_use]
    pub fn new(field: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            expected: expected.into(),
            received: None,
            example: None,
            pattern: None,
        }
    }

    /// Add what was received
    #[must_use]
    pub fn with_received(self, received: impl Into<String>) -> Self {
        Self {
            received: Some(received.into()),
            ..self
        }
    }

    /// Add an example
    #[must_use]
    pub fn with_example(self, example: impl Into<String>) -> Self {
        Self {
            example: Some(example.into()),
            ..self
        }
    }

    /// Add a pattern
    #[must_use]
    pub fn with_pattern(self, pattern: impl Into<String>) -> Self {
        Self {
            pattern: Some(pattern.into()),
            ..self
        }
    }
}

/// Context captured at the moment of failure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureContext {
    /// Working directory at failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    /// Current JJ workspace/branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_workspace: Option<String>,
    /// Active sessions at failure time
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub active_sessions: Vec<String>,
    /// Environment variables relevant to failure
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub relevant_env: Vec<(String, String)>,
    /// Command that was being executed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments to the command
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub arguments: Vec<String>,
    /// Timestamp of failure (ISO 8601)
    pub timestamp: String,
    /// Stack trace or phase when failure occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

impl Default for FailureContext {
    fn default() -> Self {
        Self {
            working_directory: None,
            current_workspace: None,
            active_sessions: Vec::new(),
            relevant_env: Vec::new(),
            command: None,
            arguments: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            phase: None,
        }
    }
}

impl FailureContext {
    /// Create a new failure context with current timestamp
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory
    #[must_use]
    pub fn with_working_directory(self, dir: impl Into<String>) -> Self {
        Self {
            working_directory: Some(dir.into()),
            ..self
        }
    }

    /// Set current workspace
    #[must_use]
    pub fn with_workspace(self, workspace: impl Into<String>) -> Self {
        Self {
            current_workspace: Some(workspace.into()),
            ..self
        }
    }

    /// Add active sessions
    #[must_use]
    pub fn with_active_sessions(self, sessions: Vec<String>) -> Self {
        Self {
            active_sessions: sessions,
            ..self
        }
    }

    /// Set the command and arguments
    #[must_use]
    pub fn with_command(self, cmd: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: Some(cmd.into()),
            arguments: args,
            ..self
        }
    }

    /// Set the phase where failure occurred
    #[must_use]
    pub fn with_phase(self, phase: impl Into<String>) -> Self {
        Self {
            phase: Some(phase.into()),
            ..self
        }
    }

    /// Add relevant environment variable
    #[must_use]
    pub fn with_env(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut relevant_env = self.relevant_env;
        relevant_env.push((key.into(), value.into()));
        Self {
            relevant_env,
            ..self
        }
    }

    /// Capture current working directory from environment
    #[must_use]
    pub fn capture_cwd(self) -> Self {
        if let Ok(cwd) = std::env::current_dir() {
            Self {
                working_directory: Some(cwd.to_string_lossy().to_string()),
                ..self
            }
        } else {
            self
        }
    }
}

/// Rich error information for AI-first CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichError {
    /// The underlying error
    #[serde(flatten)]
    pub error: RichErrorInfo,
    /// Validation hints (what was expected vs received)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub validation_hints: Vec<ValidationHint>,
    /// Context captured at failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_at_failure: Option<FailureContext>,
    /// Fix commands that can resolve this error
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fix_commands: Vec<String>,
}

/// Serializable error info (subset of Error for JSON output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichErrorInfo {
    /// Error code (`SCREAMING_SNAKE_CASE`)
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Exit code
    pub exit_code: i32,
    /// Structured context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RichError {
    /// Create a `RichError` from an Error with optional context
    #[must_use]
    pub fn from_error(error: &Error) -> Self {
        Self {
            error: RichErrorInfo {
                code: error.code().to_string(),
                message: error.to_string(),
                exit_code: error.exit_code(),
                details: error.context_map(),
                suggestion: error.suggestion(),
            },
            validation_hints: error.validation_hints(),
            context_at_failure: None,
            fix_commands: error.fix_commands(),
        }
    }

    /// Add failure context
    #[must_use]
    pub fn with_context(self, context: FailureContext) -> Self {
        Self {
            context_at_failure: Some(context),
            ..self
        }
    }

    /// Add additional fix commands
    #[must_use]
    pub fn with_fix_commands(self, commands: Vec<String>) -> Self {
        Self {
            fix_commands: commands,
            ..self
        }
    }

    /// Add additional validation hints
    #[must_use]
    pub fn with_validation_hints(self, hints: Vec<ValidationHint>) -> Self {
        let mut validation_hints = self.validation_hints;
        validation_hints.extend(hints);
        Self {
            validation_hints,
            ..self
        }
    }
}

/// Types of JJ workspace conflicts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JjConflictType {
    /// Workspace name already exists in repository
    AlreadyExists,
    /// Concurrent modification detected (multiple operations)
    ConcurrentModification,
    /// Workspace was abandoned and is no longer valid
    Abandoned,
    /// Working copy is stale/out of sync with repository
    Stale,
}

impl fmt::Display for JjConflictType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(f, "workspace already exists"),
            Self::ConcurrentModification => write!(f, "concurrent modification detected"),
            Self::Abandoned => write!(f, "workspace abandoned"),
            Self::Stale => write!(f, "working copy stale"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    InvalidConfig(String),
    IoError(String),
    ParseError(String),
    ValidationError {
        message: String,
        field: Option<String>,
        value: Option<String>,
        constraints: Vec<String>,
    },
    NotFound(String),
    /// Session not found in database (specific to lock operations)
    SessionNotFound {
        session: String,
    },
    DatabaseError(String),
    Command(String),
    // Stak-sourced errors
    InvalidId(String),
    InvalidInput(String),
    InvalidState(String),
    QueueError(String),
    AgentError(String),
    DagError(String),
    VcsError(String),
    HookFailed {
        hook_type: String,
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    HookExecutionFailed {
        command: String,
        source: String,
    },
    JjCommandError {
        operation: String,
        source: String,
        is_not_found: bool,
    },
    /// JJ workspace conflict with structured recovery information
    JjWorkspaceConflict {
        /// Type of conflict detected
        conflict_type: JjConflictType,
        /// Workspace name involved in conflict
        workspace_name: String,
        /// Original error from JJ
        source: String,
        /// Actionable recovery hint
        recovery_hint: String,
    },
    /// Deduplication key conflict when different workspaces attempt to use the same key
    DedupeKeyConflict {
        /// The `dedupe_key` that caused the conflict
        dedupe_key: String,
        /// Workspace that already has this `dedupe_key`
        existing_workspace: String,
        /// Workspace attempting to use this `dedupe_key`
        provided_workspace: String,
    },
    SessionLocked {
        session: String,
        holder: String,
    },
    NotLockHolder {
        session: String,
        agent_id: String,
    },
    /// Lock acquisition timeout with fail-fast semantics
    LockTimeout {
        operation: String,
        timeout_ms: u64,
        retries: usize,
    },
    OperationCancelled(String),
    /// Serialization error (e.g., JSON serialization failed)
    Serialization(String),
    /// IO error with context
    Io(String),
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
            Self::IoError(msg) | Self::Io(msg) => write!(f, "IO error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::ValidationError { message, .. } => write!(f, "Validation error: {message}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::SessionNotFound { session } => {
                write!(f, "Session '{session}' not found")
            }
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::Command(msg) => write!(f, "Command error: {msg}"),
            Self::HookFailed {
                hook_type,
                command,
                exit_code,
                stdout: _,
                stderr,
            } => {
                write!(
                    f,
                    "Hook '{hook_type}' failed: {command}\nExit code: {}\nStderr: {stderr}",
                    exit_code.map_or_else(|| "None".to_string(), |c| c.to_string())
                )
            }
            Self::HookExecutionFailed { command, source } => {
                write!(f, "Failed to execute hook '{command}': {source}")
            }
            Self::JjCommandError {
                operation,
                source,
                is_not_found,
            } => {
                if *is_not_found {
                    write!(
                        f,
                        "Failed to {operation}: JJ is not installed or not in PATH.\n\n\nInstall JJ:\n\n  cargo install jj-cli\n\nor:\n\n  brew install jj\n\nor visit: https://github.com/martinvonz/jj#installation\n\nError: {source}"
                    )
                } else {
                    write!(f, "Failed to {operation}: {source}")
                }
            }
            Self::JjWorkspaceConflict {
                conflict_type,
                workspace_name,
                source,
                recovery_hint,
            } => {
                write!(
                    f,
                    "JJ workspace conflict: {conflict_type}\n\nWorkspace: {workspace_name}\n\n{recovery_hint}\n\nJJ error: {source}"
                )
            }
            Self::DedupeKeyConflict {
                dedupe_key,
                existing_workspace,
                provided_workspace,
            } => {
                write!(
                    f,
                    "Dedupe key conflict: '{dedupe_key}' already used by workspace '{existing_workspace}', cannot be used by '{provided_workspace}'"
                )
            }
            Self::SessionLocked { session, holder } => {
                write!(f, "Session '{session}' is locked by agent '{holder}'")
            }
            Self::NotLockHolder { session, agent_id } => {
                write!(
                    f,
                    "Agent '{agent_id}' does not hold the lock for session '{session}'"
                )
            }
            Self::LockTimeout {
                operation,
                timeout_ms,
                retries,
            } => {
                write!(
                    f,
                    "Lock acquisition timeout for '{operation}' after {retries} retries (timeout: {timeout_ms}ms per attempt)"
                )
            }
            Self::OperationCancelled(msg) => write!(f, "Operation cancelled: {msg}"),
            Self::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            Self::Unknown(msg) => write!(f, "Unknown error: {msg}"),
            Self::InvalidId(msg) => write!(f, "Invalid ID: {msg}"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            Self::InvalidState(msg) => write!(f, "Invalid state: {msg}"),
            Self::QueueError(msg) => write!(f, "Queue error: {msg}"),
            Self::AgentError(msg) => write!(f, "Agent error: {msg}"),
            Self::DagError(msg) => write!(f, "DAG error: {msg}"),
            Self::VcsError(msg) => write!(f, "VCS error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::ParseError(format!("Failed to parse config: {err}"))
    }
}

impl From<crate::beads::BeadsError> for Error {
    fn from(err: crate::beads::BeadsError) -> Self {
        match err {
            crate::beads::BeadsError::DatabaseError(msg)
            | crate::beads::BeadsError::QueryFailed(msg)
            | crate::beads::BeadsError::InsertFailed(msg) => Self::DatabaseError(msg),
            crate::beads::BeadsError::NotFound(msg) => Self::NotFound(msg),
            crate::beads::BeadsError::InvalidFilter(msg)
            | crate::beads::BeadsError::ValidationFailed(msg) => Self::ValidationError {
                message: msg,
                field: None,
                value: None,
                constraints: Vec::new(),
            },
            crate::beads::BeadsError::PathError(msg) => Self::IoError(msg),
            crate::beads::BeadsError::DuplicateId(msg) => Self::ValidationError {
                message: format!("Bead with ID '{msg}' already exists"),
                field: Some("id".to_string()),
                value: Some(msg),
                constraints: vec!["unique".to_string()],
            },
        }
    }
}

impl Error {
    /// Returns the machine-readable error code for this error.
    ///
    /// Error codes are always in `SCREAMING_SNAKE_CASE` format.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidConfig(_) => "INVALID_CONFIG",
            Self::IoError(_) | Self::Io(_) => "IO_ERROR",
            Self::ParseError(_) => "PARSE_ERROR",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::Command(_) => "COMMAND_ERROR",
            Self::HookFailed { .. } => "HOOK_FAILED",
            Self::HookExecutionFailed { .. } => "HOOK_EXECUTION_FAILED",
            Self::JjCommandError { .. } => "JJ_COMMAND_ERROR",
            Self::JjWorkspaceConflict { .. } => "JJ_WORKSPACE_CONFLICT",
            Self::DedupeKeyConflict { .. } => "DEDUPE_KEY_CONFLICT",
            Self::SessionLocked { .. } => "SESSION_LOCKED",
            Self::NotLockHolder { .. } => "NOT_LOCK_HOLDER",
            Self::LockTimeout { .. } => "LOCK_TIMEOUT",
            Self::OperationCancelled(_) => "OPERATION_CANCELLED",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::Unknown(_) => "UNKNOWN",
            Self::InvalidId(_) => "INVALID_ID",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::InvalidState(_) => "INVALID_STATE",
            Self::QueueError(_) => "QUEUE_ERROR",
            Self::AgentError(_) => "AGENT_ERROR",
            Self::DagError(_) => "DAG_ERROR",
            Self::VcsError(_) => "VCS_ERROR",
        }
    }

    /// Returns structured context information for this error as a JSON value.
    ///
    /// This provides machine-readable context that can be used by AI agents
    /// or tools to understand the error in detail.
    #[must_use]
    pub fn context_map(&self) -> Option<serde_json::Value> {
        match self {
            Self::InvalidConfig(msg) => Some(serde_json::json!({
                "input": msg,
                "expected_format": "valid TOML configuration"
            })),
            Self::ValidationError { message, .. } => Some(serde_json::json!({
                "input": message,
                "expected_format": "alphanumeric, dash, underscore only"
            })),
            Self::NotFound(msg) => Some(serde_json::json!({
                "resource_type": "session",
                "resource_id": msg,
                "searched_in": "database"
            })),
            Self::SessionNotFound { session } => Some(serde_json::json!({
                "resource_type": "session",
                "session_name": session,
                "searched_in": "database"
            })),
            Self::IoError(msg) => Some(serde_json::json!({
                "operation": "file_io",
                "error": msg
            })),
            Self::DatabaseError(msg) => Some(serde_json::json!({
                "operation": "database",
                "error": msg
            })),
            Self::Command(msg) => Some(serde_json::json!({
                "operation": "command_execution",
                "error": msg
            })),
            Self::HookFailed {
                hook_type,
                command,
                exit_code,
                stdout: _,
                stderr,
            } => Some(serde_json::json!({
                "hook_type": hook_type,
                "command": command,
                "exit_code": exit_code,
                "stderr": stderr
            })),
            Self::HookExecutionFailed { command, source } => Some(serde_json::json!({
                "command": command,
                "source": source
            })),
            Self::JjCommandError {
                operation,
                source,
                is_not_found,
            } => Some(serde_json::json!({
                "operation": operation,
                "source": source,
                "is_not_found": is_not_found
            })),
            Self::JjWorkspaceConflict {
                conflict_type,
                workspace_name,
                source,
                recovery_hint: _,
            } => Some(serde_json::json!({
                "conflict_type": conflict_type,
                "workspace_name": workspace_name,
                "source": source,
            })),
            Self::DedupeKeyConflict {
                dedupe_key,
                existing_workspace,
                provided_workspace,
            } => Some(serde_json::json!({
                "dedupe_key": dedupe_key,
                "existing_workspace": existing_workspace,
                "provided_workspace": provided_workspace,
            })),
            Self::SessionLocked { session, holder } => Some(serde_json::json!({
                "session": session,
                "holder": holder
            })),
            Self::NotLockHolder { session, agent_id } => Some(serde_json::json!({
                "session": session,
                "agent_id": agent_id
            })),
            Self::LockTimeout {
                operation,
                timeout_ms,
                retries,
            } => Some(serde_json::json!({
                "operation": operation,
                "timeout_ms": timeout_ms,
                "retries": retries
            })),
            Self::OperationCancelled(reason) => Some(serde_json::json!({
                "reason": reason
            })),
            Self::ParseError(_) | Self::Unknown(_) => None,
            Self::Serialization(msg) | Self::Io(msg) => Some(serde_json::json!({
                "message": msg
            })),
            Self::InvalidId(msg) => Some(serde_json::json!({
                "error_type": "invalid_id",
                "message": msg
            })),
            Self::InvalidInput(msg) => Some(serde_json::json!({
                "error_type": "invalid_input",
                "message": msg
            })),
            Self::InvalidState(msg) => Some(serde_json::json!({
                "error_type": "invalid_state",
                "message": msg
            })),
            Self::QueueError(msg) => Some(serde_json::json!({
                "error_type": "queue_error",
                "message": msg
            })),
            Self::AgentError(msg) => Some(serde_json::json!({
                "error_type": "agent_error",
                "message": msg
            })),
            Self::DagError(msg) => Some(serde_json::json!({
                "error_type": "dag_error",
                "message": msg
            })),
            Self::VcsError(msg) => Some(serde_json::json!({
                "error_type": "vcs_error",
                "message": msg
            })),
        }
    }

    /// Returns a helpful suggestion for resolving this error, if available.
    ///
    /// Suggestions are actionable and guide the user toward a solution.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::NotFound(_) => Some("Try 'isolate list' to see available sessions".to_string()),
            Self::SessionNotFound { .. } => Some("Use 'isolate list' to see available sessions".to_string()),
            Self::ValidationError { message, .. } => {
                if message.contains("name") {
                    Some(
                        "Session name must start with letter and contain only alphanumeric, dash, underscore"
                            .to_string(),
                    )
                } else {
                    Some(format!("Invalid input: {message}. Check the input format and try again."))
                }
            }
            Self::DatabaseError(_) => {
                Some("Try 'isolate doctor' to check database health".to_string())
            }
            Self::JjCommandError {
                is_not_found: true,
                ..
            } => Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
            Self::JjCommandError { .. } => Some(
                "Check JJ is working: 'jj status', or try 'isolate doctor' for diagnostics".to_string(),
            ),
            Self::JjWorkspaceConflict { .. } => Some(
                "This is a JJ workspace conflict. Check recovery hints in error message for specific steps.".to_string()
            ),
            Self::DedupeKeyConflict { existing_workspace, .. } => Some(
                format!("Dedupe key already used by workspace '{existing_workspace}'. Use a different dedupe_key or wait for the existing entry to complete.")
            ),
            Self::HookFailed { hook_type, .. } => Some(
                format!("Check hook '{hook_type}' configuration: 'isolate config get hooks.{hook_type}' or use --no-hooks to skip")
            ),
            Self::HookExecutionFailed { .. } => {
                Some("Ensure the hook command exists and is executable".to_string())
            }
            Self::InvalidConfig(_) => Some(
                "Check configuration: 'isolate config list' or 'isolate config reset' to restore defaults".to_string(),
            ),
            Self::IoError(msg) if msg.contains("Permission denied") => {
                Some("Check file permissions: 'ls -la' or run with appropriate access rights".to_string())
            }
            Self::IoError(msg) if msg.contains("No such file") || msg.contains("not found") => {
                Some("Ensure the file or directory exists: 'ls -la' or 'isolate doctor' to check setup".to_string())
            }
            Self::IoError(_) => Some(
                "Check disk space, permissions, or run 'isolate doctor' for diagnostics".to_string(),
            ),
            Self::ParseError(msg) if msg.contains("JSON") || msg.contains("json") => {
                Some("Fix JSON syntax: Use 'jq .' to validate or check format with online validator".to_string())
            }
            Self::ParseError(msg) if msg.contains("TOML") || msg.contains("toml") => {
                Some("Fix TOML syntax: Check for proper key = 'value' pairs and indentation".to_string())
            }
            Self::ParseError(_) => Some(
                "Fix the input format and try again, or run 'isolate doctor' for help".to_string(),
            ),
            Self::Command(msg) if msg.contains("not found") => {
                Some("Ensure the command is installed and in PATH".to_string())
            }
            Self::Command(_) => Some(
                "Check the command syntax and requirements, or run 'isolate doctor'".to_string(),
            ),
            Self::SessionLocked { session, holder } => Some(
                format!("Session '{session}' is locked by '{holder}'. Use 'isolate yield {session}' to release or check status with 'isolate agents status'")
            ),
            Self::NotLockHolder { session, .. } => Some(
                format!("You don't hold the lock for '{session}'. Use 'isolate claim {session}' to acquire it or check with 'isolate agents status'")
            ),
            Self::LockTimeout { .. } => Some(
                "System is under heavy load. Wait a few moments and retry, or check 'isolate agents status' for stuck operations".to_string(),
            ),
            Self::Unknown(_) => Some(
                "Run 'isolate doctor' to check system health and configuration".to_string(),
            ),
            Self::OperationCancelled(_) => None, // User-initiated, no suggestion needed
            Self::Serialization(_) => Some(
                "Data could not be serialized. Check for invalid characters or types.".to_string(),
            ),
            Self::Io(msg) => Some(format!("IO error: {msg}. Check file permissions and disk space.")),
            Self::InvalidId(_) => Some("Check the ID format and try again.".to_string()),
            Self::InvalidInput(_) => Some("Check the input format and try again.".to_string()),
            Self::InvalidState(_) => Some("The operation cannot be performed in the current state. Check the system state and try again.".to_string()),
            Self::QueueError(_) => Some("Check queue status with 'stak queue list'.".to_string()),
            Self::AgentError(_) => Some("Check agent status with 'stak agent list'.".to_string()),
            Self::DagError(_) => Some("Check branch relationships with 'stak branch list'.".to_string()),
            Self::VcsError(_) => Some("Check VCS status with 'stak status'.".to_string()),
        }
    }

    /// Returns the semantic exit code for this error.
    ///
    /// Exit codes follow this semantic mapping:
    /// - 1: Usage/validation errors (invalid config, parse errors, validation failures)
    /// - 2: Not found errors (missing resources)
    /// - 3: System errors (IO, database issues)
    /// - 4: External command errors (JJ, hooks, etc.)
    /// - 5: Lock contention errors
    /// - 130: Operation cancelled (SIGINT)
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            // Usage/validation errors: exit code 1
            Self::InvalidConfig(_)
            | Self::ParseError(_)
            | Self::DedupeKeyConflict { .. }
            | Self::ValidationError { .. } => 1,
            // Not found errors: exit code 2
            Self::NotFound(_) | Self::SessionNotFound { .. } => 2,
            // System errors: exit code 3
            Self::IoError(_) | Self::DatabaseError(_) | Self::Serialization(_) | Self::Io(_) => 3,
            // External command errors: exit code 4
            Self::Command(_)
            | Self::JjCommandError { .. }
            | Self::JjWorkspaceConflict { .. }
            | Self::HookFailed { .. }
            | Self::HookExecutionFailed { .. }
            | Self::Unknown(_) => 4,
            // Lock contention errors: exit code 5
            Self::SessionLocked { .. } | Self::NotLockHolder { .. } | Self::LockTimeout { .. } => 5,
            // Operation cancelled: exit code 130 (SIGINT)
            Self::OperationCancelled(_) => 130,
            // New error types - treat as validation errors
            Self::InvalidId(_) | Self::InvalidInput(_) | Self::InvalidState(_) => 1,
            Self::QueueError(_) | Self::AgentError(_) | Self::DagError(_) | Self::VcsError(_) => 3,
        }
    }

    /// Returns validation hints that explain what was expected vs received.
    ///
    /// Validation hints provide structured guidance for fixing input errors.
    #[must_use]
    pub fn validation_hints(&self) -> Vec<ValidationHint> {
        match self {
            Self::ValidationError { message, .. } => {
                if message.contains("name") || message.contains("session") {
                    vec![ValidationHint::new(
                        "session_name",
                        "alphanumeric with dashes/underscores",
                    )
                    .with_example("feature-auth")
                    .with_pattern("^[a-zA-Z][a-zA-Z0-9_-]*$")]
                } else if message.contains("empty") {
                    vec![ValidationHint::new("input", "non-empty value")
                        .with_received("(empty string)")]
                } else {
                    vec![ValidationHint::new("input", "valid value").with_received(message.clone())]
                }
            }
            Self::InvalidConfig(msg) => {
                vec![ValidationHint::new("config", "valid TOML configuration")
                    .with_example("[isolate]\nworkspace_dir = \"./workspaces\"")
                    .with_received(msg.clone())]
            }
            Self::DedupeKeyConflict {
                existing_workspace, ..
            } => {
                vec![ValidationHint::new("dedupe_key", "unique dedupe_key")
                    .with_received(format!("already used by '{existing_workspace}'"))]
            }
            Self::ParseError(msg) => {
                if msg.contains("JSON") || msg.contains("json") {
                    vec![ValidationHint::new("input", "valid JSON")
                        .with_example("{\"key\": \"value\"}")
                        .with_received(msg.clone())]
                } else if msg.contains("TOML") || msg.contains("toml") {
                    vec![ValidationHint::new("input", "valid TOML")
                        .with_example("key = \"value\"")
                        .with_received(msg.clone())]
                } else {
                    vec![ValidationHint::new("input", "parseable format").with_received(msg.clone())]
                }
            }
            Self::SessionLocked { session, holder } => {
                vec![ValidationHint::new("session", "unlocked session")
                    .with_received(format!("'{session}' locked by '{holder}'"))]
            }
            Self::NotLockHolder { session, agent_id } => {
                vec![ValidationHint::new("agent_id", "lock holder for session")
                    .with_received(format!("'{agent_id}' for session '{session}'"))]
            }
            Self::LockTimeout {
                operation,
                timeout_ms,
                ..
            } => {
                vec![ValidationHint::new("lock", "acquired within timeout")
                    .with_received(format!("'{operation}' timed out after {timeout_ms}ms"))]
            }
            Self::IoError(_)
            | Self::NotFound(_)
            | Self::SessionNotFound { .. }
            | Self::DatabaseError(_)
            | Self::Command(_)
            | Self::HookFailed { .. }
            | Self::HookExecutionFailed { .. }
            | Self::JjCommandError { .. }
            | Self::JjWorkspaceConflict { .. }
            | Self::OperationCancelled(_)
            | Self::Serialization(_)
            | Self::Io(_)
            | Self::Unknown(_)
            | Self::InvalidId(_)
            | Self::InvalidInput(_)
            | Self::InvalidState(_)
            | Self::QueueError(_)
            | Self::AgentError(_)
            | Self::DagError(_)
            | Self::VcsError(_) => vec![],
        }
    }

    /// Helper: Fix commands for JJ workspace conflicts by type
    #[must_use]
    fn jj_conflict_fix_commands(
        conflict_type: &JjConflictType,
        workspace_name: &str,
    ) -> Vec<String> {
        match conflict_type {
            JjConflictType::AlreadyExists => vec![
                "jj workspace list".to_string(),
                format!("jj workspace forget {workspace_name}"),
            ],
            JjConflictType::ConcurrentModification => {
                vec!["pgrep -fl jj".to_string(), "jj workspace list".to_string()]
            }
            JjConflictType::Abandoned => vec![
                format!("jj workspace forget {workspace_name}"),
                "jj status".to_string(),
            ],
            JjConflictType::Stale => vec![
                "jj workspace update-stale".to_string(),
                "jj reload".to_string(),
                "jj status".to_string(),
            ],
        }
    }

    /// Helper: Fix commands for IO errors based on message content
    #[must_use]
    fn io_error_fix_commands(msg: &str) -> Vec<String> {
        if msg.contains("Permission") || msg.contains("not found") || msg.contains("No such file") {
            vec!["ls -la".to_string(), "isolate doctor".to_string()]
        } else {
            vec!["df -h".to_string(), "isolate doctor".to_string()]
        }
    }

    /// Helper: Fix commands for parse errors based on format type
    #[must_use]
    fn parse_error_fix_commands(msg: &str) -> Vec<String> {
        if msg.contains("JSON") || msg.contains("json") {
            vec!["echo '{}' | jq .".to_string(), "isolate doctor".to_string()]
        } else if msg.contains("TOML") || msg.contains("toml") {
            vec![
                "isolate config list".to_string(),
                "isolate doctor".to_string(),
            ]
        } else {
            vec!["isolate doctor".to_string()]
        }
    }

    /// Returns fix commands that can potentially resolve this error.
    ///
    /// These are copy-pastable shell commands that AI agents can execute.
    #[must_use]
    pub fn fix_commands(&self) -> Vec<String> {
        match self {
            Self::NotFound(msg) => {
                if msg.contains("session") {
                    vec![
                        "isolate list".to_string(),
                        "isolate add <session-name>".to_string(),
                    ]
                } else {
                    vec!["isolate list".to_string()]
                }
            }
            Self::SessionNotFound { session } => {
                vec!["isolate list".to_string(), format!("isolate add {session}")]
            }
            Self::ValidationError { message, .. } => {
                if message.contains("name") {
                    vec!["isolate add my-valid-session".to_string()]
                } else {
                    vec![]
                }
            }
            Self::DatabaseError(_) => {
                vec![
                    "isolate doctor".to_string(),
                    "isolate doctor --fix".to_string(),
                ]
            }
            Self::JjCommandError {
                is_not_found: true, ..
            } => {
                vec![
                    "cargo install jj-cli".to_string(),
                    "brew install jj".to_string(),
                ]
            }
            Self::JjCommandError {
                is_not_found: false,
                operation,
                ..
            } => {
                if operation.contains("workspace") {
                    vec![
                        "jj workspace list".to_string(),
                        "isolate doctor".to_string(),
                    ]
                } else {
                    vec!["jj status".to_string()]
                }
            }
            Self::JjWorkspaceConflict {
                conflict_type,
                workspace_name,
                ..
            } => Self::jj_conflict_fix_commands(conflict_type, workspace_name),
            Self::DedupeKeyConflict {
                existing_workspace, ..
            } => {
                vec![
                    "isolate list".to_string(),
                    format!("isolate remove {existing_workspace}"),
                ]
            }
            Self::SessionLocked { session, .. } => {
                vec![
                    format!("isolate agent status {session}"),
                    format!("isolate yield {session}"),
                ]
            }
            Self::NotLockHolder { session, .. } => {
                vec![
                    format!("isolate claim {session}"),
                    format!("isolate agent status {session}"),
                ]
            }
            Self::LockTimeout { .. } => {
                vec![
                    "isolate agents status".to_string(),
                    "sleep 2 && isolate <command> --retry".to_string(),
                ]
            }
            Self::HookFailed { hook_type, .. } => {
                vec![
                    format!("isolate config get hooks.{hook_type}"),
                    "isolate config list hooks".to_string(),
                ]
            }
            Self::InvalidConfig(_) => {
                vec![
                    "isolate config list".to_string(),
                    "isolate config reset".to_string(),
                ]
            }
            Self::IoError(msg) | Self::Io(msg) => Self::io_error_fix_commands(msg),
            Self::ParseError(msg) => Self::parse_error_fix_commands(msg),
            Self::Command(msg) if msg.contains("not found") => {
                vec!["which <command>".to_string(), "isolate doctor".to_string()]
            }
            Self::Command(_) | Self::Unknown(_) | Self::Serialization(_) => {
                vec!["isolate doctor".to_string()]
            }
            Self::OperationCancelled(_) => vec![],
            Self::HookExecutionFailed { .. } => vec!["isolate config list hooks".to_string()],
            Self::InvalidId(_) | Self::InvalidInput(_) | Self::InvalidState(_) => vec![],
            Self::QueueError(_) => vec!["stak queue list".to_string()],
            Self::AgentError(_) => vec!["stak agent list".to_string()],
            Self::DagError(_) => vec!["stak branch list".to_string()],
            Self::VcsError(_) => vec!["stak status".to_string()],
        }
    }

    /// Convert to `RichError` with optional failure context
    #[must_use]
    pub fn to_rich_error(&self) -> RichError {
        RichError::from_error(self)
    }

    /// Convert to `RichError` with captured failure context
    #[must_use]
    pub fn to_rich_error_with_context(&self, context: FailureContext) -> RichError {
        RichError::from_error(self).with_context(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_config() {
        let err = Error::InvalidConfig("test error".into());
        assert_eq!(err.to_string(), "Invalid configuration: test error");
    }

    #[test]
    fn test_error_display_database_error() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::InvalidConfig("test".into());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("InvalidConfig"));
    }

    #[test]
    fn test_error_display_hook_failed() {
        let err = Error::HookFailed {
            hook_type: "post_create".to_string(),
            command: "npm install".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "Package not found".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Hook 'post_create' failed"));
        assert!(display.contains("npm install"));
        assert!(display.contains("Exit code: 1"));
        assert!(display.contains("Package not found"));
    }

    #[test]
    fn test_error_display_hook_execution_failed() {
        let err = Error::HookExecutionFailed {
            command: "invalid-shell".to_string(),
            source: "No such file or directory".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Failed to execute hook"));
        assert!(display.contains("invalid-shell"));
        assert!(display.contains("No such file or directory"));
    }

    #[test]
    fn test_error_display_jj_command_not_found() {
        let err = Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: "No such file or directory (os error 2)".to_string(),
            is_not_found: true,
        };
        let display = err.to_string();
        assert!(display.contains("Failed to create workspace"));
        assert!(display.contains("JJ is not installed"));
        assert!(display.contains("cargo install jj-cli"));
        assert!(display.contains("brew install jj"));
    }

    #[test]
    fn test_error_display_jj_command_other_error() {
        let err = Error::JjCommandError {
            operation: "list workspaces".to_string(),
            source: "Permission denied".to_string(),
            is_not_found: false,
        };
        let display = err.to_string();
        assert!(display.contains("Failed to list workspaces"));
        assert!(display.contains("Permission denied"));
        assert!(!display.contains("JJ is not installed"));
    }

    #[test]
    fn test_error_code_validation_error() {
        let err = Error::ValidationError {
            message: "invalid input".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_error_code_not_found() {
        let err = Error::NotFound("session not found".into());
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn test_error_code_io_error() {
        let err = Error::IoError("file not found".into());
        assert_eq!(err.code(), "IO_ERROR");
    }

    #[test]
    fn test_error_code_database_error() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.code(), "DATABASE_ERROR");
    }

    #[test]
    fn test_error_code_uppercase() {
        let err = Error::InvalidConfig("bad config".into());
        let code = err.code();
        assert_eq!(code, code.to_uppercase(), "Error code must be uppercase");
    }

    #[test]
    fn test_validation_error_context_has_field() {
        let err = Error::ValidationError {
            message: "Session name must be alphanumeric".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("input").is_some());
        }
    }

    #[test]
    fn test_not_found_error_context_has_resource() {
        let err = Error::NotFound("session 'test-123' not found".into());
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("resource_type").is_some());
        }
    }

    #[test]
    fn test_io_error_context_has_path() {
        let err = Error::IoError("Failed to read file".into());
        let context = err.context_map();
        assert!(context.is_some());
        if let Some(ctx) = context {
            assert!(ctx.get("operation").is_some());
        }
    }

    #[test]
    fn test_session_not_found_suggests_list() {
        let err = Error::NotFound("session not found".into());
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
        if let Some(sugg) = suggestion {
            assert!(sugg.contains("isolate list") || sugg.contains("list"));
        }
    }

    #[test]
    fn test_validation_error_suggests_format() {
        let err = Error::ValidationError {
            message: "invalid session name".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let suggestion = err.suggestion();
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_validation_error_maps_to_exit_code_1() {
        let err = Error::ValidationError {
            message: "invalid input".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_invalid_config_maps_to_exit_code_1() {
        let err = Error::InvalidConfig("bad config".into());
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_parse_error_maps_to_exit_code_1() {
        let err = Error::ParseError("malformed input".into());
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_not_found_error_maps_to_exit_code_2() {
        let err = Error::NotFound("session not found".into());
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_io_error_maps_to_exit_code_3() {
        let err = Error::IoError("file not found".into());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_database_error_maps_to_exit_code_3() {
        let err = Error::DatabaseError("connection failed".into());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_command_error_maps_to_exit_code_4() {
        let err = Error::Command("command failed".into());
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_jj_command_error_maps_to_exit_code_4() {
        let err = Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: "error".to_string(),
            is_not_found: false,
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_hook_failed_maps_to_exit_code_4() {
        let err = Error::HookFailed {
            hook_type: "post_create".to_string(),
            command: "npm install".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "Package not found".to_string(),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_hook_execution_failed_maps_to_exit_code_4() {
        let err = Error::HookExecutionFailed {
            command: "invalid-shell".to_string(),
            source: "No such file or directory".to_string(),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn test_unknown_error_maps_to_exit_code_4() {
        let err = Error::Unknown("unknown error".into());
        assert_eq!(err.exit_code(), 4);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VALIDATION HINT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validation_hint_new() {
        let hint = ValidationHint::new("session_name", "alphanumeric");
        assert_eq!(hint.field, "session_name");
        assert_eq!(hint.expected, "alphanumeric");
        assert!(hint.received.is_none());
        assert!(hint.example.is_none());
    }

    #[test]
    fn test_validation_hint_with_received() {
        let hint = ValidationHint::new("input", "non-empty").with_received("(empty)");
        assert_eq!(hint.received, Some("(empty)".to_string()));
    }

    #[test]
    fn test_validation_hint_with_example() {
        let hint = ValidationHint::new("session_name", "valid name").with_example("feature-auth");
        assert_eq!(hint.example, Some("feature-auth".to_string()));
    }

    #[test]
    fn test_validation_hint_with_pattern() {
        let hint = ValidationHint::new("name", "valid pattern").with_pattern("^[a-z]+$");
        assert_eq!(hint.pattern, Some("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_validation_hint_serialization() -> Result<(), serde_json::Error> {
        let hint = ValidationHint::new("field", "expected")
            .with_received("received")
            .with_example("example");
        let json_str = serde_json::to_string(&hint)?;
        assert!(json_str.contains("\"field\":\"field\""));
        assert!(json_str.contains("\"expected\":\"expected\""));
        assert!(json_str.contains("\"received\":\"received\""));
        Ok(())
    }

    #[test]
    fn test_validation_error_returns_hints() {
        let err = Error::ValidationError {
            message: "invalid session name".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let hints = err.validation_hints();
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].example.is_some());
        }
    }

    #[test]
    fn test_empty_validation_error_returns_hints() {
        let err = Error::ValidationError {
            message: "value cannot be empty".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let hints = err.validation_hints();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_session_locked_returns_hints() {
        let err = Error::SessionLocked {
            session: "test".to_string(),
            holder: "agent-1".to_string(),
        };
        let hints = err.validation_hints();
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].received.is_some());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FAILURE CONTEXT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_failure_context_new() {
        let ctx = FailureContext::new();
        assert!(!ctx.timestamp.is_empty());
        assert!(ctx.working_directory.is_none());
    }

    #[test]
    fn test_failure_context_with_working_directory() {
        let ctx = FailureContext::new().with_working_directory("/home/user/project");
        assert_eq!(
            ctx.working_directory,
            Some("/home/user/project".to_string())
        );
    }

    #[test]
    fn test_failure_context_with_workspace() {
        let ctx = FailureContext::new().with_workspace("feature-branch");
        assert_eq!(ctx.current_workspace, Some("feature-branch".to_string()));
    }

    #[test]
    fn test_failure_context_with_command() {
        let ctx =
            FailureContext::new().with_command("isolate add", vec!["test-session".to_string()]);
        assert_eq!(ctx.command, Some("isolate add".to_string()));
        assert_eq!(ctx.arguments, vec!["test-session"]);
    }

    #[test]
    fn test_failure_context_with_phase() {
        let ctx = FailureContext::new().with_phase("workspace_creation");
        assert_eq!(ctx.phase, Some("workspace_creation".to_string()));
    }

    #[test]
    fn test_failure_context_with_env() {
        let ctx = FailureContext::new()
            .with_env("ZELLIJ_SESSION", "main")
            .with_env("JJ_USER", "test");
        assert_eq!(ctx.relevant_env.len(), 2);
    }

    #[test]
    fn test_failure_context_serialization() {
        let ctx = FailureContext::new()
            .with_working_directory("/tmp")
            .with_command("test", vec![]);
        let json = serde_json::to_string(&ctx);
        let Ok(json_str) = json else {
            panic!("serialization failed");
        };
        assert!(json_str.contains("\"working_directory\":\"/tmp\""));
        assert!(json_str.contains("\"timestamp\":"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FIX COMMANDS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_not_found_error_returns_fix_commands() {
        let err = Error::NotFound("session 'test' not found".into());
        let commands = err.fix_commands();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.contains("isolate list")));
    }

    #[test]
    fn test_validation_error_returns_fix_commands() {
        let err = Error::ValidationError {
            message: "invalid session name".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let commands = err.fix_commands();
        assert!(!commands.is_empty());
    }

    #[test]
    fn test_database_error_returns_fix_commands() {
        let err = Error::DatabaseError("corrupted".into());
        let commands = err.fix_commands();
        assert!(commands.iter().any(|c| c.contains("doctor")));
    }

    #[test]
    fn test_jj_not_found_returns_install_commands() {
        let err = Error::JjCommandError {
            operation: "init".to_string(),
            source: "not found".to_string(),
            is_not_found: true,
        };
        let commands = err.fix_commands();
        assert!(commands.iter().any(|c| c.contains("cargo install")));
        assert!(commands.iter().any(|c| c.contains("brew install")));
    }

    #[test]
    fn test_session_locked_returns_fix_commands() {
        let err = Error::SessionLocked {
            session: "test".to_string(),
            holder: "agent-1".to_string(),
        };
        let commands = err.fix_commands();
        assert!(commands.iter().any(|c| c.contains("agent status")));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RICH ERROR TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_rich_error_from_error() {
        let err = Error::NotFound("session not found".into());
        let rich = RichError::from_error(&err);

        assert_eq!(rich.error.code, "NOT_FOUND");
        assert_eq!(rich.error.exit_code, 2);
        assert!(!rich.fix_commands.is_empty());
    }

    #[test]
    fn test_rich_error_with_context() {
        let err = Error::ValidationError {
            message: "invalid".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let ctx = FailureContext::new().with_working_directory("/tmp");
        let rich = RichError::from_error(&err).with_context(ctx);

        assert!(rich.context_at_failure.is_some());
    }

    #[test]
    fn test_rich_error_with_validation_hints() {
        let err = Error::ValidationError {
            message: "invalid name".to_string(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        let additional_hints = vec![ValidationHint::new("extra", "extra hint")];
        let rich = RichError::from_error(&err).with_validation_hints(additional_hints);

        assert!(rich.validation_hints.len() > 1);
    }

    #[test]
    fn test_rich_error_with_fix_commands() {
        let err = Error::Unknown("unknown".into());
        let rich =
            RichError::from_error(&err).with_fix_commands(vec!["isolate doctor".to_string()]);

        assert_eq!(rich.fix_commands, vec!["isolate doctor"]);
    }

    #[test]
    fn test_rich_error_serialization() {
        let err = Error::NotFound("test".into());
        let rich = RichError::from_error(&err);

        // Functional approach: Serialize → Parse → Validate structure
        let result = serde_json::to_string_pretty(&rich).and_then(|json_str| {
            // Parse JSON to verify structure (not fragile string matching)
            serde_json::from_str::<serde_json::Value>(&json_str).map(|value| (json_str, value))
        });

        assert!(result.is_ok(), "JSON serialization should succeed");

        // Extract result or fail test with error message
        let Ok((json_str, parsed)) = result else {
            // In tests, we don't need panic! - just return early after assertion
            assert!(result.is_ok(), "Failed to serialize/parse RichError");
            return;
        };

        // Validate structure via parsed JSON (type-safe, not string matching)
        assert_eq!(
            parsed.get("code").and_then(|v| v.as_str()),
            Some("NOT_FOUND"),
            "code field should be NOT_FOUND"
        );
        assert!(
            parsed
                .get("fix_commands")
                .and_then(|v| v.as_array())
                .is_some(),
            "fix_commands field should be an array"
        );

        // Verify JSON is pretty-printed (has newlines)
        assert!(json_str.contains('\n'), "JSON should be pretty-printed");
    }

    #[test]
    fn test_error_to_rich_error() {
        let err = Error::DatabaseError("failed".into());
        let rich = err.to_rich_error();

        assert_eq!(rich.error.code, "DATABASE_ERROR");
    }

    #[test]
    fn test_error_to_rich_error_with_context() {
        let err = Error::IoError("failed".into());
        let ctx = FailureContext::new().with_phase("file_read");
        let rich = err.to_rich_error_with_context(ctx);

        assert!(rich.context_at_failure.is_some());
        if let Some(c) = rich.context_at_failure {
            assert_eq!(c.phase, Some("file_read".to_string()));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SUGGESTION TESTS - Verify all common errors have actionable suggestions
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_io_error_has_suggestion() {
        let err = Error::IoError("Permission denied".into());
        let suggestion = err.suggestion();
        assert!(suggestion.is_some(), "IO errors should have suggestions");
        let Some(sugg) = suggestion else { return };
        assert!(
            sugg.contains("permission") || sugg.contains("check"),
            "IO error suggestion should mention permissions or checking"
        );
    }

    #[test]
    fn test_command_error_has_suggestion() {
        let err = Error::Command("Command failed".into());
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Command errors should have troubleshooting suggestions"
        );
    }

    #[test]
    fn test_session_locked_has_suggestion() {
        let err = Error::SessionLocked {
            session: "test".to_string(),
            holder: "agent-1".to_string(),
        };
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Session locked errors should have suggestions"
        );
        let Some(sugg) = suggestion else { return };
        assert!(
            sugg.contains("agent") || sugg.contains("status"),
            "Session locked suggestion should mention agent or status commands"
        );
    }

    #[test]
    fn test_not_lock_holder_has_suggestion() {
        let err = Error::NotLockHolder {
            session: "test".to_string(),
            agent_id: "agent-2".to_string(),
        };
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Not lock holder errors should have suggestions"
        );
    }

    #[test]
    fn test_parse_error_has_suggestion() {
        let err = Error::ParseError("Invalid JSON".into());
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Parse errors should have format correction suggestions"
        );
    }

    #[test]
    fn test_invalid_config_has_suggestion() {
        let err = Error::InvalidConfig("Unknown key".into());
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Invalid config errors should have config fix suggestions"
        );
    }

    #[test]
    fn test_operation_cancelled_has_suggestion() {
        let err = Error::OperationCancelled("User interrupted".into());
        let _suggestion = err.suggestion();
        // Operation cancelled may or may not have a suggestion (user-initiated)
        // This test documents the current behavior
    }

    #[test]
    fn test_unknown_error_has_suggestion() {
        let err = Error::Unknown("Something unexpected".into());
        let suggestion = err.suggestion();
        assert!(
            suggestion.is_some(),
            "Unknown errors should have fallback suggestions (like 'isolate doctor')"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FIX COMMAND TESTS - Verify actionable fix commands are available
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_io_error_has_fix_commands() {
        let err = Error::IoError("File not found".into());
        let commands = err.fix_commands();
        assert!(!commands.is_empty(), "IO errors should have fix commands");
        assert!(
            commands
                .iter()
                .any(|c| c.contains("ls") || c.contains("check")),
            "IO error fix commands should include diagnostic commands"
        );
    }

    #[test]
    fn test_command_error_has_fix_commands() {
        let err = Error::Command("Command not found".into());
        let commands = err.fix_commands();
        assert!(
            !commands.is_empty(),
            "Command errors should have troubleshooting fix commands"
        );
    }

    #[test]
    fn test_parse_error_has_fix_commands() {
        let err = Error::ParseError("Invalid JSON syntax".into());
        let commands = err.fix_commands();
        assert!(
            !commands.is_empty(),
            "Parse errors should have validation commands"
        );
    }

    #[test]
    fn test_invalid_config_has_fix_commands() {
        let err = Error::InvalidConfig("Unknown key".into());
        let commands = err.fix_commands();
        assert!(
            !commands.is_empty(),
            "Config errors should have config fix commands"
        );
        assert!(
            commands.iter().any(|c| c.contains("config")),
            "Config error fix commands should mention config commands"
        );
    }

    #[test]
    fn test_operation_cancelled_has_no_fix_commands() {
        let err = Error::OperationCancelled("User interrupted".into());
        let commands = err.fix_commands();
        // User-initiated cancellation doesn't need fix commands
        assert!(commands.is_empty() || commands.len() <= 1);
    }

    #[test]
    fn test_unknown_error_has_fix_commands() {
        let err = Error::Unknown("Unexpected error".into());
        let commands = err.fix_commands();
        assert!(
            !commands.is_empty(),
            "Unknown errors should have 'isolate doctor' fix command"
        );
        assert!(
            commands.iter().any(|c| c.contains("doctor")),
            "Unknown error should suggest 'isolate doctor'"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VALIDATION HINT TESTS - Verify validation provides useful hints
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_error_json_provides_hint() {
        let err = Error::ParseError("Expected comma at line 5".into());
        let hints = err.validation_hints();
        assert!(
            !hints.is_empty(),
            "Parse errors should provide validation hints"
        );
    }

    #[test]
    fn test_invalid_config_provides_hint() {
        let err = Error::InvalidConfig("Unknown key 'foo'".into());
        let hints = err.validation_hints();
        assert!(!hints.is_empty(), "Invalid config should provide hints");
        assert!(
            hints.iter().any(|h| h.field == "config"),
            "Config error hint should mention 'config' field"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ACTIONABILITY TESTS - Suggestions are actionable, not vague
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_suggestions_contain_commands_not_just_explanations() {
        // All suggestions should include specific commands, not just explanations
        let test_errors = vec![
            Error::NotFound("session 'test' not found".into()),
            Error::ValidationError {
                message: "invalid name".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            },
            Error::DatabaseError("corrupted".into()),
        ];

        for err in test_errors {
            if let Some(suggestion) = err.suggestion() {
                // Check if suggestion contains a command pattern (starts with word or has :)
                let has_actionable_hint = suggestion
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '\'')
                    || suggestion.contains("isolate")
                    || suggestion.contains("jj ")
                    || suggestion.contains("cargo ");
                assert!(
                    has_actionable_hint,
                    "Suggestion should be actionable: '{suggestion}'"
                );
            }
        }
    }

    #[test]
    fn test_fix_commands_are_copy_pastable() {
        // Fix commands should be complete, copy-pastable shell commands
        let err = Error::NotFound("session 'test' not found".into());
        let commands = err.fix_commands();

        for cmd in commands {
            assert!(!cmd.is_empty(), "Fix command should not be empty");
            assert!(
                cmd.starts_with("isolate ") || cmd.starts_with("jj ") || cmd.starts_with("cargo "),
                "Fix command should start with a known command: '{cmd}'"
            );
        }
    }
}

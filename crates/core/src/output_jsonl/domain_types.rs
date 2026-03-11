//! Semantic domain types for output - following Scott Wlaschin's DDD principles
//!
//! This module implements:
//! - Parse at boundaries, validate once
//! - Use semantic newtypes instead of primitives
//! - Make illegal states unrepresentable

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

use super::OutputLineError;
// Re-export from domain (single source of truth)
pub use crate::domain::{BeadId, SessionName};

// ═══════════════════════════════════════════════════════════════════════════
// IDENTIFIER NEWTYPES - Parse at boundaries, validate once
// ═══════════════════════════════════════════════════════════════════════════

/// A validated issue identifier
///
/// # Invariants
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueId(String);

impl IssueId {
    /// Create a new issue ID, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if the ID is empty
    pub fn new(id: impl Into<String>) -> Result<Self, OutputLineError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self(id))
    }

    /// Get the issue ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for IssueId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEXT NEWTYPES - Validate once, use everywhere
// ═══════════════════════════════════════════════════════════════════════════

/// A validated issue title
///
/// # Invariants
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTitle(String);

impl IssueTitle {
    /// Create a new issue title, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if the title is empty
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    /// Get the issue title as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for IssueTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated plan title
///
/// # Invariants
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTitle(String);

impl PlanTitle {
    /// Create a new plan title, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if the title is empty
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    /// Get the plan title as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlanTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for PlanTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated plan description
///
/// # Invariants
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDescription(String);

impl PlanDescription {
    /// Create a new plan description, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyDescription` if the description is empty
    pub fn new(desc: impl Into<String>) -> Result<Self, OutputLineError> {
        let desc = desc.into();
        if desc.trim().is_empty() {
            return Err(OutputLineError::EmptyDescription);
        }
        Ok(Self(desc))
    }

    /// Get the plan description as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlanDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for PlanDescription {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated message content
///
/// # Invariants
/// - Must be non-empty after trimming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message(String);

impl Message {
    /// Create a new message, validating it's non-empty
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if the message is empty
    pub fn new(msg: impl Into<String>) -> Result<Self, OutputLineError> {
        let msg = msg.into();
        if msg.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self(msg))
    }

    /// Get the message as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Message {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Known warning codes in the system
///
/// These are predefined warning codes that have specific meanings.
/// Custom codes can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    /// Configuration file not found, using defaults
    ConfigNotFound,
    /// Invalid configuration value
    ConfigInvalid,
    /// Session limit reached
    SessionLimitReached,
    /// Workspace path not found
    WorkspaceNotFound,
    /// Git operation failed
    GitOperationFailed,
    /// Merge conflict detected
    MergeConflict,
    /// Agent not available
    AgentUnavailable,
    /// Custom warning code with string value
    #[serde(untagged)]
    Custom(String),
}

impl WarningCode {
    /// Create a warning code from known codes or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidWarningCode` if custom code doesn't
    /// follow the pattern: letter followed by alphanumeric (e.g., "W001", "E123")
    pub fn new(code: impl Into<String>) -> Result<Self, OutputLineError> {
        let code = code.into();

        // Match against known codes
        match code.as_str() {
            "CONFIG_NOT_FOUND" => Ok(Self::ConfigNotFound),
            "CONFIG_INVALID" => Ok(Self::ConfigInvalid),
            "SESSION_LIMIT_REACHED" => Ok(Self::SessionLimitReached),
            "WORKSPACE_NOT_FOUND" => Ok(Self::WorkspaceNotFound),
            "GIT_OPERATION_FAILED" => Ok(Self::GitOperationFailed),
            "MERGE_CONFLICT" => Ok(Self::MergeConflict),
            "AGENT_UNAVAILABLE" => Ok(Self::AgentUnavailable),
            custom => {
                // Validate custom code format: letter followed by alphanumeric
                if custom.is_empty() {
                    return Err(OutputLineError::InvalidWarningCode(
                        "warning code cannot be empty".to_string(),
                    ));
                }

                // Must start with a letter
                if !custom
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                {
                    return Err(OutputLineError::InvalidWarningCode(format!(
                        "warning code must start with a letter, got: {custom}"
                    )));
                }

                // All characters must be alphanumeric or underscore
                if !custom
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    return Err(OutputLineError::InvalidWarningCode(format!(
                        "warning code must be alphanumeric or underscore, got: {custom}"
                    )));
                }

                Ok(Self::Custom(custom.to_string()))
            }
        }
    }

    /// Get the warning code as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::SessionLimitReached => "SESSION_LIMIT_REACHED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::GitOperationFailed => "GIT_OPERATION_FAILED",
            Self::MergeConflict => "MERGE_CONFLICT",
            Self::AgentUnavailable => "AGENT_UNAVAILABLE",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Check if this is a custom warning code
    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for WarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for WarningCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Known action verbs in the system
///
/// These are predefined action verbs that represent operations.
/// Custom verbs can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionVerb {
    /// Run a command or operation
    Run,
    /// Execute a task
    Execute,
    /// Create a new resource
    Create,
    /// Delete a resource
    Delete,
    /// Update a resource
    Update,
    /// Merge resources
    Merge,
    /// Rebase changes
    Rebase,
    /// Sync with remote
    Sync,
    /// Fix an issue
    Fix,
    /// Check status
    Check,
    /// Focus on a target
    Focus,
    /// Attach to a session
    Attach,
    /// Switch tabs
    SwitchTab,
    /// Remove a resource
    Remove,
    /// Discover resources
    Discover,
    /// Would fix (dry run)
    WouldFix,
    /// Custom action verb with string value
    #[serde(untagged)]
    Custom(String),
}

impl ActionVerb {
    /// Create an action verb from known verbs or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidActionVerb` if custom verb doesn't
    /// follow the pattern: lowercase alphanumeric with hyphens (e.g., "run", "switch-tab")
    pub fn new(verb: impl Into<String>) -> Result<Self, OutputLineError> {
        let verb = verb.into();

        // Match against known verbs (case-insensitive)
        match verb.to_lowercase().as_str() {
            "run" => Ok(Self::Run),
            "execute" => Ok(Self::Execute),
            "create" => Ok(Self::Create),
            "delete" => Ok(Self::Delete),
            "update" => Ok(Self::Update),
            "merge" => Ok(Self::Merge),
            "rebase" => Ok(Self::Rebase),
            "sync" => Ok(Self::Sync),
            "fix" => Ok(Self::Fix),
            "check" => Ok(Self::Check),
            "focus" => Ok(Self::Focus),
            "attach" => Ok(Self::Attach),
            "switch-tab" => Ok(Self::SwitchTab),
            "remove" => Ok(Self::Remove),
            "discovered" => Ok(Self::Discover),
            "would_fix" => Ok(Self::WouldFix),
            custom => {
                // Validate custom verb format
                if custom.trim().is_empty() {
                    return Err(OutputLineError::InvalidActionVerb(
                        "action verb cannot be empty".to_string(),
                    ));
                }

                // Must be lowercase alphanumeric with hyphens
                let lower = custom.to_lowercase();
                if lower != custom {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must be lowercase, got: {custom}"
                    )));
                }

                if !lower
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must be lowercase alphanumeric with hyphens, got: {custom}"
                    )));
                }

                // Must start with a letter
                if !lower.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
                    return Err(OutputLineError::InvalidActionVerb(format!(
                        "action verb must start with a lowercase letter, got: {custom}"
                    )));
                }

                Ok(Self::Custom(lower))
            }
        }
    }

    /// Get the action verb as a string slice
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Run => "run",
            Self::Execute => "execute",
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Update => "update",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::Sync => "sync",
            Self::Fix => "fix",
            Self::Check => "check",
            Self::Focus => "focus",
            Self::Attach => "attach",
            Self::SwitchTab => "switch-tab",
            Self::Remove => "remove",
            Self::Discover => "discovered",
            Self::WouldFix => "would_fix",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Check if this is a custom action verb
    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for ActionVerb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ActionVerb {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A validated action target
///
/// # Invariants
/// - Must be non-empty after trimming
/// - Maximum length of 1000 characters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTarget(String);

impl ActionTarget {
    /// Maximum length for action target
    pub const MAX_LENGTH: usize = 1000;

    /// Create a new action target, validating format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if target is empty
    /// Returns `OutputLineError::InvalidActionTarget` if target exceeds max length
    pub fn new(target: impl Into<String>) -> Result<Self, OutputLineError> {
        let target = target.into();

        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(OutputLineError::InvalidActionTarget(format!(
                "action target exceeds maximum length of {} characters",
                Self::MAX_LENGTH
            )));
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Get the action target as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ActionTarget {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated base reference (git branch name)
///
/// Base refs are less constrained - they can be any string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRef(String);

impl BaseRef {
    /// Create a new base reference (no validation required)
    #[must_use]
    pub fn new(base_ref: impl Into<String>) -> Self {
        Self(base_ref.into())
    }

    /// Get the base reference as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BaseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for BaseRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated command string
///
/// Commands are less constrained - they can be any string (including empty for manual steps)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command(String);

impl Command {
    /// Create a new command (no validation required)
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self(command.into())
    }

    /// Get the command as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the command is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS THAT REPLACE BOOLEAN FLAGS - Make illegal states unrepresentable
// ═══════════════════════════════════════════════════════════════════════════

/// Recovery capability - replaces `recoverable: bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCapability {
    /// The issue can be recovered with a recommended action
    Recoverable { recommended_action: String },
    /// The issue cannot be recovered (requires manual intervention)
    NotRecoverable { reason: String },
}

/// Execution mode - replaces `automatic: bool`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Step executes automatically
    Automatic,
    /// Step requires manual execution
    Manual,
}

/// Merge status - replaces `merge_safe: bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeAnalysis {
    /// Whether the merge is safe
    pub safe: bool,
    /// Conflicts if not safe (empty if safe)
    pub conflicts: Vec<super::ConflictDetail>,
}

/// Outcome - replaces `success: bool`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
}

impl Outcome {
    /// Convert from boolean (for backward compatibility during migration)
    #[must_use]
    pub const fn from_bool(success: bool) -> Self {
        if success {
            Self::Success
        } else {
            Self::Failure
        }
    }

    /// Convert to boolean (for backward compatibility during migration)
    #[must_use]
    pub const fn to_bool(self) -> bool {
        match self {
            Self::Success => true,
            Self::Failure => false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS THAT REPLACE OPTION FIELDS - Explicit state representation
// ═══════════════════════════════════════════════════════════════════════════

/// Issue scope - replaces `session: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueScope {
    /// Issue is not associated with a session
    Standalone,
    /// Issue is associated with a specific session
    InSession { session: SessionName },
}

impl IssueScope {
    /// Get the session name if this is an `InSession` scope
    #[must_use]
    pub const fn session(&self) -> Option<&SessionName> {
        match self {
            Self::Standalone => None,
            Self::InSession { session } => Some(session),
        }
    }

    /// Create a standalone scope
    #[must_use]
    pub const fn standalone() -> Self {
        Self::Standalone
    }

    /// Create an `InSession` scope
    #[must_use]
    pub const fn in_session(session: SessionName) -> Self {
        Self::InSession { session }
    }
}

/// Action result - replaces `result: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult {
    /// Action is still pending
    Pending,
    /// Action completed with a result
    Completed { result: String },
}

impl ActionResult {
    /// Get the result if completed
    #[must_use]
    pub const fn result(&self) -> Option<&str> {
        match self {
            Self::Pending => None,
            Self::Completed { result } => Some(result.as_str()),
        }
    }

    /// Create a pending result
    #[must_use]
    pub const fn pending() -> Self {
        Self::Pending
    }

    /// Create a completed result
    #[must_use]
    pub fn completed(result: impl Into<String>) -> Self {
        Self::Completed {
            result: result.into(),
        }
    }
}

/// Recovery execution - replaces `command: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryExecution {
    /// Automatic execution with a command
    Automatic { command: Command },
    /// Manual execution required
    Manual,
}

impl RecoveryExecution {
    /// Get the command if this is automatic execution
    #[must_use]
    pub const fn command(&self) -> Option<&Command> {
        match self {
            Self::Automatic { command } => Some(command),
            Self::Manual => None,
        }
    }

    /// Create an automatic execution
    #[must_use]
    pub fn automatic(command: impl Into<String>) -> Self {
        Self::Automatic {
            command: Command::new(command),
        }
    }

    /// Create a manual execution
    #[must_use]
    pub const fn manual() -> Self {
        Self::Manual
    }

    /// Check if this is automatic execution
    #[must_use]
    pub const fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic { .. })
    }
}

/// Bead attachment - replaces `bead: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadAttachment {
    /// No bead attached
    None,
    /// Bead attached by ID
    Attached { bead_id: BeadId },
}

impl BeadAttachment {
    /// Get the bead ID if attached
    #[must_use]
    pub const fn bead_id(&self) -> Option<&BeadId> {
        match self {
            Self::None => None,
            Self::Attached { bead_id } => Some(bead_id),
        }
    }

    /// Create no attachment
    #[must_use]
    pub const fn none() -> Self {
        Self::None
    }

    /// Create an attachment
    #[must_use]
    pub const fn attached(bead_id: BeadId) -> Self {
        Self::Attached { bead_id }
    }
}

/// Agent assignment - replaces `agent: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentAssignment {
    /// No agent assigned
    Unassigned,
    /// Agent assigned by ID
    Assigned { agent_id: String },
}

impl AgentAssignment {
    /// Get the agent ID if assigned
    #[must_use]
    pub const fn agent_id(&self) -> Option<&str> {
        match self {
            Self::Unassigned => None,
            Self::Assigned { agent_id } => Some(agent_id.as_str()),
        }
    }

    /// Create unassigned state
    #[must_use]
    pub const fn unassigned() -> Self {
        Self::Unassigned
    }

    /// Create an assigned state
    #[must_use]
    pub fn assigned(agent_id: impl Into<String>) -> Self {
        Self::Assigned {
            agent_id: agent_id.into(),
        }
    }
}

/// Validated metadata for extensibility
///
/// Wraps `serde_json::Value` to ensure metadata is valid JSON.
/// Unlike raw `serde_json::Value`, this provides:
/// - Type-level distinction from arbitrary JSON
/// - Clear intent for metadata usage
/// - Future extensibility for validation rules
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatedMetadata(serde_json::Value);

impl ValidatedMetadata {
    /// Create new validated metadata from a JSON value
    ///
    /// Always succeeds since any `serde_json::Value` is valid.
    #[must_use]
    pub const fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Create empty metadata (null value)
    #[must_use]
    pub const fn empty() -> Self {
        Self(serde_json::Value::Null)
    }

    /// Create metadata from an object
    #[must_use]
    pub const fn from_object(obj: serde_json::Map<String, serde_json::Value>) -> Self {
        Self(serde_json::Value::Object(obj))
    }

    /// Get the underlying JSON value
    #[must_use]
    pub const fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    /// Check if metadata is empty (null)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.0, serde_json::Value::Null)
    }

    /// Get a field from the metadata
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Convert into the underlying JSON value
    #[must_use]
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
}

impl Default for ValidatedMetadata {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<serde_json::Value> for ValidatedMetadata {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<ValidatedMetadata> for serde_json::Value {
    fn from(metadata: ValidatedMetadata) -> Self {
        metadata.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // IDENTIFIER NEWTYPE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_issue_id_valid() {
        let id = IssueId::new("test-id").expect("valid id");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn test_issue_id_empty() {
        let result = IssueId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_issue_id_whitespace_only() {
        let result = IssueId::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_bead_id_valid() {
        let id = BeadId::parse("bd-abc123").expect("valid id");
        assert_eq!(id.as_str(), "bd-abc123");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TEXT NEWTYPE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_issue_title_valid() {
        let title = IssueTitle::new("Fix authentication bug").expect("valid title");
        assert_eq!(title.as_str(), "Fix authentication bug");
    }

    #[test]
    fn test_issue_title_empty() {
        let result = IssueTitle::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_title_valid() {
        let title = PlanTitle::new("Migration Plan").expect("valid title");
        assert_eq!(title.as_str(), "Migration Plan");
    }

    #[test]
    fn test_plan_description_valid() {
        let desc = PlanDescription::new("Step by step migration").expect("valid description");
        assert_eq!(desc.as_str(), "Step by step migration");
    }

    #[test]
    fn test_message_valid() {
        let msg = Message::new("Operation completed").expect("valid message");
        assert_eq!(msg.as_str(), "Operation completed");
    }

    #[test]
    fn test_message_empty() {
        let result = Message::new("");
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ENUM STATE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_recovery_capability_recoverable() {
        let capability = RecoveryCapability::Recoverable {
            recommended_action: "Run jj resolve".to_string(),
        };
        assert!(matches!(capability, RecoveryCapability::Recoverable { .. }));
    }

    #[test]
    fn test_recovery_capability_not_recoverable() {
        let capability = RecoveryCapability::NotRecoverable {
            reason: "Manual merge required".to_string(),
        };
        assert!(matches!(
            capability,
            RecoveryCapability::NotRecoverable { .. }
        ));
    }

    #[test]
    fn test_execution_mode_automatic() {
        let mode = ExecutionMode::Automatic;
        assert!(matches!(mode, ExecutionMode::Automatic));
    }

    #[test]
    fn test_outcome_from_bool() {
        assert!(matches!(Outcome::from_bool(true), Outcome::Success));
        assert!(matches!(Outcome::from_bool(false), Outcome::Failure));
    }

    #[test]
    fn test_issue_scope_standalone() {
        let scope = IssueScope::Standalone;
        assert!(scope.session().is_none());
    }

    #[test]
    fn test_issue_scope_in_session() {
        let session_name = SessionName::parse("test-session").expect("valid name");
        let scope = IssueScope::InSession {
            session: session_name.clone(),
        };
        assert!(scope.session().is_some());
        assert_eq!(scope.session(), Some(&session_name));
    }

    #[test]
    fn test_action_result_pending() {
        let result = ActionResult::Pending;
        assert!(result.result().is_none());
    }

    #[test]
    fn test_action_result_completed() {
        let result = ActionResult::completed("Success");
        assert_eq!(result.result(), Some("Success"));
    }

    #[test]
    fn test_recovery_execution_automatic() {
        let execution = RecoveryExecution::automatic("jj resolve");
        assert!(execution.is_automatic());
        assert!(execution.command().is_some());
    }

    #[test]
    fn test_recovery_execution_manual() {
        let execution = RecoveryExecution::manual();
        assert!(!execution.is_automatic());
        assert!(execution.command().is_none());
    }

    #[test]
    fn test_bead_attachment_none() {
        let attachment = BeadAttachment::None;
        assert!(attachment.bead_id().is_none());
    }

    #[test]
    fn test_bead_attachment_attached() {
        // BeadId uses the same format as TaskId: "bd-{hex}"
        let bead_id = BeadId::parse("bd-abc123").expect("valid id");
        let attachment = BeadAttachment::Attached {
            bead_id: bead_id.clone(),
        };
        assert_eq!(attachment.bead_id(), Some(&bead_id));
    }

    #[test]
    fn test_agent_assignment_unassigned() {
        let assignment = AgentAssignment::Unassigned;
        assert!(assignment.agent_id().is_none());
    }

    #[test]
    fn test_agent_assignment_assigned() {
        let assignment = AgentAssignment::assigned("agent-1");
        assert_eq!(assignment.agent_id(), Some("agent-1"));
    }

    #[test]
    fn test_validated_metadata_empty() {
        let metadata = ValidatedMetadata::empty();
        assert!(metadata.is_empty());
        assert!(metadata.as_value().is_null());
    }

    #[test]
    fn test_validated_metadata_from_value() {
        let value = serde_json::json!({"key": "value"});
        let metadata = ValidatedMetadata::new(value.clone());
        assert!(!metadata.is_empty());
        assert_eq!(metadata.as_value(), &value);
    }

    #[test]
    fn test_validated_metadata_get_field() {
        let value = serde_json::json!({"key": "value", "number": 42});
        let metadata = ValidatedMetadata::new(value);
        assert_eq!(metadata.get("key"), Some(&serde_json::json!("value")));
        assert_eq!(metadata.get("number"), Some(&serde_json::json!(42)));
        assert_eq!(metadata.get("missing"), None);
    }

    #[test]
    fn test_validated_metadata_default() {
        let metadata = ValidatedMetadata::default();
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_validated_metadata_from_json_value() {
        let json_value: serde_json::Value = serde_json::json!({"test": true});
        let metadata: ValidatedMetadata = json_value.clone().into();
        assert_eq!(metadata.as_value(), &json_value);
    }

    #[test]
    fn test_validated_metadata_into_json_value() {
        let json_value = serde_json::json!({"test": "value"});
        let metadata = ValidatedMetadata::new(json_value.clone());
        let converted: serde_json::Value = metadata.into();
        assert_eq!(converted, json_value);
    }
}

//! Semantic newtypes for domain identifiers
//!
//! # Parse-at-Boundaries Pattern
//!
//! Each identifier type:
//! - Validates its input on construction (parse-once pattern)
//! - Trims whitespace before validation (boundary sanitization)
//! - Cannot represent invalid states
//! - Provides safe access to the underlying value
//! - Implements serde serialization/deserialization with validation
//!
//! # Single Source of Truth
//!
//! This module is the canonical implementation of identifier types.
//! Other modules (`types.rs`, `cli_contracts`) should re-export these types
//! rather than defining their own implementations.
//!
//! # Unified Error Type
//!
//! All identifier validation uses a single `IdentifierError` enum with clear
//! categorization:
//! - **`Empty`**: Identifier is empty or whitespace-only
//! - **`TooLong`**: Exceeds type-specific maximum length
//! - **`InvalidCharacters`**: Contains characters not allowed for the type
//! - **`InvalidFormat`**: Generic format validation error
//! - **`InvalidStart`**: Does not start with required character
//! - **`InvalidPrefix`**: Missing required prefix (e.g., "bd-" for task IDs)
//! - **`InvalidHex`**: Invalid hexadecimal format
//! - **`NotAbsolutePath`**: Path is not absolute
//! - **`NullBytesInPath`**: Path contains null bytes
//! - **`NotAscii`**: Identifier must be ASCII-only
//! - **`ContainsPathSeparators`**: Identifier contains path separators
//!
//! This follows DDD principle of clear error taxonomy for expected domain failures.
//!
//! # Module-Specific Error Aliases
//!
//! For backward compatibility and semantic clarity, each identifier type has
//! a corresponding error alias:
//! - `SessionNameError` = `IdentifierError`
//! - `AgentIdError` = `IdentifierError`
//! - `WorkspaceNameError` = `IdentifierError`
//! - `TaskIdError` = `IdentifierError`
//! - `BeadIdError` = `IdentifierError`
//! - `SessionIdError` = `IdentifierError`
//! - `AbsolutePathError` = `IdentifierError`
//!
//! The legacy `IdError` alias is also maintained for backward compatibility.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// UNIFIED IDENTIFIER ERROR
// ============================================================================

/// Unified error type for all identifier validation.
///
/// This follows DDD principles: clear error taxonomy for expected domain failures.
/// All identifier types use this single error type, making error handling consistent
/// across the domain layer.
///
/// # Error Categories
///
/// 1. **`Empty`**: Identifier is empty or whitespace-only
/// 2. **`TooLong`**: Exceeds maximum length for identifier type
/// 3. **`InvalidCharacters`**: Contains characters not allowed for identifier type
/// 4. **`InvalidFormat`**: Does not match required format/pattern
/// 5. **`InvalidPrefix`**: Missing or wrong prefix (e.g., "bd-" for task IDs)
/// 6. **`NotAbsolutePath`**: Path is not absolute
///
/// # Module-Specific Aliases
///
/// Each module can provide type aliases for backward compatibility:
/// ```rust,ignore
/// use crate::domain::identifiers::IdentifierError;
/// type SessionNameError = IdentifierError;
/// type AgentIdError = IdentifierError;
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    /// Identifier is empty or contains only whitespace
    #[error("identifier cannot be empty")]
    Empty,

    /// Identifier exceeds maximum length
    #[error("identifier too long: {actual} characters (max {max})")]
    TooLong {
        /// The maximum allowed length
        max: usize,
        /// The actual length provided
        actual: usize,
    },

    /// Identifier contains invalid characters
    #[error("identifier contains invalid characters: {details}")]
    InvalidCharacters {
        /// Human-readable explanation of what's invalid
        details: String,
    },

    /// Identifier format is invalid (generic format error)
    #[error("invalid identifier format: {details}")]
    InvalidFormat {
        /// Human-readable explanation of format requirements
        details: String,
    },

    /// Identifier must start with a letter (alphabetic character)
    #[error("identifier must start with a letter")]
    InvalidStart {
        /// The expected starting character/pattern (for context)
        expected: char,
    },

    /// Identifier has invalid prefix (e.g., must start with "bd-")
    #[error("identifier must have prefix '{prefix}' (got: {value})")]
    InvalidPrefix {
        /// The required prefix (e.g., "bd-")
        prefix: &'static str,
        /// The actual value that was provided
        value: String,
    },

    /// Identifier hex format is invalid
    #[error("identifier has invalid hex format: {value}")]
    InvalidHex {
        /// The value that failed hex validation
        value: String,
    },

    /// Path is not absolute
    #[error("path is not absolute: {value}")]
    NotAbsolutePath {
        /// The path that was provided
        value: String,
    },

    /// Path contains null bytes
    #[error("path contains null bytes")]
    NullBytesInPath,

    /// Identifier must be ASCII
    #[error("identifier must be ASCII only: {value}")]
    NotAscii {
        /// The value that failed ASCII validation
        value: String,
    },

    /// Identifier contains path separators
    #[error("identifier cannot contain path separators")]
    ContainsPathSeparators,
}

// ============================================================================
// BACKWARD COMPATIBILITY ALIASES
// ============================================================================

/// Legacy alias for backward compatibility.
///
/// # Deprecated
/// Use `IdentifierError` instead.
pub type IdError = IdentifierError;

/// Error type for session name validation.
pub type SessionNameError = IdentifierError;

/// Error type for agent ID validation.
pub type AgentIdError = IdentifierError;

/// Error type for workspace name validation.
pub type WorkspaceNameError = IdentifierError;

/// Error type for task ID validation.
pub type TaskIdError = IdentifierError;

/// Error type for bead ID validation.
pub type BeadIdError = IdentifierError;

/// Error type for session ID validation.
pub type SessionIdError = IdentifierError;

/// Error type for absolute path validation.
pub type AbsolutePathError = IdentifierError;

// ============================================================================
// HELPER METHODS
// ============================================================================

impl IdentifierError {
    /// Create an `Empty` error variant
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Create a `TooLong` error variant
    #[must_use]
    pub const fn too_long(max: usize, actual: usize) -> Self {
        Self::TooLong { max, actual }
    }

    /// Create an `InvalidCharacters` error variant
    #[must_use]
    pub fn invalid_characters(details: impl Into<String>) -> Self {
        Self::InvalidCharacters {
            details: details.into(),
        }
    }

    /// Create an `InvalidFormat` error variant
    #[must_use]
    pub fn invalid_format(details: impl Into<String>) -> Self {
        Self::InvalidFormat {
            details: details.into(),
        }
    }

    /// Create an `InvalidStart` error variant
    #[must_use]
    pub const fn invalid_start(expected: char) -> Self {
        Self::InvalidStart { expected }
    }

    /// Create an `InvalidPrefix` error variant
    #[must_use]
    pub fn invalid_prefix(prefix: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidPrefix {
            prefix,
            value: value.into(),
        }
    }

    /// Create an `InvalidHex` error variant
    #[must_use]
    pub fn invalid_hex(value: impl Into<String>) -> Self {
        Self::InvalidHex {
            value: value.into(),
        }
    }

    /// Create a `NotAbsolutePath` error variant
    #[must_use]
    pub fn not_absolute_path(value: impl Into<String>) -> Self {
        Self::NotAbsolutePath {
            value: value.into(),
        }
    }

    /// Check if this is an `Empty` error
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Check if this is a `TooLong` error
    #[must_use]
    pub const fn is_too_long(&self) -> bool {
        matches!(self, Self::TooLong { .. })
    }

    /// Check if this is an `InvalidCharacters` error
    #[must_use]
    pub const fn is_invalid_characters(&self) -> bool {
        matches!(self, Self::InvalidCharacters { .. })
    }

    /// Check if this is an `InvalidFormat` error
    #[must_use]
    pub const fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat { .. })
    }
}

/// Validate a session name according to naming rules
///
/// Rules:
/// - Must start with a letter
/// - Can contain letters, numbers, hyphens, underscores
/// - Must be 1-63 characters
fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    if s.len() > 63 {
        return Err(IdentifierError::too_long(63, s.len()));
    }

    if !s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(IdentifierError::invalid_start(
            'a', // Represents "must start with a letter"
        ));
    }

    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IdentifierError::invalid_characters(format!(
            "session name '{s}' must contain only letters, numbers, hyphens, or underscores"
        )));
    }

    Ok(())
}

/// Validate an agent ID
///
/// Rules:
/// - Must be 1-128 characters
/// - Can contain alphanumeric, hyphen, underscore, dot, colon
fn validate_agent_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    if s.len() > 128 {
        return Err(IdentifierError::too_long(128, s.len()));
    }

    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
    {
        return Err(IdentifierError::invalid_characters(
            format!("agent ID '{s}' must contain only letters, numbers, hyphens, underscores, dots, or colons"),
        ));
    }

    Ok(())
}

/// Validate a workspace name
///
/// Rules:
/// - Must be 1-255 characters
/// - Cannot contain path separators or null bytes
fn validate_workspace_name(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    if s.len() > 255 {
        return Err(IdentifierError::too_long(255, s.len()));
    }

    if s.contains('/') || s.contains('\\') || s.contains('\0') {
        return Err(IdentifierError::ContainsPathSeparators);
    }

    Ok(())
}

/// Validate a task ID (bead ID format)
///
/// Rules:
/// - Must match pattern: bd-{hex}
/// - Example: bd-abc123def456
fn validate_task_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    if !s.starts_with("bd-") {
        return Err(IdentifierError::invalid_prefix("bd-", s));
    }

    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentifierError::invalid_hex(s));
    }

    Ok(())
}

/// Validate a bead ID (same as task ID)
#[allow(dead_code)]
fn validate_bead_id(s: &str) -> Result<(), IdentifierError> {
    validate_task_id(s)
}

/// Validate a session ID
///
/// Rules:
/// - Must be non-empty
/// - Must be valid UTF-8
/// - Can contain any printable characters (more lenient than names)
fn validate_session_id(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    if !s.is_ascii() {
        return Err(IdentifierError::NotAscii {
            value: s.to_string(),
        });
    }

    Ok(())
}

/// Validate an absolute path
///
/// Rules:
/// - Must be absolute (starts with / on Unix, C:\ or similar on Windows)
/// - Must not contain null bytes
fn validate_absolute_path(s: &str) -> Result<(), IdentifierError> {
    if s.is_empty() {
        return Err(IdentifierError::invalid_format("path cannot be empty"));
    }

    if s.contains('\0') {
        return Err(IdentifierError::NullBytesInPath);
    }

    #[cfg(unix)]
    {
        if !s.starts_with('/') {
            return Err(IdentifierError::not_absolute_path(s));
        }
    }

    #[cfg(windows)]
    {
        if !s.starts_with(r#"\"#) && !(s.len() > 2 && s.as_bytes()[1] == b':') {
            return Err(IdentifierError::not_absolute_path(s));
        }
    }

    Ok(())
}

/// A validated session name
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::SessionName;
///
/// // Parse and validate
/// let name = SessionName::parse("my-session")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with a letter
/// - Contains only alphanumeric, hyphen, underscore
/// - 1-63 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SessionName(String);

impl SessionName {
    /// Maximum allowed length for a session name
    pub const MAX_LENGTH: usize = 63;

    /// Parse and validate a session name (trims whitespace first)
    ///
    /// This follows the "parse at boundaries" DDD principle:
    /// - Trims whitespace from input
    /// - Validates once at construction
    /// - Cannot represent invalid states
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the name is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let trimmed = s.trim();
        validate_session_name(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    /// Get the session name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for SessionName {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SessionName {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for SessionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<SessionName> for String {
    #[allow(clippy::use_self)] // Self refers to String, not SessionName
    fn from(name: SessionName) -> String {
        name.0
    }
}

/// A validated agent ID
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::AgentId;
///
/// let agent = AgentId::parse("agent-123")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Contains only alphanumeric, hyphen, underscore, dot, colon
/// - 1-128 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct AgentId(String);

impl AgentId {
    /// Parse and validate an agent ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_agent_id(&s)?;
        Ok(Self(s))
    }

    /// Get the agent ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Generate a default agent ID from process ID
    #[must_use]
    pub fn from_process() -> Self {
        Self(format!("pid-{}", std::process::id()))
    }
}

impl TryFrom<String> for AgentId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for AgentId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated workspace name
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::WorkspaceName;
///
/// let workspace = WorkspaceName::parse("my-workspace")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - No path separators or null bytes
/// - 1-255 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct WorkspaceName(String);

impl WorkspaceName {
    /// Parse and validate a workspace name
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the name is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_workspace_name(&s)?;
        Ok(Self(s))
    }

    /// Get the workspace name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for WorkspaceName {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for WorkspaceName {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for WorkspaceName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated task ID (bead ID format)
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::TaskId;
///
/// let task = TaskId::parse("bd-abc123def456")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - Starts with "bd-" prefix
/// - Followed by hexadecimal characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TaskId(String);

impl TaskId {
    /// Parse and validate a task ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_task_id(&s)?;
        Ok(Self(s))
    }

    /// Get the task ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for TaskId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for TaskId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated bead ID (same as task ID)
///
/// Alias for `TaskId` since beads and tasks use the same ID format.
pub type BeadId = TaskId;

/// A validated session ID
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::SessionId;
///
/// let id = SessionId::parse("session-abc123")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Non-empty
/// - ASCII only
/// - Suitable for use as unique identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SessionId(String);

impl SessionId {
    /// Parse and validate a session ID
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the ID is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_session_id(&s)?;
        Ok(Self(s))
    }

    /// Get the session ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for SessionId {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for SessionId {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A validated absolute path
///
/// # Construction
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::domain::AbsolutePath;
///
/// let path = AbsolutePath::parse("/home/user/workspace")?;
/// # Ok(())
/// # }
/// ```
///
/// # Guarantees
///
/// - Always absolute (starts with `/` on Unix)
/// - No null bytes
/// - Suitable for filesystem operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct AbsolutePath(String);

impl AbsolutePath {
    /// Parse and validate an absolute path
    ///
    /// # Errors
    ///
    /// Returns `IdentifierError` if the path is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        validate_absolute_path(&s)?;
        Ok(Self(s))
    }

    /// Get the path as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into an owned String
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Convert to `std::path::PathBuf`
    #[must_use]
    pub fn to_path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.0)
    }

    /// Display the path (for error messages)
    #[must_use]
    pub fn display(&self) -> impl std::fmt::Display + '_ {
        struct DisplayPath<'a>(&'a str);

        impl std::fmt::Display for DisplayPath<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        DisplayPath(&self.0)
    }

    /// Check if the path exists on the filesystem
    #[must_use]
    pub fn exists(&self) -> bool {
        self.to_path_buf().exists()
    }
}

impl TryFrom<String> for AbsolutePath {
    type Error = IdentifierError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for AbsolutePath {
    type Error = IdentifierError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl std::fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AbsolutePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ===== SessionName Tests =====

    #[test]
    fn test_valid_session_name() {
        assert!(SessionName::parse("my-session").is_ok());
        assert!(SessionName::parse("my_session").is_ok());
        assert!(SessionName::parse("my-session-123").is_ok());
    }

    #[test]
    fn test_session_name_trims_whitespace() {
        // Trim-then-validate: whitespace is trimmed, then validated
        let name = SessionName::parse("  my-session  ").expect("valid");
        assert_eq!(name.as_str(), "my-session");

        let name2 = SessionName::parse("\tmy-session\t").expect("valid");
        assert_eq!(name2.as_str(), "my-session");

        let name3 = SessionName::parse("\nmy-session\n").expect("valid");
        assert_eq!(name3.as_str(), "my-session");
    }

    #[test]
    fn test_session_name_whitespace_only_is_invalid() {
        // Whitespace-only strings become empty after trimming
        let result = SessionName::parse("   ");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_invalid_session_name_empty() {
        let result = SessionName::parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_invalid_session_name_starts_with_number() {
        let result = SessionName::parse("123-session");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_session_name_special_chars() {
        assert!(SessionName::parse("my.session").is_err());
        assert!(SessionName::parse("my:session").is_err());
        assert!(SessionName::parse("my session").is_err());
    }

    #[test]
    fn test_invalid_session_name_too_long() {
        let long_name = "a".repeat(64);
        let result = SessionName::parse(&long_name);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(IdentifierError::TooLong { max: 63, .. })
        ));
    }

    #[test]
    fn test_session_name_display() {
        match SessionName::parse("test-session") {
            Ok(name) => {
                assert_eq!(name.to_string(), "test-session");
                assert_eq!(name.as_str(), "test-session");
            }
            Err(e) => panic!("Failed to parse valid session name: {e}"),
        }
    }

    // ===== AgentId Tests =====

    #[test]
    fn test_valid_agent_id() {
        assert!(AgentId::parse("agent-123").is_ok());
        assert!(AgentId::parse("agent_456").is_ok());
        assert!(AgentId::parse("agent:789").is_ok());
        assert!(AgentId::parse("agent.example").is_ok());
    }

    #[test]
    fn test_invalid_agent_id_empty() {
        let result = AgentId::parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_invalid_agent_id_special_chars() {
        assert!(AgentId::parse("agent/123").is_err());
        assert!(AgentId::parse("agent 123").is_err());
    }

    #[test]
    fn test_agent_id_from_process() {
        let agent = AgentId::from_process();
        let agent_str = agent.as_str();
        assert!(agent_str.starts_with("pid-"));
    }

    // ===== WorkspaceName Tests =====

    #[test]
    fn test_valid_workspace_name() {
        assert!(WorkspaceName::parse("my-workspace").is_ok());
        assert!(WorkspaceName::parse("my_workspace").is_ok());
    }

    #[test]
    fn test_invalid_workspace_name_with_path_separator() {
        assert!(WorkspaceName::parse("my/workspace").is_err());
        assert!(WorkspaceName::parse("my\\workspace").is_err());
    }

    #[test]
    fn test_invalid_workspace_name_with_null() {
        assert!(WorkspaceName::parse("my\0workspace").is_err());
    }

    #[test]
    fn test_invalid_workspace_name_too_long() {
        let long_name = "a".repeat(256);
        let result = WorkspaceName::parse(&long_name);
        assert!(result.is_err());
    }

    // ===== TaskId Tests =====

    #[test]
    fn test_valid_task_id() {
        assert!(TaskId::parse("bd-abc123").is_ok());
        assert!(TaskId::parse("bd-ABC123DEF456").is_ok());
        assert!(TaskId::parse("bd-1234567890abcdef").is_ok());
    }

    #[test]
    fn test_invalid_task_id_no_prefix() {
        let result = TaskId::parse("abc123");
        assert!(result.is_err());
        assert!(matches!(result, Err(IdentifierError::InvalidPrefix { .. })));
    }

    #[test]
    fn test_invalid_task_id_empty() {
        let result = TaskId::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_task_id_no_hex() {
        assert!(TaskId::parse("bd-xyz").is_err());
        assert!(TaskId::parse("bd-123-456").is_err());
    }

    #[test]
    fn test_task_id_display() {
        match TaskId::parse("bd-abc123") {
            Ok(task) => {
                assert_eq!(task.to_string(), "bd-abc123");
                assert_eq!(task.as_str(), "bd-abc123");
            }
            Err(e) => panic!("Failed to parse valid task ID: {e}"),
        }
    }

    // ===== BeadId Alias Tests =====

    #[test]
    fn test_bead_id_is_task_id() {
        match (BeadId::parse("bd-abc123"), TaskId::parse("bd-abc123")) {
            (Ok(bead), Ok(task)) => {
                assert_eq!(bead.as_str(), task.as_str());
            }
            (Err(e), _) => panic!("Failed to parse bead ID: {e}"),
            (_, Err(e)) => panic!("Failed to parse task ID: {e}"),
        }
    }
}

// ===== SessionId Tests =====

#[test]
fn test_valid_session_id() {
    assert!(SessionId::parse("session-abc123").is_ok());
    assert!(SessionId::parse("sess-123").is_ok());
    assert!(SessionId::parse("SESSION_ABC").is_ok());
}

#[test]
fn test_invalid_session_id_empty() {
    let result = SessionId::parse("");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::Empty)));
}

#[test]
fn test_invalid_session_id_non_ascii() {
    let result = SessionId::parse("session-abc-日本語");
    assert!(result.is_err());
}

#[test]
fn test_session_id_display() {
    match SessionId::parse("session-abc123") {
        Ok(id) => {
            assert_eq!(id.to_string(), "session-abc123");
            assert_eq!(id.as_str(), "session-abc123");
        }
        Err(e) => panic!("Failed to parse valid session ID: {e}"),
    }
}

// ===== AbsolutePath Tests =====

#[test]
fn test_valid_absolute_path() {
    assert!(AbsolutePath::parse("/home/user").is_ok());
    assert!(AbsolutePath::parse("/tmp/workspace").is_ok());
    assert!(AbsolutePath::parse("/").is_ok());
}

#[test]
fn test_invalid_absolute_path_empty() {
    let result = AbsolutePath::parse("");
    assert!(result.is_err());
}

#[test]
fn test_invalid_absolute_path_relative() {
    let result = AbsolutePath::parse("relative/path");
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(IdentifierError::NotAbsolutePath { .. })
    ));
}

#[test]
fn test_invalid_absolute_path_null_bytes() {
    let result = AbsolutePath::parse("/path\0with\0nulls");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdentifierError::NullBytesInPath)));
}

#[test]
fn test_absolute_path_display() {
    match AbsolutePath::parse("/home/user/workspace") {
        Ok(path) => {
            assert_eq!(path.to_string(), "/home/user/workspace");
            assert_eq!(path.as_str(), "/home/user/workspace");
        }
        Err(e) => panic!("Failed to parse valid absolute path: {e}"),
    }
}

#[test]
fn test_absolute_path_to_path_buf() {
    match AbsolutePath::parse("/home/user/workspace") {
        Ok(path) => {
            let path_buf = path.to_path_buf();
            assert_eq!(path_buf, std::path::PathBuf::from("/home/user/workspace"));
        }
        Err(e) => panic!("Failed to parse valid absolute path: {e}"),
    }
}

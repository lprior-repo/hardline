//! Pure domain validation functions
//!
//! This module contains **pure validation functions** that enforce business rules
//! without performing any I/O operations. These functions:
//! - Have no side effects
//! - Are deterministic (same input = same output)
//! - Return `Result<(), ValidationError>` for explicit error handling
//! - Use newtypes to make illegal states unrepresentable
//!
//! # Design Principle
//!
//! Following Scott Wlaschin's DDD pattern "Parse at Boundaries":
//! - Validate once when data enters the system
//! - Use validated newtypes to prevent invalid states
//! - Keep validation logic pure and testable

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

/// Validation errors for domain inputs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Input string is empty
    EmptyInput,
    /// Input contains shell metacharacters
    ShellMetacharacter,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "input cannot be empty"),
            Self::ShellMetacharacter => {
                write!(f, "input must not contain shell metacharacters")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Shell metacharacters that must be filtered for security
const SHELL_METACHARACTERS: &[char] = &[
    ';', '&', '$', '#', '(', ')', '*', '?', '|', '>', '<', '[', ']', '{', '\'', '"', '`', '\n', ',',
];

/// Check if string contains shell metacharacters
fn contains_shell_metachar(s: &str) -> bool {
    s.chars().any(|c| SHELL_METACHARACTERS.contains(&c))
}

// ========================================================================
// Newtype wrappers for domain identifiers
// ========================================================================

/// Validated session name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionName(String);

impl SessionName {
    /// Parse and validate a session name
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        validate_session_name(s).map(|()| Self(s.to_string()))
    }
}

impl fmt::Display for SessionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated agent ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

impl AgentId {
    /// Parse and validate an agent ID
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        validate_agent_id(s).map(|()| Self(s.to_string()))
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated workspace name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    /// Parse and validate a workspace name
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        validate_workspace_name(s).map(|()| Self(s.to_string()))
    }
}

impl fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated task ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// Parse and validate a task ID
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        validate_task_id(s).map(|()| Self(s.to_string()))
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated absolute path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsolutePath(String);

impl AbsolutePath {
    /// Parse and validate an absolute path
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        validate_absolute_path(s).map(|()| Self(s.to_string()))
    }
}

impl fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ========================================================================
// Core validation functions (matching contract signatures exactly)
// ========================================================================

/// Validate a session name
pub fn validate_session_name(name: &str) -> Result<(), ValidationError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if trimmed.contains('\0') || contains_shell_metachar(trimmed) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}

/// Validate an agent ID
pub fn validate_agent_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if id.contains('\0') {
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(id) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}

/// Validate a workspace name
pub fn validate_workspace_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(name) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}

/// Validate a task ID
pub fn validate_task_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if id.contains('\0') {
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(id) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}

/// Validate an absolute path
pub fn validate_absolute_path(path: &str) -> Result<(), ValidationError> {
    if path.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if path.contains('\0') {
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(path) {
        return Err(ValidationError::ShellMetacharacter);
    }
    #[cfg(unix)]
    if !path.starts_with('/') {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}

/// Validate both session name and agent ID together
pub fn validate_session_and_agent(
    session_name: &str,
    agent_id: &str,
) -> Result<(), ValidationError> {
    validate_session_name(session_name)?;
    validate_agent_id(agent_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Session name tests
    #[test]
    fn test_validate_session_name_valid() {
        assert!(validate_session_name("my-session").is_ok());
        assert!(validate_session_name("my_session").is_ok());
        assert!(validate_session_name("session-123").is_ok());
    }

    #[test]
    fn test_validate_session_name_empty() {
        assert_eq!(validate_session_name(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_validate_session_name_ampersand() {
        assert_eq!(
            validate_session_name("foo&bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_semicolon() {
        assert_eq!(
            validate_session_name("foo;bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_dollar() {
        assert_eq!(
            validate_session_name("foo$bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_pipe() {
        assert_eq!(
            validate_session_name("foo|bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_backtick() {
        assert_eq!(
            validate_session_name("foo`bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_null_byte() {
        assert_eq!(
            validate_session_name("foo\0bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // Agent ID tests
    #[test]
    fn test_validate_agent_id_valid() {
        assert!(validate_agent_id("agent-123").is_ok());
        assert!(validate_agent_id("agent_456").is_ok());
    }

    #[test]
    fn test_validate_agent_id_empty() {
        assert_eq!(validate_agent_id(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_validate_agent_id_shell_metachar() {
        assert_eq!(
            validate_agent_id("agent$test"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_agent_id_null_byte() {
        assert_eq!(
            validate_agent_id("agent\0test"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // Workspace name tests
    #[test]
    fn test_validate_workspace_name_valid() {
        assert!(validate_workspace_name("my-workspace").is_ok());
        assert!(validate_workspace_name("my_workspace").is_ok());
    }

    #[test]
    fn test_validate_workspace_name_empty() {
        assert_eq!(
            validate_workspace_name(""),
            Err(ValidationError::EmptyInput)
        );
    }

    #[test]
    fn test_validate_workspace_name_shell_metachar() {
        assert_eq!(
            validate_workspace_name("work|space"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_workspace_name_path_separator() {
        assert_eq!(
            validate_workspace_name("my/workspace"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // Task ID tests
    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("bd-abc123").is_ok());
        assert!(validate_task_id("bd-ABC123DEF456").is_ok());
    }

    #[test]
    fn test_validate_task_id_empty() {
        assert_eq!(validate_task_id(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_validate_task_id_shell_metachar() {
        assert_eq!(
            validate_task_id("bd-abc;def"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_task_id_null_byte() {
        assert_eq!(
            validate_task_id("bd-abc\0def"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // Absolute path tests
    #[test]
    fn test_validate_absolute_path_valid() {
        assert!(validate_absolute_path("/home/user").is_ok());
        assert!(validate_absolute_path("/tmp/workspace").is_ok());
        assert!(validate_absolute_path("/").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_empty() {
        assert_eq!(validate_absolute_path(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_validate_absolute_path_relative() {
        assert_eq!(
            validate_absolute_path("relative/path"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_absolute_path_null_byte() {
        assert_eq!(
            validate_absolute_path("/path\0/invalid"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_absolute_path_backtick() {
        assert_eq!(
            validate_absolute_path("/path/with`backtick`"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // Helper tests
    #[test]
    fn test_contains_shell_metachar() {
        assert!(contains_shell_metachar("foo&bar"));
        assert!(contains_shell_metachar("foo;bar"));
        assert!(contains_shell_metachar("foo$bar"));
        assert!(contains_shell_metachar("foo|bar"));
        assert!(contains_shell_metachar("foo`bar"));
        assert!(!contains_shell_metachar("foo_bar"));
        assert!(!contains_shell_metachar("foo-bar"));
    }

    // Newtype tests
    #[test]
    fn test_session_name_newtype_valid() {
        assert!(SessionName::parse("my-session").is_ok());
    }

    #[test]
    fn test_session_name_newtype_empty() {
        assert_eq!(SessionName::parse(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_agent_id_newtype_valid() {
        assert!(AgentId::parse("agent-123").is_ok());
    }

    #[test]
    fn test_workspace_name_newtype_valid() {
        assert!(WorkspaceName::parse("my-workspace").is_ok());
    }

    #[test]
    fn test_task_id_newtype_valid() {
        assert!(TaskId::parse("bd-abc123").is_ok());
    }

    #[test]
    fn test_absolute_path_newtype_valid() {
        assert!(AbsolutePath::parse("/home/user").is_ok());
    }
}

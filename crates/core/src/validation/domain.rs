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
    /// Parse and validate a session name.
    ///
    /// # Errors
    ///
    /// Returns an error if the session name is empty, contains null bytes,
    /// or contains shell metacharacters.
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
    /// Parse and validate an agent ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent ID is empty, contains null bytes,
    /// or contains shell metacharacters.
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
    /// Parse and validate a workspace name.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace name is empty, contains path
    /// separators, null bytes, or shell metacharacters.
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
    /// Parse and validate a task ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the task ID is empty, contains null bytes,
    /// or contains shell metacharacters.
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
    /// Parse and validate an absolute path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is empty, contains null bytes,
    /// shell metacharacters, or (on Unix) does not start with `/`.
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

/// Validate a session name.
///
/// # Errors
///
/// Returns `ValidationError::EmptyInput` if the name is empty or whitespace-only,
/// or `ValidationError::ShellMetacharacter` if it contains disallowed characters.
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

/// Validate an agent ID.
///
/// # Errors
///
/// Returns `ValidationError::EmptyInput` if the ID is empty,
/// or `ValidationError::ShellMetacharacter` if it contains disallowed characters.
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

/// Validate a workspace name.
///
/// # Errors
///
/// Returns `ValidationError::EmptyInput` if the name is empty,
/// or `ValidationError::ShellMetacharacter` if it contains path separators,
/// null bytes, or shell metacharacters.
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

/// Validate a task ID.
///
/// # Errors
///
/// Returns `ValidationError::EmptyInput` if the ID is empty,
/// or `ValidationError::ShellMetacharacter` if it contains disallowed characters.
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

/// Validate an absolute path.
///
/// # Errors
///
/// Returns `ValidationError::EmptyInput` if the path is empty,
/// or `ValidationError::ShellMetacharacter` if it contains disallowed characters
/// or (on Unix) is not an absolute path.
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

/// Validate both session name and agent ID together.
///
/// # Errors
///
/// Returns an error if either the session name or agent ID is invalid.
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

    // ── ValidationError Display ──────────────────────────────────────────────

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            ValidationError::EmptyInput.to_string(),
            "input cannot be empty"
        );
        assert_eq!(
            ValidationError::ShellMetacharacter.to_string(),
            "input must not contain shell metacharacters"
        );
    }

    #[test]
    fn test_validation_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(ValidationError::EmptyInput);
        assert_eq!(err.to_string(), "input cannot be empty");

        let err: Box<dyn std::error::Error> = Box::new(ValidationError::ShellMetacharacter);
        assert_eq!(
            err.to_string(),
            "input must not contain shell metacharacters"
        );
    }

    #[test]
    fn test_validation_error_equality() {
        assert_eq!(ValidationError::EmptyInput, ValidationError::EmptyInput);
        assert_eq!(
            ValidationError::ShellMetacharacter,
            ValidationError::ShellMetacharacter
        );
        assert_ne!(
            ValidationError::EmptyInput,
            ValidationError::ShellMetacharacter
        );
    }

    // ── validate_session_name additional cases ───────────────────────────────

    #[test]
    fn test_validate_session_name_whitespace_only() {
        assert_eq!(
            validate_session_name("   "),
            Err(ValidationError::EmptyInput)
        );
    }

    #[test]
    fn test_validate_session_name_whitespace_trimmed_valid() {
        // Leading/trailing whitespace is trimmed
        assert!(validate_session_name("  valid-name  ").is_ok());
    }

    #[test]
    fn test_validate_session_name_hash() {
        assert_eq!(
            validate_session_name("foo#bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_parentheses() {
        assert_eq!(
            validate_session_name("foo(bar)"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_newline() {
        assert_eq!(
            validate_session_name("foo\nbar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_comma() {
        assert_eq!(
            validate_session_name("foo,bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_redirect_operators() {
        assert_eq!(
            validate_session_name("foo>bar"),
            Err(ValidationError::ShellMetacharacter)
        );
        assert_eq!(
            validate_session_name("foo<bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_single_quote() {
        assert_eq!(
            validate_session_name("foo'bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_double_quote() {
        assert_eq!(
            validate_session_name("foo\"bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_brackets() {
        assert_eq!(
            validate_session_name("foo[bar]"),
            Err(ValidationError::ShellMetacharacter)
        );
        assert_eq!(
            validate_session_name("foo{bar}"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_asterisk() {
        assert_eq!(
            validate_session_name("foo*bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_session_name_question_mark() {
        assert_eq!(
            validate_session_name("foo?bar"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    // ── validate_agent_id additional cases ───────────────────────────────────

    #[test]
    fn test_validate_agent_id_valid_with_numbers() {
        assert!(validate_agent_id("12345").is_ok());
    }

    #[test]
    fn test_validate_agent_id_valid_simple_string() {
        assert!(validate_agent_id("agent").is_ok());
    }

    // ── validate_workspace_name additional cases ─────────────────────────────

    #[test]
    fn test_validate_workspace_name_backslash() {
        assert_eq!(
            validate_workspace_name("my\\workspace"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_workspace_name_null_byte() {
        assert_eq!(
            validate_workspace_name("work\0space"),
            Err(ValidationError::ShellMetacharacter)
        );
    }

    #[test]
    fn test_validate_workspace_name_valid_with_numbers() {
        assert!(validate_workspace_name("workspace-123").is_ok());
    }

    // ── validate_task_id additional cases ────────────────────────────────────

    #[test]
    fn test_validate_task_id_valid_numeric() {
        assert!(validate_task_id("12345").is_ok());
    }

    #[test]
    fn test_validate_task_id_metacharacters() {
        // Test all shell metacharacters produce ShellMetacharacter error
        for ch in SHELL_METACHARACTERS {
            let input = format!("bd-abc{}def", ch);
            assert_eq!(
                validate_task_id(&input),
                Err(ValidationError::ShellMetacharacter),
                "Character '{ch}' should be rejected"
            );
        }
    }

    // ── validate_absolute_path additional cases ──────────────────────────────

    #[test]
    fn test_validate_absolute_path_root() {
        assert!(validate_absolute_path("/").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_with_spaces() {
        assert!(validate_absolute_path("/home/user/my workspace").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_double_slash() {
        // Double slash is not a metacharacter, should be valid
        assert!(validate_absolute_path("/home//user").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_shell_chars() {
        for ch in SHELL_METACHARACTERS {
            if *ch == '/' {
                continue; // skip slash, it's valid in paths
            }
            let input = format!("/path/with{}char", ch);
            assert_eq!(
                validate_absolute_path(&input),
                Err(ValidationError::ShellMetacharacter),
                "Character '{ch}' should be rejected in path"
            );
        }
    }

    // ── validate_session_and_agent ───────────────────────────────────────────

    #[test]
    fn test_validate_session_and_agent_both_valid() {
        assert!(validate_session_and_agent("my-session", "agent-1").is_ok());
    }

    #[test]
    fn test_validate_session_and_agent_invalid_session() {
        assert!(validate_session_and_agent("bad;session", "agent-1").is_err());
    }

    #[test]
    fn test_validate_session_and_agent_invalid_agent() {
        assert!(validate_session_and_agent("my-session", "agent$1").is_err());
    }

    #[test]
    fn test_validate_session_and_agent_both_invalid() {
        assert!(validate_session_and_agent("bad session", "agent$1").is_err());
    }

    #[test]
    fn test_validate_session_and_agent_empty_session() {
        assert_eq!(
            validate_session_and_agent("", "agent-1"),
            Err(ValidationError::EmptyInput)
        );
    }

    #[test]
    fn test_validate_session_and_agent_empty_agent() {
        assert_eq!(
            validate_session_and_agent("my-session", ""),
            Err(ValidationError::EmptyInput)
        );
    }

    // ── Newtype Display ──────────────────────────────────────────────────────

    #[test]
    fn test_session_name_display() {
        let name = SessionName::parse("test-session").expect("valid");
        assert_eq!(name.to_string(), "test-session");
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::parse("agent-123").expect("valid");
        assert_eq!(id.to_string(), "agent-123");
    }

    #[test]
    fn test_workspace_name_display() {
        let name = WorkspaceName::parse("my-workspace").expect("valid");
        assert_eq!(name.to_string(), "my-workspace");
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::parse("bd-abc123").expect("valid");
        assert_eq!(id.to_string(), "bd-abc123");
    }

    #[test]
    fn test_absolute_path_display() {
        let path = AbsolutePath::parse("/home/user").expect("valid");
        assert_eq!(path.to_string(), "/home/user");
    }

    // ── Newtype PartialEq/Hash ───────────────────────────────────────────────

    #[test]
    fn test_session_name_equality() {
        let a = SessionName::parse("session").expect("valid");
        let b = SessionName::parse("session").expect("valid");
        let c = SessionName::parse("other").expect("valid");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_agent_id_equality() {
        let a = AgentId::parse("agent-1").expect("valid");
        let b = AgentId::parse("agent-1").expect("valid");
        let c = AgentId::parse("agent-2").expect("valid");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_newtype_hash_set() {
        let mut set = std::collections::HashSet::new();
        let a = SessionName::parse("session").expect("valid");
        let b = SessionName::parse("session").expect("valid");
        assert!(set.insert(a));
        assert!(!set.insert(b)); // duplicate
        assert_eq!(set.len(), 1);
    }

    // ── Newtype invalid ──────────────────────────────────────────────────────

    #[test]
    fn test_agent_id_newtype_empty() {
        assert_eq!(AgentId::parse(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_workspace_name_newtype_empty() {
        assert_eq!(WorkspaceName::parse(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_workspace_name_newtype_with_path_sep() {
        assert!(WorkspaceName::parse("my/workspace").is_err());
    }

    #[test]
    fn test_workspace_name_newtype_with_metachar() {
        assert!(WorkspaceName::parse("my|workspace").is_err());
    }

    #[test]
    fn test_task_id_newtype_empty() {
        assert_eq!(TaskId::parse(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_task_id_newtype_metachar() {
        assert!(TaskId::parse("bd-abc;def").is_err());
    }

    #[test]
    fn test_absolute_path_newtype_empty() {
        assert_eq!(AbsolutePath::parse(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_absolute_path_newtype_relative() {
        assert!(AbsolutePath::parse("relative/path").is_err());
    }

    // ── contains_shell_metachar comprehensive ────────────────────────────────

    #[test]
    fn test_contains_shell_metachar_all_metacharacters() {
        for &ch in SHELL_METACHARACTERS {
            let input = format!("middle{}end", ch);
            assert!(
                contains_shell_metachar(&input),
                "Expected metacharacter '{}' to be detected",
                ch
            );
        }
    }

    #[test]
    fn test_contains_shell_metachar_no_metacharacters() {
        let safe_strings = [
            "hello-world",
            "hello_world",
            "hello123",
            "hello.world",
            "hello world",
            "hello\tworld",
            "hello~world",
            "hello!world",
            "hello%world",
            "hello@world",
            "hello+world",
            "hello=world",
        ];
        for s in &safe_strings {
            assert!(
                !contains_shell_metachar(s),
                "Expected '{}' to be free of shell metacharacters",
                s
            );
        }
    }
}

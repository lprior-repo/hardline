//! Pure domain validation functions
//!
//! This module contains **pure validation functions** that enforce business rules
//! without performing any I/O operations. These functions:
//! - Have no side effects
//! - Are deterministic (same input = same output)
//! - Return `Result<(), IdentifierError>` for explicit error handling
//! - Can be composed using `and_then`, `map`, etc.
//!
//! # Design Principle
//!
//! Following Scott Wlaschin's DDD pattern "Parse at Boundaries":
//! - Validate once when data enters the system
//! - Use validated newtypes to prevent invalid states
//! - Keep validation logic pure and testable
//!
//! # Usage
//!
//! ```rust
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use isolate_core::validation::domain::*;
//!
//! // Direct validation
//! validate_session_name("my-session")?;
//!
//! // Composed validation
//! let result = validate_session_name("session").and_then(|_| validate_agent_id("agent-123"));
//! # Ok(())
//! # }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::identifiers::IdentifierError;

// ============================================================================
// SESSION NAME VALIDATION
// ============================================================================

/// Validate a session name according to domain rules.
///
/// # Validation Rules
///
/// - Must start with a letter (a-z, A-Z)
/// - Can contain letters, numbers, hyphens, underscores
/// - Maximum length: 63 characters
/// - Leading/trailing whitespace is trimmed before validation
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `Empty`: name is empty or whitespace-only
/// - `TooLong`: exceeds 63 characters
/// - `InvalidStart`: doesn't start with a letter
/// - `InvalidCharacters`: contains disallowed characters
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_session_name;
///
/// // Valid names
/// assert!(validate_session_name("my-session").is_ok());
/// assert!(validate_session_name("my_session").is_ok());
/// assert!(validate_session_name("session-123").is_ok());
///
/// // Invalid names
/// assert!(validate_session_name("123-session").is_err()); // doesn't start with letter
/// assert!(validate_session_name("my.session").is_err()); // invalid character
/// assert!(validate_session_name(&"a".repeat(64)).is_err()); // too long
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
    // Trim whitespace as part of boundary sanitization
    let trimmed = s.trim();

    // Rule 1: Non-empty after trimming
    if trimmed.is_empty() {
        return Err(IdentifierError::empty());
    }

    // Rule 2: Maximum length
    if trimmed.len() > 63 {
        return Err(IdentifierError::too_long(63, trimmed.len()));
    }

    // Rule 3: Must start with a letter
    if !trimmed
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
    {
        return Err(IdentifierError::invalid_start('a'));
    }

    // Rule 4: Valid characters only
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(IdentifierError::invalid_characters(
            "session name must contain only letters, numbers, hyphens, or underscores",
        ));
    }

    Ok(())
}

// ============================================================================
// AGENT ID VALIDATION
// ============================================================================

/// Validate an agent ID according to domain rules.
///
/// # Validation Rules
///
/// - Can contain alphanumeric, hyphen, underscore, dot, colon
/// - Maximum length: 128 characters
/// - Non-empty
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `Empty`: ID is empty
/// - `TooLong`: exceeds 128 characters
/// - `InvalidCharacters`: contains disallowed characters
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_agent_id;
///
/// // Valid IDs
/// assert!(validate_agent_id("agent-123").is_ok());
/// assert!(validate_agent_id("agent_456").is_ok());
/// assert!(validate_agent_id("agent:789").is_ok());
/// assert!(validate_agent_id("agent.example").is_ok());
///
/// // Invalid IDs
/// assert!(validate_agent_id("agent/123").is_err()); // invalid character
/// assert!(validate_agent_id(&"a".repeat(129)).is_err()); // too long
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_agent_id(s: &str) -> Result<(), IdentifierError> {
    // Rule 1: Non-empty
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    // Rule 2: Maximum length
    if s.len() > 128 {
        return Err(IdentifierError::too_long(128, s.len()));
    }

    // Rule 3: Valid characters only
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    {
        return Err(IdentifierError::invalid_characters(
            "agent ID must contain only letters, numbers, hyphens, underscores, dots, or colons",
        ));
    }

    Ok(())
}

// ============================================================================
// WORKSPACE NAME VALIDATION
// ============================================================================

/// Validate a workspace name according to domain rules.
///
/// # Validation Rules
///
/// - Cannot contain path separators (/ or \)
/// - Cannot contain null bytes
/// - Maximum length: 255 characters
/// - Non-empty
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `Empty`: name is empty
/// - `TooLong`: exceeds 255 characters
/// - `ContainsPathSeparators`: contains / or \
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_workspace_name;
///
/// // Valid names
/// assert!(validate_workspace_name("my-workspace").is_ok());
/// assert!(validate_workspace_name("my_workspace").is_ok());
///
/// // Invalid names
/// assert!(validate_workspace_name("my/workspace").is_err()); // path separator
/// assert!(validate_workspace_name("my\\workspace").is_err()); // path separator
/// assert!(validate_workspace_name("my\0workspace").is_err()); // null byte
/// assert!(validate_workspace_name(&"a".repeat(256)).is_err()); // too long
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_workspace_name(s: &str) -> Result<(), IdentifierError> {
    // Rule 1: Non-empty
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    // Rule 2: Maximum length
    if s.len() > 255 {
        return Err(IdentifierError::too_long(255, s.len()));
    }

    // Rule 3: No path separators or null bytes
    if s.contains('/') || s.contains('\\') || s.contains('\0') {
        return Err(IdentifierError::ContainsPathSeparators);
    }

    Ok(())
}

// ============================================================================
// TASK ID VALIDATION
// ============================================================================

/// Validate a task ID (bead ID format) according to domain rules.
///
/// # Validation Rules
///
/// - Must start with "bd-" prefix
/// - Followed by hexadecimal characters (0-9, a-f, A-F)
/// - Non-empty after prefix
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `Empty`: ID is empty
/// - `InvalidPrefix`: doesn't start with "bd-"
/// - `InvalidHex`: has non-hexadecimal characters after prefix
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_task_id;
///
/// // Valid IDs
/// assert!(validate_task_id("bd-abc123").is_ok());
/// assert!(validate_task_id("bd-ABC123DEF456").is_ok());
/// assert!(validate_task_id("bd-1234567890abcdef").is_ok());
///
/// // Invalid IDs
/// assert!(validate_task_id("abc123").is_err()); // missing prefix
/// assert!(validate_task_id("bd-xyz").is_err()); // non-hex
/// assert!(validate_task_id("bd-123-456").is_err()); // non-hex
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_task_id(s: &str) -> Result<(), IdentifierError> {
    // Rule 1: Non-empty
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    // Rule 2: Must have "bd-" prefix
    if !s.starts_with("bd-") {
        return Err(IdentifierError::invalid_prefix("bd-", s));
    }

    // Rule 3: Hexadecimal after prefix
    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentifierError::invalid_hex(s));
    }

    Ok(())
}

/// Validate a bead ID (alias for task ID validation).
///
/// Beads and tasks use the same ID format, so this is an alias for
/// [`validate_task_id`].
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_bead_id;
///
/// assert!(validate_bead_id("bd-abc123").is_ok());
/// assert!(validate_bead_id("xyz").is_err());
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails.
pub fn validate_bead_id(s: &str) -> Result<(), IdentifierError> {
    validate_task_id(s)
}

// ============================================================================
// SESSION ID VALIDATION
// ============================================================================

/// Validate a session ID according to domain rules.
///
/// # Validation Rules
///
/// - Must be ASCII-only
/// - Non-empty
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `Empty`: ID is empty
/// - `NotAscii`: contains non-ASCII characters
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_session_id;
///
/// // Valid IDs
/// assert!(validate_session_id("session-abc123").is_ok());
/// assert!(validate_session_id("SESSION_ABC").is_ok());
///
/// // Invalid IDs
/// assert!(validate_session_id("session-abc-日本語").is_err()); // non-ASCII
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_session_id(s: &str) -> Result<(), IdentifierError> {
    // Rule 1: Non-empty
    if s.is_empty() {
        return Err(IdentifierError::empty());
    }

    // Rule 2: ASCII only
    if !s.is_ascii() {
        return Err(IdentifierError::NotAscii {
            value: s.to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// ABSOLUTE PATH VALIDATION
// ============================================================================

/// Validate an absolute path according to domain rules.
///
/// # Validation Rules
///
/// - Must be absolute (starts with / on Unix)
/// - Cannot contain null bytes
/// - Non-empty
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails:
/// - `InvalidFormat`: path is empty
/// - `NotAbsolutePath`: path is not absolute
/// - `NullBytesInPath`: path contains null bytes
///
/// # Examples
///
/// ```rust
/// use isolate_core::validation::domain::validate_absolute_path;
///
/// // Valid paths (Unix)
/// assert!(validate_absolute_path("/home/user").is_ok());
/// assert!(validate_absolute_path("/tmp/workspace").is_ok());
/// assert!(validate_absolute_path("/").is_ok());
///
/// // Invalid paths
/// assert!(validate_absolute_path("relative/path").is_err()); // not absolute
/// assert!(validate_absolute_path("./path").is_err()); // not absolute
/// assert!(validate_absolute_path("/path\0with\0nulls").is_err()); // null bytes
/// ```
///
/// # Platform Specificity
///
/// This function has platform-specific behavior:
/// - **Unix**: Path must start with `/`
/// - **Windows**: Path must start with `\` or have a drive letter (e.g., `C:\`)
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_absolute_path(s: &str) -> Result<(), IdentifierError> {
    // Rule 1: Non-empty
    if s.is_empty() {
        return Err(IdentifierError::invalid_format("path cannot be empty"));
    }

    // Rule 2: No null bytes
    if s.contains('\0') {
        return Err(IdentifierError::NullBytesInPath);
    }

    // Rule 3: Must be absolute (platform-specific)
    #[cfg(unix)]
    {
        if !s.starts_with('/') {
            return Err(IdentifierError::not_absolute_path(s));
        }
    }

    #[cfg(windows)]
    {
        // Windows absolute paths: C:\ or \\server\share
        let is_absolute = s.starts_with('\\')
            || (s.len() > 2 && s.as_bytes()[1] == b':' && s.as_bytes()[2] == b'\\');

        if !is_absolute {
            return Err(IdentifierError::not_absolute_path(s));
        }
    }

    Ok(())
}

// ============================================================================
// COMPOSED VALIDATORS
// ============================================================================

/// Validate both session name and derive an agent ID from it.
///
/// This demonstrates validator composition using pure functions.
///
/// # Errors
///
/// Returns `IdentifierError` if either validation fails.
///
/// # Examples
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use isolate_core::validation::domain::validate_session_and_agent;
///
/// let result = validate_session_and_agent("my-session", "agent-my-session")?;
/// # Ok(())
/// # }
/// ```
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_session_and_agent(
    session_name: &str,
    agent_id: &str,
) -> Result<(), IdentifierError> {
    validate_session_name(session_name)?;
    validate_agent_id(agent_id)?;
    Ok(())
}

/// Validate that a workspace name is safe for filesystem operations.
///
/// This is a composed validator that checks workspace name rules
/// suitable for use in directory creation.
///
/// # Errors
///
/// Returns `IdentifierError` if validation fails.
///
/// # Pure Function
///
/// This function is pure (no side effects, deterministic).
pub fn validate_workspace_name_safe(s: &str) -> Result<(), IdentifierError> {
    validate_workspace_name(s)?;

    // Additional safety: disallow shell metacharacters
    if s.contains('$')
        || s.contains('`')
        || s.contains(';')
        || s.contains('|')
        || s.contains('&')
        || s.contains('(')
        || s.contains(')')
    {
        return Err(IdentifierError::invalid_characters(
            "workspace name must not contain shell metacharacters",
        ));
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Session Name Tests =====

    #[test]
    fn test_validate_session_name_valid() {
        assert!(validate_session_name("my-session").is_ok());
        assert!(validate_session_name("my_session").is_ok());
        assert!(validate_session_name("session-123").is_ok());
        assert!(validate_session_name("a").is_ok());
    }

    #[test]
    fn test_validate_session_name_trims_whitespace() {
        assert!(validate_session_name("  my-session  ").is_ok());
        assert!(validate_session_name("\tmy-session\t").is_ok());
        assert!(validate_session_name("\nmy-session\n").is_ok());
    }

    #[test]
    fn test_validate_session_name_whitespace_only_is_empty() {
        let result = validate_session_name("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_session_name_invalid_start() {
        assert!(validate_session_name("123-session").is_err());
        assert!(validate_session_name("-session").is_err());
        assert!(validate_session_name("_session").is_err());
    }

    #[test]
    fn test_validate_session_name_invalid_characters() {
        assert!(validate_session_name("my.session").is_err());
        assert!(validate_session_name("my:session").is_err());
        assert!(validate_session_name("my session").is_err());
        assert!(validate_session_name("my/session").is_err());
    }

    #[test]
    fn test_validate_session_name_too_long() {
        let long_name = "a".repeat(64);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_too_long());
    }

    #[test]
    fn test_validate_session_name_max_length() {
        let max_name = "a".repeat(63);
        assert!(validate_session_name(&max_name).is_ok());
    }

    // ===== Agent ID Tests =====

    #[test]
    fn test_validate_agent_id_valid() {
        assert!(validate_agent_id("agent-123").is_ok());
        assert!(validate_agent_id("agent_456").is_ok());
        assert!(validate_agent_id("agent:789").is_ok());
        assert!(validate_agent_id("agent.example").is_ok());
        assert!(validate_agent_id("a").is_ok());
    }

    #[test]
    fn test_validate_agent_id_empty() {
        let result = validate_agent_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_agent_id_invalid_characters() {
        assert!(validate_agent_id("agent/123").is_err());
        assert!(validate_agent_id("agent 123").is_err());
        assert!(validate_agent_id("agent@123").is_err());
    }

    #[test]
    fn test_validate_agent_id_too_long() {
        let long_id = "a".repeat(129);
        let result = validate_agent_id(&long_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_too_long());
    }

    #[test]
    fn test_validate_agent_id_max_length() {
        let max_id = "a".repeat(128);
        assert!(validate_agent_id(&max_id).is_ok());
    }

    // ===== Workspace Name Tests =====

    #[test]
    fn test_validate_workspace_name_valid() {
        assert!(validate_workspace_name("my-workspace").is_ok());
        assert!(validate_workspace_name("my_workspace").is_ok());
        assert!(validate_workspace_name("a").is_ok());
    }

    #[test]
    fn test_validate_workspace_name_empty() {
        let result = validate_workspace_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_workspace_name_path_separators() {
        assert!(validate_workspace_name("my/workspace").is_err());
        assert!(validate_workspace_name("my\\workspace").is_err());
    }

    #[test]
    fn test_validate_workspace_name_null_byte() {
        let result = validate_workspace_name("my\0workspace");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_name_too_long() {
        let long_name = "a".repeat(256);
        let result = validate_workspace_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_too_long());
    }

    #[test]
    fn test_validate_workspace_name_max_length() {
        let max_name = "a".repeat(255);
        assert!(validate_workspace_name(&max_name).is_ok());
    }

    // ===== Task ID Tests =====

    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("bd-abc123").is_ok());
        assert!(validate_task_id("bd-ABC123DEF456").is_ok());
        assert!(validate_task_id("bd-1234567890abcdef").is_ok());
        assert!(validate_task_id("bd-a").is_ok());
    }

    #[test]
    fn test_validate_task_id_empty() {
        let result = validate_task_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_task_id_no_prefix() {
        assert!(validate_task_id("abc123").is_err());
        assert!(validate_task_id("task-abc123").is_err());
    }

    #[test]
    fn test_validate_task_id_no_hex() {
        assert!(validate_task_id("bd-xyz").is_err());
        assert!(validate_task_id("bd-123-456").is_err());
        assert!(validate_task_id("bd-").is_err());
    }

    // ===== Bead ID Tests =====

    #[test]
    fn test_validate_bead_id_matches_task_id() {
        assert!(validate_bead_id("bd-abc123").is_ok());
        assert!(validate_bead_id("xyz").is_err());
    }

    // ===== Session ID Tests =====

    #[test]
    fn test_validate_session_id_valid() {
        assert!(validate_session_id("session-abc123").is_ok());
        assert!(validate_session_id("SESSION_ABC").is_ok());
        assert!(validate_session_id("a").is_ok());
    }

    #[test]
    fn test_validate_session_id_empty() {
        let result = validate_session_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_empty());
    }

    #[test]
    fn test_validate_session_id_non_ascii() {
        assert!(validate_session_id("session-abc-日本語").is_err());
        assert!(validate_session_id("session-emoji-😀").is_err());
    }

    // ===== Absolute Path Tests =====

    #[test]
    fn test_validate_absolute_path_valid() {
        assert!(validate_absolute_path("/home/user").is_ok());
        assert!(validate_absolute_path("/tmp/workspace").is_ok());
        assert!(validate_absolute_path("/").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_empty() {
        let result = validate_absolute_path("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_absolute_path_relative() {
        assert!(validate_absolute_path("relative/path").is_err());
        assert!(validate_absolute_path("./path").is_err());
        assert!(validate_absolute_path("../path").is_err());
    }

    #[test]
    fn test_validate_absolute_path_null_bytes() {
        let result = validate_absolute_path("/path\0with\0nulls");
        assert!(result.is_err());
    }

    // ===== Composed Validators =====

    #[test]
    fn test_validate_session_and_agent_both_valid() {
        assert!(validate_session_and_agent("my-session", "agent-123").is_ok());
    }

    #[test]
    fn test_validate_session_and_agent_invalid_session() {
        let result = validate_session_and_agent("123-session", "agent-123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_session_and_agent_invalid_agent() {
        let result = validate_session_and_agent("my-session", "agent/123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_name_safe_rejects_metachars() {
        assert!(validate_workspace_name_safe("my$workspace").is_err());
        assert!(validate_workspace_name_safe("my`workspace").is_err());
        assert!(validate_workspace_name_safe("my;workspace").is_err());
        assert!(validate_workspace_name_safe("my|workspace").is_err());
        assert!(validate_workspace_name_safe("my&workspace").is_err());
        assert!(validate_workspace_name_safe("my(workspace)").is_err());
    }

    #[test]
    fn test_validate_workspace_name_safe_accepts_valid() {
        assert!(validate_workspace_name_safe("my-workspace").is_ok());
        assert!(validate_workspace_name_safe("my_workspace").is_ok());
    }
}

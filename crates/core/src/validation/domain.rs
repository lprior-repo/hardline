//! Pure domain validation functions
//!
//! This module contains **pure validation functions** that enforce business rules
//! without performing any I/O operations. These functions:
//! - Have no side effects
//! - Are deterministic (same input = same output)
//! - Return `Result<(), Error>` for explicit error handling
//! - Can be composed using `and_then`, `map`, etc.
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

pub mod identifiers;

use crate::error::{Error, Result};

pub fn validate_session_name(s: &str) -> Result<()> {
    let trimmed = s.trim();

    if trimmed.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "session name cannot be empty".to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    if trimmed.len() > 63 {
        return Err(Error::ValidationFieldError {
            message: format!(
                "session name exceeds maximum length of 63 characters (got {})",
                trimmed.len()
            ),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    if !trimmed
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
    {
        return Err(Error::ValidationFieldError {
            message: "session name must start with a letter".to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::ValidationFieldError {
            message: "session name must contain only letters, numbers, hyphens, or underscores"
                .to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

pub fn validate_agent_id(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "agent ID cannot be empty".to_string(),
            field: "agent_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    if s.len() > 128 {
        return Err(Error::ValidationFieldError {
            message: format!(
                "agent ID exceeds maximum length of 128 characters (got {})",
                s.len()
            ),
            field: "agent_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    {
        return Err(Error::ValidationFieldError {
            message:
                "agent ID must contain only letters, numbers, hyphens, underscores, dots, or colons"
                    .to_string(),
            field: "agent_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

pub fn validate_workspace_name(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "workspace name cannot be empty".to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    if s.len() > 255 {
        return Err(Error::ValidationFieldError {
            message: format!(
                "workspace name exceeds maximum length of 255 characters (got {})",
                s.len()
            ),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    if s.contains('/') || s.contains('\\') || s.contains('\0') {
        return Err(Error::ValidationFieldError {
            message: "workspace name cannot contain path separators or null bytes".to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

pub fn validate_task_id(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "task ID cannot be empty".to_string(),
            field: "task_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    if !s.starts_with("bd-") {
        return Err(Error::ValidationFieldError {
            message: "task ID must start with 'bd-' prefix".to_string(),
            field: "task_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    let hex_part = &s[3..];
    if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::ValidationFieldError {
            message: "task ID must contain only hexadecimal characters after 'bd-' prefix"
                .to_string(),
            field: "task_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

pub fn validate_bead_id(s: &str) -> Result<()> {
    validate_task_id(s)
}

pub fn validate_session_id(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "session ID cannot be empty".to_string(),
            field: "session_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    if !s.is_ascii() {
        return Err(Error::ValidationFieldError {
            message: "session ID must contain only ASCII characters".to_string(),
            field: "session_id".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

pub fn validate_absolute_path(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::ValidationFieldError {
            message: "path cannot be empty".to_string(),
            field: "path".to_string(),
            value: Some(s.to_string()),
        });
    }

    if s.contains('\0') {
        return Err(Error::ValidationFieldError {
            message: "path cannot contain null bytes".to_string(),
            field: "path".to_string(),
            value: Some(s.to_string()),
        });
    }

    #[cfg(unix)]
    {
        if !s.starts_with('/') {
            return Err(Error::ValidationFieldError {
                message: "path must be absolute (must start with /)".to_string(),
                field: "path".to_string(),
                value: Some(s.to_string()),
            });
        }
    }

    #[cfg(windows)]
    {
        let is_absolute = s.starts_with('\\')
            || (s.len() > 2 && s.as_bytes()[1] == b':' && s.as_bytes()[2] == b'\\');

        if !is_absolute {
            return Err(Error::ValidationFieldError {
                message: "path must be absolute".to_string(),
                field: "path".to_string(),
                value: Some(s.to_string()),
            });
        }
    }

    Ok(())
}

pub fn validate_session_and_agent(session_name: &str, agent_id: &str) -> Result<()> {
    validate_session_name(session_name)?;
    validate_agent_id(agent_id)?;
    Ok(())
}

pub fn validate_workspace_name_safe(s: &str) -> Result<()> {
    validate_workspace_name(s)?;

    if s.contains('$')
        || s.contains('`')
        || s.contains(';')
        || s.contains('|')
        || s.contains('&')
        || s.contains('(')
        || s.contains(')')
    {
        return Err(Error::ValidationFieldError {
            message: "workspace name must not contain shell metacharacters".to_string(),
            field: "name".to_string(),
            value: Some(s.to_string()),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn test_validate_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_session_name_invalid_start() {
        assert!(validate_session_name("123-session").is_err());
        assert!(validate_session_name("-session").is_err());
    }

    #[test]
    fn test_validate_session_name_too_long() {
        let long_name = "a".repeat(64);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_agent_id_valid() {
        assert!(validate_agent_id("agent-123").is_ok());
        assert!(validate_agent_id("agent_456").is_ok());
        assert!(validate_agent_id("agent:789").is_ok());
        assert!(validate_agent_id("agent.example").is_ok());
    }

    #[test]
    fn test_validate_agent_id_empty() {
        let result = validate_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_name_valid() {
        assert!(validate_workspace_name("my-workspace").is_ok());
        assert!(validate_workspace_name("my_workspace").is_ok());
    }

    #[test]
    fn test_validate_workspace_name_path_separators() {
        assert!(validate_workspace_name("my/workspace").is_err());
        assert!(validate_workspace_name("my\\workspace").is_err());
    }

    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("bd-abc123").is_ok());
        assert!(validate_task_id("bd-ABC123DEF456").is_ok());
    }

    #[test]
    fn test_validate_task_id_no_prefix() {
        assert!(validate_task_id("abc123").is_err());
    }

    #[test]
    fn test_validate_session_id_valid() {
        assert!(validate_session_id("session-abc123").is_ok());
        assert!(validate_session_id("SESSION_ABC").is_ok());
    }

    #[test]
    fn test_validate_session_id_non_ascii() {
        assert!(validate_session_id("session-abc-日本語").is_err());
    }

    #[test]
    fn test_validate_absolute_path_valid() {
        assert!(validate_absolute_path("/home/user").is_ok());
        assert!(validate_absolute_path("/tmp/workspace").is_ok());
        assert!(validate_absolute_path("/").is_ok());
    }

    #[test]
    fn test_validate_absolute_path_relative() {
        assert!(validate_absolute_path("relative/path").is_err());
        assert!(validate_absolute_path("./path").is_err());
    }

    #[test]
    fn test_validate_workspace_name_safe_rejects_metachars() {
        assert!(validate_workspace_name_safe("my$workspace").is_err());
        assert!(validate_workspace_name_safe("my`workspace").is_err());
        assert!(validate_workspace_name_safe("my;workspace").is_err());
    }
}

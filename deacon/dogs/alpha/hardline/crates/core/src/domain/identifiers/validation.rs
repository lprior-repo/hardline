//! Validation helper functions
//!
//! This module contains pure validation functions for identifier types.
//! Each validation function returns `Result<(), IdentifierError>`.

use crate::domain::identifiers::error::IdentifierError;

pub fn validate_session_name(s: &str) -> Result<(), IdentifierError> {
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
pub fn validate_agent_id(s: &str) -> Result<(), IdentifierError> {
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
pub fn validate_workspace_name(s: &str) -> Result<(), IdentifierError> {
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

pub fn validate_task_id(s: &str) -> Result<(), IdentifierError> {
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
pub fn validate_bead_id(s: &str) -> Result<(), IdentifierError> {
    validate_task_id(s)
}

/// Validate a session ID
///
/// Rules:
/// - Must be non-empty
/// - Must be valid UTF-8
/// - Can contain any printable characters (more lenient than names)
pub fn validate_session_id(s: &str) -> Result<(), IdentifierError> {
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

pub fn validate_absolute_path(s: &str) -> Result<(), IdentifierError> {
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
        if !s.starts_with('\\') && !(s.len() > 2 && s.as_bytes()[1] == b':') {
            return Err(IdentifierError::not_absolute_path(s));
        }
    }

    Ok(())
}

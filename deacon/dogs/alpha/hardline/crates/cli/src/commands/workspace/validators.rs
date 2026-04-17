//! Workspace name validators

use scp_core::Error;

/// Validate workspace name (P1)
/// Returns Some(Error) if invalid, None if valid
/// Enforces regex: ^[a-zA-Z][a-zA-Z0-9_-]*$
#[must_use]
pub fn validate_workspace_name(name: &str) -> Option<Error> {
    if name.is_empty() {
        return Some(Error::invalid_identifier("workspace name cannot be empty"));
    }

    let mut chars = name.chars();
    let first = chars.next()?;

    // Must start with a letter
    if !first.is_alphabetic() {
        return Some(Error::invalid_identifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    // Remaining chars must be alphanumeric, dash, or underscore
    if !chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Some(Error::invalid_identifier(format!(
            "workspace name must be alphanumeric, dash, or underscore only, got '{}'",
            name
        )));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_name_returns_error() {
        assert!(validate_workspace_name("").is_some());
    }

    #[test]
    fn test_validate_starts_with_digit_returns_error() {
        assert!(validate_workspace_name("123invalid").is_some());
    }

    #[test]
    fn test_validate_starts_with_special_char_returns_error() {
        assert!(validate_workspace_name("@invalid").is_some());
    }

    #[test]
    fn test_validate_valid_simple_name_returns_none() {
        assert!(validate_workspace_name("abc").is_none());
    }

    #[test]
    fn test_validate_valid_with_dash_returns_none() {
        assert!(validate_workspace_name("abc-def").is_none());
    }

    #[test]
    fn test_validate_valid_with_underscore_returns_none() {
        assert!(validate_workspace_name("abc_def").is_none());
    }

    #[test]
    fn test_validate_valid_with_numbers_returns_none() {
        assert!(validate_workspace_name("abc123").is_none());
    }

    #[test]
    fn test_validate_valid_mixed_returns_none() {
        assert!(validate_workspace_name("abc-def_123").is_none());
    }

    #[test]
    fn test_validate_invalid_with_special_char_returns_error() {
        assert!(validate_workspace_name("abc@def").is_some());
    }

    #[test]
    fn test_validate_invalid_with_exclamation_returns_error() {
        assert!(validate_workspace_name("valid-name!").is_some());
    }

    #[test]
    fn test_validate_invalid_with_at_sign_returns_error() {
        assert!(validate_workspace_name("abc@#$%").is_some());
    }

    #[test]
    fn test_validate_invalid_with_space_returns_error() {
        assert!(validate_workspace_name("abc def").is_some());
    }
}

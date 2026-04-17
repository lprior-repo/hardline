//! Data types for the rename command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Maximum session name length.
pub const MAX_NAME_LENGTH: usize = 64;

/// Options for the rename command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct RenameOptions {
    /// Current session name.
    pub old_name: String,
    /// New session name.
    pub new_name: String,
    /// Dry-run mode.
    pub dry_run: bool,
}

/// Output from the rename command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenameOutput {
    /// Whether rename succeeded.
    pub success: bool,
    /// Old session name.
    pub old_name: String,
    /// New session name.
    pub new_name: String,
    /// Whether this was a dry-run.
    pub dry_run: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// ============================================================================
// Pure computation functions (Tier 2)
// ============================================================================

/// Validate session name length.
///
/// # Errors
///
/// Returns error if name exceeds maximum length.
pub fn validate_name_length(name: &str) -> Result<(), String> {
    if name.len() > MAX_NAME_LENGTH {
        return Err(format!(
            "Session name too long: {} characters (max: {})",
            name.len(),
            MAX_NAME_LENGTH
        ));
    }
    Ok(())
}

/// Validate session name format (alphanumeric + dash/underscore, starts with letter).
pub fn validate_session_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Session name cannot be empty".to_string());
    }

    let first = name.chars().next();
    if !first.is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err("Session name must start with a letter".to_string());
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Session name contains invalid characters".to_string());
    }

    Ok(())
}

/// Check if a name is reserved.
pub fn is_reserved_name(name: &str) -> bool {
    matches!(name, "main" | "default" | "trunk" | "master")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_output_default() {
        let output = RenameOutput::default();
        assert!(!output.success);
        assert!(output.old_name.is_empty());
        assert!(output.new_name.is_empty());
        assert!(!output.dry_run);
        assert!(output.error.is_none());
    }

    #[test]
    fn rename_output_serialization_roundtrip() {
        let output = RenameOutput {
            success: true,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            dry_run: false,
            error: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RenameOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.success);
        assert_eq!(deserialized.old_name, "old");
        assert_eq!(deserialized.new_name, "new");
    }

    #[test]
    fn rename_output_with_error() {
        let output = RenameOutput {
            success: false,
            old_name: "a".to_string(),
            new_name: "b".to_string(),
            dry_run: false,
            error: Some("Session exists".to_string()),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"error\":"));
    }

    #[test]
    fn validate_name_length_ok() {
        assert!(validate_name_length("short").is_ok());
    }

    #[test]
    fn validate_name_length_too_long() {
        let long = "a".repeat(MAX_NAME_LENGTH + 1);
        assert!(validate_name_length(&long).is_err());
    }

    #[test]
    fn validate_name_length_exact_max() {
        let exact = "a".repeat(MAX_NAME_LENGTH);
        assert!(validate_name_length(&exact).is_ok());
    }

    #[test]
    fn validate_session_name_valid() {
        assert!(validate_session_name("feature-auth").is_ok());
        assert!(validate_session_name("my_session").is_ok());
        assert!(validate_session_name("abc").is_ok());
    }

    #[test]
    fn validate_session_name_empty() {
        assert!(validate_session_name("").is_err());
    }

    #[test]
    fn validate_session_name_starts_with_number() {
        assert!(validate_session_name("123-abc").is_err());
    }

    #[test]
    fn validate_session_name_invalid_chars() {
        assert!(validate_session_name("has space").is_err());
        assert!(validate_session_name("has\\slash").is_err());
    }

    #[test]
    fn is_reserved_name_main() {
        assert!(is_reserved_name("main"));
        assert!(is_reserved_name("master"));
        assert!(!is_reserved_name("feature"));
    }
}

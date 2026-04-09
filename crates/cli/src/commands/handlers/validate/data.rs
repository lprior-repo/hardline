//! Data types for the validate command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Reserved session names.
pub const RESERVED_NAMES: &[&str] = &["main", "default", "trunk", "master"];

/// Options for the validate command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// Command to validate inputs for.
    pub command: String,
    /// Arguments to validate.
    pub args: Vec<String>,
    /// Dry run mode - preview without side effects.
    pub dry_run: bool,
}

/// Validation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOutput {
    /// Whether all inputs are valid.
    pub valid: bool,
    /// The command being validated.
    pub command: String,
    /// Validated arguments.
    pub args: Vec<ArgValidation>,
    /// Overall validation errors.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    /// Warnings (valid but may cause issues).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Suggestions for improvement.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<String>,
}

/// Validation for a single argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgValidation {
    /// Argument name or position.
    pub name: String,
    /// The value provided.
    pub value: String,
    /// Whether this argument is valid.
    pub valid: bool,
    /// Error message if invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Suggestion if invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

// ============================================================================
// Pure computation functions (Tier 2)
// ============================================================================

/// Validate a session name.
///
/// Rules: must start with a letter, can contain letters/numbers/hyphens/underscores.
pub fn validate_session_name(name: &str) -> ArgValidation {
    if name.is_empty() {
        return ArgValidation {
            name: "name".to_string(),
            value: name.to_string(),
            valid: false,
            error: Some("Session name cannot be empty".to_string()),
            suggestion: Some("Provide a name like 'feature-auth'".to_string()),
        };
    }

    let first = name.chars().next();
    if !first.is_some_and(|c| c.is_ascii_alphabetic()) {
        return ArgValidation {
            name: "name".to_string(),
            value: name.to_string(),
            valid: false,
            error: Some("Session name must start with a letter".to_string()),
            suggestion: Some(format!("Try 'x{name}' or 'session-{name}'")),
        };
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return ArgValidation {
            name: "name".to_string(),
            value: name.to_string(),
            valid: false,
            error: Some("Session name contains invalid characters".to_string()),
            suggestion: Some("Use only letters, numbers, hyphens, and underscores".to_string()),
        };
    }

    ArgValidation {
        name: "name".to_string(),
        value: name.to_string(),
        valid: true,
        error: None,
        suggestion: None,
    }
}

/// Check if a name is reserved.
pub fn is_reserved_name(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

/// Validate bead ID format (prefix-id like "isolate-abc12").
pub fn validate_bead_id_format(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    let prefix = parts[0];
    let suffix = parts[1];

    !prefix.is_empty()
        && prefix.chars().all(|c| c.is_ascii_lowercase())
        && !suffix.is_empty()
        && suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_session_name_valid() {
        let result = validate_session_name("feature-auth");
        assert!(result.valid);
    }

    #[test]
    fn validate_session_name_empty() {
        assert!(!validate_session_name("").valid);
    }

    #[test]
    fn validate_session_name_starts_with_number() {
        assert!(!validate_session_name("123-feature").valid);
    }

    #[test]
    fn validate_session_name_backslash_rejected() {
        assert!(!validate_session_name("test\\nname").valid);
    }

    #[test]
    fn validate_session_name_space_rejected() {
        assert!(!validate_session_name("feature auth").valid);
    }

    #[test]
    fn validate_session_name_valid_variants() {
        for name in &["my-session-123", "feature_auth", "testSession", "abc"] {
            assert!(
                validate_session_name(name).valid,
                "Expected '{name}' to be valid"
            );
        }
    }

    #[test]
    fn validate_bead_id_valid() {
        assert!(validate_bead_id_format("isolate-abc12"));
        assert!(validate_bead_id_format("hl-xyz99"));
    }

    #[test]
    fn validate_bead_id_invalid() {
        assert!(!validate_bead_id_format("invalid"));
        assert!(!validate_bead_id_format("a-b-c"));
        assert!(!validate_bead_id_format("-abc"));
        assert!(!validate_bead_id_format("ABC-123"));
    }

    #[test]
    fn is_reserved() {
        assert!(is_reserved_name("main"));
        assert!(is_reserved_name("master"));
        assert!(!is_reserved_name("feature"));
    }

    #[test]
    fn validate_output_serialization() {
        let output = ValidateOutput {
            valid: true,
            command: "spawn".to_string(),
            args: vec![],
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let parsed: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(parsed.valid);
    }

    #[test]
    fn validate_output_with_errors_serialization() {
        let output = ValidateOutput {
            valid: false,
            command: "spawn".to_string(),
            args: vec![ArgValidation {
                name: "name".to_string(),
                value: "123".to_string(),
                valid: false,
                error: Some("Must start with letter".to_string()),
                suggestion: Some("Try x123".to_string()),
            }],
            errors: vec!["Invalid name".to_string()],
            warnings: vec![],
            suggestions: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("\"error\":"));
    }
}

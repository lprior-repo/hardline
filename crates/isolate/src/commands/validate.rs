//! Validate command - Pre-validate inputs before execution
//!
//! Allows AI agents to validate inputs without executing commands.

use anyhow::Result;
use isolate_core::{json::SchemaEnvelope, OutputFormat, ValidationHint};
use serde::{Deserialize, Serialize};

/// Options for the validate command
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// Command to validate inputs for
    pub command: String,
    /// Arguments to validate
    pub args: Vec<String>,
    /// Output format
    pub format: OutputFormat,
    /// Dry run mode - preview without side effects
    pub dry_run: bool,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether all inputs are valid
    pub valid: bool,
    /// The command being validated
    pub command: String,
    /// Validated arguments
    pub args: Vec<ArgValidation>,
    /// Overall validation errors
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    /// Warnings (valid but may cause issues)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Suggestions for improvement
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<String>,
    /// Structured hints for each validation issue
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hints: Vec<ValidationHint>,
}

/// Validation for a single argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgValidation {
    /// Argument name or position
    pub name: String,
    /// The value provided
    pub value: String,
    /// Whether this argument is valid
    pub valid: bool,
    /// Error message if invalid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Suggestion if invalid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Run the validate command
pub fn run(options: &ValidateOptions) -> Result<()> {
    let result = validate_command(&options.command, &options.args);

    let is_valid = result.valid;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("validate-response", "single", result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        if options.dry_run {
            println!("[DRY RUN] Validation preview:");
            println!();
        }
        if result.valid {
            println!("✓ All inputs valid for '{}'", result.command);
        } else {
            println!("✗ Validation failed for '{}'", result.command);
            println!();
            result.args.iter().for_each(|arg| {
                if arg.valid {
                    println!("  ✓ {}: {}", arg.name, arg.value);
                } else {
                    println!("  ✗ {}: {}", arg.name, arg.value);
                    if let Some(err) = &arg.error {
                        println!("    Error: {err}");
                    }
                    if let Some(sugg) = &arg.suggestion {
                        println!("    Suggestion: {sugg}");
                    }
                }
            });
            if !result.errors.is_empty() {
                println!();
                println!("Errors:");
                result.errors.iter().for_each(|err| {
                    println!("  - {err}");
                });
            }
        }
        if !result.warnings.is_empty() {
            println!();
            println!("Warnings:");
            result.warnings.iter().for_each(|warn| {
                println!("  - {warn}");
            });
        }
    }

    if is_valid {
        Ok(())
    } else {
        anyhow::bail!("Validation failed")
    }
}

fn validate_command(command: &str, args: &[String]) -> ValidationResult {
    match command {
        "add" | "work" => validate_add_args(command, args),
        "remove" => validate_remove_args(args),
        "focus" => validate_focus_args(args),
        "spawn" => validate_spawn_args(args),
        _ => ValidationResult {
            valid: true,
            command: command.to_string(),
            args: vec![],
            errors: vec![],
            warnings: vec![format!("No specific validation for command '{command}'")],
            suggestions: vec![],
            hints: vec![],
        },
    }
}

fn validate_add_args(command: &str, args: &[String]) -> ValidationResult {
    let mut result = ValidationResult {
        valid: true,
        command: command.to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
        hints: vec![],
    };

    if args.is_empty() {
        result.valid = false;
        result.errors.push("Session name is required".to_string());
        result.hints.push(
            ValidationHint::new("name", "session name")
                .with_example("feature-auth")
                .with_pattern("^[a-zA-Z][a-zA-Z0-9_-]*$"),
        );
        return result;
    }

    let name = &args[0];
    let name_validation = validate_session_name(name);
    result.args.push(name_validation.clone());

    if !name_validation.valid {
        result.valid = false;
        result.errors.push(format!("Invalid session name: {name}"));
        if let Some(sugg) = &name_validation.suggestion {
            result.suggestions.push(sugg.clone());
        }
        result.hints.push(
            ValidationHint::new(
                "name",
                "alphanumeric with dashes/underscores, starting with letter",
            )
            .with_received(name.clone())
            .with_example("feature-auth")
            .with_pattern("^[a-zA-Z][a-zA-Z0-9_-]*$"),
        );
    }

    // Check for reserved names
    let reserved = ["main", "default", "trunk", "master"];
    if reserved.contains(&name.as_str()) {
        result.valid = false;
        result.errors.push(format!("'{name}' is a reserved name"));
        result.hints.push(
            ValidationHint::new("name", "non-reserved name")
                .with_received(name.clone())
                .with_example("feature-auth"),
        );
    }

    // Warn about very long names
    if name.len() > 50 {
        result
            .warnings
            .push("Session name is very long (>50 chars), may cause display issues".to_string());
    }

    result
}

fn validate_remove_args(args: &[String]) -> ValidationResult {
    let mut result = ValidationResult {
        valid: true,
        command: "remove".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
        hints: vec![],
    };

    if args.is_empty() {
        result.valid = false;
        result.errors.push("Session name is required".to_string());
        result
            .hints
            .push(ValidationHint::new("name", "existing session name"));
        return result;
    }

    let name = &args[0];
    result.args.push(ArgValidation {
        name: "name".to_string(),
        value: name.clone(),
        valid: true,
        error: None,
        suggestion: Some("Use --idempotent for safe retries".to_string()),
    });

    result
        .warnings
        .push("This operation is destructive. Use --dry-run to preview.".to_string());

    result
}

fn validate_focus_args(args: &[String]) -> ValidationResult {
    let mut result = ValidationResult {
        valid: true,
        command: "focus".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
        hints: vec![],
    };

    if args.is_empty() {
        result
            .suggestions
            .push("No name provided - will use interactive selection".to_string());
        return result;
    }

    let name = &args[0];
    result.args.push(ArgValidation {
        name: "name".to_string(),
        value: name.clone(),
        valid: true,
        error: None,
        suggestion: None,
    });

    result
}

fn validate_spawn_args(args: &[String]) -> ValidationResult {
    let mut result = ValidationResult {
        valid: true,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
        hints: vec![],
    };

    if args.is_empty() {
        result.valid = false;
        result.errors.push("Bead ID is required".to_string());
        result.hints.push(
            ValidationHint::new("bead_id", "bead ID like isolate-xxxx")
                .with_example("isolate-abc12")
                .with_pattern("^[a-z]+-[a-z0-9]+$"),
        );
        return result;
    }

    let bead_id = &args[0];

    // Validate bead ID format (prefix-id like isolate-abc12)
    let valid = validate_bead_id_format(bead_id);

    result.args.push(ArgValidation {
        name: "bead_id".to_string(),
        value: bead_id.clone(),
        valid,
        error: if valid {
            None
        } else {
            Some("Invalid bead ID format".to_string())
        },
        suggestion: if valid {
            None
        } else {
            Some("Use format: prefix-id (e.g., isolate-abc12)".to_string())
        },
    });

    if !valid {
        result.valid = false;
        result.errors.push(format!("Invalid bead ID: {bead_id}"));
        result.hints.push(
            ValidationHint::new("bead_id", "format: prefix-id")
                .with_received(bead_id.clone())
                .with_example("isolate-abc12"),
        );
    }

    result
}

/// Validate bead ID format (prefix-id like isolate-abc12)
fn validate_bead_id_format(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    let prefix = parts[0];
    let suffix = parts[1];

    // Prefix must be lowercase letters only
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }

    // Suffix must be lowercase alphanumeric
    if suffix.is_empty()
        || !suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return false;
    }

    true
}

fn validate_session_name(name: &str) -> ArgValidation {
    // Session name rules:
    // 1. Must start with a letter
    // 2. Can contain letters, numbers, hyphens, underscores
    // 3. Cannot be empty

    if name.is_empty() {
        return ArgValidation {
            name: "name".to_string(),
            value: name.to_string(),
            valid: false,
            error: Some("Session name cannot be empty".to_string()),
            suggestion: Some("Provide a name like 'feature-auth'".to_string()),
        };
    }

    let first_char = name.chars().next();
    if !first_char.is_some_and(|c| c.is_ascii_alphabetic()) {
        return ArgValidation {
            name: "name".to_string(),
            value: name.to_string(),
            valid: false,
            error: Some("Session name must start with a letter".to_string()),
            suggestion: Some(format!("Try 'x{name}' or 'session-{name}'")),
        };
    }

    let valid_chars = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_chars {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_session_name_valid() {
        let result = validate_session_name("feature-auth");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_session_name_empty() {
        let result = validate_session_name("");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_session_name_starts_with_number() {
        let result = validate_session_name("123-feature");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_session_name_invalid_chars() {
        let result = validate_session_name("feature auth");
        assert!(!result.valid);
    }

    // Tests for bead isolate-33ub: Reject backslash and escape sequences in session names

    #[test]
    fn test_validate_session_name_backslash_n_rejected() {
        let result = validate_session_name("test\\nname");
        assert!(
            !result.valid,
            "Session name with literal backslash-n should be rejected"
        );
        assert!(result.error.is_some());
        if let Some(error) = result.error.as_ref() {
            assert!(
                error.contains("invalid characters"),
                "Error should mention invalid characters"
            );
        }
    }

    #[test]
    fn test_validate_session_name_backslash_t_rejected() {
        let result = validate_session_name("test\\tname");
        assert!(
            !result.valid,
            "Session name with literal backslash-t should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_backslash_r_rejected() {
        let result = validate_session_name("test\\rname");
        assert!(
            !result.valid,
            "Session name with literal backslash-r should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_backslash_zero_rejected() {
        let result = validate_session_name("test\\0name");
        assert!(
            !result.valid,
            "Session name with literal backslash-zero should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_plain_backslash_rejected() {
        let result = validate_session_name("test\\name");
        assert!(
            !result.valid,
            "Session name with plain backslash should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_multiple_backslashes_rejected() {
        let result = validate_session_name("test\\n\\t\\r");
        assert!(
            !result.valid,
            "Session name with multiple escape sequences should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_backslash_at_start_rejected() {
        let result = validate_session_name("\\ntest");
        assert!(
            !result.valid,
            "Session name with backslash at start should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_backslash_at_end_rejected() {
        let result = validate_session_name("test\\n");
        assert!(
            !result.valid,
            "Session name with backslash at end should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_only_backslash_rejected() {
        let result = validate_session_name("\\");
        assert!(
            !result.valid,
            "Session name that is only a backslash should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_mixed_escape_and_valid_rejected() {
        let result = validate_session_name("test-\\n-name");
        assert!(
            !result.valid,
            "Session name with backslash surrounded by valid chars should be rejected"
        );
    }

    #[test]
    fn test_validate_session_name_valid_names_accepted() {
        let valid_names = vec![
            "my-session-123",
            "feature_auth",
            "testSession",
            "abc",
            "test_123",
            "MY-SESSION",
        ];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(
                result.valid,
                "Valid session name '{name}' should be accepted, but was rejected: {:?}",
                result.error
            );
        }
    }

    #[test]
    fn test_validate_add_args_empty() {
        let result = validate_add_args("add", &[]);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_add_args_valid() {
        let result = validate_add_args("add", &["feature-auth".to_string()]);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_add_args_reserved_name() {
        let result = validate_add_args("add", &["main".to_string()]);
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_spawn_args_empty() {
        let result = validate_spawn_args(&[]);
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_spawn_args_valid() {
        let result = validate_spawn_args(&["isolate-abc12".to_string()]);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_spawn_args_invalid_format() {
        let result = validate_spawn_args(&["invalid".to_string()]);
        assert!(!result.valid);
    }
}

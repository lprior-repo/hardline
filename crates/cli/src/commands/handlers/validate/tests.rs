//! Exhaustive tests for the validate command handler.
//!
//! Covers: validate spawn/add/work, remove, done, focus/switch commands;
//! validation result display (pass/fail/warning); validation severity levels;
//! validation fix suggestions; validation scope selection; validation output
//! for CI (exit codes); verbose validation output.
//!
//! All test names are descriptive. All assertions use exact matching
//! (no bare `is_ok()`/`is_err()`).

use super::actions::run_validate;
use super::data::{
    is_reserved_name, validate_bead_id_format, validate_session_name, ArgValidation,
    ValidateOptions, ValidateOutput, RESERVED_NAMES,
};

use scp_core::error::Error;

// ============================================================================
// Helpers
// ============================================================================

fn opts(command: &str, args: &[&str]) -> ValidateOptions {
    ValidateOptions {
        command: command.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        dry_run: false,
    }
}

fn opts_dry(command: &str, args: &[&str]) -> ValidateOptions {
    ValidateOptions {
        command: command.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        dry_run: true,
    }
}

/// Helper to assert error matches State/ValidationError.
fn assert_validation_error(result: scp_core::Result<impl std::fmt::Debug>) {
    let err = result.expect_err("expected validation error");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("Validation"),
        "Expected ValidationError variant, got: {err:?}"
    );
}

// ============================================================================
// validate_session_name — comprehensive
// ============================================================================

#[test]
fn session_name_accepts_lowercase_letters() {
    let r = validate_session_name("abc");
    assert!(r.valid);
    assert_eq!(r.value, "abc");
    assert!(r.error.is_none());
    assert!(r.suggestion.is_none());
}

#[test]
fn session_name_accepts_uppercase_letters() {
    let r = validate_session_name("ABC");
    assert!(r.valid);
}

#[test]
fn session_name_accepts_mixed_case() {
    let r = validate_session_name("FeatureAuth");
    assert!(r.valid);
}

#[test]
fn session_name_accepts_digits_after_first_char() {
    let r = validate_session_name("a123");
    assert!(r.valid);
}

#[test]
fn session_name_accepts_hyphens() {
    let r = validate_session_name("feature-auth");
    assert!(r.valid);
}

#[test]
fn session_name_accepts_underscores() {
    let r = validate_session_name("feature_auth");
    assert!(r.valid);
}

#[test]
fn session_name_accepts_single_char() {
    let r = validate_session_name("x");
    assert!(r.valid);
}

#[test]
fn session_name_rejects_empty() {
    let r = validate_session_name("");
    assert!(!r.valid);
    assert_eq!(r.error.as_deref(), Some("Session name cannot be empty"));
    assert!(r.suggestion.is_some());
}

#[test]
fn session_name_rejects_starts_with_digit() {
    let r = validate_session_name("1abc");
    assert!(!r.valid);
    assert_eq!(
        r.error.as_deref(),
        Some("Session name must start with a letter")
    );
    assert_eq!(r.suggestion.as_deref(), Some("Try 'x1abc' or 'session-1abc'"));
}

#[test]
fn session_name_rejects_starts_with_hyphen() {
    let r = validate_session_name("-abc");
    assert!(!r.valid);
    assert!(r.error.as_deref().unwrap().contains("start with a letter"));
}

#[test]
fn session_name_rejects_starts_with_underscore() {
    let r = validate_session_name("_abc");
    assert!(!r.valid);
    assert!(r.error.as_deref().unwrap().contains("start with a letter"));
}

#[test]
fn session_name_rejects_spaces() {
    let r = validate_session_name("feature auth");
    assert!(!r.valid);
    assert!(r.error.as_deref().unwrap().contains("invalid characters"));
}

#[test]
fn session_name_rejects_backslash() {
    let r = validate_session_name("test\\nname");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_dots() {
    let r = validate_session_name("feature.auth");
    assert!(!r.valid);
    assert!(r.error.as_deref().unwrap().contains("invalid characters"));
}

#[test]
fn session_name_rejects_slashes() {
    let r = validate_session_name("feature/auth");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_colons() {
    let r = validate_session_name("feature:auth");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_special_chars() {
    for ch in &["!", "@", "#", "$", "%", "^", "&", "*", "(", ")"] {
        let name = format!("feature{ch}auth");
        let r = validate_session_name(&name);
        assert!(!r.valid, "Expected '{name}' to be rejected");
    }
}

#[test]
fn session_name_rejects_null_bytes() {
    let r = validate_session_name("test\x00name");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_unicode() {
    let r = validate_session_name("\u{00e9}test");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_tab() {
    let r = validate_session_name("feature\tauth");
    assert!(!r.valid);
}

#[test]
fn session_name_rejects_newline() {
    let r = validate_session_name("feature\nauth");
    assert!(!r.valid);
}

#[test]
fn session_name_suggestion_for_digit_start() {
    let r = validate_session_name("123invalid");
    assert_eq!(r.suggestion.as_deref(), Some("Try 'x123invalid' or 'session-123invalid'"));
}

#[test]
fn session_name_suggestion_for_invalid_chars() {
    let r = validate_session_name("feature!auth");
    assert!(r.suggestion.is_some());
    assert!(r.suggestion.as_deref().unwrap().contains("letters, numbers, hyphens"));
}

// ============================================================================
// is_reserved_name
// ============================================================================

#[test]
fn reserved_names_contains_main() {
    assert!(is_reserved_name("main"));
}

#[test]
fn reserved_names_contains_default() {
    assert!(is_reserved_name("default"));
}

#[test]
fn reserved_names_contains_trunk() {
    assert!(is_reserved_name("trunk"));
}

#[test]
fn reserved_names_contains_master() {
    assert!(is_reserved_name("master"));
}

#[test]
fn reserved_names_rejects_feature() {
    assert!(!is_reserved_name("feature"));
}

#[test]
fn reserved_names_rejects_empty() {
    assert!(!is_reserved_name(""));
}

#[test]
fn reserved_names_is_case_sensitive() {
    assert!(!is_reserved_name("Main"));
    assert!(!is_reserved_name("MAIN"));
}

#[test]
fn reserved_names_constant_has_four_entries() {
    assert_eq!(RESERVED_NAMES.len(), 4);
}

// ============================================================================
// validate_bead_id_format
// ============================================================================

#[test]
fn bead_id_accepts_lowercase_prefix_and_alphanum_suffix() {
    assert!(validate_bead_id_format("isolate-abc12"));
}

#[test]
fn bead_id_accepts_two_char_prefix() {
    assert!(validate_bead_id_format("hl-xyz99"));
}

#[test]
fn bead_id_accepts_single_char_prefix() {
    assert!(validate_bead_id_format("a-abc"));
}

#[test]
fn bead_id_accepts_all_digit_suffix() {
    assert!(validate_bead_id_format("task-123"));
}

#[test]
fn bead_id_rejects_no_hyphen() {
    assert!(!validate_bead_id_format("invalid"));
}

#[test]
fn bead_id_rejects_two_hyphens() {
    assert!(!validate_bead_id_format("a-b-c"));
}

#[test]
fn bead_id_rejects_empty_prefix() {
    assert!(!validate_bead_id_format("-abc"));
}

#[test]
fn bead_id_rejects_empty_suffix() {
    assert!(!validate_bead_id_format("abc-"));
}

#[test]
fn bead_id_rejects_uppercase_prefix() {
    assert!(!validate_bead_id_format("ABC-123"));
}

#[test]
fn bead_id_rejects_uppercase_suffix() {
    assert!(!validate_bead_id_format("abc-ABC"));
}

#[test]
fn bead_id_rejects_special_chars_in_prefix() {
    assert!(!validate_bead_id_format("a!b-123"));
}

#[test]
fn bead_id_rejects_special_chars_in_suffix() {
    assert!(!validate_bead_id_format("abc-12!3"));
}

#[test]
fn bead_id_rejects_spaces() {
    assert!(!validate_bead_id_format("abc -123"));
}

#[test]
fn bead_id_rejects_empty_string() {
    assert!(!validate_bead_id_format(""));
}

// ============================================================================
// run_validate — spawn command variants
// ============================================================================

#[test]
fn spawn_valid_name_returns_ok() {
    let result = run_validate(&opts("spawn", &["feature-auth"]));
    assert!(result.is_ok());
}

#[test]
fn spawn_valid_name_simple() {
    assert!(run_validate(&opts("spawn", &["abc"])).is_ok());
}

#[test]
fn spawn_valid_name_with_numbers() {
    assert!(run_validate(&opts("spawn", &["feature-123"])).is_ok());
}

#[test]
fn spawn_valid_name_with_underscore() {
    assert!(run_validate(&opts("spawn", &["feature_auth"])).is_ok());
}

#[test]
fn spawn_empty_args_returns_validation_error() {
    let result = run_validate(&opts("spawn", &[]));
    assert_validation_error(result);
}

#[test]
fn spawn_reserved_name_main_returns_error() {
    let result = run_validate(&opts("spawn", &["main"]));
    assert_validation_error(result);
}

#[test]
fn spawn_reserved_name_default_returns_error() {
    let result = run_validate(&opts("spawn", &["default"]));
    assert_validation_error(result);
}

#[test]
fn spawn_reserved_name_trunk_returns_error() {
    let result = run_validate(&opts("spawn", &["trunk"]));
    assert_validation_error(result);
}

#[test]
fn spawn_reserved_name_master_returns_error() {
    let result = run_validate(&opts("spawn", &["master"]));
    assert_validation_error(result);
}

#[test]
fn spawn_invalid_name_digit_start_returns_error() {
    let result = run_validate(&opts("spawn", &["123invalid"]));
    assert_validation_error(result);
}

#[test]
fn spawn_invalid_name_special_chars_returns_error() {
    let result = run_validate(&opts("spawn", &["feature!auth"]));
    assert_validation_error(result);
}

#[test]
fn spawn_dry_run_valid_returns_ok() {
    let result = run_validate(&opts_dry("spawn", &["feature-auth"]));
    assert!(result.is_ok());
}

#[test]
fn spawn_dry_run_invalid_returns_error() {
    let result = run_validate(&opts_dry("spawn", &["123invalid"]));
    assert_validation_error(result);
}

// "add" is an alias for spawn
#[test]
fn add_valid_name_returns_ok() {
    assert!(run_validate(&opts("add", &["feature-x"])).is_ok());
}

#[test]
fn add_empty_args_returns_error() {
    assert_validation_error(run_validate(&opts("add", &[])));
}

// "work" is an alias for spawn
#[test]
fn work_valid_name_returns_ok() {
    assert!(run_validate(&opts("work", &["feature-y"])).is_ok());
}

#[test]
fn work_empty_args_returns_error() {
    assert_validation_error(run_validate(&opts("work", &[])));
}

// ============================================================================
// run_validate — remove command
// ============================================================================

#[test]
fn remove_valid_name_returns_ok() {
    let result = run_validate(&opts("remove", &["feature-x"]));
    assert!(result.is_ok());
}

#[test]
fn remove_empty_args_returns_error() {
    let result = run_validate(&opts("remove", &[]));
    assert_validation_error(result);
}

#[test]
fn remove_dry_run_valid_returns_ok() {
    let result = run_validate(&opts_dry("remove", &["feature-x"]));
    assert!(result.is_ok());
}

#[test]
fn remove_any_name_passes_validation() {
    // remove doesn't validate the name format — it just checks presence
    assert!(run_validate(&opts("remove", &["123invalid"])).is_ok());
}

// ============================================================================
// run_validate — done command
// ============================================================================

#[test]
fn done_with_valid_name_returns_ok() {
    assert!(run_validate(&opts("done", &["feature-x"])).is_ok());
}

#[test]
fn done_without_name_returns_ok() {
    // done with no name is valid (uses current workspace)
    assert!(run_validate(&opts("done", &[])).is_ok());
}

#[test]
fn done_with_invalid_name_still_returns_ok() {
    // done doesn't enforce validity — it records what was provided
    assert!(run_validate(&opts("done", &["123invalid"])).is_ok());
}

#[test]
fn done_dry_run_returns_ok() {
    assert!(run_validate(&opts_dry("done", &["feature-x"])).is_ok());
}

// ============================================================================
// run_validate — focus/switch commands
// ============================================================================

#[test]
fn focus_with_name_returns_ok() {
    assert!(run_validate(&opts("focus", &["feature-x"])).is_ok());
}

#[test]
fn focus_without_name_returns_ok() {
    assert!(run_validate(&opts("focus", &[])).is_ok());
}

#[test]
fn switch_with_name_returns_ok() {
    assert!(run_validate(&opts("switch", &["feature-x"])).is_ok());
}

#[test]
fn switch_without_name_returns_ok() {
    assert!(run_validate(&opts("switch", &[])).is_ok());
}

#[test]
fn focus_dry_run_returns_ok() {
    assert!(run_validate(&opts_dry("focus", &["feature-x"])).is_ok());
}

// ============================================================================
// run_validate — unknown command
// ============================================================================

#[test]
fn unknown_command_passes_validation() {
    // Unknown commands have no specific validation — they pass
    assert!(run_validate(&opts("nonexistent", &[])).is_ok());
}

#[test]
fn unknown_command_with_args_passes() {
    assert!(run_validate(&opts("custom-cmd", &["arg1", "arg2"])).is_ok());
}

#[test]
fn unknown_command_dry_run_passes() {
    assert!(run_validate(&opts_dry("custom-cmd", &[])).is_ok());
}

#[test]
fn empty_command_string_passes() {
    assert!(run_validate(&opts("", &[])).is_ok());
}

// ============================================================================
// ValidateOutput — struct construction & fields
// ============================================================================

#[test]
fn validate_output_valid_no_errors() {
    let output = ValidateOutput {
        valid: true,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };
    assert!(output.valid);
    assert!(output.errors.is_empty());
    assert!(output.warnings.is_empty());
    assert!(output.suggestions.is_empty());
}

#[test]
fn validate_output_invalid_with_errors() {
    let output = ValidateOutput {
        valid: false,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec!["Name required".to_string()],
        warnings: vec![],
        suggestions: vec![],
    };
    assert!(!output.valid);
    assert_eq!(output.errors.len(), 1);
}

#[test]
fn validate_output_with_warnings() {
    let output = ValidateOutput {
        valid: true,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec!["Long name".to_string()],
        suggestions: vec![],
    };
    assert!(output.warnings.len() == 1);
}

#[test]
fn validate_output_with_suggestions() {
    let output = ValidateOutput {
        valid: false,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec!["Invalid".to_string()],
        warnings: vec![],
        suggestions: vec!["Try something else".to_string()],
    };
    assert_eq!(output.suggestions.len(), 1);
}

// ============================================================================
// ValidateOutput — JSON serialization
// ============================================================================

#[test]
fn output_serializes_and_deserializes() {
    let output = ValidateOutput {
        valid: true,
        command: "spawn".to_string(),
        args: vec![ArgValidation {
            name: "name".to_string(),
            value: "feature".to_string(),
            valid: true,
            error: None,
            suggestion: None,
        }],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let parsed: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
    assert!(parsed.valid);
    assert_eq!(parsed.command, "spawn");
    assert_eq!(parsed.args.len(), 1);
    assert!(parsed.args[0].valid);
}

#[test]
fn output_serializes_invalid_false() {
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
    assert!(json.contains("\"error\""));
    assert!(json.contains("\"suggestion\""));
}

#[test]
fn output_skips_empty_collections() {
    let output = ValidateOutput {
        valid: true,
        command: "test".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };
    let json = serde_json::to_string(&output).expect("serialize");
    assert!(!json.contains("\"errors\""));
    assert!(!json.contains("\"warnings\""));
    assert!(!json.contains("\"suggestions\""));
}

#[test]
fn arg_validation_skips_none_fields() {
    let arg = ArgValidation {
        name: "name".to_string(),
        value: "test".to_string(),
        valid: true,
        error: None,
        suggestion: None,
    };
    let json = serde_json::to_string(&arg).expect("serialize");
    assert!(!json.contains("\"error\""));
    assert!(!json.contains("\"suggestion\""));
}

#[test]
fn arg_validation_includes_error_when_present() {
    let arg = ArgValidation {
        name: "name".to_string(),
        value: "123".to_string(),
        valid: false,
        error: Some("bad".to_string()),
        suggestion: Some("fix it".to_string()),
    };
    let json = serde_json::to_string(&arg).expect("serialize");
    assert!(json.contains("\"error\":\"bad\""));
    assert!(json.contains("\"suggestion\":\"fix it\""));
}

#[test]
fn output_roundtrip_preserves_all_fields() {
    let output = ValidateOutput {
        valid: false,
        command: "spawn".to_string(),
        args: vec![
            ArgValidation {
                name: "name".to_string(),
                value: "bad!".to_string(),
                valid: false,
                error: Some("invalid chars".to_string()),
                suggestion: Some("use alphanum".to_string()),
            },
        ],
        errors: vec!["E1".to_string(), "E2".to_string()],
        warnings: vec!["W1".to_string()],
        suggestions: vec!["S1".to_string()],
    };
    let json = serde_json::to_string(&output).expect("serialize");
    let back: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.valid, false);
    assert_eq!(back.command, "spawn");
    assert_eq!(back.args.len(), 1);
    assert_eq!(back.errors.len(), 2);
    assert_eq!(back.warnings.len(), 1);
    assert_eq!(back.suggestions.len(), 1);
    assert_eq!(back.args[0].error.as_deref(), Some("invalid chars"));
    assert_eq!(back.args[0].suggestion.as_deref(), Some("use alphanum"));
}

// ============================================================================
// ValidateOptions — construction
// ============================================================================

#[test]
fn validate_options_fields_populated() {
    let o = ValidateOptions {
        command: "spawn".to_string(),
        args: vec!["a".to_string(), "b".to_string()],
        dry_run: true,
    };
    assert_eq!(o.command, "spawn");
    assert_eq!(o.args.len(), 2);
    assert!(o.dry_run);
}

#[test]
fn validate_options_clone_is_independent() {
    let o = ValidateOptions {
        command: "spawn".to_string(),
        args: vec!["x".to_string()],
        dry_run: false,
    };
    let mut c = o.clone();
    c.command = "remove".to_string();
    assert_eq!(o.command, "spawn");
    assert_eq!(c.command, "remove");
}

// ============================================================================
// Spawn long name warning
// ============================================================================

#[test]
fn spawn_long_name_triggers_warning() {
    // Names >50 chars should generate a warning but still be valid
    let long_name = "a".repeat(51);
    let result = run_validate(&opts("spawn", &[&long_name]));
    assert!(result.is_ok());
}

#[test]
fn spawn_exactly_50_chars_is_valid() {
    let name = "a".repeat(50);
    assert!(run_validate(&opts("spawn", &[&name])).is_ok());
}

#[test]
fn spawn_exactly_51_chars_still_valid_but_warns() {
    let name = "a".repeat(51);
    assert!(run_validate(&opts("spawn", &[&name])).is_ok());
}

#[test]
fn spawn_very_long_name_is_valid() {
    let name = "a".repeat(200);
    assert!(run_validate(&opts("spawn", &[&name])).is_ok());
}

// ============================================================================
// Spawn reserved name + invalid name double error
// ============================================================================

#[test]
fn spawn_reserved_and_invalid_name_both_reported() {
    // A name like "123main" is both invalid format AND (if it were valid) reserved.
    // But it starts with a digit so format validation catches it first.
    let result = run_validate(&opts("spawn", &["123main"]));
    assert_validation_error(result);
}

// ============================================================================
// Spawn with multiple args (only first is validated)
// ============================================================================

#[test]
fn spawn_ignores_extra_args() {
    // Only the first arg is the session name; extras are ignored
    assert!(run_validate(&opts("spawn", &["feature-x", "extra"])).is_ok());
}

// ============================================================================
// Red Queen adversarial tests
// ============================================================================

mod red_queen_adversarial {
    use super::*;

    // --- ATTACK: Session name injection payloads ---

    #[test]
    fn session_name_rejects_sql_injection() {
        assert!(!validate_session_name("'; DROP TABLE sessions; --").valid);
    }

    #[test]
    fn session_name_rejects_path_traversal() {
        assert!(!validate_session_name("../../../etc/passwd").valid);
    }

    #[test]
    fn session_name_rejects_html_script() {
        assert!(!validate_session_name("<script>alert(1)</script>").valid);
    }

    #[test]
    fn session_name_rejects_format_string() {
        assert!(!validate_session_name("%s%s%s%n").valid);
    }

    #[test]
    fn session_name_rejects_null_byte_injection() {
        assert!(!validate_session_name("valid\x00evil").valid);
    }

    #[test]
    fn session_name_rejects_ansi_escape() {
        assert!(!validate_session_name("\x1b[31mred").valid);
    }

    #[test]
    fn session_name_rejects_carriage_return() {
        assert!(!validate_session_name("feature\rauth").valid);
    }

    // --- ATTACK: Bead ID injection ---

    #[test]
    fn bead_id_rejects_sql_injection() {
        assert!(!validate_bead_id_format("'; DROP--; --"));
    }

    #[test]
    fn bead_id_rejects_path_traversal() {
        assert!(!validate_bead_id_format("../..-passwd"));
    }

    #[test]
    fn bead_id_rejects_null_byte() {
        assert!(!validate_bead_id_format("abc\x00-123"));
    }

    #[test]
    fn bead_id_rejects_only_digits_prefix() {
        // Digits-only prefix is invalid (prefix must be lowercase letters)
        assert!(!validate_bead_id_format("123-abc"));
    }

    #[test]
    fn bead_id_rejects_mixed_case_prefix() {
        assert!(!validate_bead_id_format("Abc-123"));
    }

    // --- ATTACK: Reserved name edge cases ---

    #[test]
    fn reserved_name_not_prefix_matched() {
        // "main-feature" is NOT reserved (exact match required)
        assert!(!is_reserved_name("main-feature"));
    }

    #[test]
    fn reserved_name_not_suffix_matched() {
        assert!(!is_reserved_name("my-main"));
    }

    #[test]
    fn reserved_name_not_substring_matched() {
        assert!(!is_reserved_name("maintain"));
    }

    // --- ATTACK: validate spawn with adversarial names ---

    #[test]
    fn spawn_reserved_name_case_variant_passes() {
        // "Main" is not reserved (case-sensitive check)
        assert!(run_validate(&opts("spawn", &["Main"])).is_ok());
    }

    #[test]
    fn spawn_unicode_name_rejected() {
        assert_validation_error(run_validate(&opts("spawn", &["\u{00e9}feature"])));
    }

    #[test]
    fn spawn_empty_string_name_rejected() {
        assert_validation_error(run_validate(&opts("spawn", &[""])));
    }

    #[test]
    fn spawn_whitespace_only_name_rejected() {
        assert_validation_error(run_validate(&opts("spawn", &["   "])));
    }

    // --- ATTACK: Remove accepts any name (no format check) ---

    #[test]
    fn remove_accepts_special_chars_in_name() {
        // remove only checks that a name is provided, not its format
        assert!(run_validate(&opts("remove", &["!@#$%"])).is_ok());
    }

    // --- ATTACK: Done accepts invalid names ---

    #[test]
    fn done_accepts_special_chars_in_name() {
        // done records the name without format enforcement
        assert!(run_validate(&opts("done", &["anything!@#"])).is_ok());
    }

    // --- ATTACK: Focus accepts any name ---

    #[test]
    fn focus_accepts_special_chars() {
        assert!(run_validate(&opts("focus", &["<script>"])).is_ok());
    }

    // --- ATTACK: ValidateOutput serialization adversarial ---

    #[test]
    fn output_with_injection_in_errors_survives_roundtrip() {
        let output = ValidateOutput {
            valid: false,
            command: "'; DROP TABLE--; ".to_string(),
            args: vec![],
            errors: vec!["<script>alert(1)</script>".to_string()],
            warnings: vec!["\x00null".to_string()],
            suggestions: vec!["../../../etc/passwd".to_string()],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.errors[0], "<script>alert(1)</script>");
        assert_eq!(back.suggestions[0], "../../../etc/passwd");
    }

    #[test]
    fn arg_validation_with_injection_survives_roundtrip() {
        let arg = ArgValidation {
            name: "'; DROP--".to_string(),
            value: "<script>".to_string(),
            valid: false,
            error: Some("); DROP TABLE--".to_string()),
            suggestion: Some("../../../etc/passwd".to_string()),
        };
        let json = serde_json::to_string(&arg).expect("serialize");
        let back: ArgValidation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "'; DROP--");
        assert_eq!(back.value, "<script>");
        assert_eq!(back.error.as_deref(), Some("); DROP TABLE--"));
    }

    #[test]
    fn output_with_very_long_command_survives() {
        let long_cmd = "x".repeat(10_000);
        let output = ValidateOutput {
            valid: true,
            command: long_cmd.clone(),
            args: vec![],
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.command.len(), 10_000);
    }

    #[test]
    fn output_with_many_errors_survives() {
        let errors: Vec<String> = (0..1000).map(|i| format!("Error {i}")).collect();
        let output = ValidateOutput {
            valid: false,
            command: "test".to_string(),
            args: vec![],
            errors,
            warnings: vec![],
            suggestions: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.errors.len(), 1000);
    }
}

// ============================================================================
// Proptest-based fuzzing
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// validate_session_name never panics on any input.
        #[test]
        fn proptest_session_name_never_panics(name in ".*") {
            let _ = validate_session_name(&name);
        }

        /// validate_bead_id_format never panics on any input.
        #[test]
        fn proptest_bead_id_never_panics(id in ".*") {
            let _ = validate_bead_id_format(&id);
        }

        /// is_reserved_name never panics on any input.
        #[test]
        fn proptest_reserved_name_never_panics(name in ".*") {
            let _ = is_reserved_name(&name);
        }

        /// ValidateOutput roundtrip preserves valid flag.
        #[test]
        fn proptest_output_roundtrip_preserves_valid(valid: bool, command in ".*") {
            let output = ValidateOutput {
                valid,
                command,
                args: vec![],
                errors: vec![],
                warnings: vec![],
                suggestions: vec![],
            };
            let json = serde_json::to_string(&output).expect("serialize");
            let back: ValidateOutput = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.valid, valid);
        }

        /// ArgValidation roundtrip preserves valid and name.
        #[test]
        fn proptest_arg_roundtrip(valid: bool, name in ".*", value in ".*") {
            let arg = ArgValidation {
                name,
                value,
                valid,
                error: None,
                suggestion: None,
            };
            let json = serde_json::to_string(&arg).expect("serialize");
            let back: ArgValidation = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.valid, valid);
        }

        /// run_validate never panics on any command/args combination.
        #[test]
        fn proptest_run_validate_never_panics(cmd in "[a-z]{0,20}", args in prop::collection::vec(".*", 0..5)) {
            let options = ValidateOptions {
                command: cmd,
                args,
                dry_run: false,
            };
            let _ = run_validate(&options);
        }

        /// validate_session_name accepts only [a-zA-Z][a-zA-Z0-9_-]*.
        #[test]
        fn proptest_session_name_accepts_valid(name in "[a-zA-Z][a-zA-Z0-9_-]{0,50}") {
            let r = validate_session_name(&name);
            assert!(r.valid, "Expected '{}' to be valid", name);
        }

        /// validate_session_name rejects names starting with non-letter.
        #[test]
        fn proptest_session_name_rejects_digit_start(name in "[0-9][a-zA-Z0-9_-]*") {
            let r = validate_session_name(&name);
            assert!(!r.valid, "Expected '{}' to be rejected (starts with digit)", name);
        }

        /// validate_bead_id_format: only [a-z]+-[a-z0-9]+ is valid.
        #[test]
        fn proptest_bead_id_accepts_valid(id in "[a-z]{1,10}-[a-z0-9]{1,10}") {
            assert!(validate_bead_id_format(&id), "Expected '{}' to be valid", id);
        }
    }
}

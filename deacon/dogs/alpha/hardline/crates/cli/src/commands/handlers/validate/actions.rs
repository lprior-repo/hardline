//! Action functions for the validate command handler (Tier 3).
//!
//! I/O operations that validate command inputs.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    is_reserved_name, validate_bead_id_format, validate_session_name, ArgValidation,
    ValidateOptions, ValidateOutput,
};

/// Execute the validate command.
///
/// # Errors
///
/// Returns error if validation fails.
pub fn run_validate(options: &ValidateOptions) -> Result<()> {
    let output = validate_command(&options.command, &options.args);

    if options.dry_run {
        Output::info("[dry-run] Validation preview:");
    }

    if output.valid {
        Output::success(&format!("All inputs valid for '{}'", output.command));
    } else {
        Output::info(&format!("Validation failed for '{}'", output.command));
        for arg in &output.args {
            if arg.valid {
                Output::info(&format!("  {}: {}", arg.name, arg.value));
            } else {
                Output::info(&format!("  {}: {} (INVALID)", arg.name, arg.value));
                if let Some(err) = &arg.error {
                    Output::info(&format!("    Error: {err}"));
                }
                if let Some(sugg) = &arg.suggestion {
                    Output::info(&format!("    Suggestion: {sugg}"));
                }
            }
        }
        for err in &output.errors {
            Output::info(&format!("  Error: {err}"));
        }
    }

    if output.valid {
        Ok(())
    } else {
        Err(Error::validation_error("Validation failed"))
    }
}

/// Validate command inputs and return structured output.
fn validate_command(command: &str, args: &[String]) -> ValidateOutput {
    match command {
        "spawn" | "add" | "work" => validate_spawn_args(args),
        "remove" => validate_remove_args(args),
        "done" => validate_done_args(args),
        "focus" | "switch" => validate_focus_args(args),
        _ => ValidateOutput {
            valid: true,
            command: command.to_string(),
            args: vec![],
            errors: vec![],
            warnings: vec![format!("No specific validation for command '{command}'")],
            suggestions: vec![],
        },
    }
}

fn validate_spawn_args(args: &[String]) -> ValidateOutput {
    let mut output = ValidateOutput {
        valid: true,
        command: "spawn".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };

    if args.is_empty() {
        output.valid = false;
        output.errors.push("Session name is required".to_string());
        return output;
    }

    let name = &args[0];
    let name_validation = validate_session_name(name);
    output.args.push(name_validation.clone());

    if !name_validation.valid {
        output.valid = false;
        if let Some(err) = &name_validation.error {
            output.errors.push(err.clone());
        }
    }

    if is_reserved_name(name) {
        output.valid = false;
        output.errors.push(format!("'{name}' is a reserved name"));
    }

    if name.len() > 50 {
        output
            .warnings
            .push("Session name is very long (>50 chars)".to_string());
    }

    output
}

fn validate_remove_args(args: &[String]) -> ValidateOutput {
    let mut output = ValidateOutput {
        valid: true,
        command: "remove".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };

    if args.is_empty() {
        output.valid = false;
        output.errors.push("Session name is required".to_string());
        return output;
    }

    output.args.push(ArgValidation {
        name: "name".to_string(),
        value: args[0].clone(),
        valid: true,
        error: None,
        suggestion: Some("Use --force for destructive operation".to_string()),
    });

    output
        .warnings
        .push("This operation is destructive. Use --dry-run to preview.".to_string());

    output
}

fn validate_done_args(args: &[String]) -> ValidateOutput {
    let mut output = ValidateOutput {
        valid: true,
        command: "done".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };

    if let Some(name) = args.first() {
        let name_validation = validate_session_name(name);
        output.args.push(name_validation);
    }

    output
}

fn validate_focus_args(args: &[String]) -> ValidateOutput {
    let mut output = ValidateOutput {
        valid: true,
        command: "focus".to_string(),
        args: vec![],
        errors: vec![],
        warnings: vec![],
        suggestions: vec![],
    };

    if let Some(name) = args.first() {
        output.args.push(ArgValidation {
            name: "name".to_string(),
            value: name.clone(),
            valid: true,
            error: None,
            suggestion: None,
        });
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_validate_spawn_valid() {
        let options = ValidateOptions {
            command: "spawn".to_string(),
            args: vec!["feature-auth".to_string()],
            dry_run: false,
        };
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn run_validate_spawn_empty() {
        let options = ValidateOptions {
            command: "spawn".to_string(),
            args: vec![],
            dry_run: false,
        };
        assert!(run_validate(&options).is_err());
    }

    #[test]
    fn run_validate_spawn_reserved() {
        let options = ValidateOptions {
            command: "spawn".to_string(),
            args: vec!["main".to_string()],
            dry_run: false,
        };
        assert!(run_validate(&options).is_err());
    }

    #[test]
    fn run_validate_spawn_invalid_name() {
        let options = ValidateOptions {
            command: "spawn".to_string(),
            args: vec!["123invalid".to_string()],
            dry_run: false,
        };
        assert!(run_validate(&options).is_err());
    }

    #[test]
    fn run_validate_dry_run() {
        let options = ValidateOptions {
            command: "spawn".to_string(),
            args: vec!["feature-auth".to_string()],
            dry_run: true,
        };
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn run_validate_unknown_command() {
        let options = ValidateOptions {
            command: "nonexistent".to_string(),
            args: vec![],
            dry_run: false,
        };
        // Unknown commands have no specific validation, so they pass
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn run_validate_remove_valid() {
        let options = ValidateOptions {
            command: "remove".to_string(),
            args: vec!["feature-x".to_string()],
            dry_run: false,
        };
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn run_validate_remove_empty() {
        let options = ValidateOptions {
            command: "remove".to_string(),
            args: vec![],
            dry_run: false,
        };
        assert!(run_validate(&options).is_err());
    }

    #[test]
    fn run_validate_done_with_name() {
        let options = ValidateOptions {
            command: "done".to_string(),
            args: vec!["feature-x".to_string()],
            dry_run: false,
        };
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn run_validate_done_no_name() {
        let options = ValidateOptions {
            command: "done".to_string(),
            args: vec![],
            dry_run: false,
        };
        // done with no name is ok (uses current workspace)
        assert!(run_validate(&options).is_ok());
    }

    #[test]
    fn validate_bead_id_format_in_actions() {
        assert!(validate_bead_id_format("isolate-abc12"));
        assert!(validate_bead_id_format("hl-xyz99"));
        assert!(!validate_bead_id_format("invalid"));
        assert!(!validate_bead_id_format("a-b-c"));
    }
}

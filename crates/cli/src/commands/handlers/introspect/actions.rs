//! Action functions for the introspect command handler (Tier 3).
//!
//! I/O operations that display introspection data about hardline capabilities.

use scp_core::{output::Output, Error, Result};

use super::data::{known_commands, CommandInfo, IntrospectOptions, IntrospectTarget};

/// Pure function: resolve a command name to its metadata.
///
/// Returns `Some(CommandInfo)` if found, `None` otherwise.
/// No I/O side effects.
pub fn resolve_command(name: &str) -> Option<CommandInfo> {
    known_commands().into_iter().find(|c| c.name == name)
}

/// Execute the introspect command with the given options.
///
/// When target is `IntrospectTarget::Specific(command)`, displays detailed
/// introspection for that command. When `IntrospectTarget::All`, lists all
/// known commands.
///
/// # Errors
///
/// Returns `Error::not_found` if the requested command is not in the registry.
pub fn run_introspect(options: &IntrospectOptions) -> Result<()> {
    match &options.target {
        IntrospectTarget::Specific(command_name) => {
            let cmd = resolve_command(command_name).ok_or_else(|| {
                Error::not_found(format!(
                    "Unknown command '{command_name}'. Use 'scp introspect' to list all commands."
                ))
            })?;
            print_command_detail(&cmd);
        }
        IntrospectTarget::All => {
            print_all_commands();
        }
    }
    Ok(())
}

/// Print a summary listing of all known commands.
fn print_all_commands() {
    let commands = known_commands();
    Output::info(&format!(
        "Hardline Capabilities ({} commands):",
        commands.len()
    ));
    Output::info("");
    commands.iter().for_each(print_command_summary_line);
}

/// Print a single summary line for a command in the listing.
fn print_command_summary_line(cmd: &CommandInfo) {
    let aliases = if cmd.aliases.is_empty() {
        String::new()
    } else {
        format!(" ({})", cmd.aliases.join(", "))
    };
    Output::info(&format!("  {}{} - {}", cmd.name, aliases, cmd.description));
}

/// Print detailed information about a single command.
fn print_command_detail(cmd: &CommandInfo) {
    print_command_header(cmd);
    print_command_arguments(cmd);
    print_command_flags(cmd);
    print_command_examples(cmd);
    print_command_errors(cmd);
}

/// Print command name, description, aliases, and requirements.
fn print_command_header(cmd: &CommandInfo) {
    Output::info(&format!("Command: {}", cmd.name));
    Output::info(&format!("Description: {}", cmd.description));
    if !cmd.aliases.is_empty() {
        Output::info(&format!("Aliases: {}", cmd.aliases.join(", ")));
    }
    Output::info(&format!(
        "Requires init: {}",
        if cmd.requires_init { "yes" } else { "no" }
    ));
    Output::info(&format!(
        "Requires git: {}",
        if cmd.requires_git { "yes" } else { "no" }
    ));
}

/// Print argument details for a command.
fn print_command_arguments(cmd: &CommandInfo) {
    if cmd.arguments.is_empty() {
        return;
    }
    Output::info("Arguments:");
    cmd.arguments.iter().for_each(|arg| {
        let required = if arg.required { "required" } else { "optional" };
        Output::info(&format!(
            "  {} ({}, {}) - {}",
            arg.name, arg.arg_type, required, arg.description
        ));
        if !arg.examples.is_empty() {
            Output::info(&format!("    Examples: {}", arg.examples.join(", ")));
        }
    });
}

/// Print flag details for a command.
fn print_command_flags(cmd: &CommandInfo) {
    if cmd.flags.is_empty() {
        return;
    }
    Output::info("Flags:");
    cmd.flags.iter().for_each(|flag| {
        let short = flag
            .short
            .as_ref()
            .map(|s| format!("-{s}, "))
            .unwrap_or_default();
        Output::info(&format!("  {short}--{}", flag.long));
        Output::info(&format!("    Type: {}", flag.flag_type));
        Output::info(&format!("    Description: {}", flag.description));
        if let Some(ref default) = flag.default {
            Output::info(&format!("    Default: {default}"));
        }
    });
}

/// Print usage examples for a command.
fn print_command_examples(cmd: &CommandInfo) {
    if cmd.examples.is_empty() {
        return;
    }
    Output::info("Examples:");
    cmd.examples.iter().for_each(|example| {
        Output::info(&format!("  {}", example.command));
        Output::info(&format!("    {}", example.description));
    });
    if !cmd.side_effects.is_empty() {
        Output::info(&format!("Side effects: {}", cmd.side_effects.join(", ")));
    }
}

/// Print error conditions for a command.
fn print_command_errors(cmd: &CommandInfo) {
    if cmd.error_conditions.is_empty() {
        return;
    }
    Output::info("Error conditions:");
    cmd.error_conditions.iter().for_each(|ec| {
        Output::info(&format!("  {} - {}", ec.code, ec.description));
        Output::info(&format!("    Resolution: {}", ec.resolution));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_introspect_list_all() {
        let options = IntrospectOptions {
            target: IntrospectTarget::All,
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_specific_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("add".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_unknown_command_returns_error() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("nonexistent".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_err());
    }

    #[test]
    fn run_introspect_init_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("init".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_done_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("done".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_remove_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("remove".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_list_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("list".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_status_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("status".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_sync_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("sync".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_diff_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("diff".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_introspect_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("introspect".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_doctor_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("doctor".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_query_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("query".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_revert_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("revert".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn error_message_for_unknown_command() {
        let options = IntrospectOptions {
            target: IntrospectTarget::Specific("foobar".to_string()),
        };
        let result = run_introspect(&options);
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("foobar"));
        assert!(err_msg.contains("Unknown command"));
    }

    #[test]
    fn resolve_command_finds_known_command() {
        let cmd = resolve_command("add");
        assert!(cmd.is_some());
        assert_eq!(cmd.map(|c| c.name), Some("add".to_string()));
    }

    #[test]
    fn resolve_command_returns_none_for_unknown() {
        let cmd = resolve_command("nonexistent");
        assert!(cmd.is_none());
    }
}

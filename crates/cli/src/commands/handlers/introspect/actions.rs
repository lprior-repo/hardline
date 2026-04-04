//! Action functions for the introspect command handler (Tier 3).
//!
//! I/O operations that display introspection data about hardline capabilities.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{known_commands, IntrospectOptions};

/// Execute the introspect command with the given options.
///
/// When `options.target` is `Some(command)`, displays detailed introspection
/// for that specific command. When `None`, lists all known commands.
///
/// # Errors
///
/// Returns `Error::not_found` if the requested command is not in the registry.
pub fn run_introspect(options: &IntrospectOptions) -> Result<()> {
    let commands = known_commands();

    match &options.target {
        Some(command_name) => {
            let cmd = commands
                .iter()
                .find(|c| c.name == *command_name)
                .ok_or_else(|| {
                    Error::not_found(format!(
                        "Unknown command '{command_name}'. Use 'scp introspect' to list all commands."
                    ))
                })?;

            print_command_detail(cmd);
        }
        None => {
            Output::info(&format!("Hardline Capabilities ({} commands):", commands.len()));
            Output::info("");
            for cmd in &commands {
                let aliases = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", cmd.aliases.join(", "))
                };
                Output::info(&format!("  {}{} - {}", cmd.name, aliases, cmd.description));
            }
        }
    }

    Ok(())
}

/// Print detailed information about a single command.
fn print_command_detail(cmd: &super::data::CommandInfo) {
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

    if !cmd.arguments.is_empty() {
        Output::info("Arguments:");
        for arg in &cmd.arguments {
            let required = if arg.required { "required" } else { "optional" };
            Output::info(&format!(
                "  {} ({}, {}) - {}",
                arg.name, arg.arg_type, required, arg.description
            ));
            if !arg.examples.is_empty() {
                Output::info(&format!("    Examples: {}", arg.examples.join(", ")));
            }
        }
    }

    if !cmd.flags.is_empty() {
        Output::info("Flags:");
        for flag in &cmd.flags {
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
        }
    }

    if !cmd.examples.is_empty() {
        Output::info("Examples:");
        for example in &cmd.examples {
            Output::info(&format!("  {}", example.command));
            Output::info(&format!("    {}", example.description));
        }
    }

    if !cmd.side_effects.is_empty() {
        Output::info(&format!("Side effects: {}", cmd.side_effects.join(", ")));
    }

    if !cmd.error_conditions.is_empty() {
        Output::info("Error conditions:");
        for ec in &cmd.error_conditions {
            Output::info(&format!("  {} - {}", ec.code, ec.description));
            Output::info(&format!("    Resolution: {}", ec.resolution));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_introspect_list_all() {
        let options = IntrospectOptions { target: None };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_specific_command() {
        let options = IntrospectOptions {
            target: Some("add".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_unknown_command_returns_error() {
        let options = IntrospectOptions {
            target: Some("nonexistent".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_err());
    }

    #[test]
    fn run_introspect_init_command() {
        let options = IntrospectOptions {
            target: Some("init".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_done_command() {
        let options = IntrospectOptions {
            target: Some("done".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_remove_command() {
        let options = IntrospectOptions {
            target: Some("remove".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_list_command() {
        let options = IntrospectOptions {
            target: Some("list".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_status_command() {
        let options = IntrospectOptions {
            target: Some("status".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_sync_command() {
        let options = IntrospectOptions {
            target: Some("sync".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_diff_command() {
        let options = IntrospectOptions {
            target: Some("diff".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_introspect_command() {
        let options = IntrospectOptions {
            target: Some("introspect".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_doctor_command() {
        let options = IntrospectOptions {
            target: Some("doctor".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_query_command() {
        let options = IntrospectOptions {
            target: Some("query".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_introspect_revert_command() {
        let options = IntrospectOptions {
            target: Some("revert".to_string()),
        };
        let result = run_introspect(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn error_message_for_unknown_command() {
        let options = IntrospectOptions {
            target: Some("foobar".to_string()),
        };
        let result = run_introspect(&options);
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("foobar"));
        assert!(err_msg.contains("Unknown command"));
    }
}

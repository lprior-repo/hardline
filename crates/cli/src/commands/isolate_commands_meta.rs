//! Meta commands: contract, examples, validate, whatif

use clap::{Arg, Command as ClapCommand};

/// Show command contracts for AI integration
pub fn cmd_contract() -> ClapCommand {
    ClapCommand::new("contract")
        .about("Show command contracts for AI integration")
        .long_about(
            "Displays structured contracts for commands, including:

            - Input/output schemas

            - Argument types and constraints

            - Flags and their effects

            - Side effects and rollback information


            Useful for AI agents to understand command capabilities.",
        )
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to show contract for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline contract                    Show all command contracts",
                "hardline contract add                Show contract for 'add' command",
                "hardline contract --json             Output as JSON",
                "hardline contract --contract         Show contract command's own contract",
            ],
            None,
        ))
}

/// Show usage examples for commands
pub fn cmd_examples() -> ClapCommand {
    ClapCommand::new("examples")
        .about("Show usage examples for commands")
        .long_about(
            "Provides copy-pastable examples for AI agents and users.


            Filter by command or use case to find relevant examples.",
        )
        .arg(
            Arg::new("command")
                .required(false)
                .help("Filter examples for specific command"),
        )
        .arg(
            Arg::new("use-case")
                .long("use-case")
                .value_name("CASE")
                .help("Filter by use case (workflow, single-command, error-handling, etc.)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
}

/// Pre-validate inputs before execution
pub fn cmd_validate() -> ClapCommand {
    ClapCommand::new("validate")
        .about("Pre-validate inputs before execution")
        .long_about(
            "Validates inputs without executing commands.


            Use this to check:

            - Session name format

            - Bead ID format

            - Required arguments

            - Reserved names


            Returns structured validation results for AI agents.",
        )
        .arg(
            Arg::new("command")
                .required_unless_present("contract")
                .help("Command to validate inputs for"),
        )
        .arg(
            Arg::new("args")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Arguments to validate"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview validation without side effects (validation has no side effects, but flag accepted for compatibility)"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline validate add feature-x       Validate inputs for 'add' command",
                "hardline validate spawn hardline-abc1      Validate bead spawn inputs",
                "hardline validate --json              Output validation as JSON",
                "hardline validate --contract          Show AI contract (inputs/outputs schema)",
            ],
            None,
        ))
}

/// Preview command effects without executing
pub fn cmd_whatif() -> ClapCommand {
    ClapCommand::new("whatif")
        .about("Preview command effects without executing")
        .long_about(
            "Shows what a command would do without actually doing it.


            More detailed than --dry-run, includes:

            - Steps that would be executed

            - Resource changes (files, sessions)

            - Prerequisite checks

            - Reversibility information",
        )
        .arg(
            Arg::new("command")
                .required_unless_present_any(["contract", "ai-hints"])
                .help("Command to preview"),
        )
        .arg(
            Arg::new("args")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Command arguments"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show execution hints and common patterns"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline whatif done add feature-x    Preview 'add' command execution",
                "hardline whatif spawn hardline-abc1        Preview bead spawn",
                "hardline whatif --json                Output preview as JSON",
                "hardline whatif --contract            Show AI contract",
                "hardline whatif --ai-hints            Show AI execution hints",
            ],
            None,
        ))
}

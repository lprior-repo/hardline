//! Agent info commands: whereami, whoami, can_i

use clap::{Arg, Command as ClapCommand};

/// Quick location query - returns 'main' or 'workspace:<name>'
pub fn cmd_whereami() -> ClapCommand {
    ClapCommand::new("whereami")
        .about("Quick location query - returns 'main' or 'workspace:<name>'")
        .long_about(
            "AI-optimized command for quick orientation.


            Returns a simple, parseable string:

            - 'main' if on main branch

            - 'workspace:<name>' if in a workspace


            Use this before operations that depend on location.",
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
                .help("AI: Show machine-readable contract"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show execution hints and common patterns"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline whereami                    Returns 'main' or 'workspace:<name>'",
                "hardline whereami --json             Output location as JSON",
                "hardline whereami --contract         Show AI contract",
            ],
            None,
        ))
}

/// Agent identity query - returns agent ID or 'unregistered'
pub fn cmd_whoami() -> ClapCommand {
    ClapCommand::new("whoami")
        .about("Agent identity query - returns agent ID or 'unregistered'")
        .long_about(
            "AI-optimized command for identity verification.


            Returns:

            - Agent ID if registered (from Hardline_AGENT_ID env var)

            - 'unregistered' if no agent registered


            Also shows current session and bead from environment.",
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
                "hardline whoami                      Returns agent ID or 'unregistered'",
                "hardline whoami --json               Output identity as JSON",
            ],
            None,
        ))
}

/// Check if an action is permitted
pub fn cmd_can_i() -> ClapCommand {
    ClapCommand::new("can-i")
        .about("Check if an action is permitted")
        .long_about(
            "Checks preconditions before attempting operations.


            Returns whether an action is allowed, and if not, what prerequisites are missing.

            Useful for AI agents to check before executing commands.",
        )
        .arg(
            Arg::new("action")
                .required(true)
                .help("Action to check (add, remove, done, undo, sync, spawn, claim, merge)"),
        )
        .arg(
            Arg::new("resource")
                .required(false)
                .help("Resource to check (session name, bead ID, etc.)"),
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
                "hardline can-i done                  Check if done will succeed",
                "hardline can-i add feature-x         Check if session can be created",
                "hardline can-i spawn hardline-abc1        Check if bead can be spawned",
            ],
            None,
        ))
}

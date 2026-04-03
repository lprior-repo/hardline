//! Status command: show detailed session status

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Show detailed session status
pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline status                      Show status for all sessions",
                "hardline status feature-auth         Inspect a specific workspace",
                "hardline status --watch              Watch live updates (JSON available with --json)",
            ],
            Some(json_docs::status()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
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
}

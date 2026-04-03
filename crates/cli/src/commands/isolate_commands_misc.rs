//! Miscellaneous commands part 1: pause, resume, wait, schema

use clap::{Arg, Command as ClapCommand};

/// Pause an active session (suspend agent work)
pub fn cmd_pause() -> ClapCommand {
    ClapCommand::new("pause")
        .about("Pause an active session (suspend agent work)")
        .arg(Arg::new("name").help("Session name to pause"))
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
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &["hardline pause feature-x              Pause session"],
            None,
        ))
}

/// Resume a paused session
pub fn cmd_resume() -> ClapCommand {
    ClapCommand::new("resume")
        .about("Resume a paused session")
        .arg(Arg::new("name").help("Session name to resume"))
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
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &["hardline resume feature-x             Resume paused session"],
            None,
        ))
}

/// Wait for a condition to be met
pub fn cmd_wait() -> ClapCommand {
    ClapCommand::new("wait")
        .about("Wait for a condition to be met")
        .arg(
            Arg::new("condition")
                .required(true)
                .value_parser([
                    "session-exists",
                    "session-unlocked",
                    "healthy",
                    "session-status",
                ])
                .help("Condition to wait for"),
        )
        .arg(Arg::new("name").help("Session name (for session conditions)"))
        .arg(
            Arg::new("status")
                .long("status")
                .help("Expected status (for session-status condition)"),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_parser(clap::value_parser!(f64))
                .default_value("30")
                .help("Timeout in seconds"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .value_parser(clap::value_parser!(f64))
                .default_value("1")
                .help("Polling interval in seconds"),
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
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline wait session-exists feat      Wait for session to exist",
                "hardline wait -t 60 healthy           Wait up to 60s for healthy state",
            ],
            None,
        ))
}

/// Show JSON schemas for hardline protocol
pub fn cmd_schema() -> ClapCommand {
    ClapCommand::new("schema")
        .about("Show JSON schemas for hardline protocol")
        .arg(
            Arg::new("name")
                .help("Schema name (e.g., add-response)")
                .conflicts_with_all(["list", "all"]),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .conflicts_with_all(["all", "name"])
                .action(clap::ArgAction::SetTrue)
                .help("List all available schemas"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .short('a')
                .conflicts_with_all(["list", "name"])
                .action(clap::ArgAction::SetTrue)
                .help("Show all schemas"),
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
                .help("AI: Show command flow hints"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline schema                      List available schemas",
                "hardline schema add-response          Show specific schema",
                "hardline schema --list               List available schemas",
            ],
            None,
        ))
}

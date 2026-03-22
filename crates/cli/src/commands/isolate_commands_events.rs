//! Events command

use clap::{Arg, Command as ClapCommand};

/// Listen for or query system events
pub fn cmd_events() -> ClapCommand {
    ClapCommand::new("events")
        .about("Listen for or query system events")
        .long_about(
            "Provides access to the isolate event log.


            Use this to track session lifecycle, agent heartbeats, and resource claims.",
        )
        .arg(
            Arg::new("session")
                .long("session")
                .value_name("NAME")
                .help("Filter by session"),
        )
        .arg(
            Arg::new("type")
                .long("type")
                .value_name("TYPE")
                .help("Filter by event type"),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .short('l')
                .value_name("COUNT")
                .value_parser(clap::value_parser!(usize))
                .help("Limit number of events returned"),
        )
        .arg(
            Arg::new("follow")
                .long("follow")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Stream new events as they occur"),
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
                "isolate events                       Show recent events",
                "isolate events --follow             Stream events in real-time",
                "isolate events -l 20                Show last 20 events",
                "isolate events --type session       Filter by event type",
            ],
            None,
        ))
}

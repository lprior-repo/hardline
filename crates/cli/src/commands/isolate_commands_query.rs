//! Query commands: query, context, introspect

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Query system state programmatically
pub fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline query session-exists feature   Check if session exists",
                "hardline query session-count             Count active sessions",
                "hardline query can-run                   Check if hardline can run",
                "hardline query suggest-name PATTERN      Suggest next available sequential name",
                "hardline query --contract                Show AI contract (inputs/outputs schema)",
            ],
            Some(json_docs::query()),
        ))
        .arg(
            Arg::new("query_type")
                .required_unless_present_any(["contract", "ai-hints"])
                .help("Type of query (session-exists, session-count, can-run, suggest-name)"),
        )
        .arg(
            Arg::new("args")
                .required(false)
                .allow_hyphen_values(true)
                .help("Query-specific arguments"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (default for query)"),
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
                .help("AI: Show execution hints"),
        )
}

/// Show complete environment context (AI agent query)
pub fn cmd_context() -> ClapCommand {
    ClapCommand::new("context")
        .about("Show complete environment context (AI agent query)")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline context                     Show environment context summary",
                "hardline context --field=repository.branch  Extract a single field",
                "hardline context --json               Emit JSON (default when not TTY)",
            ],
            Some(json_docs::context()),
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (default when not TTY)"),
        )
        .arg(
            Arg::new("field")
                .long("field")
                .value_name("PATH")
                .help("Extract single field (e.g., --field=repository.branch)"),
        )
        .arg(
            Arg::new("no-beads")
                .long("no-beads")
                .action(clap::ArgAction::SetTrue)
                .help("Skip beads database query (faster)"),
        )
        .arg(
            Arg::new("no-health")
                .long("no-health")
                .action(clap::ArgAction::SetTrue)
                .help("Skip health checks (faster)"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("Show machine-readable contract for AI agents"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show execution hints and common patterns"),
        )
}

/// Discover hardline capabilities and command details
pub fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover hardline capabilities and command details")
        .long_about(
            "AI-optimized capability discovery.


            Use this to understand:
  
            - Available commands and their arguments
  
            - System state and dependencies
  
            - Environment variables hardline uses
  
            - Common workflow patterns",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline introspect                Show commands and their arguments",
                "hardline introspect focus          Inspect focus command contract",
                "hardline introspect --json         Emit machine-readable capability data",
            ],
            Some(json_docs::introspect()),
        ))
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to introspect (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("ai")
                .long("ai")
                .action(clap::ArgAction::SetTrue)
                .help("AI-optimized output: combines capabilities, state, and recommendations"),
        )
        .arg(
            Arg::new("env-vars")
                .long("env-vars")
                .action(clap::ArgAction::SetTrue)
                .help("Show environment variables hardline reads and sets"),
        )
        .arg(
            Arg::new("workflows")
                .long("workflows")
                .action(clap::ArgAction::SetTrue)
                .help("Show common workflow patterns for AI agents"),
        )
        .arg(
            Arg::new("session-states")
                .long("session-states")
                .action(clap::ArgAction::SetTrue)
                .help("Show valid session state transitions"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
}

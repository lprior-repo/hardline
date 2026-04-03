//! Session management commands: add, list, remove, focus

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Create session for manual work (Git workspace)
#[allow(clippy::too_many_lines)]
pub fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create session for manual work (Git workspace)")
        .long_about(
            "Creates a Git workspace for interactive development.
  
            Use this when YOU will work in the session.

            For automated agent workflows, use 'hardline spawn' instead.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline add feature-auth              Create session with standard layout",
                "hardline add bugfix-123 --no-open       Create without opening terminal",
                "hardline add quick-test --no-hooks      Skip post-create hooks",
                "hardline add work --bead hardline-abc123     Associate with bead hardline-abc123",
                "hardline add --example-json            Show example JSON output",
            ],
            Some(json_docs::add()),
        ))
        .arg(
            Arg::new("name")
                .required_unless_present_any(["example-json", "contract", "ai-hints"])
                .allow_hyphen_values(true)
                .help("Name for the new session (must start with a letter)"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID")
                .help("Associate this session with a bead/issue ID"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .help("Skip executing post_create hooks"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .help("Create workspace without opening terminal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("example-json")
                .long("example-json")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .conflicts_with("name")
                .help("Show example JSON output without executing"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Succeed if session already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Preview without creating"),
        )
        .arg(
            Arg::new("contract")
                .long("contract")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("AI: Show execution hints and common patterns"),
        )
}

/// List all sessions
pub fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline list                        Show all active sessions",
                "hardline list --verbose              Include workspace paths and bead titles",
                "hardline list --all --json           Dump every session in JSON",
                "hardline list --contract             Show AI contract (inputs/outputs schema)",
                "hardline list --ai-hints             Show AI execution hints",
            ],
            Some(json_docs::list()),
        ))
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(clap::ArgAction::SetTrue)
                .help("Show verbose output with workspace paths and bead titles"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .value_name("BEAD_ID")
                .help("Filter sessions by bead ID"),
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .value_name("NAME")
                .action(clap::ArgAction::Set)
                .help("Filter sessions by agent owner"),
        )
        .arg(
            Arg::new("state")
                .long("state")
                .value_name("STATE")
                .action(clap::ArgAction::Set)
                .help("Filter sessions by workspace state (created, working, ready, merged, abandoned, conflict, active, complete, terminal, non-terminal)"),
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

/// Remove a session and its workspace
pub fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline remove old-feature            Remove session (no confirmation)",
                "hardline remove test-session -f        Remove and skip pre_remove hooks",
                "hardline remove feature-x --merge       Merge changes to main first",
                "hardline remove experiment -k -f       Keep branch, skip hooks",
                "hardline remove stale-session --idempotent  Succeed if already removed",
                "hardline remove --contract             Show AI contract for this command",
            ],
            Some(json_docs::remove()),
        ))
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
                .help("AI: Show execution hints"),
        )
        .arg(
            Arg::new("name")
                .required_unless_present_any(["contract", "ai-hints"])
                .help("Name of the session to remove"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip pre_remove hooks (no-op for confirmation)"),
        )
        .arg(
            Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue)
                .help("Squash-merge to main before removal"),
        )
        .arg(
            Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue)
                .help("Preserve branch after removal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session doesn't exist (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview removal without executing"),
        )
}

/// Switch to session's workspace
pub fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to session's workspace")
        .long_about(
            "Switch to a session's workspace.

            Use this to navigate between workspaces.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline focus feature-auth            Switch to session's workspace",
                "hardline focus                         Interactive session selection",
                "hardline focus bugfix-123 --json       Get JSON output of focus operation",
            ],
            Some(json_docs::focus()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Name of the session to focus (interactive if omitted)"),
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
}

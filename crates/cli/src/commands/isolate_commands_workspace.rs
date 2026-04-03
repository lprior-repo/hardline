//! Workspace management commands: sync, diff, submit, undo, revert

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::json_docs;

/// Sync session workspace with main (rebase onto latest)
pub fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync session workspace with main (rebase onto latest)")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "DEFAULT BEHAVIOR (safe and explicit):",
                "  hardline sync                          Sync current workspace only",
                "  hardline sync <name>                   Sync ONLY the named session",
                "  hardline sync --all                    Sync ALL sessions (explicit)",
                "",
                "OPTIONS:",
                "  hardline sync --dry-run                Preview without changes",
                "  hardline sync --json                   JSON output with SchemaEnvelope",
                "",
                "SAFETY: Named sync is isolated. Default syncs only current workspace.",
            ],
            Some(json_docs::sync()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to sync (default: sync current workspace only)"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("name")
                .help("Sync ALL active sessions (must be explicit)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview sync without executing"),
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
                .help("AI: Show execution hints"),
        )
}

/// Show diff between session and main branch
pub fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline diff                        Auto-detect session from workspace",
                "hardline diff feature-auth           Show diff between feature workspace and main",
                "hardline diff --stat                 Show diffstat for auto-detected session",
                "hardline diff feature-auth --stat    Show diffstat summary",
                "hardline diff feature-auth --json    Output diff metadata in JSON",
            ],
            Some(json_docs::diff()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show diff for (auto-detected if not provided)"),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .action(clap::ArgAction::SetTrue)
                .help("Show diffstat only (summary of changes)"),
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

/// Submit changes for review/merge
pub fn cmd_submit() -> ClapCommand {
    ClapCommand::new("submit")
        .about("Submit changes for review/merge")
        .long_about(
            "Prepares and submits the current workspace changes for review or direct merge.

            This command will:
            1. Validate workspace state
            2. Optionally commit changes
            3. Create merge request or merge directly

            Use --dry-run to preview what would happen.",
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline submit                        Submit current workspace",
                "hardline submit --dry-run              Preview submit without changes",
                "hardline submit --auto-commit          Auto-commit before submitting",
                "hardline submit -m \"Fix bug\"          Submit with custom commit message",
                "hardline submit --json                 Output as JSON",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to submit (default: current workspace)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Show what would happen without making changes"),
        )
        .arg(
            Arg::new("auto-commit")
                .long("auto-commit")
                .action(clap::ArgAction::SetTrue)
                .help("Automatically commit changes if needed"),
        )
        .arg(
            Arg::new("message")
                .long("message")
                .short('m')
                .value_name("MESSAGE")
                .help("Custom commit message"),
        )
}

/// Revert last done operation
pub fn cmd_undo() -> ClapCommand {
    ClapCommand::new("undo")
        .about("Revert last done operation")
        .long_about(
            "Reverts the most recent 'hardline done' operation, rolling back to the state before the merge.

            Works only if changes haven't been pushed to remote.

            Undo history is kept for 24 hours.",
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .action(clap::ArgAction::SetTrue)
                .help("List undo history without reverting"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline undo                        Undo most recent done",
                "hardline undo --list                 Show undo history",
                "hardline undo --dry-run              Preview undo",
            ],
            None,
        ))
}

/// Revert specific session merge
pub fn cmd_revert() -> ClapCommand {
    ClapCommand::new("revert")
        .about("Revert specific session merge")
        .long_about(
            "Reverts a specific session's merge operation, identified by session name.

            Works only if changes haven't been pushed to remote.

            Undo history is kept for 24 hours.",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .help("Name of session to revert"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(crate::commands::isolate_commands::after_help_text(
            &[
                "hardline revert feature-x            Revert specific session merge",
                "hardline revert --dry-run feat       Preview revert",
            ],
            None,
        ))
}

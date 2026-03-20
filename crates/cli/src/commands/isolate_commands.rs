//! CLI command definitions using `clap`

use clap::{Arg, Command as ClapCommand};

use crate::commands::isolate_mod::{json_docs, object_commands};

pub fn cmd_ai() -> ClapCommand {
    ClapCommand::new("ai")
        .about("AI-first commands for streamlined workflows")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("work")
                .about("Start work on a task in an isolated environment")
                .arg(
                    Arg::new("task_id")
                        .required(false)
                        .help("The identifier for the task"),
                ),
        )
}

pub fn after_help_text(examples: &[&str], json_output: Option<&'static str>) -> String {
    let mut text = String::from("EXAMPLES:\n");
    for example in examples {
        text.push_str("  ");
        text.push_str(example);
        text.push('\n');
    }
    if let Some(json) = json_output {
        text.push('\n');
        text.push_str(json);
        if !json.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}

pub fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize isolate in a JJ repository (or create one)")
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
                .help("Preview initialization without executing"),
        )
        .after_help(after_help_text(
            &[
                "isolate init                        Initialize Isolate in the current JJ repository",
                "isolate init --json                 Output JSON metadata for automation",
                "isolate init --dry-run              Preview initialization",
            ],
            Some(json_docs::init()),
        ))
}

#[allow(clippy::too_many_lines)]
pub fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create session for manual work (JJ workspace)")
        .long_about(
            "Creates a JJ workspace for interactive development.
 
            Use this when YOU will work in the session.

            For automated agent workflows, use 'isolate spawn' instead.",
        )
        .after_help(after_help_text(
            &[
                "isolate add feature-auth              Create session with standard layout",
                "isolate add bugfix-123 --no-open       Create without opening terminal",
                "isolate add quick-test --no-hooks      Skip post-create hooks",
                "isolate add work --bead isolate-abc123     Associate with bead isolate-abc123",
                "isolate add --example-json            Show example JSON output",
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

pub fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .after_help(after_help_text(
            &[
                "isolate list                        Show all active sessions",
                "isolate list --verbose              Include workspace paths and bead titles",
                "isolate list --all --json           Dump every session in JSON",
                "isolate list --contract             Show AI contract (inputs/outputs schema)",
                "isolate list --ai-hints             Show AI execution hints",
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

#[allow(clippy::too_many_lines)]
pub fn cmd_bookmark() -> ClapCommand {
    ClapCommand::new("bookmark")
        .about("Manage JJ bookmarks/branches")
        .long_about(
            "Manage bookmarks (branches) in JJ workspaces.


            isolate wraps JJ completely - use 'isolate bookmark' not 'jj bookmark'.

            Provides: list, create, delete, move operations.",
        )
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("list")
                .about("List bookmarks in a session workspace")
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show all bookmarks including remote"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("create")
                .about("Create a new bookmark at current revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new bookmark"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("push")
                        .long("push")
                        .short('p')
                        .action(clap::ArgAction::SetTrue)
                        .help("Push bookmark to remote after creation"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("delete")
                .about("Delete a bookmark")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to delete"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("move")
                .about("Move a bookmark to a different revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to move"),
                )
                .arg(
                    Arg::new("to")
                        .long("to")
                        .required(true)
                        .value_name("REVISION")
                        .help("Target revision (commit hash or revset)"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .after_help(after_help_text(
            &[
                "isolate bookmark list                List bookmarks in current workspace",
                "isolate bookmark list --all          Show all bookmarks including remote",
                "isolate bookmark create feature-x    Create bookmark at current revision",
                "isolate bookmark create -p stable    Create and push to remote",
                "isolate bookmark delete old-fix      Delete a bookmark",
                "isolate bookmark move stable --to @  Move bookmark to current revision",
            ],
            None,
        ))
}

pub fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .after_help(after_help_text(
            &[
                "isolate remove old-feature            Remove session (no confirmation)",
                "isolate remove test-session -f        Remove and skip pre_remove hooks",
                "isolate remove feature-x --merge       Merge changes to main first",
                "isolate remove experiment -k -f       Keep branch, skip hooks",
                "isolate remove stale-session --idempotent  Succeed if already removed",
                "isolate remove --contract             Show AI contract for this command",
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

pub fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to session's workspace")
        .long_about(
            "Switch to a session's workspace.

            Use this to navigate between workspaces.",
        )
        .after_help(after_help_text(
            &[
                "isolate focus feature-auth            Switch to session's workspace",
                "isolate focus                         Interactive session selection",
                "isolate focus bugfix-123 --json       Get JSON output of focus operation",
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

pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
        .after_help(after_help_text(
            &[
                "isolate status                      Show status for all sessions",
                "isolate status feature-auth         Inspect a specific workspace",
                "isolate status --watch              Watch live updates (JSON available with --json)",
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

pub fn cmd_switch() -> ClapCommand {
    ClapCommand::new("switch")
        .about("Switch to a different workspace")
        .long_about(
            "Navigate between workspaces.

            Use this for quick workspace switching. Similar to 'isolate focus' but 
            emphasizes navigation between existing sessions.",
        )
        .after_help(after_help_text(
            &[
                "isolate switch feature-auth           Switch to named session",
                "isolate switch                        Interactive session selection",
                "isolate switch test --show-context    Switch and show session details",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Name of the session to switch to (interactive if omitted)"),
        )
        .arg(
            Arg::new("show-context")
                .long("show-context")
                .action(clap::ArgAction::SetTrue)
                .help("Show session details after switching"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync session workspace with main (rebase onto latest)")
        .after_help(after_help_text(
            &[
                "DEFAULT BEHAVIOR (safe and explicit):",
                "  isolate sync                          Sync current workspace only",
                "  isolate sync <name>                   Sync ONLY the named session",
                "  isolate sync --all                    Sync ALL sessions (explicit)",
                "",
                "OPTIONS:",
                "  isolate sync --dry-run                Preview without changes",
                "  isolate sync --json                   JSON output with SchemaEnvelope",
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
        .after_help(after_help_text(
            &[
                "isolate submit                        Submit current workspace",
                "isolate submit --dry-run              Preview submit without changes",
                "isolate submit --auto-commit          Auto-commit before submitting",
                "isolate submit -m \"Fix bug\"          Submit with custom commit message",
                "isolate submit --json                 Output as JSON",
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

pub fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
        .after_help(after_help_text(
            &[
                "isolate diff                        Auto-detect session from workspace",
                "isolate diff feature-auth           Show diff between feature workspace and main",
                "isolate diff --stat                 Show diffstat for auto-detected session",
                "isolate diff feature-auth --stat    Show diffstat summary",
                "isolate diff feature-auth --json    Output diff metadata in JSON",
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

pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .after_help(after_help_text(
            &[
                "isolate config                      Show current project config",
                "isolate config workspace_dir        Display the workspace_dir setting",
                "isolate config workspace_dir /new/path --json  Update key and emit JSON",
            ],
            Some(json_docs::config()),
        ))
        .arg(Arg::new("key").help("Config key to view/set (dot notation)"))
        .arg(Arg::new("value").help("Value to set (omit to view)"))
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help("Operate on global config instead of project"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_clean() -> ClapCommand {
    ClapCommand::new("clean")
        .about("Remove stale sessions (where workspace no longer exists)")
        .after_help(after_help_text(
            &[
                "isolate clean                       Remove stale sessions",
                "isolate clean --dry-run             List stale sessions without deleting",
                "isolate clean --force --json        Force clean and emit JSON summary",
            ],
            Some(json_docs::clean()),
        ))
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List stale sessions without removing"),
        )
        .arg(
            Arg::new("periodic")
                .long("periodic")
                .action(clap::ArgAction::SetTrue)
                .help("Run as periodic cleanup daemon (1hr interval)"),
        )
        .arg(
            Arg::new("age-threshold")
                .long("age-threshold")
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(u64))
                .help("Age threshold for periodic cleanup (default: 7200 = 2hr)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_prune_invalid() -> ClapCommand {
    ClapCommand::new("prune-invalid")
        .about("Remove all invalid session records in one deterministic command")
        .long_about(
            "Bulk cleanup primitive to remove all invalid session records.

Invalid sessions are those where the workspace directory no longer exists
but the session record still exists in the database.

This is useful for cleaning up after workspace directory deletions
or when sessions become orphaned.

Use --yes to skip confirmation for scripting/CI use.",
        )
        .after_help(after_help_text(
            &[
                "isolate prune-invalid                Remove invalid sessions (with prompt)",
                "isolate prune-invalid --yes         Remove invalid sessions (no prompt)",
                "isolate prune-invalid --dry-run     List invalid sessions without deleting",
                "isolate prune-invalid --yes --json Remove with JSON output",
            ],
            None,
        ))
        .arg(
            Arg::new("yes")
                .long("yes")
                .short('y')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt (for scripting/CI)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List invalid sessions without removing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover isolate capabilities and command details")
        .long_about(
            "AI-optimized capability discovery.


            Use this to understand:
  
            - Available commands and their arguments
  
            - System state and dependencies
  
            - Environment variables isolate uses
  
            - Common workflow patterns",
        )
        .after_help(after_help_text(
            &[
                "isolate introspect                Show commands and their arguments",
                "isolate introspect focus          Inspect focus command contract",
                "isolate introspect --json         Emit machine-readable capability data",
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
                .help("Show environment variables isolate reads and sets"),
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

#[allow(clippy::too_many_lines)]
pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run diagnostics and health checks")
        .alias("check")
        .subcommand_required(false)
        .subcommand(
            ClapCommand::new("check")
                .about("Run diagnostics")
                .alias("check")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("fix")
                .about("Fix detected issues")
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
                        .help("Preview what would be fixed without making changes"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed progress during fixes"),
                ),
        )
        .subcommand(
            ClapCommand::new("integrity")
                .about("Check system integrity")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("clean")
                .about("Clean up invalid sessions")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("List stale sessions without removing"),
                ),
        )
        .after_help(after_help_text(
            &[
                "isolate doctor                    Run all system health checks (legacy)",
                "isolate doctor check             Run all system health checks",
                "isolate doctor fix              Auto-fix issues where possible",
                "isolate doctor fix --dry-run    Preview what would be fixed without making changes",
                "isolate doctor fix --verbose    Show detailed progress during fixes",
                "isolate doctor integrity        Run database integrity check",
                "isolate doctor clean            Remove stale sessions",
                "isolate doctor --json           Export check results to JSON (legacy)",
            ],
            Some(json_docs::doctor()),
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (legacy mode)"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible (legacy mode)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .requires("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Preview what would be fixed without making changes"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .requires("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Show detailed progress during fixes"),
        )
}

pub fn cmd_integrity() -> ClapCommand {
    ClapCommand::new("integrity")
        .about("Manage workspace integrity and corruption recovery")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("validate")
                .about("Validate workspace integrity")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("repair")
                .about("Repair corrupted workspace")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                )
                .arg(
                    Arg::new("rebind")
                        .long("rebind")
                        .action(clap::ArgAction::SetTrue)
                        .help("Update session record when workspace is detected in a new location"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("backup")
                .about("Manage workspace backups")
                .subcommand_required(true)
                .subcommand(
                    ClapCommand::new("list")
                        .about("List available backups")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                )
                .subcommand(
                    ClapCommand::new("restore")
                        .about("Restore from a backup")
                        .arg(
                            Arg::new("backup_id")
                                .required(true)
                                .help("Backup ID to restore"),
                        )
                        .arg(
                            Arg::new("force")
                                .long("force")
                                .short('f')
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                ),
        )
        .after_help(after_help_text(
            &[
                "isolate integrity validate feature-x    Validate workspace integrity",
                "isolate integrity repair feature-x      Repair corrupted workspace",
                "isolate integrity repair -f feature-x   Repair without confirmation",
                "isolate integrity backup list           List available backups",
                "isolate integrity backup restore 123    Restore from backup ID",
            ],
            None,
        ))
}

pub fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .after_help(after_help_text(
            &[
                "isolate query session-exists feature   Check if session exists",
                "isolate query session-count             Count active sessions",
                "isolate query can-run                   Check if isolate can run",
                "isolate query suggest-name PATTERN      Suggest next available sequential name",
                "isolate query --contract                Show AI contract (inputs/outputs schema)",
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
pub fn cmd_context() -> ClapCommand {
    ClapCommand::new("context")
        .about("Show complete environment context (AI agent query)")
        .after_help(after_help_text(
            &[
                "isolate context                     Show environment context summary",
                "isolate context --field=repository.branch  Extract a single field",
                "isolate context --json               Emit JSON (default when not TTY)",
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

pub fn cmd_spawn() -> ClapCommand {
    ClapCommand::new("spawn")
        .about("Create session for automated agent work on a bead (issue)")
        .long_about(
            "Creates a JJ workspace, runs an agent (default: claude), and auto-merges on success.

            Use this when an AI AGENT should work autonomously on a bead.


            For manual interactive work, use 'isolate add' instead.",
        )
        .after_help(after_help_text(
            &[
                "isolate spawn isolate-abc12               Spawn workspace for bead with Claude",
                "isolate spawn isolate-xyz34 -b            Run agent in background",
                "isolate spawn isolate-def56 --agent-command=llm-run  Use custom agent",
                "isolate spawn isolate-ghi78 --no-auto-merge  Don't auto-merge on success",
            ],
            Some(json_docs::spawn()),
        ))
        .arg(
            Arg::new("bead")
                .required(true)
                .help("Bead ID to work on (e.g., isolate-xxxx)"),
        )
        .arg(
            Arg::new("agent-command")
                .long("agent-command")
                .value_name("COMMAND")
                .default_value("claude")
                .help("Agent command to run"),
        )
        .arg(
            Arg::new("agent-args")
                .long("agent-args")
                .value_name("ARGS")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Additional agent arguments"),
        )
        .arg(
            Arg::new("no-auto-merge")
                .long("no-auto-merge")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Don't merge on success"),
        )
        .arg(
            Arg::new("no-auto-cleanup")
                .long("no-auto-cleanup")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Don't cleanup on failure"),
        )
        .arg(
            Arg::new("background")
                .long("background")
                .short('b')
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Run agent in background"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .default_value("14400")
                .help("Timeout in seconds (default: 14400 = 4 hours)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Output as JSON"),
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
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Succeed if workspace already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .default_value("false")
                .help("Preview spawn without executing"),
        )
}

pub fn cmd_checkpoint() -> ClapCommand {
    ClapCommand::new("checkpoint")
        .about("Save and restore session state snapshots")
        .alias("ckpt")
        .subcommand_required(true)
        .after_help(after_help_text(
            &[
                "isolate checkpoint create --description=\"before lunch\"  Snapshot current sessions",
                "isolate checkpoint list                 Show all available checkpoints",
                "isolate checkpoint restore ckpt-123     Restore workspace state from checkpoint",
            ],
            Some(json_docs::checkpoint()),
        ))
        .subcommand(
            ClapCommand::new("create")
                .about("Create a checkpoint of all current sessions")
                .arg(
                    Arg::new("description")
                        .short('d')
                        .long("description")
                        .value_name("DESC")
                        .help("Description for this checkpoint"),
                ),
        )
        .subcommand(
            ClapCommand::new("restore")
                .about("Restore sessions to a checkpoint state")
                .arg(
                    Arg::new("checkpoint_id")
                        .required(true)
                        .help("Checkpoint ID to restore"),
                ),
        )
        .subcommand(ClapCommand::new("list").about("List all available checkpoints"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
}

pub fn cmd_done() -> ClapCommand {
    ClapCommand::new("done")
        .about("Complete work and merge workspace to main")
        .after_help(after_help_text(
            &[
                "isolate done                            Complete work and merge to main",
                "isolate done -m \"Fix auth bug\"         Use custom commit message",
                "isolate done --workspace feature-x      Complete specific workspace from main",
                "isolate done --dry-run                  Preview without executing",
                "isolate done --keep-workspace           Keep workspace after merge",
                "isolate done --detect-conflicts         Check for conflicts before merging",
                "isolate done --json                     Get JSON output",
            ],
            Some(json_docs::done()),
        ))
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .value_name("NAME")
                .help("Workspace to complete (uses current if not specified)"),
        )
        .arg(
            Arg::new("message")
                .short('m')
                .long("message")
                .value_name("MSG")
                .help("Commit message (auto-generated if not provided)"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .conflicts_with("no-keep")
                .action(clap::ArgAction::SetTrue)
                .help("Keep workspace after merge"),
        )
        .arg(
            Arg::new("squash")
                .long("squash")
                .action(clap::ArgAction::SetTrue)
                .help("Squash all commits into one"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("detect-conflicts")
                .long("detect-conflicts")
                .action(clap::ArgAction::SetTrue)
                .help("Check for conflicts before merging"),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue)
                .help("Skip bead status update"),
        )
        .arg(
            Arg::new("no-keep")
                .long("no-keep")
                .conflicts_with("keep-workspace")
                .action(clap::ArgAction::SetTrue)
                .help("Skip workspace retention (cleanup immediately)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
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
                .help("AI: Show workflow patterns and best practices"),
        )
}

pub fn cmd_undo() -> ClapCommand {
    ClapCommand::new("undo")
        .about("Revert last done operation")
        .long_about(
            "Reverts the most recent 'isolate done' operation, rolling back to the state before the merge.

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
        .after_help(after_help_text(
            &[
                "isolate undo                        Undo most recent done",
                "isolate undo --list                 Show undo history",
                "isolate undo --dry-run              Preview undo",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate revert feature-x            Revert specific session merge",
                "isolate revert --dry-run feat       Preview revert",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate whereami                    Returns 'main' or 'workspace:<name>'",
                "isolate whereami --json             Output location as JSON",
                "isolate whereami --contract         Show AI contract",
            ],
            None,
        ))
}

pub fn cmd_whoami() -> ClapCommand {
    ClapCommand::new("whoami")
        .about("Agent identity query - returns agent ID or 'unregistered'")
        .long_about(
            "AI-optimized command for identity verification.


            Returns:

            - Agent ID if registered (from Isolate_AGENT_ID env var)

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
        .after_help(after_help_text(
            &[
                "isolate whoami                      Returns agent ID or 'unregistered'",
                "isolate whoami --json               Output identity as JSON",
            ],
            None,
        ))
}

pub fn cmd_work() -> ClapCommand {
    ClapCommand::new("work")
        .about("Start working on a task (create workspace + register agent)")
        .long_about(
            "Unified workflow start command for AI agents.


            Combines multiple steps:
  
            1. Create workspace (or reuse if --idempotent)
  
            2. Register as agent (unless --no-agent)
  
            3. Set environment variables
  
            4. Output workspace info


            This is the AI-friendly entry point for starting work.",
        )
        .after_help(after_help_text(
            &[
                "isolate work feature-auth              Start working on feature-auth",
                "isolate work bug-fix --bead isolate-123    Start work on bead",
                "isolate work test --idempotent         Reuse existing session if exists",
                "isolate work --dry-run feature         Preview what would be created",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required_unless_present_any(["contract", "ai-hints"])
                .help("Session name to create/use"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID")
                .help("Bead ID to associate with this work"),
        )
        .arg(
            Arg::new("agent-id")
                .long("agent-id")
                .value_name("ID")
                .help("Agent ID to register (auto-generated if not provided)"),
        )
        .arg(
            Arg::new("no-agent")
                .long("no-agent")
                .action(clap::ArgAction::SetTrue)
                .help("Don't register as agent"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without creating"),
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
                .help("AI: Show machine-readable contract (JSON schema)"),
        )
        .arg(
            Arg::new("ai-hints")
                .long("ai-hints")
                .action(clap::ArgAction::SetTrue)
                .help("AI: Show workflow patterns and best practices"),
        )
}

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
        .after_help(after_help_text(
            &[
                "isolate can-i done                  Check if done will succeed",
                "isolate can-i add feature-x         Check if session can be created",
                "isolate can-i spawn isolate-abc1        Check if bead can be spawned",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate contract                    Show all command contracts",
                "isolate contract add                Show contract for 'add' command",
                "isolate contract --json             Output as JSON",
                "isolate contract --contract         Show contract command's own contract",
            ],
            None,
        ))
}

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

pub fn cmd_help() -> ClapCommand {
    ClapCommand::new("help")
        .about("Print help for a command")
        .arg(
            Arg::new("command")
                .required(false)
                .num_args(0..)
                .action(clap::ArgAction::Append)
                .allow_hyphen_values(true)
                .help("Command path to show help for (omit for top-level help)"),
        )
}

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
        .after_help(after_help_text(
            &[
                "isolate validate add feature-x       Validate inputs for 'add' command",
                "isolate validate spawn isolate-abc1      Validate bead spawn inputs",
                "isolate validate --json              Output validation as JSON",
                "isolate validate --contract          Show AI contract (inputs/outputs schema)",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate whatif done add feature-x    Preview 'add' command execution",
                "isolate whatif spawn isolate-abc1        Preview bead spawn",
                "isolate whatif --json                Output preview as JSON",
                "isolate whatif --contract            Show AI contract",
                "isolate whatif --ai-hints            Show AI execution hints",
            ],
            None,
        ))
}

pub fn cmd_batch() -> ClapCommand {
    ClapCommand::new("batch")
        .about("Execute multiple commands in a batch")
        .long_about(
            "Runs multiple commands in sequence or from a file.


            Features:
  
            - Atomic transactional mode (all succeed or all roll back)
  
            - Stop-on-error control
  
            - Combined results output",
        )
        .arg(
            Arg::new("file")
                .long("file")
                .short('f')
                .value_name("FILE")
                .help("File containing commands (one per line)"),
        )
        .arg(
            Arg::new("atomic")
                .long("atomic")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .help("Execute all or none (requires checkpoint support)"),
        )
        .arg(
            Arg::new("stop-on-error")
                .long("stop-on-error")
                .action(clap::ArgAction::SetTrue)
                .help("Stop execution if a command fails"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("commands")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Commands to execute"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview batch execution"),
        )
        .after_help(after_help_text(
            &[
                "isolate batch add feat1 add feat2     Execute multiple commands",
                "isolate batch -f commands.txt        Execute commands from file",
                "isolate batch --atomic --dry-run     Preview execution",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate events                       Show recent events",
                "isolate events --follow             Stream events in real-time",
                "isolate events -l 20                Show last 20 events",
                "isolate events --type session       Filter by event type",
            ],
            None,
        ))
}

pub fn cmd_completions() -> ClapCommand {
    ClapCommand::new("completions")
        .about("Generate shell completions")
        .arg(
            Arg::new("shell")
                .required(true)
                .help("Shell to generate completions for"),
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
        .after_help(after_help_text(
            &[
                "isolate completions bash             Generate bash completions",
                "isolate completions zsh              Generate zsh completions",
                "isolate completions fish             Generate fish completions",
            ],
            None,
        ))
}

pub fn cmd_rename() -> ClapCommand {
    ClapCommand::new("rename")
        .about("Rename an existing session")
        .arg(
            Arg::new("old_name")
                .required(true)
                .help("Current session name"),
        )
        .arg(Arg::new("new_name").required(true).help("New session name"))
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
        .after_help(after_help_text(
            &["isolate rename old-name new-name        Rename a session"],
            None,
        ))
}

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
        .after_help(after_help_text(
            &["isolate pause feature-x              Pause session"],
            None,
        ))
}

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
        .after_help(after_help_text(
            &["isolate resume feature-x             Resume paused session"],
            None,
        ))
}

pub fn cmd_clone() -> ClapCommand {
    ClapCommand::new("clone")
        .about("Clone a session into a new one")
        .arg(
            Arg::new("source")
                .required(true)
                .help("Source session name"),
        )
        .arg(
            Arg::new("dest")
                .required(true)
                .help("Destination session name"),
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
        .after_help(after_help_text(
            &["isolate clone feature-x feature-y     Clone session"],
            None,
        ))
}

pub fn cmd_export() -> ClapCommand {
    ClapCommand::new("export")
        .about("Export session state to a file")
        .long_about(
            "Export session state to a file or stdout.

The SESSION argument specifies which session to export. If omitted, all sessions
are exported. To write to a file, you MUST use the -o/--output flag. This
prevents ambiguity between session names and file paths.

IMPORTANT: Output file paths require -o/--output flag:
  - 'isolate export -o export.json'    - Correct: export all sessions to file
  - 'isolate export export.json'       - WRONG: 'export.json' treated as session name!",
        )
        .arg(Arg::new("session").help("Session name to export (all if omitted)"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path (REQUIRED when writing to a file)"),
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
        .after_help(after_help_text(
            &[
                "isolate export feature-x -o state.json  Export specific session to file",
                "isolate export -o state.json            Export all sessions to file",
                "isolate export --json                   Export all sessions as JSON to stdout",
                "isolate export                          Export all sessions to stdout",
                "",
                "NOTE: Always use -o when writing to a file:",
                "  CORRECT:   isolate export -o output.json",
                "  INCORRECT: isolate export output.json   (interprets as session name!)",
            ],
            Some(json_docs::export()),
        ))
}

pub fn cmd_import() -> ClapCommand {
    ClapCommand::new("import")
        .about("Import session state from a file")
        .arg(Arg::new("file").required(true).help("Input file path"))
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("skip-existing")
                .help("Overwrite existing sessions"),
        )
        .arg(
            Arg::new("skip-existing")
                .long("skip-existing")
                .action(clap::ArgAction::SetTrue)
                .help("Skip sessions that already exist"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview import without changes"),
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
        .after_help(after_help_text(
            &[
                "isolate import state.json           Import session from file",
                "isolate import -f state.json        Force overwrite existing",
                "isolate import --dry-run state.json  Preview import",
            ],
            None,
        ))
}

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
        .after_help(after_help_text(
            &[
                "isolate wait session-exists feat      Wait for session to exist",
                "isolate wait -t 60 healthy           Wait up to 60s for healthy state",
            ],
            None,
        ))
}

pub fn cmd_schema() -> ClapCommand {
    ClapCommand::new("schema")
        .about("Show JSON schemas for isolate protocol")
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
        .after_help(after_help_text(
            &[
                "isolate schema                      List available schemas",
                "isolate schema add-response          Show specific schema",
                "isolate schema --list               List available schemas",
            ],
            None,
        ))
}

pub fn cmd_recover() -> ClapCommand {
    ClapCommand::new("recover")
        .about("Recover from inconsistent state or restore from operation log")
        .arg(
            Arg::new("session")
                .value_name("SESSION")
                .help("Session name to recover (optional, uses current workspace if not specified)")
                .num_args(0..=1)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("diagnose")
                .short('d')
                .long("diagnose")
                .action(clap::ArgAction::SetTrue)
                .help("Only diagnose system issues without fixing (system recovery mode)"),
        )
        .arg(
            Arg::new("op")
                .long("op")
                .value_name("ID")
                .help("Restore to specific operation ID (operation log mode)")
                .num_args(1)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("last")
                .long("last")
                .action(clap::ArgAction::SetTrue)
                .help("Restore to previous operation (quick undo)"),
        )
        .arg(
            Arg::new("list-ops")
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List operation log without restoring (default when no --op or --last)"),
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
        .after_help(after_help_text(
            &[
                "isolate recover                      Auto-diagnose and fix issues",
                "isolate recover --diagnose           Only diagnose, don't fix",
                "isolate recover feature-x            Recover specific session",
            ],
            None,
        ))
}

pub fn cmd_retry() -> ClapCommand {
    ClapCommand::new("retry")
        .about("Retry the last failed operation")
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
        .after_help(after_help_text(
            &["isolate retry                       Retry last failed operation"],
            None,
        ))
}

pub fn cmd_rollback() -> ClapCommand {
    ClapCommand::new("rollback")
        .about("Rollback session to a specific checkpoint")
        .arg(Arg::new("session").required(true).help("Session name"))
        .arg(
            Arg::new("to")
                .long("to")
                .required(true)
                .help("Checkpoint ID to rollback to"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview rollback without executing"),
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
        .after_help(after_help_text(
            &[
                "isolate rollback feature-x --to 123  Rollback to checkpoint",
                "isolate rollback --dry-run feature-x --to 123  Preview rollback",
            ],
            None,
        ))
}

pub fn cmd_abort() -> ClapCommand {
    ClapCommand::new("abort")
        .about("Abort work and abandon workspace changes")
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .visible_alias("session")
                .value_name("NAME")
                .help("Workspace/session to abort (uses current if not specified)"),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue)
                .help("Don't update bead status"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue)
                .help("Keep workspace files (just remove from isolate tracking)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
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
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(after_help_text(
            &[
                "isolate abort                       Abandon current workspace",
                "isolate abort --session feature-x   Abort specific workspace",
                "isolate abort --keep-workspace      Keep files, just remove from isolate",
                "isolate abort --dry-run             Preview abort without executing",
            ],
            None,
        ))
}

/// Backup command - manage database backups
pub fn cmd_backup() -> ClapCommand {
    ClapCommand::new("backup")
        .about("Manage automated database backups")
        .long_about(
            "Create, list, restore, and manage backups of isolate databases (state.db, beads.db).\n\n\
            Backups include:\n  \
            - state.db: Session, workspace state, and merge queue\n  \
            - beads.db: Issue tracking database\n\n\
            Note: queue.db has been consolidated into state.db.\n\n\
            Backups are stored with timestamps and SHA-256 checksums for integrity verification.",
        )
        .after_help(after_help_text(
            &[
                "isolate backup --create                     Create backups of all databases",
                "isolate backup --list                       List all available backups",
                "isolate backup --restore state.db           Restore latest backup of state.db",
                "isolate backup --restore beads.db --timestamp 20250101-010101  Restore specific backup by timestamp",
                "isolate backup --status                     Show backup status and retention info",
                "isolate backup --retention                  Apply retention policy (remove old backups)",
                "isolate backup --create --json              Create backups with JSON output",
            ],
            None,
        ))
        .arg(
            Arg::new("create")
                .long("create")
                .action(clap::ArgAction::SetTrue)
                .help("Create new backups of all databases"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List all available backups"),
        )
        .arg(
            Arg::new("restore")
                .long("restore")
                .value_name("DATABASE")
                .help("Restore database from backup (state.db, beads.db)"),
        )
        .arg(
            Arg::new("timestamp")
                .short('t')
                .long("timestamp")
                .value_name("TIMESTAMP")
                .requires("restore")
                .help("Specific backup timestamp to restore (format: YYYYMMDD-HHMMSS)"),
        )
        .arg(
            Arg::new("status")
                .long("status")
                .action(clap::ArgAction::SetTrue)
                .help("Show backup status and retention policy information"),
        )
        .arg(
            Arg::new("retention")
                .long("retention")
                .action(clap::ArgAction::SetTrue)
                .help("Apply retention policy and remove old backups"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn build_cli() -> ClapCommand {
    object_commands::build_object_cli()
}

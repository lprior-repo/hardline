//! Ported CLI commands from isolate
//!
//! This module ports the command definitions from isolate's CLI to hardline's
//! clap derive structure. Isolate used the builder API while hardline uses
//! derive macros with `#[derive(Parser)]` and `#[derive(Subcommand)]`.
//!
//! Command mapping from isolate -> hardline:
//! - `isolate ai work` -> `hardline ai work`
//! - `isolate init` -> `hardline init` (already exists)
//! - `isolate add` -> `hardline workspace add` / `session add`
//! - `isolate list` -> `hardline session list` (partially exists)
//! - `isolate bookmark` -> `hardline workspace bookmark`
//! - `isolate remove` -> `hardline session remove` (partially exists)
//! - `isolate focus` -> `hardline session focus` (exists)
//! - `isolate status` -> `hardline status` (exists)
//! - `isolate sync` -> `hardline sync` (exists)
//! - `isolate submit` -> `hardline session submit` (partially exists)
//! - `isolate diff` -> `hardline workspace diff`
//! - `isolate clean` -> `hardline clean`
//! - `isolate doctor` -> `hardline doctor` (exists)
//! - `isolate context` -> `hardline context` (exists)
//! - `isolate spawn` -> `hardline workspace spawn` / `task spawn`
//! - `isolate checkpoint` -> `hardline checkpoint`
//! - `isolate done` -> `hardline workspace done` / `session done`
//! - `isolate undo` -> `hardline workspace undo`
//! - `isolate revert` -> `hardline workspace revert`
//! - `isolate whereami` -> `hardline whereami` (exists)
//! - `isolate whoami` -> `hardline whoami`
//! - `isolate work` -> `hardline work`
//! - `isolate can-i` -> `hardline can-i`
//! - `isolate contract` -> `hardline contract`
//! - `isolate examples` -> `hardline examples` (exists)
//! - `isolate validate` -> `hardline validate`
//! - `isolate whatif` -> `hardline whatif` (exists)
//! - `isolate batch` -> `hardline batch` (exists)
//! - `isolate events` -> `hardline events`
//! - `isolate completions` -> `hardline completions`
//! - `isolate rename` -> `hardline workspace rename`
//! - `isolate pause/resume` -> `hardline workspace pause/resume`
//! - `isolate clone` -> `hardline workspace clone`
//! - `isolate export/import` -> `hardline workspace export/import`
//! - `isolate wait` -> `hardline wait`
//! - `isolate schema` -> `hardline schema`
//! - `isolate recover/retry/rollback` -> `hardline workspace recover/retry/rollback`
//! - `isolate abort` -> `hardline workspace abort`
//! - `isolate backup` -> `hardline backup`
//! - `isolate prune-invalid` -> `hardline clean --prune-invalid`
//! - `isolate integrity` -> `hardline workspace integrity`
//! - `isolate query` -> `hardline query`
//! - `isolate introspect` -> `hardline introspect`

use clap::{Parser, Subcommand};

/// AI-first commands for streamlined workflows
#[derive(Parser, Debug)]
pub struct AiCommands {
    #[command(subcommand)]
    pub command: AiSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum AiSubcommands {
    /// Start work on a task in an isolated environment
    Work {
        /// The identifier for the task
        #[arg(required = false)]
        task_id: Option<String>,
    },
}

/// Add command - Create session for manual work (JJ workspace)
#[derive(Parser, Debug)]
#[command(
    about = "Create session for manual work (JJ workspace)",
    long_about = "Creates a JJ workspace for interactive development.\n\nUse this when YOU will work in the session.\n\nFor automated agent workflows, use 'workspace spawn' instead."
)]
pub struct AddCommand {
    /// Name for the new session (must start with a letter)
    #[arg(required_unless_present_any = ["example_json", "contract", "ai_hints"])]
    pub name: Option<String>,

    /// Associate this session with a bead/issue ID
    #[arg(short, long, value_name = "BEAD_ID")]
    pub bead: Option<String>,

    /// Skip executing post_create hooks
    #[arg(long)]
    pub no_hooks: bool,

    /// Create workspace without opening terminal
    #[arg(long)]
    pub no_open: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Show example JSON output without executing
    #[arg(long)]
    pub example_json: bool,

    /// Succeed if session already exists (safe for retries)
    #[arg(long)]
    pub idempotent: bool,

    /// Preview without creating
    #[arg(long)]
    pub dry_run: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// List command - List all sessions
#[derive(Parser, Debug)]
#[command(about = "List all sessions")]
pub struct ListCommand {
    /// Include completed and failed sessions
    #[arg(long)]
    pub all: bool,

    /// Show verbose output with workspace paths and bead titles
    #[arg(short, long)]
    pub verbose: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Filter sessions by bead ID
    #[arg(long, value_name = "BEAD_ID")]
    pub bead: Option<String>,

    /// Filter sessions by agent owner
    #[arg(long, value_name = "NAME")]
    pub agent: Option<String>,

    /// Filter sessions by workspace state
    #[arg(long, value_name = "STATE")]
    pub state: Option<String>,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Bookmark subcommands for managing JJ bookmarks/branches
#[derive(Subcommand, Debug)]
pub enum BookmarkSubcommands {
    /// List bookmarks in a session workspace
    List {
        /// Session name (uses current workspace if omitted)
        session: Option<String>,
        /// Show all bookmarks including remote
        #[arg(short, long)]
        all: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a new bookmark at current revision
    Create {
        /// Name for the new bookmark
        #[arg(required = true)]
        name: String,
        /// Session name (uses current workspace if omitted)
        session: Option<String>,
        /// Push bookmark to remote after creation
        #[arg(short, long)]
        push: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Delete a bookmark
    Delete {
        /// Name of the bookmark to delete
        #[arg(required = true)]
        name: String,
        /// Session name (uses current workspace if omitted)
        session: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Move a bookmark to a different revision
    Move {
        /// Name of the bookmark to move
        #[arg(required = true)]
        name: String,
        /// Target revision (commit hash or revset)
        #[arg(long, required = true, value_name = "REVISION")]
        to: String,
        /// Session name (uses current workspace if omitted)
        session: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Bookmark command - Manage JJ bookmarks/branches
#[derive(Parser, Debug)]
#[command(
    about = "Manage JJ bookmarks/branches",
    long_about = "Manage bookmarks (branches) in JJ workspaces.\n\nisolate wraps JJ completely - use 'isolate bookmark' not 'jj bookmark'.\n\nProvides: list, create, delete, move operations."
)]
pub struct BookmarkCommand {
    #[command(subcommand)]
    pub command: BookmarkSubcommands,
}

/// Remove command - Remove a session and its workspace
#[derive(Parser, Debug)]
#[command(about = "Remove a session and its workspace")]
pub struct RemoveCommand {
    /// Name of the session to remove
    #[arg(required_unless_present_any = ["contract", "ai_hints"])]
    pub name: Option<String>,

    /// Skip pre_remove hooks (no-op for confirmation)
    #[arg(short, long)]
    pub force: bool,

    /// Squash-merge to main before removal
    #[arg(short, long)]
    pub merge: bool,

    /// Preserve branch after removal
    #[arg(short, long)]
    pub keep_branch: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Succeed if session doesn't exist (safe for retries)
    #[arg(long)]
    pub idempotent: bool,

    /// Preview removal without executing
    #[arg(long)]
    pub dry_run: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Focus command - Switch to session's workspace
#[derive(Parser, Debug)]
#[command(
    about = "Switch to session's workspace",
    long_about = "Switch to a session's workspace.\n\nUse this to navigate between workspaces."
)]
pub struct FocusCommand {
    /// Name of the session to focus (interactive if omitted)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Status command - Show detailed session status
#[derive(Parser, Debug)]
#[command(about = "Show detailed session status")]
pub struct StatusCommand {
    /// Session name to show status for (shows all if omitted)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Continuously update status (1s refresh)
    #[arg(long)]
    pub watch: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Switch command - Switch to a different workspace
#[derive(Parser, Debug)]
#[command(
    about = "Switch to a different workspace",
    long_about = "Navigate between workspaces.\n\nUse this for quick workspace switching. Similar to 'isolate focus' but emphasizes navigation between existing sessions."
)]
pub struct SwitchCommand {
    /// Name of the session to switch to (interactive if omitted)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Show session details after switching
    #[arg(long)]
    pub show_context: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Sync command - Sync session workspace with main (rebase onto latest)
#[derive(Parser, Debug)]
#[command(about = "Sync session workspace with main (rebase onto latest)")]
pub struct SyncCommand {
    /// Session name to sync (default: sync current workspace only)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Sync ALL active sessions (must be explicit)
    #[arg(long)]
    pub all: bool,

    /// Preview sync without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Submit command - Submit changes for review/merge
#[derive(Parser, Debug)]
#[command(
    about = "Submit changes for review/merge",
    long_about = "Prepares and submits the current workspace changes for review or direct merge.\n\nThis command will:\n1. Validate workspace state\n2. Optionally commit changes\n3. Create merge request or merge directly\n\nUse --dry-run to preview what would happen."
)]
pub struct SubmitCommand {
    /// Session name to submit (default: current workspace)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Show what would happen without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Automatically commit changes if needed
    #[arg(long)]
    pub auto_commit: bool,

    /// Custom commit message
    #[arg(short, long, value_name = "MESSAGE")]
    pub message: Option<String>,
}

/// Diff command - Show diff between session and main branch
#[derive(Parser, Debug)]
#[command(about = "Show diff between session and main branch")]
pub struct DiffCommand {
    /// Session name to show diff for (auto-detected if not provided)
    #[arg(required = false)]
    pub name: Option<String>,

    /// Show diffstat only (summary of changes)
    #[arg(long)]
    pub stat: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Config command - View or modify configuration
#[derive(Parser, Debug)]
#[command(about = "View or modify configuration", alias = "cfg")]
pub struct ConfigCommand {
    /// Config key to view/set (dot notation)
    pub key: Option<String>,

    /// Value to set (omit to view)
    pub value: Option<String>,

    /// Operate on global config instead of project
    #[arg(short, long)]
    pub global: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Clean command - Remove stale sessions (where workspace no longer exists)
#[derive(Parser, Debug)]
#[command(about = "Remove stale sessions (where workspace no longer exists)")]
pub struct CleanCommand {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub force: bool,

    /// List stale sessions without removing
    #[arg(long)]
    pub dry_run: bool,

    /// Run as periodic cleanup daemon (1hr interval)
    #[arg(long)]
    pub periodic: bool,

    /// Age threshold for periodic cleanup (default: 7200 = 2hr)
    #[arg(long, value_name = "SECONDS")]
    pub age_threshold: Option<u64>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Prune invalid command - Remove all invalid session records
#[derive(Parser, Debug)]
#[command(
    about = "Remove all invalid session records in one deterministic command",
    long_about = "Bulk cleanup primitive to remove all invalid session records.\n\nInvalid sessions are those where the workspace directory no longer exists but the session record still exists in the database.\n\nThis is useful for cleaning up after workspace directory deletions or when sessions become orphaned.\n\nUse --yes to skip confirmation for scripting/CI use."
)]
pub struct PruneInvalidCommand {
    /// Skip confirmation prompt (for scripting/CI)
    #[arg(short, long)]
    pub yes: bool,

    /// List invalid sessions without removing
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Introspect command - Discover isolate capabilities and command details
#[derive(Parser, Debug)]
#[command(
    about = "Discover isolate capabilities and command details",
    long_about = "AI-optimized capability discovery.\n\nUse this to understand:\n- Available commands and their arguments\n- System state and dependencies\n- Environment variables isolate uses\n- Common workflow patterns"
)]
pub struct IntrospectCommand {
    /// Command to introspect (shows all if omitted)
    #[arg(required = false)]
    pub command: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI-optimized output: combines capabilities, state, and recommendations
    #[arg(long)]
    pub ai: bool,

    /// Show environment variables isolate reads and sets
    #[arg(long)]
    pub env_vars: bool,

    /// Show common workflow patterns for AI agents
    #[arg(long)]
    pub workflows: bool,

    /// Show valid session state transitions
    #[arg(long)]
    pub session_states: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Integrity subcommands for workspace integrity management
#[derive(Subcommand, Debug)]
pub enum IntegritySubcommands {
    /// Validate workspace integrity
    Validate {
        /// Workspace name or path
        #[arg(required = true)]
        workspace: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Repair corrupted workspace
    Repair {
        /// Workspace name or path
        #[arg(required = true)]
        workspace: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Update session record when workspace is detected in a new location
        #[arg(long)]
        rebind: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage workspace backups
    Backup {
        #[command(subcommand)]
        command: BackupSubcommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum BackupSubcommands {
    /// List available backups
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Restore from a backup
    Restore {
        /// Backup ID to restore
        #[arg(required = true)]
        backup_id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Integrity command - Manage workspace integrity and corruption recovery
#[derive(Parser, Debug)]
#[command(about = "Manage workspace integrity and corruption recovery")]
pub struct IntegrityCommand {
    #[command(subcommand)]
    pub command: IntegritySubcommands,
}

/// Query command - Query system state programmatically
#[derive(Parser, Debug)]
#[command(about = "Query system state programmatically")]
pub struct QueryCommand {
    /// Type of query (session-exists, session-count, can-run, suggest-name)
    #[arg(required_unless_present_any = ["contract", "ai_hints"])]
    pub query_type: Option<String>,

    /// Query-specific arguments
    #[arg(required = false, allow_hyphen_values = true)]
    pub args: Option<String>,

    /// Output as JSON (default for query)
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Context command - Show complete environment context (AI agent query)
#[derive(Parser, Debug)]
#[command(
    about = "Show complete environment context (AI agent query)",
    long_about = "Returns:\n- Agent ID if registered (from HARDLINE_AGENT_ID env var)\n- 'unregistered' if no agent registered\n- Also shows current session and bead from environment"
)]
pub struct ContextCommand {
    /// Output as JSON (default when not TTY)
    #[arg(long)]
    pub json: bool,

    /// Extract single field (e.g., --field=repository.branch)
    #[arg(long, value_name = "PATH")]
    pub field: Option<String>,

    /// Skip beads database query (faster)
    #[arg(long)]
    pub no_beads: bool,

    /// Skip health checks (faster)
    #[arg(long)]
    pub no_health: bool,

    /// Show machine-readable contract for AI agents
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Spawn command - Create session for automated agent work on a bead (issue)
#[derive(Parser, Debug)]
#[command(
    about = "Create session for automated agent work on a bead (issue)",
    long_about = "Creates a JJ workspace, runs an agent (default: claude), and auto-merges on success.\n\nUse this when an AI AGENT should work autonomously on a bead.\n\nFor manual interactive work, use 'workspace add' instead."
)]
pub struct SpawnCommand {
    /// Bead ID to work on (e.g., isolate-xxxx)
    #[arg(required = true)]
    pub bead: String,

    /// Agent command to run
    #[arg(long, value_name = "COMMAND", default_value = "claude")]
    pub agent_command: String,

    /// Additional agent arguments
    #[arg(long, value_name = "ARGS")]
    pub agent_args: Option<Vec<String>>,

    /// Don't merge on success
    #[arg(long)]
    pub no_auto_merge: bool,

    /// Don't cleanup on failure
    #[arg(long)]
    pub no_auto_cleanup: bool,

    /// Run agent in background
    #[arg(short, long)]
    pub background: bool,

    /// Timeout in seconds (default: 14400 = 4 hours)
    #[arg(long, value_name = "SECONDS", default_value = "14400")]
    pub timeout: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,

    /// Succeed if workspace already exists (safe for retries)
    #[arg(long)]
    pub idempotent: bool,

    /// Preview spawn without executing
    #[arg(long)]
    pub dry_run: bool,
}

/// Checkpoint subcommands for save/restore snapshots
#[derive(Subcommand, Debug)]
pub enum CheckpointSubcommands {
    /// Create a checkpoint of all current sessions
    Create {
        /// Description for this checkpoint
        #[arg(short, long, value_name = "DESC")]
        description: Option<String>,
    },
    /// Restore sessions to a checkpoint state
    Restore {
        /// Checkpoint ID to restore
        #[arg(required = true)]
        checkpoint_id: String,
    },
    /// List all available checkpoints
    List,
}

/// Checkpoint command - Save and restore session state snapshots
#[derive(Parser, Debug)]
#[command(about = "Save and restore session state snapshots", alias = "ckpt")]
pub struct CheckpointCommand {
    #[command(subcommand)]
    pub command: CheckpointSubcommands,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,
}

/// Done command - Complete work and merge workspace to main
#[derive(Parser, Debug)]
#[command(
    about = "Complete work and merge workspace to main",
    long_about = "Completes work and merges the workspace to main."
)]
pub struct DoneCommand {
    /// Workspace to complete (uses current if not specified)
    #[arg(short, long, value_name = "NAME")]
    pub workspace: Option<String>,

    /// Commit message (auto-generated if not provided)
    #[arg(short, long, value_name = "MSG")]
    pub message: Option<String>,

    /// Keep workspace after merge
    #[arg(long)]
    pub keep_workspace: bool,

    /// Squash all commits into one
    #[arg(long)]
    pub squash: bool,

    /// Preview without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Check for conflicts before merging
    #[arg(long)]
    pub detect_conflicts: bool,

    /// Skip bead status update
    #[arg(long)]
    pub no_bead_update: bool,

    /// Skip workspace retention (cleanup immediately)
    #[arg(long)]
    pub no_keep: bool,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show workflow patterns and best practices
    #[arg(long)]
    pub ai_hints: bool,
}

/// Undo command - Revert last done operation
#[derive(Parser, Debug)]
#[command(
    about = "Revert last done operation",
    long_about = "Reverts the most recent 'done' operation, rolling back to the state before the merge.\n\nWorks only if changes haven't been pushed to remote.\n\nUndo history is kept for 24 hours."
)]
pub struct UndoCommand {
    /// List undo history without reverting
    #[arg(short, long)]
    pub list: bool,

    /// Preview without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

/// Revert command - Revert specific session merge
#[derive(Parser, Debug)]
#[command(
    about = "Revert specific session merge",
    long_about = "Reverts a specific session's merge operation, identified by session name.\n\nWorks only if changes haven't been pushed to remote.\n\nUndo history is kept for 24 hours."
)]
pub struct RevertCommand {
    /// Name of session to revert
    #[arg(required = true)]
    pub name: String,

    /// Preview without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

/// Whereami command - Quick location query
#[derive(Parser, Debug)]
#[command(
    about = "Quick location query - returns 'main' or 'workspace:<name>'",
    long_about = "AI-optimized command for quick orientation.\n\nReturns a simple, parseable string:\n- 'main' if on main branch\n- 'workspace:<name>' if in a workspace\n\nUse this before operations that depend on location."
)]
pub struct WhereamiCommand {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Whoami command - Agent identity query
#[derive(Parser, Debug)]
#[command(
    about = "Agent identity query - returns agent ID or 'unregistered'",
    long_about = "AI-optimized command for identity verification.\n\nReturns:\n- Agent ID if registered (from HARDLINE_AGENT_ID env var)\n- 'unregistered' if no agent registered\n\nAlso shows current session and bead from environment."
)]
pub struct WhoamiCommand {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Work command - Start working on a task (create workspace + register agent)
#[derive(Parser, Debug)]
#[command(
    about = "Start working on a task (create workspace + register agent)",
    long_about = "Unified workflow start command for AI agents.\n\nCombines multiple steps:\n1. Create workspace (or reuse if --idempotent)\n2. Register as agent (unless --no-agent)\n3. Set environment variables\n4. Output workspace info\n\nThis is the AI-friendly entry point for starting work."
)]
pub struct WorkCommand {
    /// Session name to create/use
    #[arg(required_unless_present_any = ["contract", "ai_hints"])]
    pub name: Option<String>,

    /// Bead ID to associate with this work
    #[arg(short, long, value_name = "BEAD_ID")]
    pub bead: Option<String>,

    /// Agent ID to register (auto-generated if not provided)
    #[arg(long, value_name = "ID")]
    pub agent_id: Option<String>,

    /// Don't register as agent
    #[arg(long)]
    pub no_agent: bool,

    /// Succeed if session already exists (safe for retries)
    #[arg(long)]
    pub idempotent: bool,

    /// Preview without creating
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show workflow patterns and best practices
    #[arg(long)]
    pub ai_hints: bool,
}

/// CanI command - Check if an action is permitted
#[derive(Parser, Debug)]
#[command(
    about = "Check if an action is permitted",
    long_about = "Checks preconditions before attempting operations.\n\nReturns whether an action is allowed, and if not, what prerequisites are missing.\n\nUseful for AI agents to check before executing commands."
)]
pub struct CanICommand {
    /// Action to check (add, remove, done, undo, sync, spawn, claim, merge)
    #[arg(required = true)]
    pub action: String,

    /// Resource to check (session name, bead ID, etc.)
    #[arg(required = false)]
    pub resource: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Contract command - Show command contracts for AI integration
#[derive(Parser, Debug)]
#[command(
    about = "Show command contracts for AI integration",
    long_about = "Displays structured contracts for commands, including:\n- Input/output schemas\n- Argument types and constraints\n- Flags and their effects\n- Side effects and rollback information\n\nUseful for AI agents to understand command capabilities."
)]
pub struct ContractCommand {
    /// Command to show contract for (shows all if omitted)
    #[arg(required = false)]
    pub command: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Examples command - Show usage examples for commands
#[derive(Parser, Debug)]
#[command(about = "Show usage examples for commands")]
pub struct ExamplesCommand {
    /// Filter examples for specific command
    #[arg(required = false)]
    pub command: Option<String>,

    /// Filter by use case (workflow, single-command, error-handling, etc.)
    #[arg(long, value_name = "CASE")]
    pub use_case: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Help command - Print help for a command
#[derive(Parser, Debug)]
#[command(about = "Print help for a command")]
pub struct HelpCommand {
    /// Command path to show help for (omit for top-level help)
    #[arg(required = false, num_args = 0..)]
    pub command: Option<Vec<String>>,
}

/// Validate command - Pre-validate inputs before execution
#[derive(Parser, Debug)]
#[command(
    about = "Pre-validate inputs before execution",
    long_about = "Validates inputs without executing commands.\n\nUse this to check:\n- Session name format\n- Bead ID format\n- Required arguments\n- Reserved names\n\nReturns structured validation results for AI agents."
)]
pub struct ValidateCommand {
    /// Command to validate inputs for
    #[arg(required_unless_present = "contract")]
    pub command: Option<String>,

    /// Arguments to validate
    #[arg(num_args = 0..)]
    pub args: Option<Vec<String>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Preview validation without side effects
    #[arg(long)]
    pub dry_run: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,
}

/// Whatif command - Preview command effects without executing
#[derive(Parser, Debug)]
#[command(
    about = "Preview command effects without executing",
    long_about = "Shows what a command would do without actually doing it.\n\nMore detailed than --dry-run, includes:\n- Steps that would be executed\n- Resource changes (files, sessions)\n- Prerequisite checks\n- Reversibility information"
)]
pub struct WhatifCommand {
    /// Command to preview
    #[arg(required_unless_present_any = ["contract", "ai_hints"])]
    pub command: Option<String>,

    /// Command arguments
    #[arg(num_args = 0..)]
    pub args: Option<Vec<String>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,
}

/// Events command - Listen for or query system events
#[derive(Parser, Debug)]
#[command(
    about = "Listen for or query system events",
    long_about = "Provides access to the event log.\n\nUse this to track session lifecycle, agent heartbeats, and resource claims."
)]
pub struct EventsCommand {
    /// Filter by session
    #[arg(long, value_name = "NAME")]
    pub session: Option<String>,

    /// Filter by event type
    #[arg(long, value_name = "TYPE")]
    pub type_filter: Option<String>,

    /// Limit number of events returned
    #[arg(short, long, value_name = "COUNT")]
    pub limit: Option<usize>,

    /// Stream new events as they occur
    #[arg(short, long)]
    pub follow: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Completions command - Generate shell completions
#[derive(Parser, Debug)]
#[command(about = "Generate shell completions")]
pub struct CompletionsCommand {
    /// Shell to generate completions for
    #[arg(required = true)]
    pub shell: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Rename command - Rename an existing session
#[derive(Parser, Debug)]
#[command(about = "Rename an existing session")]
pub struct RenameCommand {
    /// Current session name
    #[arg(required = true)]
    pub old_name: String,

    /// New session name
    #[arg(required = true)]
    pub new_name: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Pause command - Pause an active session (suspend agent work)
#[derive(Parser, Debug)]
#[command(about = "Pause an active session (suspend agent work)")]
pub struct PauseCommand {
    /// Session name to pause
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Resume command - Resume a paused session
#[derive(Parser, Debug)]
#[command(about = "Resume a paused session")]
pub struct ResumeCommand {
    /// Session name to resume
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Clone command - Clone a session into a new one
#[derive(Parser, Debug)]
#[command(about = "Clone a session into a new one")]
pub struct CloneCommand {
    /// Source session name
    #[arg(required = true)]
    pub source: String,

    /// Destination session name
    #[arg(required = true)]
    pub dest: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Export command - Export session state to a file
#[derive(Parser, Debug)]
#[command(
    about = "Export session state to a file",
    long_about = "Export session state to a file or stdout.\n\nThe SESSION argument specifies which session to export. If omitted, all sessions are exported.\n\nIMPORTANT: Output file paths require -o/--output flag to prevent ambiguity."
)]
pub struct ExportCommand {
    /// Session name to export (all if omitted)
    pub session: Option<String>,

    /// Output file path (REQUIRED when writing to a file)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Import command - Import session state from a file
#[derive(Parser, Debug)]
#[command(about = "Import session state from a file")]
pub struct ImportCommand {
    /// Input file path
    #[arg(required = true)]
    pub file: String,

    /// Overwrite existing sessions
    #[arg(short, long)]
    pub force: bool,

    /// Skip sessions that already exist
    #[arg(long)]
    pub skip_existing: bool,

    /// Preview import without changes
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Wait command - Wait for a condition to be met
#[derive(Parser, Debug)]
#[command(about = "Wait for a condition to be met")]
pub struct WaitCommand {
    /// Condition to wait for
    #[arg(required = true, value_enum)]
    pub condition: WaitCondition,

    /// Session name (for session conditions)
    pub name: Option<String>,

    /// Expected status (for session-status condition)
    #[arg(long)]
    pub status: Option<String>,

    /// Timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: f64,

    /// Polling interval in seconds
    #[arg(short, long, default_value = "1")]
    pub interval: f64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum WaitCondition {
    SessionExists,
    SessionUnlocked,
    Healthy,
    SessionStatus,
}

/// Schema command - Show JSON schemas for isolate protocol
#[derive(Parser, Debug)]
#[command(about = "Show JSON schemas for isolate protocol")]
pub struct SchemaCommand {
    /// Schema name (e.g., add-response)
    pub name: Option<String>,

    /// List all available schemas
    #[arg(short, long)]
    pub list: bool,

    /// Show all schemas
    #[arg(short, long)]
    pub all: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Recover command - Recover from inconsistent state or restore from operation log
#[derive(Parser, Debug)]
#[command(
    about = "Recover from inconsistent state or restore from operation log",
    long_about = "Recover from inconsistent state or restore from operation log."
)]
pub struct RecoverCommand {
    /// Session name to recover (optional, uses current workspace if not specified)
    #[arg(value_name = "SESSION", num_args = 0..=1)]
    pub session: Option<String>,

    /// Only diagnose system issues without fixing (system recovery mode)
    #[arg(short, long)]
    pub diagnose: bool,

    /// Restore to specific operation ID (operation log mode)
    #[arg(long, value_name = "ID")]
    pub op: Option<String>,

    /// Restore to previous operation (quick undo)
    #[arg(long)]
    pub last: bool,

    /// List operation log without restoring (default when no --op or --last)
    #[arg(long)]
    pub list_ops: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Retry command - Retry the last failed operation
#[derive(Parser, Debug)]
#[command(about = "Retry the last failed operation")]
pub struct RetryCommand {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Rollback command - Rollback session to a specific checkpoint
#[derive(Parser, Debug)]
#[command(about = "Rollback session to a specific checkpoint")]
pub struct RollbackCommand {
    /// Session name
    #[arg(required = true)]
    pub session: String,

    /// Checkpoint ID to rollback to
    #[arg(long, required = true)]
    pub to: String,

    /// Preview rollback without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// AI: Show machine-readable contract
    #[arg(long)]
    pub contract: bool,

    /// AI: Show command flow hints
    #[arg(long)]
    pub ai_hints: bool,
}

/// Abort command - Abort work and abandon workspace changes
#[derive(Parser, Debug)]
#[command(
    about = "Abort work and abandon workspace changes",
    long_about = "Abort work and abandon workspace changes."
)]
pub struct AbortCommand {
    /// Workspace/session to abort (uses current if not specified)
    #[arg(short, long, visible_alias = "session", value_name = "NAME")]
    pub workspace: Option<String>,

    /// Don't update bead status
    #[arg(long)]
    pub no_bead_update: bool,

    /// Keep workspace files (just remove from tracking)
    #[arg(long)]
    pub keep_workspace: bool,

    /// Preview without executing
    #[arg(long)]
    pub dry_run: bool,

    /// AI: Show machine-readable contract (JSON schema of inputs/outputs)
    #[arg(long)]
    pub contract: bool,

    /// AI: Show execution hints and common patterns
    #[arg(long)]
    pub ai_hints: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Backup command - Manage automated database backups
#[derive(Parser, Debug)]
#[command(
    about = "Manage automated database backups",
    long_about = "Create, list, restore, and manage backups of databases.\n\nBackups include:\n- state.db: Session, workspace state, and merge queue\n- beads.db: Issue tracking database\n\nBackups are stored with timestamps and SHA-256 checksums for integrity verification."
)]
pub struct BackupCommand {
    /// Create new backups of all databases
    #[arg(long)]
    pub create: bool,

    /// List all available backups
    #[arg(long)]
    pub list: bool,

    /// Restore database from backup (state.db, beads.db)
    #[arg(long, value_name = "DATABASE")]
    pub restore: Option<String>,

    /// Specific backup timestamp to restore (format: YYYYMMDD-HHMMSS)
    #[arg(short, long, requires = "restore", value_name = "TIMESTAMP")]
    pub timestamp: Option<String>,

    /// Show backup status and retention policy information
    #[arg(long)]
    pub status: bool,

    /// Apply retention policy and remove old backups
    #[arg(long)]
    pub retention: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Isolate port top-level commands
/// These are commands that were ported from isolate's CLI
#[derive(Subcommand, Debug)]
pub enum IsolatePortCommands {
    /// AI-first commands for streamlined workflows
    Ai(AiCommands),
    /// Create session for manual work
    Add(AddCommand),
    /// List all sessions
    List(ListCommand),
    /// Manage JJ bookmarks/branches
    Bookmark(BookmarkCommand),
    /// Remove a session and its workspace
    Remove(RemoveCommand),
    /// Switch to session's workspace
    Focus(FocusCommand),
    /// Show detailed session status
    Status(StatusCommand),
    /// Switch to a different workspace
    Switch(SwitchCommand),
    /// Sync session workspace with main
    Sync(SyncCommand),
    /// Submit changes for review/merge
    Submit(SubmitCommand),
    /// Show diff between session and main branch
    Diff(DiffCommand),
    /// View or modify configuration
    Config(ConfigCommand),
    /// Remove stale sessions
    Clean(CleanCommand),
    /// Remove all invalid session records
    PruneInvalid(PruneInvalidCommand),
    /// Discover capabilities and command details
    Introspect(IntrospectCommand),
    /// Manage workspace integrity
    Integrity(IntegrityCommand),
    /// Query system state programmatically
    Query(QueryCommand),
    /// Show complete environment context
    Context(ContextCommand),
    /// Create session for automated agent work
    Spawn(SpawnCommand),
    /// Save and restore session state snapshots
    Checkpoint(CheckpointCommand),
    /// Complete work and merge to main
    Done(DoneCommand),
    /// Revert last done operation
    Undo(UndoCommand),
    /// Revert specific session merge
    Revert(RevertCommand),
    /// Quick location query
    Whereami(WhereamiCommand),
    /// Agent identity query
    Whoami(WhoamiCommand),
    /// Start working on a task
    Work(WorkCommand),
    /// Check if an action is permitted
    CanI(CanICommand),
    /// Show command contracts for AI integration
    Contract(ContractCommand),
    /// Show usage examples for commands
    Examples(ExamplesCommand),
    /// Print help for a command
    Help(HelpCommand),
    /// Pre-validate inputs before execution
    Validate(ValidateCommand),
    /// Preview command effects without executing
    Whatif(WhatifCommand),
    /// Listen for or query system events
    Events(EventsCommand),
    /// Generate shell completions
    Completions(CompletionsCommand),
    /// Rename an existing session
    Rename(RenameCommand),
    /// Pause an active session
    Pause(PauseCommand),
    /// Resume a paused session
    Resume(ResumeCommand),
    /// Clone a session into a new one
    Clone(CloneCommand),
    /// Export session state to a file
    Export(ExportCommand),
    /// Import session state from a file
    Import(ImportCommand),
    /// Wait for a condition to be met
    Wait(WaitCommand),
    /// Show JSON schemas for protocol
    Schema(SchemaCommand),
    /// Recover from inconsistent state
    Recover(RecoverCommand),
    /// Retry the last failed operation
    Retry(RetryCommand),
    /// Rollback session to a checkpoint
    Rollback(RollbackCommand),
    /// Abort work and abandon workspace changes
    Abort(AbortCommand),
    /// Manage automated database backups
    Backup(BackupCommand),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_commands_parse() {
        let cmd = AiCommands::try_parse_from(["ai", "work", "task-123"]);
        assert!(cmd.is_ok());
        let AiCommands { command } = cmd.unwrap();
        match command {
            AiSubcommands::Work { task_id } => assert_eq!(task_id, Some("task-123".to_string())),
        }
    }

    #[test]
    fn test_add_command_parse() {
        let cmd = AddCommand::try_parse_from(["add", "my-session"]);
        assert!(cmd.is_ok());
        let AddCommand { name, .. } = cmd.unwrap();
        assert_eq!(name, Some("my-session".to_string()));
    }

    #[test]
    fn test_add_command_with_bead() {
        let cmd = AddCommand::try_parse_from(["add", "-b", "bead-123", "my-session"]);
        assert!(cmd.is_ok());
        let AddCommand { name, bead, .. } = cmd.unwrap();
        assert_eq!(name, Some("my-session".to_string()));
        assert_eq!(bead, Some("bead-123".to_string()));
    }

    #[test]
    fn test_list_command_parse() {
        let cmd = ListCommand::try_parse_from(["list", "--all", "-v"]);
        assert!(cmd.is_ok());
        let ListCommand { all, verbose, .. } = cmd.unwrap();
        assert!(all);
        assert!(verbose);
    }

    #[test]
    fn test_spawn_command_parse() {
        let cmd =
            SpawnCommand::try_parse_from(["spawn", "bead-abc", "--agent-command", "claude", "-b"]);
        assert!(cmd.is_ok());
        let SpawnCommand {
            bead,
            agent_command,
            background,
            ..
        } = cmd.unwrap();
        assert_eq!(bead, "bead-abc");
        assert_eq!(agent_command, "claude");
        assert!(background);
    }

    #[test]
    fn test_done_command_parse() {
        let cmd = DoneCommand::try_parse_from(["done", "-m", "Fix auth bug", "--squash"]);
        assert!(cmd.is_ok());
        let DoneCommand {
            message, squash, ..
        } = cmd.unwrap();
        assert_eq!(message, Some("Fix auth bug".to_string()));
        assert!(squash);
    }

    #[test]
    fn test_wait_condition_parse() {
        let cmd = WaitCommand::try_parse_from(["wait", "session-exists", "my-session", "-t", "60"]);
        assert!(cmd.is_ok());
        let WaitCommand {
            condition,
            name,
            timeout,
            ..
        } = cmd.unwrap();
        assert_eq!(condition, WaitCondition::SessionExists);
        assert_eq!(name, Some("my-session".to_string()));
        assert_eq!(timeout, 60.0);
    }

    #[test]
    fn test_checkpoint_subcommands() {
        let create =
            CheckpointCommand::try_parse_from(["checkpoint", "create", "-d", "before lunch"]);
        assert!(create.is_ok());

        let list = CheckpointCommand::try_parse_from(["checkpoint", "list"]);
        assert!(list.is_ok());

        let restore = CheckpointCommand::try_parse_from(["checkpoint", "restore", "ckpt-123"]);
        assert!(restore.is_ok());
    }

    #[test]
    fn test_bookmark_subcommands() {
        let list = BookmarkCommand::try_parse_from(["bookmark", "list", "--all"]);
        assert!(list.is_ok());

        let create = BookmarkCommand::try_parse_from(["bookmark", "create", "-p", "feature-x"]);
        assert!(create.is_ok());

        let delete = BookmarkCommand::try_parse_from(["bookmark", "delete", "old-fix"]);
        assert!(delete.is_ok());

        let move_cmd = BookmarkCommand::try_parse_from(["bookmark", "move", "stable", "--to", "@"]);
        assert!(move_cmd.is_ok());
    }

    #[test]
    fn test_integrity_subcommands() {
        let validate = IntegrityCommand::try_parse_from(["integrity", "validate", "my-workspace"]);
        assert!(validate.is_ok());

        let repair =
            IntegrityCommand::try_parse_from(["integrity", "repair", "-f", "my-workspace"]);
        assert!(repair.is_ok());
    }

    #[test]
    fn test_backup_subcommands() {
        let list = BackupCommand::try_parse_from(["backup", "--list"]);
        assert!(list.is_ok());

        let restore = BackupCommand::try_parse_from([
            "backup",
            "--restore",
            "state.db",
            "-t",
            "20250101-010101",
        ]);
        assert!(restore.is_ok());
    }

    #[test]
    fn test_whoami_command() {
        let cmd = WhoamiCommand::try_parse_from(["whoami", "--json"]);
        assert!(cmd.is_ok());
        let WhoamiCommand { json, .. } = cmd.unwrap();
        assert!(json);
    }

    #[test]
    fn test_context_command_with_field() {
        let cmd = ContextCommand::try_parse_from(["context", "--field=repository.branch"]);
        assert!(cmd.is_ok());
        let ContextCommand { field, .. } = cmd.unwrap();
        assert_eq!(field, Some("repository.branch".to_string()));
    }

    #[test]
    fn test_can_i_command() {
        let cmd = CanICommand::try_parse_from(["can-i", "done"]);
        assert!(cmd.is_ok());
        let CanICommand {
            action, resource, ..
        } = cmd.unwrap();
        assert_eq!(action, "done");
        assert_eq!(resource, None);
    }

    #[test]
    fn test_validate_command() {
        let cmd = ValidateCommand::try_parse_from(["validate", "add", "feature-x"]);
        assert!(cmd.is_ok());
        let ValidateCommand { command, args, .. } = cmd.unwrap();
        assert_eq!(command, Some("add".to_string()));
        assert_eq!(args, Some(vec!["feature-x".to_string()]));
    }

    #[test]
    fn test_whatif_command() {
        let cmd = WhatifCommand::try_parse_from(["whatif", "done", "my-session", "--json"]);
        assert!(cmd.is_ok());
        let WhatifCommand {
            command,
            args,
            json,
            ..
        } = cmd.unwrap();
        assert_eq!(command, Some("done".to_string()));
        assert_eq!(args, Some(vec!["my-session".to_string()]));
        assert!(json);
    }

    #[test]
    fn test_export_import_commands() {
        let export = ExportCommand::try_parse_from(["export", "-o", "state.json"]);
        assert!(export.is_ok());

        let import = ImportCommand::try_parse_from(["import", "state.json", "--dry-run"]);
        assert!(import.is_ok());
    }

    #[test]
    fn test_recover_command() {
        let cmd = RecoverCommand::try_parse_from(["recover", "--diagnose"]);
        assert!(cmd.is_ok());
        let RecoverCommand { diagnose, .. } = cmd.unwrap();
        assert!(diagnose);
    }

    #[test]
    fn test_abort_command() {
        let cmd = AbortCommand::try_parse_from(["abort", "-w", "feature-x", "--keep-workspace"]);
        assert!(cmd.is_ok());
        let AbortCommand {
            workspace,
            keep_workspace,
            ..
        } = cmd.unwrap();
        assert_eq!(workspace, Some("feature-x".to_string()));
        assert!(keep_workspace);
    }

    #[test]
    fn test_undo_command() {
        let cmd = UndoCommand::try_parse_from(["undo", "--list", "--dry-run"]);
        assert!(cmd.is_ok());
        let UndoCommand { list, dry_run, .. } = cmd.unwrap();
        assert!(list);
        assert!(dry_run);
    }

    #[test]
    fn test_pause_resume_commands() {
        let pause = PauseCommand::try_parse_from(["pause", "my-session"]);
        assert!(pause.is_ok());

        let resume = ResumeCommand::try_parse_from(["resume", "my-session"]);
        assert!(resume.is_ok());
    }

    #[test]
    fn test_clone_command() {
        let cmd = CloneCommand::try_parse_from(["clone", "source-session", "dest-session"]);
        assert!(cmd.is_ok());
        let CloneCommand { source, dest, .. } = cmd.unwrap();
        assert_eq!(source, "source-session");
        assert_eq!(dest, "dest-session");
    }

    #[test]
    fn test_rename_command() {
        let cmd = RenameCommand::try_parse_from(["rename", "old-name", "new-name"]);
        assert!(cmd.is_ok());
        let RenameCommand {
            old_name, new_name, ..
        } = cmd.unwrap();
        assert_eq!(old_name, "old-name");
        assert_eq!(new_name, "new-name");
    }

    #[test]
    fn test_events_command() {
        let cmd =
            EventsCommand::try_parse_from(["events", "-l", "20", "--follow", "--type", "session"]);
        assert!(cmd.is_ok());
        let EventsCommand {
            limit,
            follow,
            type_filter,
            ..
        } = cmd.unwrap();
        assert_eq!(limit, Some(20));
        assert!(follow);
        assert_eq!(type_filter, Some("session".to_string()));
    }

    #[test]
    fn test_schema_command() {
        let list = SchemaCommand::try_parse_from(["schema", "--list"]);
        assert!(list.is_ok());

        let show = SchemaCommand::try_parse_from(["schema", "add-response"]);
        assert!(show.is_ok());
    }

    #[test]
    fn test_retry_rollback_commands() {
        let retry = RetryCommand::try_parse_from(["retry", "--json"]);
        assert!(retry.is_ok());

        let rollback =
            RollbackCommand::try_parse_from(["rollback", "my-session", "--to", "ckpt-123"]);
        assert!(rollback.is_ok());
    }
}

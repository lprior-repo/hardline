//! Workspace command definitions
//!
//! Subcommand enum for workspace management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    /// Create a new workspace
    Spawn {
        /// Workspace name or task ID
        name: String,

        /// Sync with main after creation
        #[arg(short, long)]
        sync: bool,
    },

    /// Switch to a workspace
    Switch {
        /// Workspace name
        name: String,
    },

    /// List all workspaces
    List,

    /// Show workspace status
    Status,

    /// Sync workspace with main
    Sync {
        /// Workspace name (default: current)
        name: Option<String>,

        /// Sync all workspaces
        #[arg(short, long)]
        all: bool,
    },

    /// Complete workspace and merge
    Done {
        /// Workspace name (default: current)
        name: Option<String>,

        /// Commit message (auto-generated if not provided)
        #[arg(short, long)]
        message: Option<String>,

        /// Keep workspace after merge
        #[arg(long)]
        keep_workspace: bool,

        /// Squash all commits into one
        #[arg(long)]
        squash: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,

        /// Detect conflicts before merging
        #[arg(long)]
        detect_conflicts: bool,

        /// Skip bead status update
        #[arg(long)]
        no_bead_update: bool,
    },

    /// Abort workspace
    Abort {
        /// Workspace name (default: current)
        name: Option<String>,
    },

    /// Show workspace log
    Log {
        /// Number of commits to show
        limit: Option<usize>,
    },

    /// Show diff of changes
    Diff {
        /// File path to diff
        path: Option<String>,
    },

    /// Show uncommitted changes
    Uncommitted,

    /// Commit uncommitted changes
    Commit {
        /// Commit message
        message: String,
    },

    /// List branches
    Branches,

    /// Create a new branch
    Branch {
        /// Branch name
        name: String,
    },

    /// Delete a branch
    BranchDelete {
        /// Branch name
        name: String,
    },

    /// Show current branch
    BranchCurrent,

    /// Rename a branch
    BranchRename {
        /// Current branch name
        old_name: String,

        /// New branch name
        new_name: String,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Fork a workspace from current or another workspace
    Fork {
        /// Name of the new workspace
        name: String,

        /// Source workspace to fork from (default: current)
        from: Option<String>,
    },

    /// Merge workspace into main
    Merge {
        /// Workspace name to merge
        name: String,
    },

    /// Revert a specific session merge
    Revert {
        /// Session name to revert
        name: String,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate workspace integrity
    IntegrityValidate {
        /// Workspace name
        workspace: String,
    },

    /// Repair a corrupted workspace
    IntegrityRepair {
        /// Workspace name
        workspace: String,

        /// Force repair without confirmation
        #[arg(long)]
        force: bool,
    },

    /// List available backups
    IntegrityBackupList,

    /// Restore from a backup
    IntegrityBackupRestore {
        /// Backup ID to restore
        backup_id: String,

        /// Force restore without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Recover from broken Git/workspace state
    Recover {
        /// Target session or workspace (auto-detect if omitted)
        target: Option<String>,

        /// Diagnose only, don't fix
        #[arg(long)]
        diagnose: bool,

        /// Preview fixes without executing
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(long)]
        verbose: bool,
    },

    /// Rollback workspace to a specific commit
    Rollback {
        /// Session/workspace name
        session: String,

        /// Commit hash to rollback to
        commit: String,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Add an existing path as a workspace
    Add {
        /// Path to add
        path: String,
    },

    /// Rename a workspace/session
    Rename {
        /// Current session name
        old_name: String,

        /// New session name
        new_name: String,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Export sessions to file or stdout
    Export {
        /// Session to export (or all if omitted)
        session: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import sessions from file
    Import {
        /// Input file path
        input: String,

        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,

        /// Skip existing sessions
        #[arg(long)]
        skip_existing: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate command inputs
    Validate {
        /// Command to validate
        command: String,

        /// Arguments to validate
        args: Vec<String>,

        /// Preview without side effects
        #[arg(long)]
        dry_run: bool,
    },

    /// Show command contracts
    Contract {
        /// Specific command to show contract for
        command: Option<String>,
    },

    /// Query session state
    Query {
        /// Query type (session-exists, sessions, session-info, blockers, session-count, help)
        query_type: String,

        /// Query argument (e.g., session name)
        argument: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by agent
        #[arg(long)]
        agent: Option<String>,
    },

    /// Check if an operation is permitted
    CanI {
        /// Action to check (add, remove, done, merge, undo, sync, spawn)
        action: String,

        /// Resource to check (optional)
        resource: Option<String>,
    },

    /// Show session event history
    Events {
        /// Filter by session name
        #[arg(long)]
        session: Option<String>,

        /// Filter by event type
        #[arg(long)]
        event_type: Option<String>,

        /// Follow mode (stream new events)
        #[arg(short, long)]
        follow: bool,

        /// Maximum events to show
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Remove stale workspaces and temp files
    Clean {
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,

        /// Force cleanup without confirmation
        #[arg(long)]
        force: bool,

        /// Verbose output
        #[arg(long)]
        verbose: bool,
    },
    /// Manage bookmarks (Git branches)
    Bookmark {
        #[command(subcommand)]
        command: BookmarkCommands,
    },

    /// Show current work context
    Work {
        /// Session name to create/use
        name: Option<String>,

        /// Bead ID to associate
        #[arg(long)]
        bead: Option<String>,

        /// Agent ID to register
        #[arg(long)]
        agent: Option<String>,

        /// Don't register as agent
        #[arg(long)]
        no_agent: bool,

        /// Idempotent mode
        #[arg(long)]
        idempotent: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Show who you are (agent identity)
    Whoami {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Wait for a condition to be met
    Wait {
        /// Condition to wait for
        condition: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "60")]
        timeout: u64,

        /// Poll interval in seconds
        #[arg(long, default_value = "5")]
        poll_interval: u64,
    },

    /// Undo last workspace operation
    Undo {
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,

        /// List undo history
        #[arg(long)]
        list: bool,
    },

    /// Manage session checkpoints
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },

    /// Introspect command metadata
    Introspect {
        /// Command to introspect
        target: Option<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell type (bash, zsh, fish, powershell, elvish)
        shell: String,
    },

    /// Prune invalid/orphaned data
    Prune {
        /// Confirm pruning
        #[arg(long)]
        yes: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Display JSON Schema definitions
    Schema {
        /// Schema name to display
        name: Option<String>,

        /// List available schemas
        #[arg(long)]
        list: bool,

        /// Show all schemas
        #[arg(long)]
        all: bool,
    },
}

/// Bookmark subcommands
#[derive(Subcommand)]
pub enum BookmarkCommands {
    /// Create a bookmark
    Create {
        /// Bookmark name
        name: String,
    },
    /// List bookmarks
    List,
    /// Delete a bookmark
    Delete {
        /// Bookmark name
        name: String,
    },
    /// Track (set upstream for) a bookmark
    Track {
        /// Bookmark name
        name: String,
    },
}

/// Checkpoint subcommands
#[derive(Subcommand)]
pub enum CheckpointCommands {
    /// Create a checkpoint
    Create {
        /// Checkpoint description
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Restore from a checkpoint
    Restore {
        /// Checkpoint ID
        id: String,
    },
    /// List checkpoints
    List,
}

#[cfg(test)]
#[path = "workspace_args_tests.rs"]
mod tests;

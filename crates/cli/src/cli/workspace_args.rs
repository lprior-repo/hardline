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

    /// Show stack-aware commit log (branch history with lineage)
    StackLog {
        /// Number of commits per branch (default: 50)
        #[arg(short, long)]
        limit: Option<usize>,

        /// Output format: tree, linear, json
        #[arg(short, long, default_value = "tree")]
        format: String,

        /// Omit commit messages (show only branch structure)
        #[arg(long)]
        no_messages: bool,

        /// Show ahead/behind counts
        #[arg(long)]
        ahead_behind: bool,

        /// Filter to a specific branch and its lineage
        #[arg(short, long)]
        branch: Option<String>,
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
        #[arg(short, long)]
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
        #[arg(short, long)]
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
mod tests {
    use super::*;
    use clap::Parser;

    /// Wrapper to parse WorkspaceCommands directly via clap
    #[derive(Parser)]
    struct WorkspaceParser {
        #[command(subcommand)]
        command: WorkspaceCommands,
    }

    fn parse(args: &[&str]) -> WorkspaceCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        WorkspaceParser::parse_from(full).command
    }

    // -- List / Status / Uncommitted / Branches / BranchCurrent --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), WorkspaceCommands::List));
    }

    #[test]
    fn status_no_args() {
        assert!(matches!(parse(&["status"]), WorkspaceCommands::Status));
    }

    #[test]
    fn uncommitted_no_args() {
        assert!(matches!(
            parse(&["uncommitted"]),
            WorkspaceCommands::Uncommitted
        ));
    }

    #[test]
    fn branches_no_args() {
        assert!(matches!(parse(&["branches"]), WorkspaceCommands::Branches));
    }

    #[test]
    fn branch_current_no_args() {
        assert!(matches!(
            parse(&["branch-current"]),
            WorkspaceCommands::BranchCurrent
        ));
    }

    // -- Spawn (required name, bool flag) --
    #[test]
    fn spawn_default_sync() {
        match parse(&["spawn", "ws-name"]) {
            WorkspaceCommands::Spawn { name, sync } => {
                assert_eq!(name, "ws-name");
                assert!(!sync);
            }
            other => panic!("Expected Spawn, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn spawn_with_sync() {
        match parse(&["spawn", "ws-name", "--sync"]) {
            WorkspaceCommands::Spawn { sync, .. } => assert!(sync),
            other => panic!("Expected Spawn, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn spawn_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "spawn"]);
        assert!(result.is_err());
    }

    // -- Switch (required name) --
    #[test]
    fn switch_parses() {
        match parse(&["switch", "ws-name"]) {
            WorkspaceCommands::Switch { name } => assert_eq!(name, "ws-name"),
            other => panic!("Expected Switch, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn switch_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "switch"]);
        assert!(result.is_err());
    }

    // -- Sync (optional name, bool flag) --
    #[test]
    fn sync_defaults() {
        match parse(&["sync"]) {
            WorkspaceCommands::Sync { name, all } => {
                assert_eq!(name, None);
                assert!(!all);
            }
            other => panic!("Expected Sync, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn sync_with_name_and_all() {
        match parse(&["sync", "ws-name", "--all"]) {
            WorkspaceCommands::Sync { name, all } => {
                assert_eq!(name, Some("ws-name".to_string()));
                assert!(all);
            }
            other => panic!("Expected Sync, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Done (all bool flags default false, optional fields default None) --
    #[test]
    fn done_all_defaults() {
        match parse(&["done"]) {
            WorkspaceCommands::Done {
                name,
                message,
                keep_workspace,
                squash,
                dry_run,
                detect_conflicts,
                no_bead_update,
            } => {
                assert_eq!(name, None);
                assert_eq!(message, None);
                assert!(!keep_workspace);
                assert!(!squash);
                assert!(!dry_run);
                assert!(!detect_conflicts);
                assert!(!no_bead_update);
            }
            other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn done_all_flags_set() {
        match parse(&[
            "done",
            "ws-name",
            "-m",
            "msg",
            "--keep-workspace",
            "--squash",
            "--dry-run",
            "--detect-conflicts",
            "--no-bead-update",
        ]) {
            WorkspaceCommands::Done {
                name,
                message,
                keep_workspace,
                squash,
                dry_run,
                detect_conflicts,
                no_bead_update,
            } => {
                assert_eq!(name, Some("ws-name".to_string()));
                assert_eq!(message, Some("msg".to_string()));
                assert!(keep_workspace);
                assert!(squash);
                assert!(dry_run);
                assert!(detect_conflicts);
                assert!(no_bead_update);
            }
            other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Abort (optional name) --
    #[test]
    fn abort_defaults() {
        match parse(&["abort"]) {
            WorkspaceCommands::Abort { name } => assert_eq!(name, None),
            other => panic!("Expected Abort, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn abort_with_name() {
        match parse(&["abort", "ws-name"]) {
            WorkspaceCommands::Abort { name } => {
                assert_eq!(name, Some("ws-name".to_string()));
            }
            other => panic!("Expected Abort, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Log (optional limit) --
    #[test]
    fn log_defaults() {
        match parse(&["log"]) {
            WorkspaceCommands::Log { limit } => assert_eq!(limit, None),
            other => panic!("Expected Log, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn log_with_limit() {
        match parse(&["log", "10"]) {
            WorkspaceCommands::Log { limit } => assert_eq!(limit, Some(10)),
            other => panic!("Expected Log, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- StackLog (optional limit, format, flags) --
    #[test]
    fn stack_log_defaults() {
        match parse(&["stack-log"]) {
            WorkspaceCommands::StackLog {
                limit,
                format,
                no_messages,
                ahead_behind,
                branch,
            } => {
                assert_eq!(limit, None);
                assert_eq!(format, "tree");
                assert!(!no_messages);
                assert!(!ahead_behind);
                assert!(branch.is_none());
            }
            other => panic!("Expected StackLog, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn stack_log_with_limit() {
        match parse(&["stack-log", "-l", "20"]) {
            WorkspaceCommands::StackLog { limit, .. } => assert_eq!(limit, Some(20)),
            other => panic!("Expected StackLog, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn stack_log_json_format() {
        match parse(&["stack-log", "-f", "json"]) {
            WorkspaceCommands::StackLog { format, .. } => assert_eq!(format, "json"),
            other => panic!("Expected StackLog, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn stack_log_with_branch_filter() {
        match parse(&["stack-log", "-b", "feature-a"]) {
            WorkspaceCommands::StackLog { branch, .. } => {
                assert_eq!(branch, Some("feature-a".to_string()));
            }
            other => panic!("Expected StackLog, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn stack_log_all_flags() {
        match parse(&["stack-log", "-l", "5", "-f", "linear", "--no-messages", "--ahead-behind", "-b", "feat"]) {
            WorkspaceCommands::StackLog {
                limit,
                format,
                no_messages,
                ahead_behind,
                branch,
            } => {
                assert_eq!(limit, Some(5));
                assert_eq!(format, "linear");
                assert!(no_messages);
                assert!(ahead_behind);
                assert_eq!(branch, Some("feat".to_string()));
            }
            other => panic!("Expected StackLog, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Diff (optional path) --
    #[test]
    fn diff_defaults() {
        match parse(&["diff"]) {
            WorkspaceCommands::Diff { path } => assert_eq!(path, None),
            other => panic!("Expected Diff, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn diff_with_path() {
        match parse(&["diff", "src/main.rs"]) {
            WorkspaceCommands::Diff { path } => {
                assert_eq!(path, Some("src/main.rs".to_string()));
            }
            other => panic!("Expected Diff, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Commit (required message) --
    #[test]
    fn commit_parses() {
        match parse(&["commit", "my changes"]) {
            WorkspaceCommands::Commit { message } => assert_eq!(message, "my changes"),
            other => panic!("Expected Commit, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn commit_requires_message() {
        let result = WorkspaceParser::try_parse_from(["scp", "commit"]);
        assert!(result.is_err());
    }

    // -- Branch (required name) --
    #[test]
    fn branch_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "branch"]);
        assert!(result.is_err());
    }

    // -- BranchDelete (required name) --
    #[test]
    fn branch_delete_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "branch-delete"]);
        assert!(result.is_err());
    }

    // -- Fork (required name, optional from) --
    #[test]
    fn fork_defaults() {
        match parse(&["fork", "new-ws"]) {
            WorkspaceCommands::Fork { name, from } => {
                assert_eq!(name, "new-ws");
                assert_eq!(from, None);
            }
            other => panic!("Expected Fork, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn fork_with_from() {
        match parse(&["fork", "new-ws", "existing-ws"]) {
            WorkspaceCommands::Fork { name, from } => {
                assert_eq!(name, "new-ws");
                assert_eq!(from, Some("existing-ws".to_string()));
            }
            other => panic!("Expected Fork, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Merge (required name) --
    #[test]
    fn merge_parses() {
        match parse(&["merge", "ws-name"]) {
            WorkspaceCommands::Merge { name } => assert_eq!(name, "ws-name"),
            other => panic!("Expected Merge, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn merge_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "merge"]);
        assert!(result.is_err());
    }

    // -- Add (required path) --
    #[test]
    fn add_parses() {
        match parse(&["add", "/path/to/dir"]) {
            WorkspaceCommands::Add { path } => assert_eq!(path, "/path/to/dir"),
            other => panic!("Expected Add, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn add_requires_path() {
        let result = WorkspaceParser::try_parse_from(["scp", "add"]);
        assert!(result.is_err());
    }

    // -- Revert (required name, optional dry_run flag) --
    #[test]
    fn revert_defaults() {
        match parse(&["revert", "feature-x"]) {
            WorkspaceCommands::Revert { name, dry_run } => {
                assert_eq!(name, "feature-x");
                assert!(!dry_run);
            }
            other => panic!("Expected Revert, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn revert_with_dry_run() {
        match parse(&["revert", "feature-x", "--dry-run"]) {
            WorkspaceCommands::Revert { name, dry_run } => {
                assert_eq!(name, "feature-x");
                assert!(dry_run);
            }
            other => panic!("Expected Revert, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn revert_requires_name() {
        let result = WorkspaceParser::try_parse_from(["scp", "revert"]);
        assert!(result.is_err());
    }

    // -- BranchRename (required old_name, new_name) --
    #[test]
    fn branch_rename_defaults() {
        match parse(&["branch-rename", "old-name", "new-name"]) {
            WorkspaceCommands::BranchRename {
                old_name,
                new_name,
                dry_run,
            } => {
                assert_eq!(old_name, "old-name");
                assert_eq!(new_name, "new-name");
                assert!(!dry_run);
            }
            other => panic!(
                "Expected BranchRename, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn branch_rename_with_dry_run() {
        match parse(&["branch-rename", "old-name", "new-name", "--dry-run"]) {
            WorkspaceCommands::BranchRename {
                old_name,
                new_name,
                dry_run,
            } => {
                assert_eq!(old_name, "old-name");
                assert_eq!(new_name, "new-name");
                assert!(dry_run);
            }
            other => panic!(
                "Expected BranchRename with dry_run, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn branch_rename_requires_names() {
        let result = WorkspaceParser::try_parse_from(["scp", "branch-rename"]);
        assert!(result.is_err());
    }
}

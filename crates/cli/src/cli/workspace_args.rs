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

    /// Add an existing path as a workspace
    Add {
        /// Path to add
        path: String,
    },
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
}

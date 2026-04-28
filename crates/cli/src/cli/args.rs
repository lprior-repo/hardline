//! CLI argument definitions
//!
//! This module contains all Clap-derived argument structs and subcommand enums
//! that define the CLI surface area. Each enum represents a distinct command family.

use clap::{Parser, Subcommand};

use crate::cli::{
    agent_args::AgentCommands, batch_args::BatchCommands, config_args::ConfigCommands,
    lock_args::LockCommands, queue_args::QueueCommands, session_args::SessionCommands,
    stash_args::StashCommands, tag_args::TagCommands, task_args::TaskCommands,
    workspace_args::WorkspaceCommands,
};

/// Main CLI entry point
#[derive(Parser)]
#[command(name = "scp")]
#[command(about = "Source Control Plane - Unified workspace and queue management", long_about = None)]
#[command(version = "0.5.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress normal output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output format
    #[arg(short, long, global = true, default_value = "human")]
    pub format: String,

    /// Database path
    #[arg(long, global = true)]
    pub database: Option<String>,
}

/// Top-level command enum
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize SCP in current directory
    Init {
        /// VCS type to use (git only)
        #[arg(long, default_value = "git")]
        vcs: String,
    },

    /// Workspace management (from Isolate)
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
    },

    /// Queue management (from Stak)
    /// Lock management
    Lock {
        #[command(subcommand)]
        command: LockCommands,
    },

    Queue {
        #[command(subcommand)]
        command: QueueCommands,
    },

    /// Agent management
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Session management
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Task management (beads)
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Git stash operations
    Stash {
        #[command(subcommand)]
        command: StashCommands,
    },

    /// Git tag operations
    Tag {
        #[command(subcommand)]
        command: TagCommands,
    },

    /// Batch command execution (atomic)
    Batch {
        #[command(subcommand)]
        command: BatchCommands,
    },

    /// Fetch from remotes
    Fetch {
        /// Remote to fetch from (default: all)
        remote: Option<String>,

        /// Prune remote-tracking branches
        #[arg(short, long)]
        prune: bool,

        /// Fetch all tags
        #[arg(short, long)]
        tags: bool,

        /// Fetch from all remotes
        #[arg(short, long)]
        all: bool,
    },

    /// Pull from remote
    Pull,

    /// Push to remote
    Push {
        /// Remote to push to
        #[arg(short, long, default_value = "origin")]
        remote: String,

        /// Branch to push
        #[arg(short, long)]
        branch: Option<String>,

        /// Set upstream tracking branch
        #[arg(short, long)]
        set_upstream: bool,

        /// Force push
        #[arg(long)]
        force: bool,

        /// Force push with lease
        #[arg(long)]
        force_with_lease: bool,

        /// Push tags
        #[arg(short, long)]
        tags: bool,

        /// Delete remote branch
        #[arg(short, long)]
        delete: bool,
    },

    /// Health check
    Doctor {
        /// Run full diagnostics
        #[arg(long)]
        full: bool,
    },

    /// Show status (short or detailed)
    Status {
        /// Short output (single line)
        #[arg(short, long)]
        short: bool,
    },

    /// Switch to a workspace
    Switch {
        /// Workspace name
        name: String,
    },

    /// Show current context (workspace, branch, VCS status)
    Context,

    /// Alias for context - shows current location
    Whereami,

    /// Preview what a command would do without executing it
    Whatif {
        /// Command to preview
        command: String,

        /// Arguments for the command
        args: Vec<String>,
    },

    /// Show usage examples for commands
    Examples {
        /// Filter by specific command
        command: Option<String>,

        /// Filter by use case
        #[arg(long)]
        use_case: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn parse_no_args_fails_requires_subcommand() {
        let result = Cli::try_parse_from(["scp"]);
        assert!(
            result.is_err(),
            "CLI should require a subcommand, but parsing succeeded"
        );
    }

    #[test]
    fn parse_verbose_flag() {
        let cli = Cli::parse_from(["scp", "-v", "status"]);
        assert!(cli.verbose);
        assert!(!cli.quiet);
    }

    #[test]
    fn parse_quiet_flag() {
        let cli = Cli::parse_from(["scp", "--quiet", "status"]);
        assert!(cli.quiet);
        assert!(!cli.verbose);
    }

    #[test]
    fn parse_format_flag_default() {
        let cli = Cli::parse_from(["scp", "status"]);
        assert_eq!(cli.format, "human");
    }

    #[test]
    fn parse_format_flag_custom() {
        let cli = Cli::parse_from(["scp", "-f", "json", "status"]);
        assert_eq!(cli.format, "json");
    }

    #[test]
    fn parse_format_flag_long() {
        let cli = Cli::parse_from(["scp", "--format", "yaml", "status"]);
        assert_eq!(cli.format, "yaml");
    }

    #[test]
    fn parse_database_flag() {
        let cli = Cli::parse_from(["scp", "--database", "/tmp/test.db", "status"]);
        assert_eq!(cli.database, Some("/tmp/test.db".to_string()));
    }

    #[test]
    fn parse_database_flag_absent() {
        let cli = Cli::parse_from(["scp", "status"]);
        assert_eq!(cli.database, None);
    }

    // -- Init command --
    #[test]
    fn parse_init_default_vcs() {
        let cli = Cli::parse_from(["scp", "init"]);
        match cli.command {
            Commands::Init { vcs } => assert_eq!(vcs, "git"),
            other => panic!("Expected Init, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_init_with_vcs_git() {
        let cli = Cli::parse_from(["scp", "init", "--vcs", "git"]);
        match cli.command {
            Commands::Init { vcs } => assert_eq!(vcs, "git"),
            other => panic!("Expected Init, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Status command --
    #[test]
    fn parse_status_short_flag() {
        let cli = Cli::parse_from(["scp", "status", "-s"]);
        match cli.command {
            Commands::Status { short } => assert!(short),
            other => panic!("Expected Status, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_status_default() {
        let cli = Cli::parse_from(["scp", "status"]);
        match cli.command {
            Commands::Status { short } => assert!(!short),
            other => panic!("Expected Status, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Doctor command --
    #[test]
    fn parse_doctor_defaults() {
        let cli = Cli::parse_from(["scp", "doctor"]);
        match cli.command {
            Commands::Doctor { full } => assert!(!full),
            other => panic!("Expected Doctor, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_doctor_full() {
        let cli = Cli::parse_from(["scp", "doctor", "--full"]);
        match cli.command {
            Commands::Doctor { full } => assert!(full),
            other => panic!("Expected Doctor, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Fetch command --
    #[test]
    fn parse_fetch_defaults() {
        let cli = Cli::parse_from(["scp", "fetch"]);
        match cli.command {
            Commands::Fetch {
                remote,
                prune,
                tags,
                all,
            } => {
                assert_eq!(remote, None);
                assert!(!prune);
                assert!(!tags);
                assert!(!all);
            }
            other => panic!("Expected Fetch, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_fetch_with_remote() {
        let cli = Cli::parse_from(["scp", "fetch", "upstream"]);
        match cli.command {
            Commands::Fetch { remote, .. } => {
                assert_eq!(remote, Some("upstream".to_string()));
            }
            other => panic!("Expected Fetch, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_fetch_with_all_flags() {
        let cli = Cli::parse_from(["scp", "fetch", "-p", "-t", "-a"]);
        match cli.command {
            Commands::Fetch {
                prune, tags, all, ..
            } => {
                assert!(prune);
                assert!(tags);
                assert!(all);
            }
            other => panic!("Expected Fetch, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Push command --
    #[test]
    fn parse_push_defaults() {
        let cli = Cli::parse_from(["scp", "push"]);
        match cli.command {
            Commands::Push {
                remote,
                branch,
                set_upstream,
                force,
                force_with_lease,
                tags,
                delete,
            } => {
                assert_eq!(remote, "origin");
                assert_eq!(branch, None);
                assert!(!set_upstream);
                assert!(!force);
                assert!(!force_with_lease);
                assert!(!tags);
                assert!(!delete);
            }
            other => panic!("Expected Push, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_push_with_force() {
        let cli = Cli::parse_from(["scp", "push", "--force"]);
        match cli.command {
            Commands::Push { force, .. } => assert!(force),
            other => panic!("Expected Push, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn parse_push_with_all_flags() {
        let cli = Cli::parse_from([
            "scp",
            "push",
            "-r",
            "upstream",
            "-b",
            "feature",
            "-s",
            "--force",
            "--force-with-lease",
            "-t",
            "-d",
        ]);
        match cli.command {
            Commands::Push {
                remote,
                branch,
                set_upstream,
                force,
                force_with_lease,
                tags,
                delete,
            } => {
                assert_eq!(remote, "upstream");
                assert_eq!(branch, Some("feature".to_string()));
                assert!(set_upstream);
                assert!(force);
                assert!(force_with_lease);
                assert!(tags);
                assert!(delete);
            }
            other => panic!("Expected Push, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Pull command --
    #[test]
    fn parse_pull() {
        let cli = Cli::parse_from(["scp", "pull"]);
        match cli.command {
            Commands::Pull => {}
            other => panic!("Expected Pull, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Context / Whereami --
    #[test]
    fn parse_context() {
        let cli = Cli::parse_from(["scp", "context"]);
        assert!(matches!(cli.command, Commands::Context));
    }

    #[test]
    fn parse_whereami() {
        let cli = Cli::parse_from(["scp", "whereami"]);
        assert!(matches!(cli.command, Commands::Whereami));
    }

    // -- Switch (top-level alias) --
    #[test]
    fn parse_switch() {
        let cli = Cli::parse_from(["scp", "switch", "my-workspace"]);
        match cli.command {
            Commands::Switch { name } => assert_eq!(name, "my-workspace"),
            other => panic!("Expected Switch, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Global flags combine with subcommands --
    #[test]
    fn global_flags_combine_with_subcommand() {
        let cli = Cli::parse_from(["scp", "-v", "--format", "json", "--quiet", "status"]);
        assert!(cli.verbose);
        assert!(cli.quiet);
        assert_eq!(cli.format, "json");
    }

    // -- Discriminant checks: every top-level command parses correctly --
    #[test]
    fn parse_workspace_subcommand() {
        let cli = Cli::parse_from(["scp", "workspace", "list"]);
        assert!(matches!(cli.command, Commands::Workspace { .. }));
    }

    #[test]
    fn parse_lock_subcommand() {
        let cli = Cli::parse_from(["scp", "lock", "list"]);
        assert!(matches!(cli.command, Commands::Lock { .. }));
    }

    #[test]
    fn parse_queue_subcommand() {
        let cli = Cli::parse_from(["scp", "queue", "list"]);
        assert!(matches!(cli.command, Commands::Queue { .. }));
    }

    #[test]
    fn parse_agent_subcommand() {
        let cli = Cli::parse_from(["scp", "agent", "list"]);
        assert!(matches!(cli.command, Commands::Agent { .. }));
    }

    #[test]
    fn parse_session_subcommand() {
        let cli = Cli::parse_from(["scp", "session", "list"]);
        assert!(matches!(cli.command, Commands::Session { .. }));
    }

    #[test]
    fn parse_task_subcommand() {
        let cli = Cli::parse_from(["scp", "task", "list"]);
        assert!(matches!(cli.command, Commands::Task { .. }));
    }

    #[test]
    fn parse_config_subcommand() {
        let cli = Cli::parse_from(["scp", "config", "list"]);
        assert!(matches!(cli.command, Commands::Config { .. }));
    }

    #[test]
    fn parse_stash_subcommand() {
        let cli = Cli::parse_from(["scp", "stash", "list"]);
        assert!(matches!(cli.command, Commands::Stash { .. }));
    }

    #[test]
    fn parse_tag_subcommand() {
        let cli = Cli::parse_from(["scp", "tag", "list"]);
        assert!(matches!(cli.command, Commands::Tag { .. }));
    }

    #[test]
    fn parse_batch_subcommand() {
        let cli = Cli::parse_from(["scp", "batch", "run", "echo", "hello"]);
        assert!(matches!(cli.command, Commands::Batch { .. }));
    }

    // -- Required positional args --
    #[test]
    fn parse_switch_requires_name() {
        let result = Cli::try_parse_from(["scp", "switch"]);
        assert!(result.is_err());
    }
}

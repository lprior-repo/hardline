//! CLI argument definitions
//!
//! This module contains all Clap-derived argument structs and subcommand enums
//! that define the CLI surface area. Each enum represents a distinct command family.

use clap::{Parser, Subcommand};

use crate::cli::agent_args::AgentCommands;
use crate::cli::batch_args::BatchCommands;
use crate::cli::config_args::ConfigCommands;
use crate::cli::lock_args::LockCommands;
use crate::cli::queue_args::QueueCommands;
use crate::cli::session_args::SessionCommands;
use crate::cli::stash_args::StashCommands;
use crate::cli::tag_args::TagCommands;
use crate::cli::task_args::TaskCommands;
use crate::cli::workspace_args::WorkspaceCommands;

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
}

/// Top-level command enum
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize SCP in current directory
    Init {
        /// VCS type to use (jj/git)
        #[arg(long, default_value = "jj")]
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
        #[arg(short, long)]
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
        #[arg(short, long)]
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
}

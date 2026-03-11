//! Source Control Plane - Unified CLI
//!
//! One CLI for workspace isolation (Isolate) and queue management (Stak).

use clap::{Parser, Subcommand};
use scp_core::Result;
use std::process::ExitCode;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;

#[derive(Parser)]
#[command(name = "scp")]
#[command(about = "Source Control Plane - Unified workspace and queue management", long_about = None)]
#[command(version = "0.5.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format
    #[arg(short, long, global = true, default_value = "human")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize SCP in current directory
    Init {
        /// VCS type to use (jj/git)
        #[arg(short, long, default_value = "jj")]
        vcs: String,
    },

    /// Workspace management (from Isolate)
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
    },

    /// Queue management (from Stak)
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

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Health check
    Doctor {
        /// Run full diagnostics
        #[arg(short, long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommands {
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
}

#[derive(Subcommand)]
enum QueueCommands {
    /// List queue items
    List,

    /// Add item to queue
    Enqueue {
        /// Branch name
        branch: String,

        /// Priority (low/normal/high/critical)
        #[arg(short, long)]
        priority: Option<String>,
    },

    /// Remove front item from queue
    Dequeue,

    /// Process next item in queue
    Process {
        /// Run pre-flight checks
        #[arg(short, long)]
        checks: bool,
    },

    /// Insert item at position
    Insert {
        /// Position
        position: usize,

        /// Branch name
        branch: String,
    },

    /// Remove item from queue
    Remove {
        /// Branch name or ID
        branch: String,
    },

    /// Show queue status
    Status,
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Create an agent
    Create {
        /// Agent name
        name: String,
    },

    /// List agents
    List,

    /// Kill an agent
    Kill {
        /// Agent ID
        id: String,
    },

    /// Show agent status
    Status {
        /// Agent ID
        id: Option<String>,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// List sessions
    List,

    /// Show session status
    Status,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get config value
    Get {
        /// Key
        key: String,
    },

    /// Set config value
    Set {
        /// Key
        key: String,

        /// Value
        value: String,
    },

    /// List all config
    List,
}

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Run the appropriate command
    let result = run_command(cli);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            if let Some(suggestion) = e.suggestion() {
                eprintln!("{}", suggestion);
            }
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { vcs } => commands::init::run(&vcs),

        Commands::Workspace { command } => match command {
            WorkspaceCommands::Spawn { name, sync } => commands::workspace::spawn(&name, sync),
            WorkspaceCommands::Switch { name } => commands::workspace::switch(&name),
            WorkspaceCommands::List {} => commands::workspace::list(),
            WorkspaceCommands::Status {} => commands::workspace::status(),
            WorkspaceCommands::Sync { name, all } => {
                commands::workspace::sync(name.as_deref(), all)
            }
            WorkspaceCommands::Done { name } => commands::workspace::done(name.as_deref()),
            WorkspaceCommands::Abort { name } => commands::workspace::abort(name.as_deref()),
            WorkspaceCommands::Log { limit } => commands::workspace::log(limit),
            WorkspaceCommands::Diff { path } => commands::workspace::diff(path.as_deref()),
            WorkspaceCommands::Uncommitted {} => commands::workspace::uncommitted(),
            WorkspaceCommands::Commit { message } => commands::workspace::commit(&message),
            WorkspaceCommands::Branches {} => commands::workspace::branches(),
            WorkspaceCommands::Branch { name } => commands::workspace::branch_create(&name),
            WorkspaceCommands::BranchDelete { name } => commands::workspace::branch_delete(&name),
            WorkspaceCommands::BranchCurrent {} => commands::workspace::branch_current(),
        },

        Commands::Queue { command } => match command {
            QueueCommands::List {} => commands::queue::list(),
            QueueCommands::Enqueue { branch, priority } => {
                commands::queue::enqueue(&branch, priority.as_deref())
            }
            QueueCommands::Dequeue {} => commands::queue::dequeue(),
            QueueCommands::Process { checks } => commands::queue::process(checks),
            QueueCommands::Insert { position, branch } => {
                commands::queue::insert(position, &branch)
            }
            QueueCommands::Remove { branch } => commands::queue::remove(&branch),
            QueueCommands::Status {} => commands::queue::status(),
        },

        Commands::Agent { command } => match command {
            AgentCommands::Create { name } => commands::agent::create(&name),
            AgentCommands::List {} => commands::agent::list(),
            AgentCommands::Kill { id } => commands::agent::kill(&id),
            AgentCommands::Status { id } => commands::agent::status(id.as_deref()),
        },

        Commands::Session { command } => match command {
            SessionCommands::List {} => commands::session::list(),
            SessionCommands::Status {} => commands::session::status(),
        },

        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => commands::config::get(&key),
            ConfigCommands::Set { key, value } => commands::config::set(&key, &value),
            ConfigCommands::List {} => commands::config::list(),
        },

        Commands::Doctor { full } => commands::doctor::run(full),
    }
}

//! Source Control Plane - Unified CLI
//!
//! One CLI for workspace isolation (Isolate) and queue management (Stak).

use clap::{Parser, Subcommand};
use scp_core::{output::Output, Result};
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

    /// Suppress normal output
    #[arg(short, long, global = true)]
    quiet: bool,

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

#[derive(Subcommand)]
enum StashCommands {
    /// Save changes to stash
    Save {
        /// Stash message
        #[arg(short, long)]
        message: Option<String>,

        /// Include untracked files
        #[arg(short, long)]
        include_untracked: bool,

        /// Interactively select hunks to stash
        #[arg(short, long)]
        patch: bool,
    },

    /// Apply and remove stash
    Pop {
        /// Stash to pop
        stash: Option<String>,

        /// Also restore staged changes
        #[arg(short, long)]
        index: bool,
    },

    /// List stashed changes
    List,

    /// Drop a stash
    Drop {
        /// Stash reference
        stash: String,

        /// Force drop without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show stash contents
    Show {
        /// Stash reference
        stash: Option<String>,

        /// Show diffstat only
        #[arg(short, long)]
        stat: bool,
    },
}

#[derive(Subcommand)]
enum TagCommands {
    /// Create a tag
    Create {
        /// Tag name
        name: String,

        /// Annotated tag message
        #[arg(short, long)]
        message: Option<String>,

        /// Tag specific commit
        #[arg(short, long)]
        commit: Option<String>,

        /// Replace existing tag
        #[arg(short, long)]
        force: bool,
    },

    /// List tags
    List {
        /// Pattern to match
        #[arg(short, long)]
        pattern: Option<String>,

        /// Sort by key
        #[arg(long)]
        sort: Option<String>,
    },

    /// Delete a tag
    Delete {
        /// Tag to delete
        tag: String,

        /// Delete remote tag
        #[arg(short, long)]
        remote: bool,
    },

    /// Push tags to remote
    Push {
        /// Specific tag to push
        tag: Option<String>,

        /// Remote to push to
        #[arg(short, long, default_value = "origin")]
        remote: String,

        /// Force push
        #[arg(short, long)]
        force: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up verbosity for output module
    Output::set_verbose(cli.verbose, cli.quiet);

    // Initialize logging with appropriate level
    let log_level = if cli.quiet {
        "error".to_string()
    } else if cli.verbose {
        "debug".to_string()
    } else {
        "info".to_string()
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| log_level),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

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
            WorkspaceCommands::Add { path } => commands::workspace::add(&path),
            WorkspaceCommands::Fork { name, from } => {
                commands::workspace::fork(&name, from.as_deref())
            }
            WorkspaceCommands::Merge { name } => commands::workspace::merge(&name),
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

        Commands::Stash { command } => match command {
            StashCommands::Save {
                message,
                include_untracked,
                patch,
            } => commands::stash::save(message.as_deref(), include_untracked, patch),
            StashCommands::Pop { stash, index } => commands::stash::pop(stash.as_deref(), index),
            StashCommands::List {} => commands::stash::list(),
            StashCommands::Drop { stash, force } => commands::stash::drop(&stash, force),
            StashCommands::Show { stash, stat } => commands::stash::show(stash.as_deref(), stat),
        },

        Commands::Tag { command } => match command {
            TagCommands::Create {
                name,
                message,
                commit,
                force,
            } => commands::tag::create(&name, message.as_deref(), commit.as_deref(), force),
            TagCommands::List { pattern, sort } => {
                commands::tag::list(pattern.as_deref(), sort.as_deref())
            }
            TagCommands::Delete { tag, remote } => commands::tag::delete(&tag, remote),
            TagCommands::Push { tag, remote, force } => {
                commands::tag::push(tag.as_deref(), &remote, force)
            }
        },

        Commands::Fetch {
            remote,
            prune,
            tags,
            all,
        } => commands::sync::fetch(remote.as_deref(), prune, tags, all),

        Commands::Pull {} => commands::sync::pull(),

        Commands::Push {
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        } => commands::sync::push(
            &remote,
            branch.as_deref(),
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        ),

        Commands::Doctor { full } => commands::doctor::run(full),

        Commands::Status { short } => commands::status::run(short),
    }
}

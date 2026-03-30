//! Lock management arguments

use clap::Subcommand;

/// Lock management commands
#[derive(Subcommand)]
pub enum LockCommands {
    /// Acquire a lock on a session
    Acquire {
        /// Session name to lock
        session: String,
        /// Agent ID acquiring the lock
        #[arg(short, long)]
        agent: String,
        /// Time-To-Live in seconds (default: 300)
        #[arg(short, long)]
        ttl: Option<u64>,
    },
    /// Release a lock on a session
    Release {
        /// Session name to unlock
        session: String,
        /// Agent ID releasing the lock
        #[arg(short, long)]
        agent: String,
    },
    /// Send a heartbeat to maintain a lock
    Heartbeat {
        /// Session name
        session: String,
        /// Agent ID sending the heartbeat
        #[arg(short, long)]
        agent: String,
    },
    /// Show the status of a lock
    Status {
        /// Session name to check
        session: String,
    },
    /// List all active locks
    List,
}

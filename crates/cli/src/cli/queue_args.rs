//! Queue command definitions
//!
//! Subcommand enum for queue management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum QueueCommands {
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

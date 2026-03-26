//! Task command definitions
//!
//! Subcommand enum for task (bead) management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum TaskCommands {
    /// List all tasks
    List,

    /// Show task details
    Show {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Claim a task (assign to self)
    Claim {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Yield a task (release assignment)
    Yield {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Start working on a task
    Start {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },

    /// Complete a task
    Done {
        /// Task ID
        task_id: String,

        /// User performing the action
        #[arg(long, default_value = "current-user")]
        user: String,
    },
}

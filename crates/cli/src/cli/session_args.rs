//! Session command definitions
//!
//! Subcommand enum for session management operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionCommands {
    /// List sessions
    List,

    /// Show session status
    Status,

    /// Focus (switch to) a session
    Focus {
        /// Session name
        name: String,
    },

    /// Submit session changes for review
    Submit {
        /// Session name (default: current)
        name: Option<String>,

        /// Automatically commit dirty changes
        #[arg(short, long)]
        auto_commit: bool,

        /// Custom commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Remove a session
    Remove {
        /// Session name
        name: String,

        /// Force removal (skip confirmation)
        #[arg(short, long)]
        force: bool,

        /// Merge changes to main before removing
        #[arg(short, long)]
        merge: bool,
    },
}

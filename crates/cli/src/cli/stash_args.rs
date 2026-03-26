//! Stash command definitions
//!
//! Subcommand enum for Git stash operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum StashCommands {
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

//! Tag command definitions
//!
//! Subcommand enum for Git tag operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum TagCommands {
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

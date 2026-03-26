//! Batch command definitions
//!
//! Subcommand enum for batch command execution.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum BatchCommands {
    /// Execute a batch of commands atomically
    Run {
        /// Workspace name (default: current workspace)
        #[arg(short, long)]
        workspace: Option<String>,

        /// Commands to execute
        #[arg(trailing_var_arg = true)]
        commands: Vec<String>,
    },
}

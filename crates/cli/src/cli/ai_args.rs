//! CLI argument definitions for the `ai` subcommand.

use clap::Subcommand;

/// AI subcommands.
#[derive(Subcommand, Debug)]
pub enum AiCommands {
    /// Show AI status
    Status,
    /// Run AI workflow
    Workflow,
    /// Quick start AI
    #[command(name = "quick-start")]
    QuickStart,
    /// Get next AI action
    Next,
    /// Default AI action
    Default,
}

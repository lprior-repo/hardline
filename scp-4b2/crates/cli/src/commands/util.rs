//! Util command - utility commands

// This module is reserved. Util commands exist but are not yet wired up.
// Kept as placeholder for future SCP-level utility commands.

use scp_core::Result;

#[derive(clap::Subcommand)]
pub enum UtilCommands {
    /// Show current timestamp in Unix epoch format
    Timestamp,
    /// Show current datetime in ISO 8601 format
    Now,
    /// Show environment information
    Env,
    /// Generate a unique ID
    Id,
}

pub fn run(_command: UtilCommands) -> Result<()> {
    Ok(())
}

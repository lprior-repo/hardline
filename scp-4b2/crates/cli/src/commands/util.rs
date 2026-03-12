//! Util command - utility commands

use scp_core::Result;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn run(command: UtilCommands) -> Result<()> {
    match command {
        UtilCommands::Timestamp => timestamp(),
        UtilCommands::Now => now(),
        UtilCommands::Env => env_info(),
        UtilCommands::Id => generate_id(),
    }
}

fn timestamp() -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| scp_core::Error::Internal(e.to_string()))?;
    println!("{}", now.as_secs());
    Ok(())
}

fn now() -> Result<()> {
    let now = chrono::Local::now();
    println!("{}", now.format("%Y-%m-%dT%H:%M:%S%:z"));
    Ok(())
}

fn env_info() -> Result<()> {
    println!("SCP Version: {}", env!("CARGO_PKG_VERSION"));

    if let Ok(cwd) = std::env::current_dir() {
        println!("CWD: {}", cwd.display());
    }

    if let Ok(vcs) = std::env::var("VCS") {
        println!("VCS: {}", vcs);
    }

    if let Ok(editor) = std::env::var("EDITOR") {
        println!("EDITOR: {}", editor);
    }

    Ok(())
}

fn generate_id() -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| scp_core::Error::Internal(e.to_string()))?;

    let id = format!("{:x}{:06x}", now.as_secs(), now.subsec_nanos());
    println!("{}", id);
    Ok(())
}

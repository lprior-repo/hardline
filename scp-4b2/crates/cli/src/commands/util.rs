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

/// Extracts relevant environment variables as a vector of (name, value) pairs.
/// This is a pure calculation with no I/O side effects.
fn extract_env_vars() -> Vec<(&'static str, Option<String>)> {
    ["VCS", "EDITOR"]
        .into_iter()
        .map(|key| (key, std::env::var(key).ok()))
        .collect()
}

/// Formats env info for display - pure data transformation.
fn format_env_display(version: &str, cwd: Option<&std::path::Path>, env_vars: &[(&str, Option<String>)]) -> Vec<String> {
    std::iter::once(format!("SCP Version: {}", version))
        .chain(cwd.map(|path| format!("CWD: {}", path.display())))
        .chain(env_vars.iter().filter_map(|(key, val)| {
            val.as_ref().map(|v| format!("{}: {}", key, v))
        }))
        .collect()
}

    lines.extend(
        env_vars
            .iter()
            .filter_map(|(key, val)| val.as_ref().map(|v| format!("{}: {}", key, v))),
    );

    lines
}

fn env_info() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let cwd = std::env::current_dir().ok();
    let env_vars = extract_env_vars();

    format_env_display(version, cwd.as_deref(), &env_vars)
        .into_iter()
        .for_each(|line| println!("{}", line));

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

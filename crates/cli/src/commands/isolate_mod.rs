//! CLI utilities and helpers

pub use crate::commands::isolate_commands as commands;
pub use crate::commands::isolate_alias_handler as alias_handler;
pub use crate::commands::isolate_json_docs as json_docs;
pub use crate::commands::isolate_object_commands as object_commands;

use anyhow::{Context, Result};
use rand::Rng;
use tokio::process::Command;

/// Get a secure directory for temporary files
/// Prefers XDG_RUNTIME_DIR (Linux) which has proper permissions (0700)
/// Falls back to std::env::temp_dir()
fn secure_temp_dir() -> std::path::PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = std::path::PathBuf::from(runtime_dir);
            if path.exists() {
                return path;
            }
        }
    }
    // SECURITY: temp_dir fallback is acceptable - only used when XDG_RUNTIME_DIR unavailable
    std::env::temp_dir()
}

/// Create a secure temporary file path with random name
fn secure_temp_file(prefix: &str, suffix: &str) -> std::path::PathBuf {
    let dir = secure_temp_dir();
    let random_id: u64 = rand::thread_rng().gen();
    dir.join(format!("{prefix}-{random_id:016x}{suffix}"))
}

/// Execute a shell command and return its output
pub async fn run_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to execute {program}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout).context("Invalid UTF-8 output from command")
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program} failed: {stderr}")
    }
}

/// Check if we're running in a terminal (TTY)
/// Uses `std::io::IsTerminal` (Rust 1.70+)
pub fn is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal()
}

/// Check if current directory is a JJ repository
pub async fn is_jj_repo() -> Result<bool> {
    let result = scp_core::jj::get_jj_command()
        .args(["root"])
        .output()
        .await
        .context("Failed to run jj")?;

    Ok(result.status.success())
}

/// Get JJ repository root
pub async fn jj_root() -> Result<String> {
    let output = scp_core::jj::get_jj_command()
        .arg("root")
        .output()
        .await
        .context("Failed to execute jj root")?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .context("Invalid UTF-8 output from jj root")
            .map(|s| s.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj root failed: {stderr}")
    }
}

/// Check if a command is available in PATH
pub async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if JJ is installed
pub async fn is_jj_installed() -> bool {
    scp_core::jj::is_jj_installed().await
}

/// Check if current directory is a Git repository
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let result = run_command("echo", &["hello"]).await;
        assert!(result.is_ok());
        let Ok(output) = result else {
            panic!("command failed");
        };
        assert_eq!(output.trim(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let result = run_command("false", &[]).await;
        assert!(result.is_err());
    }
}

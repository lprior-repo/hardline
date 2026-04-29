//! Shared CLI fallback helpers for gix operations.
//!
//! When gitoxide does not support an operation natively (e.g. push, stash,
//! worktree management), we shell out to the `git` CLI and parse its output.
//! This module centralises that pattern so every call-site stays consistent.

use std::path::Path;

use crate::error::{GitError, GitResult};

/// Result of running a git CLI command.
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Run a git sub-command inside the given working directory and return its
/// captured output.
///
/// # Errors
/// - `GitError::Network` if the `git` binary cannot be spawned.
pub fn run_git(workdir: &Path, args: &[&str]) -> GitResult<CliOutput> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Network(format!("Failed to execute git: {e}")))?;

    Ok(CliOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    })
}

/// Extract the working directory from a gix repository, returning an error
/// for bare repos.
pub fn require_workdir<'a>(
    repo: &'a gix::Repository,
    context: &str,
) -> GitResult<&'a std::path::Path> {
    repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "workdir".to_string(),
        reason: format!("{context}: cannot operate on a bare repository"),
    })
}

/// Interpret a failed CLI output as a [`GitError::Network`], folding auth
/// signals into [`GitError::Unauthorized`].
pub fn cli_error(output: &CliOutput, context: &str) -> GitError {
    let msg = output.stderr.trim().to_string();
    if msg.is_empty() {
        return GitError::Network(format!("{context}: unknown failure"));
    }
    if msg.contains("authentication")
        || msg.contains("credential")
        || msg.contains("403")
        || msg.contains("Permission denied")
    {
        GitError::Unauthorized(format!("{context}: {msg}"))
    } else {
        GitError::Network(format!("{context}: {msg}"))
    }
}

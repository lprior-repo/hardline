//! Gitoxide Stash Operations
//!
//! Hybrid implementation: CLI for mutation operations (push/pop/drop),
//! gix for ref reading where practical. Pure calc functions for parsing.
//!
//! Git stash has no native gix API, so we follow the same CLI-fallback
//! pattern used by worktree.rs for operations gix cannot perform natively.

use crate::error::{GitError, GitResult};

// ============================================================================
// Data
// ============================================================================

/// A single stash entry from `refs/stash` reflog.
#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

// ============================================================================
// Actions — mutation operations (CLI-backed)
// ============================================================================

/// Save working directory changes to the stash (stash_push).
///
/// Creates a stash entry with optional message and untracked file inclusion.
pub fn save(
    repo: &gix::Repository,
    message: Option<&str>,
    include_untracked: bool,
) -> GitResult<()> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "push"]);

    if let Some(msg) = message {
        cmd.args(["-m", msg]);
    }

    if include_untracked {
        cmd.arg("--include-untracked");
    }

    let output = run_in(workdir, &mut cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // "No local changes to save" is not an error
        if stderr.contains("No local changes to save") {
            return Ok(());
        }
        return Err(GitError::InvalidRef {
            name: "stash".to_string(),
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

/// Apply and remove a stash entry (stash_pop).
///
/// Applies the stash at `index` to the working directory and removes it
/// from the stash list.
pub fn pop(repo: &gix::Repository, index: usize) -> GitResult<()> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let stash_ref = format!("stash@{{{index}}}");
    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "pop", &stash_ref]);
    let output = run_in(workdir, &mut cmd)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: stash_ref,
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

/// Drop a stash entry without applying it.
pub fn drop(repo: &gix::Repository, index: usize) -> GitResult<()> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let stash_ref = format!("stash@{{{index}}}");
    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "drop", &stash_ref]);
    let output = run_in(workdir, &mut cmd)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: stash_ref,
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// Actions — read operations
// ============================================================================

/// List all stash entries.
///
/// Parses `git stash list` output into structured `StashEntry` items.
pub fn list(repo: &gix::Repository) -> GitResult<Vec<StashEntry>> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "list"]);
    let output = run_in(workdir, &mut cmd)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: "stash".to_string(),
            reason: stderr.to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_stash_list(&stdout)
}

/// Show the diff for a stash entry.
///
/// Returns the patch output for the stash at `index`.
pub fn show(repo: &gix::Repository, index: usize) -> GitResult<String> {
    let workdir = repo.workdir().ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: "repository has no working directory".to_string(),
    })?;

    let stash_ref = format!("stash@{{{index}}}");
    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "show", "-p", &stash_ref]);
    let output = run_in(workdir, &mut cmd)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidRef {
            name: stash_ref,
            reason: stderr.to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ============================================================================
// Calc — pure functions
// ============================================================================

/// Parse `git stash list` output into structured entries.
///
/// Each line has format: `stash@{N}: On branch: message`
/// or: `stash@{N}: WIP on branch: message`
/// The message may contain colons.
fn parse_stash_list(output: &str) -> GitResult<Vec<StashEntry>> {
    let mut entries = Vec::new();

    for (i, line) in output.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let message = extract_stash_message(line);
        entries.push(StashEntry { index: i, message });
    }

    Ok(entries)
}

/// Extract the human-readable message from a stash list line.
///
/// Input: `stash@{0}: On main: my commit message`
/// Output: `my commit message`
///
/// Skips the ref prefix (`stash@{N}:`), then the branch label
/// (`On branch:` or `WIP on branch:`), returning everything after.
fn extract_stash_message(line: &str) -> String {
    // Skip "stash@{N}:"
    let after_ref = match line.find(':') {
        Some(pos) => &line[pos + 1..],
        None => return line.to_string(),
    };

    // Skip "On branch:" or "WIP on branch:"
    match after_ref.find(':') {
        Some(pos) => after_ref[pos + 1..].trim().to_string(),
        None => after_ref.trim().to_string(),
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Run a git command in the given working directory.
fn run_in(
    dir: &std::path::Path,
    cmd: &mut std::process::Command,
) -> GitResult<std::process::Output> {
    cmd.current_dir(dir).output().map_err(|e| {
        GitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("git command failed: {e}"),
        ))
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_stash_list tests --

    #[test]
    fn parse_empty_list() {
        let entries = parse_stash_list("").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_single_entry() {
        let output = "stash@{0}: On main: my message\n";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[0].message, "my message");
    }

    #[test]
    fn parse_multiple_entries() {
        let output = "stash@{0}: On main: latest\nstash@{1}: On main: older\n";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "latest");
        assert_eq!(entries[1].message, "older");
    }

    #[test]
    fn parse_wip_prefix() {
        let output = "stash@{0}: WIP on main: abc123 work in progress\n";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries[0].message, "abc123 work in progress");
    }

    #[test]
    fn parse_message_with_colons() {
        let output = "stash@{0}: On main: fix: handle colons: safely\n";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries[0].message, "fix: handle colons: safely");
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let output = "stash@{0}: On main: msg\n\nstash@{1}: On main: other\n";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "msg");
        assert_eq!(entries[1].message, "other");
    }

    // -- extract_stash_message tests --

    #[test]
    fn extract_from_on_prefix() {
        assert_eq!(
            extract_stash_message("stash@{0}: On main: hello world"),
            "hello world"
        );
    }

    #[test]
    fn extract_from_wip_prefix() {
        assert_eq!(
            extract_stash_message("stash@{0}: WIP on feature: work"),
            "work"
        );
    }

    #[test]
    fn extract_no_colon_after_ref() {
        assert_eq!(
            extract_stash_message("stash@{0}: malformed entry"),
            "malformed entry"
        );
    }

    #[test]
    fn extract_no_colon_at_all() {
        assert_eq!(
            extract_stash_message("no colons here"),
            "no colons here"
        );
    }

    #[test]
    fn extract_empty_message() {
        assert_eq!(
            extract_stash_message("stash@{0}: On main:"),
            ""
        );
    }

    // -- StashEntry tests --

    #[test]
    fn stash_entry_debug() {
        let entry = StashEntry {
            index: 2,
            message: "test".to_string(),
        };
        let debug = format!("{entry:?}");
        assert!(debug.contains("test"));
        assert!(debug.contains("2"));
    }

    #[test]
    fn stash_entry_clone() {
        let entry = StashEntry {
            index: 1,
            message: "clone me".to_string(),
        };
        let cloned = entry.clone();
        assert_eq!(entry.index, cloned.index);
        assert_eq!(entry.message, cloned.message);
    }
}

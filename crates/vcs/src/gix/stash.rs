//! Gitoxide Stash Operations
//!
//! Stash operations use the `git stash` CLI as a fallback because gix does
//! not yet provide native stash support.  `list()` uses the gix reference
//! log for `refs/stash` where possible.

use crate::error::{GitError, GitResult};
use crate::gix::cli::{cli_error, require_workdir, run_git};

// -- public API ---------------------------------------------------------------

/// List stash entries.
///
/// Reads the reflog of `refs/stash` via gix when the reference exists,
/// falling back to `git stash list` otherwise.
pub fn list(repo: &gix::Repository) -> GitResult<Vec<StashEntry>> {
    match list_via_gix(repo) {
        Ok(entries) => Ok(entries),
        Err(_) => list_via_cli(repo),
    }
}

/// Save (push) a new stash entry.
///
/// Uses the `git stash push` CLI because gix lacks stash write support.
pub fn save(
    repo: &gix::Repository,
    message: Option<&str>,
    include_untracked: bool,
) -> GitResult<StashEntry> {
    let workdir = require_workdir(repo, "stash save")?;

    let mut args: Vec<&str> = vec!["stash", "push"];
    if include_untracked {
        args.push("--include-untracked");
    }
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }

    let output = run_git(workdir, &args)?;

    if !output.success {
        return Err(cli_error(&output, "stash save"));
    }

    // After a successful push, re-read the list to find the new entry at
    // index 0 and return it.
    let entries = list(repo)?;
    entries
        .into_iter()
        .next()
        .ok_or_else(|| GitError::InvalidRef {
            name: "stash".to_string(),
            reason: "stash save succeeded but no stash entry found".to_string(),
        })
}

/// Pop a stash entry (apply and remove).
pub fn pop(repo: &gix::Repository, index: usize) -> GitResult<()> {
    let workdir = require_workdir(repo, "stash pop")?;
    let output = run_git(workdir, &["stash", "pop", &index.to_string()])?;

    if !output.success {
        return Err(cli_error(&output, "stash pop"));
    }
    Ok(())
}

/// Drop a stash entry without applying it.
pub fn drop(repo: &gix::Repository, index: usize) -> GitResult<()> {
    let workdir = require_workdir(repo, "stash drop")?;
    let output = run_git(workdir, &["stash", "drop", &index.to_string()])?;

    if !output.success {
        return Err(cli_error(&output, "stash drop"));
    }
    Ok(())
}

/// Show the diff of a stash entry.
pub fn show(repo: &gix::Repository, index: usize) -> GitResult<String> {
    let workdir = require_workdir(repo, "stash show")?;
    let output = run_git(workdir, &["stash", "show", "-p", &index.to_string()])?;

    if !output.success {
        return Err(cli_error(&output, "stash show"));
    }
    Ok(output.stdout)
}

// -- domain type --------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

// -- internal helpers ---------------------------------------------------------

/// Read stash entries from the gix reflog of `refs/stash`.
fn list_via_gix(repo: &gix::Repository) -> GitResult<Vec<StashEntry>> {
    let stash_ref = repo
        .find_reference("refs/stash")
        .map_err(|_| GitError::InvalidRef {
            name: "refs/stash".to_string(),
            reason: "no stash reference found".to_string(),
        })?;

    let mut log_iter = stash_ref.log_iter();
    let iter = log_iter.rev().map_err(|e| GitError::InvalidRef {
        name: "refs/stash".to_string(),
        reason: format!("failed to read stash reflog: {e}"),
    })?;

    let Some(iter) = iter else {
        return Ok(Vec::new());
    };

    let mut entries = Vec::new();
    for (i, entry_result) in iter.enumerate() {
        let entry = entry_result.map_err(|e| GitError::InvalidRef {
            name: "refs/stash".to_string(),
            reason: format!("failed to read reflog entry {i}: {e}"),
        })?;

        let message = String::from_utf8_lossy(&entry.message)
            .trim()
            .to_string();

        entries.push(StashEntry { index: i, message });
    }

    Ok(entries)
}

/// Fall back to `git stash list` and parse each line.
fn list_via_cli(repo: &gix::Repository) -> GitResult<Vec<StashEntry>> {
    let workdir = require_workdir(repo, "stash list")?;
    let output = run_git(workdir, &["stash", "list"])?;

    if !output.success {
        return Err(cli_error(&output, "stash list"));
    }

    parse_stash_list(&output.stdout)
}

/// Parse `git stash list` output lines.
///
/// Expected format per line:
/// ```text
/// stash@{0}: On branch: WIP commit msg
/// ```
fn parse_stash_list(stdout: &str) -> GitResult<Vec<StashEntry>> {
    let mut entries = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Extract index from "stash@{N}:"
        let colon_pos = trimmed
            .find(':')
            .ok_or_else(|| GitError::InvalidRef {
                name: "stash".to_string(),
                reason: format!("malformed stash line: {trimmed}"),
            })?;

        let index_part = &trimmed[..colon_pos];
        let index = parse_stash_index(index_part)?;

        // Everything after "stash@{N}: " is the message.
        let message_start = colon_pos + 2;
        let message = if message_start < trimmed.len() {
            trimmed[message_start..].to_string()
        } else {
            format!("stash@{{{index}}}")
        };

        entries.push(StashEntry { index, message });
    }

    Ok(entries)
}

/// Extract the numeric index from a string like `stash@{0}`.
fn parse_stash_index(s: &str) -> GitResult<usize> {
    let open = s.find('{').ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: format!("missing '{{' in stash ref: {s}"),
    })?;
    let close = s.find('}').ok_or_else(|| GitError::InvalidRef {
        name: "stash".to_string(),
        reason: format!("missing '}}' in stash ref: {s}"),
    })?;

    s[open + 1..close]
        .parse::<usize>()
        .map_err(|e| GitError::InvalidRef {
            name: "stash".to_string(),
            reason: format!("invalid stash index: {e}"),
        })
}

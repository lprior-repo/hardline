//! Session commands (from Isolate)

use scp_core::{vcs, Result};

/// List sessions
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        println!("No sessions found");
    } else {
        println!("Sessions:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            println!("  - {} on branch {}{}", ws.name, ws.branch, current);
        }
    }

    Ok(())
}

/// Convert VcsStatus to a human-readable string (pure function)
const fn vcs_status_to_str(status: scp_core::vcs::VcsStatus) -> &'static str {
    match status {
        scp_core::vcs::VcsStatus::Clean => "clean",
        scp_core::vcs::VcsStatus::Dirty => "dirty",
        scp_core::vcs::VcsStatus::Conflicted => "conflicted",
        scp_core::vcs::VcsStatus::Detached => "detached",
    }
}

/// Extract the first line from a commit message (pure function)
fn first_commit_line(message: &str) -> String {
    message.lines().next().map(String::from).unwrap_or_default()
}

/// Format a single commit for display (pure function)
fn format_commit(commit: &scp_core::vcs::Commit) -> (String, String) {
    let id = commit.id.chars().take(8).collect::<String>();
    let msg = first_commit_line(&commit.message);
    (id, msg)
}

/// Format recent commits for display (pure function)
fn format_recent_commits(log: &[scp_core::vcs::Commit], limit: usize) -> Vec<(String, String)> {
    log.iter().take(limit).map(format_commit).collect()
}

/// Show session status
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;
    let state = vcs_status_to_str(vcs_status);

    println!("Session Status:");
    println!("  Branch: {}", branch);
    println!("  State: {}", state);

    let log = backend.log(5)?;
    let recent = format_recent_commits(&log, 3);
    if !recent.is_empty() {
        println!("  Recent commits:");
        for (id, msg) in recent {
            println!("    - {}", id);
            if !msg.is_empty() {
                println!("      {}", msg);
            }
        }
    }

    Ok(())
}

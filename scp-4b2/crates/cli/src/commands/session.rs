//! Session commands (from Isolate)

use scp_core::{vcs, Result};

/// List sessions
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::Io(e))?;

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

/// Show session status
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| scp_core::Error::Io(e))?;

    let backend = vcs::create_backend(&cwd)?;

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    let state = match vcs_status {
        scp_core::vcs::VcsStatus::Clean => "clean",
        scp_core::vcs::VcsStatus::Dirty => "dirty",
        scp_core::vcs::VcsStatus::Conflicted => "conflicted",
        scp_core::vcs::VcsStatus::Detached => "detached",
    };

    println!("Session Status:");
    println!("  Branch: {}", branch);
    println!("  State: {}", state);

    let log = backend.log(5)?;
    if !log.is_empty() {
        println!("  Recent commits:");
        for commit in log.iter().take(3) {
            println!("    - {}", commit.id.chars().take(8).collect::<String>());
            if !commit.message.is_empty() {
                println!("      {}", commit.message.lines().next().unwrap_or(""));
            }
        }
    }

    Ok(())
}

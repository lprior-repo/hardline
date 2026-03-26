//! Session commands (from Isolate)

use scp_core::{output::Output, vcs, Error, Result};

/// List sessions
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

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
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

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

/// Focus (switch to) a session
pub fn focus(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    Output::info(&format!("Focusing session '{}'...", name));

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    backend.switch_workspace(name)?;
    Output::success(&format!("Focused session '{}'", name));
    Ok(())
}

/// Submit session changes for review
pub fn submit(name: Option<&str>, auto_commit: bool, message: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspace_name = if let Some(n) = name {
        n.to_string()
    } else {
        let workspaces = backend.list_workspaces()?;
        workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.clone())
            .ok_or_else(|| Error::workspace_not_found("no current session".to_string()))?
    };

    Output::info(&format!("Submitting session '{}'...", workspace_name));

    let vcs_status = backend.status()?;
    if vcs_status == scp_core::vcs::VcsStatus::Dirty {
        if auto_commit {
            if let Some(msg) = message {
                let output = std::process::Command::new("jj")
                    .args(["describe", "-m", msg])
                    .current_dir(&cwd)
                    .output()
                    .map_err(|e| Error::io_error(e.to_string()))?;
                if !output.status.success() {
                    return Err(Error::vcs_conflict(
                        "commit",
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
            } else {
                return Err(Error::invalid_state(
                    "dirty working copy requires --message".to_string(),
                ));
            }
        } else {
            return Err(Error::working_copy_dirty());
        }
    }

    backend.push()?;
    Output::success("Pushed to remote");

    println!("✓ Submitted session '{}'", workspace_name);
    Ok(())
}

/// Remove a session
pub fn remove(name: &str, force: bool, merge: bool) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(
            "session name cannot be empty".to_string(),
        ));
    }

    if name == "main" {
        return Err(Error::invalid_state(
            "cannot remove the main session".to_string(),
        ));
    }

    Output::info(&format!("Removing session '{}'...", name));

    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if !workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_not_found(name.to_string()));
    }

    if merge {
        backend.rebase("main")?;
        Output::success("Merged with main");
    }

    backend.delete_workspace(name)?;
    Output::success(&format!("Removed session '{}'", name));
    Ok(())
}

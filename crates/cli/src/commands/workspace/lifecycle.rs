//! Workspace lifecycle commands

use scp_core::output::Output;
use scp_core::vcs;

use super::operations::*;
use super::types::SyncOption;
use crate::Error;

/// Create a new workspace
pub fn spawn(name: &str, sync: SyncOption) -> Result<(), Error> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = super::validators::validate_workspace_name(name) {
        return Err(err);
    }

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace already exists
    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::WorkspaceExists(name.to_string()));
    }

    spawn_with_sync(backend.as_ref(), name, sync.is_sync())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<(), Error> {
    // P1: Validate workspace name is not empty
    if name.is_empty() {
        return Err(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists and working copy is clean
    if !workspace_exists(backend.as_ref(), name)? {
        return Err(Error::WorkspaceNotFound(name.to_string()));
    }
    require_clean_working_copy(backend.as_ref())?;

    backend.switch_workspace(name)?;
    Output::success(&format!("Switched to '{}'", name));
    Ok(())
}

/// List workspaces
pub fn list() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        Output::info("No workspaces found");
    } else {
        Output::info("Workspaces:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            Output::info(&format!("  - {}{}", ws.name, current));
        }
    }

    Ok(())
}

/// Show workspace status
pub fn status() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info(&format!("Current branch: {}", branch));
    Output::info(&format!("Status: {}", vcs_status));

    Ok(())
}

/// Sync workspace with main
pub fn sync(name: Option<&str>, all: bool) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    if all {
        // Sync all workspaces
        let workspaces = backend.list_workspaces()?;
        for ws in workspaces {
            if !ws.is_current {
                backend.switch_workspace(&ws.name)?;
            }
            backend.rebase("main")?;
            Output::success(&format!("Synced {}", ws.name));
        }
    } else {
        backend.rebase("main")?;
        Output::success("Synced with main");
    }

    Ok(())
}

/// Split workspace
pub fn add(path: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    split_workspace(backend.as_ref(), path)
}

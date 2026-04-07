//! Workspace navigation commands

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

use super::workspace_helpers::{
    ensure_not_main_workspace, execute_workspace_abort, find_next_workspace, find_prev_workspace,
    require_clean_working_copy, resolve_workspace_name, spawn_with_sync, workspace_exists,
};
use super::workspace_types::validate_workspace_name;

/// Create a new workspace
pub fn spawn(name: &str, sync: bool) -> Result<()> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = validate_workspace_name(name) {
        return Err(err);
    }

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace already exists
    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_exists(name));
    }

    spawn_with_sync(backend.as_ref(), name, sync)
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<()> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = validate_workspace_name(name) {
        return Err(err);
    }

    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists and working copy is clean
    if !workspace_exists(backend.as_ref(), name)? {
        return Err(Error::workspace_not_found(name));
    }
    require_clean_working_copy(backend.as_ref())?;

    backend.switch_workspace(name)?;
    Output::success(&format!("Switched to '{}'", name));
    Ok(())
}

/// List workspaces
pub fn list() -> Result<()> {
    let cwd = std::env::current_dir()?;

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
pub fn status() -> Result<()> {
    let cwd = std::env::current_dir()?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info(&format!("Current branch: {}", branch));
    Output::info(&format!("Status: {}", vcs_status));

    Ok(())
}

/// Switch to next workspace (alphabetically)
pub fn next() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspaces.is_empty() {
        return Err(Error::workspace_not_found("no workspaces exist"));
    }

    // Use helper function to find next workspace
    let target_name = find_next_workspace(&workspaces)?;

    // P4: Check for dirty working copy
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::working_copy_dirty());
    }

    Output::info(&format!("Switching to workspace '{}'...", target_name));
    backend.switch_workspace(&target_name)?;
    Output::success(&format!("Switched to '{}'", target_name));
    Ok(())
}

/// Switch to previous workspace (alphabetically)
pub fn prev() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspaces.is_empty() {
        return Err(Error::workspace_not_found("no workspaces exist"));
    }

    // Use helper function to find previous workspace
    let target_name = find_prev_workspace(&workspaces)?;

    // P4: Check for dirty working copy
    let status = backend.status()?;
    if status != VcsStatus::Clean {
        return Err(Error::working_copy_dirty());
    }

    Output::info(&format!("Switching to workspace '{}'...", target_name));
    backend.switch_workspace(&target_name)?;
    Output::success(&format!("Switched to '{}'", target_name));
    Ok(())
}

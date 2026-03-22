//! Workspace operations

use std::process::Command;

use scp_core::{
    output::Output,
    vcs::{self, VcsStatus},
    Error, Result,
};

use super::workspace_helpers::{
    complete_workspace_workflow, ensure_not_main_workspace, execute_workspace_abort,
    require_clean_working_copy, resolve_workspace_name, workspace_exists,
};

/// Sync workspace with main
pub fn sync(_name: Option<&str>, all: bool) -> Result<()> {
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

/// Complete workspace and merge
pub fn done(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    // P4: Check for dirty working copy BEFORE any operations
    require_clean_working_copy(backend.as_ref())?;

    // Resolve workspace name: if None, get current workspace
    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;

    // P3: Check workspace exists
    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::WorkspaceNotFound(workspace_name.clone()));
    }

    Output::info(&format!("Completing workspace '{}'...", workspace_name));

    complete_workspace_workflow(backend.as_ref(), &workspace_name)
}

/// Abort workspace
pub fn abort(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    require_clean_working_copy(backend.as_ref())?;

    let workspace_name = resolve_workspace_name(backend.as_ref(), name)?;
    ensure_not_main_workspace(&workspace_name)?;

    if !workspace_exists(backend.as_ref(), &workspace_name)? {
        return Err(Error::WorkspaceNotFound(workspace_name.clone()));
    }

    Output::info(&format!("Aborting workspace '{}'...", workspace_name));
    execute_workspace_abort(backend.as_ref(), &workspace_name)
}

/// Fork a workspace from another workspace
pub fn fork(name: &str, from: Option<&str>) -> Result<()> {
    let source = from.unwrap_or("main");
    println!("Forking workspace '{}' from '{}'...", name, source);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    backend.fork_workspace(source, name)?;
    println!("✓ Forked workspace '{}' from '{}'", name, source);

    Ok(())
}

/// Merge a workspace into main
pub fn merge(name: &str) -> Result<()> {
    println!("Merging workspace '{}' into main...", name);

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    backend.merge_workspace(name)?;
    println!("✓ Merged workspace '{}' into main", name);

    Ok(())
}

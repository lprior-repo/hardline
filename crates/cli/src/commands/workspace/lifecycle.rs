//! Workspace lifecycle commands

use scp_core::output::Output;
use scp_core::vcs;
use scp_core::Error;

use super::operations::*;
use super::types::SyncOption;

/// Create a new workspace
pub fn spawn(name: &str, sync: SyncOption) -> Result<(), Error> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = super::validators::validate_workspace_name(name) {
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

    spawn_with_sync(backend.as_ref(), name, sync.is_sync())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<(), Error> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = super::validators::validate_workspace_name(name) {
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
pub fn list() -> Result<(), Error> {
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
pub fn status() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info(&format!("Current branch: {}", branch));
    Output::info(&format!("Status: {}", vcs_status));

    Ok(())
}

/// Sync workspace with main
pub fn sync(name: Option<&str>, all: bool) -> Result<(), Error> {
    let options = crate::commands::handlers::sync::SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: crate::commands::handlers::sync::RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
        },
    };

    tokio::runtime::Handle::current().block_on(async {
        if all {
            crate::commands::handlers::sync::sync_all_sessions(options).await
        } else if let Some(n) = name {
            let session_name = scp_core::domain::SessionName::parse(n).map_err(|e| {
                crate::commands::handlers::sync::SyncError::InvalidIdentifier(e.to_string())
            })?;
            crate::commands::handlers::sync::sync_named_session(session_name, options).await
        } else {
            crate::commands::handlers::sync::sync_current_workspace(options).await
        }
    })?;

    Ok(())
}

/// Split workspace
pub fn add(path: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    split_workspace(backend.as_ref(), path)
}

//! Context command - shows current workspace/branch/location

use scp_core::{output::Output, vcs, Result};

/// Show current context (workspace, branch, VCS status)
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().map_err(scp_core::Error::Io)?;

    let backend = vcs::create_backend(&cwd)?;

    // Get current workspace by finding the one with is_current = true
    let workspaces = backend.list_workspaces()?;
    let workspace_name = workspaces
        .into_iter()
        .find(|w| w.is_current)
        .map(|w| w.name)
        .unwrap_or_else(|| "unknown".to_string());

    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info("Current Context:");
    Output::info(&format!("  Workspace: {}", workspace_name));
    Output::info(&format!("  Branch: {}", branch));
    Output::info(&format!("  Status: {}", vcs_status));

    Ok(())
}

/// Alias for run() - shows current context
pub fn whereami() -> Result<()> {
    run()
}

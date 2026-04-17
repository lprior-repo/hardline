//! Workspace navigation commands

use scp_core::output::Output;
use scp_core::vcs;
use scp_core::Error;

/// Next workspace
pub fn next() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let next_name = super::operations::find_next_workspace(&workspaces)?;

    backend.switch_workspace(&next_name)?;
    Output::success(&format!("Switched to next workspace: '{}'", next_name));

    Ok(())
}

/// Previous workspace
pub fn prev() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let prev_name = super::operations::find_prev_workspace(&workspaces)?;

    backend.switch_workspace(&prev_name)?;
    Output::success(&format!("Switched to previous workspace: '{}'", prev_name));

    Ok(())
}

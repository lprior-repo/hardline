//! Workspace navigation commands

use scp_core::{output::Output, vcs, Error};

/// Next workspace
pub fn next() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let next_name = super::operations::find_next_workspace(&workspaces)?;

    let path = backend.switch_workspace(&next_name)?;
    Output::success(&format!("Switched to next workspace: '{}'", next_name));
    Output::info(&format!("Path: {}", path.display()));
    Output::info(&format!(
        "Run `cd {}` to move your shell there.",
        path.display()
    ));

    Ok(())
}

/// Previous workspace
pub fn prev() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    let prev_name = super::operations::find_prev_workspace(&workspaces)?;

    let path = backend.switch_workspace(&prev_name)?;
    Output::success(&format!("Switched to previous workspace: '{}'", prev_name));
    Output::info(&format!("Path: {}", path.display()));
    Output::info(&format!(
        "Run `cd {}` to move your shell there.",
        path.display()
    ));

    Ok(())
}

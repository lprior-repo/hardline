//! Workspace fork and merge commands

use std::process::Command;

use scp_core::{output::Output, vcs, Error};

/// Fork workspace
pub fn fork(name: &str, from: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_exists(name));
    }

    let from_branch = from.unwrap_or("main");

    let output = Command::new("git")
        .args(["worktree", "add", name, from_branch])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("worktree add", stderr));
    }

    Output::success(&format!(
        "Forked workspace '{}' from '{}'",
        name, from_branch
    ));

    Ok(())
}

/// Merge workspace
pub fn merge(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    backend.switch_workspace(name)?;
    backend.rebase("main")?;

    Output::success(&format!("Merged workspace '{}'", name));

    Ok(())
}

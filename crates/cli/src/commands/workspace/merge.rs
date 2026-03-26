//! Workspace fork and merge commands

use std::process::Command;

use crate::Error;
use scp_core::output::Output;
use scp_core::vcs;

/// Fork workspace
pub fn fork(name: &str, from: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::WorkspaceExists(name.to_string()));
    }

    let from_branch = from.unwrap_or("main");

    let output = Command::new("jj")
        .args(["workspace", "fork", name, "--from", from_branch])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj workspace fork".to_string(),
            stderr.to_string(),
        ));
    }

    Output::success(&format!(
        "Forked workspace '{}' from '{}'",
        name, from_branch
    ));

    Ok(())
}

/// Merge workspace
pub fn merge(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let backend = vcs::create_backend(&cwd)?;

    backend.switch_workspace(name)?;
    backend.rebase("main")?;

    Output::success(&format!("Merged workspace '{}'", name));

    Ok(())
}

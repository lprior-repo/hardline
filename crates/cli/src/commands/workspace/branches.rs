//! Workspace branch commands

use std::process::Command;

use scp_core::output::Output;
use scp_core::Error;

/// List branches
pub fn branches() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let output = Command::new("jj")
        .args(["branch", "list"])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj branch list", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Create branch
pub fn branch_create(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let output = Command::new("jj")
        .args(["branch", "move", name])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj branch move", stderr));
    }

    Output::success(&format!("Created branch '{}'", name));

    Ok(())
}

/// Delete branch
pub fn branch_delete(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let output = Command::new("jj")
        .args(["branch", "delete", name])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj branch delete", stderr));
    }

    Output::success(&format!("Deleted branch '{}'", name));

    Ok(())
}

/// Show current branch
pub fn branch_current() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let output = Command::new("jj")
        .args(["branch", "show", "@"])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj branch show", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

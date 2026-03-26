//! Workspace branch commands

use std::process::Command;

use crate::Error;
use scp_core::output::Output;

/// List branches
pub fn branches() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["branch", "list"])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj branch list".to_string(),
            stderr.to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Create branch
pub fn branch_create(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["branch", "move", name])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj branch move".to_string(),
            stderr.to_string(),
        ));
    }

    Output::success(&format!("Created branch '{}'", name));

    Ok(())
}

/// Delete branch
pub fn branch_delete(name: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["branch", "delete", name])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj branch delete".to_string(),
            stderr.to_string(),
        ));
    }

    Output::success(&format!("Deleted branch '{}'", name));

    Ok(())
}

/// Show current branch
pub fn branch_current() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["branch", "show", "@"])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj branch show".to_string(),
            stderr.to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

//! Workspace diff and commit commands

use std::process::Command;

use crate::Error;
use scp_core::output::Output;

/// Show diff
pub fn diff(path: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let cmd = super::operations::build_jj_diff_command(&cwd, path.as_deref());
    let output = cmd.output().map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj diff".to_string(),
            stderr.to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Show uncommitted changes
pub fn uncommitted() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["diff"])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj diff".to_string(),
            stderr.to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Commit changes
pub fn commit(message: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["commit", "-m", message])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "jj commit".to_string(),
            stderr.to_string(),
        ));
    }

    Output::success("Committed changes");

    Ok(())
}

/// Show workspace log
pub fn log(limit: Option<usize>) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let mut args = vec!["log"];
    if let Some(l) = limit {
        args.push("-l");
        args.push(&l.to_string());
    }

    let output = Command::new("jj")
        .args(&args)
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict("jj log".to_string(), stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

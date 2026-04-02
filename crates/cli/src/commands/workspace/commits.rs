//! Workspace diff and commit commands

use std::process::Command;

use scp_core::output::Output;
use scp_core::Error;

/// Show diff
pub fn diff(path: Option<&str>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let mut cmd = super::operations::build_jj_diff_command(&cwd, path);
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj diff", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Show uncommitted changes
pub fn uncommitted() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let output = Command::new("jj")
        .args(["diff"])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj diff", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

/// Commit changes
pub fn commit(message: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let output = Command::new("jj")
        .args(["commit", "-m", message])
        .current_dir(&cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj commit", stderr));
    }

    Output::success("Committed changes");

    Ok(())
}

/// Show workspace log
pub fn log(limit: Option<usize>) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let mut args = vec!["log"];
    let limit_str = limit.map(|l| l.to_string());
    #[allow(unused_assignments)]
    if let Some(ref s) = limit_str {
        args.push("-l");
        args.push(s);
    }

    let output = Command::new("jj").args(&args).current_dir(&cwd).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::vcs_conflict("jj log", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Output::info(&stdout);

    Ok(())
}

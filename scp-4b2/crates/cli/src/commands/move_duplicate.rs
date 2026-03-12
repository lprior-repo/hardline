//! Move and Duplicate commands

use std::process::Command;

use scp_core::{Error, Result};

/// Move changes from one revision to another using jj move
pub fn move_changes(source: &str, dest: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["move", "--from", source, "--to", dest])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::VcsConflict(
            "jj move".to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Moved changes from {} to {}", source, dest);
    Ok(())
}

/// Duplicate a revision using jj duplicate
pub fn duplicate(revision: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["duplicate", revision])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::VcsConflict(
            "jj duplicate".to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Duplicated {}", revision);
    Ok(())
}

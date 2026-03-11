//! Rebase commands - restack, rebase, move, and duplicate

use scp_core::{Error, Result};

pub fn restack() -> Result<()> {
    use std::process::Command;

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["restack"])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsRebaseFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Restacked successfully");
    Ok(())
}

pub fn rebase(dest: &str) -> Result<()> {
    use std::process::Command;

    if dest.is_empty() {
        return Err(Error::InvalidIdentifier(
            "destination cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["rebase", "-d", dest])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsRebaseFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Rebased onto {}", dest);
    Ok(())
}

pub fn mv(source: &str, dest: &str) -> Result<()> {
    use std::process::Command;

    if source.is_empty() || dest.is_empty() {
        return Err(Error::InvalidIdentifier(
            "source and destination cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["move", "--from", source, "--to", dest])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsRebaseFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Moved changes from {} to {}", source, dest);
    Ok(())
}

pub fn duplicate(revision: &str) -> Result<()> {
    use std::process::Command;

    if revision.is_empty() {
        return Err(Error::InvalidIdentifier(
            "revision cannot be empty".to_string(),
        ));
    }

    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let output = Command::new("jj")
        .args(["duplicate", revision])
        .current_dir(&cwd)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsRebaseFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("{}", stdout.trim());
    }

    println!("✓ Duplicated {}", revision);
    Ok(())
}

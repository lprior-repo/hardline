//! Tag commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};

pub fn create(name: &str, message: Option<&str>, commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "tag is only supported for Git repositories".to_string(),
        ));
    }

    let mut cmd = Command::new("git");
    cmd.arg("tag");

    if force {
        cmd.arg("-f");
    }

    if let Some(msg) = message {
        cmd.arg("-a").arg(name).arg("-m").arg(msg);
    } else {
        let commit_ref = commit.unwrap_or("HEAD");
        cmd.arg(name).arg(commit_ref);
    }

    let output = cmd.current_dir(&cwd).output().map_err(Error::Io)?;

    if output.status.success() {
        Output::success(&format!("Created tag: {}", name));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git tag".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn list(pattern: Option<&str>, sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "tag is only supported for Git repositories".to_string(),
        ));
    }

    let mut cmd = Command::new("git");
    cmd.arg("tag");

    if let Some(pat) = pattern {
        cmd.arg("-l").arg(pat);
    } else {
        cmd.arg("-l");
    }

    if let Some(sort_key) = sort {
        cmd.arg("--sort").arg(sort_key);
    }

    let output = cmd.current_dir(&cwd).output().map_err(Error::Io)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            Output::info("No tags found");
        } else {
            print!("{}", stdout);
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git tag list".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn delete(tag: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "tag is only supported for Git repositories".to_string(),
        ));
    }

    let output = if remote {
        Command::new("git")
            .args(["push", "origin", "--delete", tag])
            .current_dir(&cwd)
            .output()
            .map_err(Error::Io)?
    } else {
        Command::new("git")
            .args(["tag", "-d", tag])
            .current_dir(&cwd)
            .output()
            .map_err(Error::Io)?
    };

    if output.status.success() {
        let scope = if remote { "remote" } else { "local" };
        Output::success(&format!("Deleted {} tag: {}", scope, tag));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git tag delete".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn push(tag: Option<&str>, remote: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "tag is only supported for Git repositories".to_string(),
        ));
    }

    let output = if let Some(t) = tag {
        let mut cmd = Command::new("git");
        cmd.arg("push").arg(remote).arg(t);
        if force {
            cmd.arg("--force");
        }
        cmd.current_dir(&cwd).output().map_err(Error::Io)?
    } else {
        let mut cmd = Command::new("git");
        cmd.arg("push").arg(remote).arg("--tags");
        if force {
            cmd.arg("--force");
        }
        cmd.current_dir(&cwd).output().map_err(Error::Io)?
    };

    if output.status.success() {
        Output::success(&format!("Pushed tags to {}", remote));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsPushFailed(stderr.to_string()));
    }

    Ok(())
}

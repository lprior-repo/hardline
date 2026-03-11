//! Stash commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};

fn build_git_stash_save_command(
    cwd: &std::path::Path,
    message: Option<&str>,
    include_untracked: bool,
    patch: bool,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("push");

    if let Some(msg) = message {
        cmd.arg("-m").arg(msg);
    }

    if include_untracked {
        cmd.arg("-u");
    }

    if patch {
        cmd.arg("-p");
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_git_stash_pop_command(
    cwd: &std::path::Path,
    stash: Option<&str>,
    restore_index: bool,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("pop");

    if let Some(s) = stash {
        cmd.arg(s);
    }

    if restore_index {
        cmd.arg("--index");
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_git_stash_list_command(cwd: &std::path::Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["stash", "list"]).current_dir(cwd);
    cmd
}

fn build_git_stash_drop_command(cwd: &std::path::Path, stash: &str, force: bool) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("drop");

    if force {
        cmd.arg("-f");
    }

    cmd.arg(stash);
    cmd.current_dir(cwd);
    cmd
}

fn build_git_stash_show_command(cwd: &std::path::Path, stash_ref: &str, stat: bool) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("show");

    if stat {
        cmd.arg("--stat");
    }

    cmd.arg(stash_ref);
    cmd.current_dir(cwd);
    cmd
}

pub fn save(message: Option<&str>, include_untracked: bool, patch: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "stash is only supported for Git repositories".to_string(),
        ));
    }

    let output = build_git_stash_save_command(&cwd, message, include_untracked, patch)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        let msg = message.unwrap_or("changes");
        Output::success(&format!("Stashed: {}", msg));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git stash".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn pop(stash: Option<&str>, restore_index: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "stash is only supported for Git repositories".to_string(),
        ));
    }

    let output = build_git_stash_pop_command(&cwd, stash, restore_index)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        Output::success("Applied stash and removed from stash list");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git stash pop".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "stash is only supported for Git repositories".to_string(),
        ));
    }

    let output = build_git_stash_list_command(&cwd)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            Output::info("No stashed changes");
        } else {
            print!("{}", stdout);
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git stash list".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn drop(stash: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "stash is only supported for Git repositories".to_string(),
        ));
    }

    let output = build_git_stash_drop_command(&cwd, stash, force)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        Output::success(&format!("Dropped stash: {}", stash));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git stash drop".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

pub fn show(stash: Option<&str>, stat: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::InvalidState(
            "stash is only supported for Git repositories".to_string(),
        ));
    }

    let stash_ref = stash.unwrap_or("stash@{0}");

    let output = build_git_stash_show_command(&cwd, stash_ref, stat)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        print!("{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsConflict(
            "git stash show".to_string(),
            stderr.to_string(),
        ));
    }

    Ok(())
}

//! Tag commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};

fn build_git_tag_create_command(
    cwd: &std::path::Path,
    name: &str,
    message: Option<&str>,
    commit: Option<&str>,
    force: bool,
) -> Command {
    let commit_ref = commit.unwrap_or_else(|| "HEAD");
    let tag_args = if force {
        vec!["tag", "-f", name, commit_ref]
    } else {
        vec!["tag", name, commit_ref]
    };

    Command::new("git")
        .args(&tag_args)
        .pipe(|cmd| {
            message.map_or(cmd, |msg| {
                Command::new("git")
                    .args(["tag", "-a", name, "-m", msg])
                    .current_dir(cwd)
            })
        })
        .current_dir(cwd)
}

fn build_git_tag_list_command(
    cwd: &std::path::Path,
    pattern: Option<&str>,
    sort: Option<&str>,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("tag");

    if pattern.is_some() {
        cmd.arg("-l");
    } else {
        cmd.arg("-l");
    }

    if let Some(pat) = pattern {
        cmd.arg(pat);
    }

    if let Some(sort_key) = sort {
        cmd.args(["--sort", sort_key]);
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_git_tag_delete_command(cwd: &std::path::Path, tag: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["tag", "-d", tag]).current_dir(cwd);
    cmd
}

fn build_git_tag_delete_remote_command(cwd: &std::path::Path, tag: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["push", "origin", "--delete", tag])
        .current_dir(cwd);
    cmd
}

fn build_git_tag_push_command(
    cwd: &std::path::Path,
    remote: &str,
    tag: &str,
    force: bool,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("push").arg(remote).arg(tag);
    if force {
        cmd.arg("--force");
    }
    cmd.current_dir(cwd);
    cmd
}

fn build_git_tag_push_all_command(cwd: &std::path::Path, remote: &str, force: bool) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("push").arg(remote).arg("--tags");
    if force {
        cmd.arg("--force");
    }
    cmd.current_dir(cwd);
    cmd
}

/// Validates the current directory is a Git repository.
fn validate_git_vcs(cwd: &std::path::Path) -> Result<()> {
    detect_vcs(cwd)
        .filter(|&vcs| vcs == scp_core::vcs::VcsType::Git)
        .map(|_| ())
        .ok_or_else(|| {
            Error::InvalidState("tag is only supported for Git repositories".to_string())
        })
}

/// Processes command output, printing success message or returning error.
fn process_command_output(output: &std::process::Output, context: &str) -> Result<()> {
    output.status.success().then(|| ()).ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Error::VcsConflict(context.to_string(), stderr.to_string())
    })
}

/// Prints tags to stdout, or info message if none found.
fn print_tags(stdout: &str) {
    if stdout.trim().is_empty() {
        Output::info("No tags found");
    } else {
        print!("{}", stdout);
    }
}

pub fn create(name: &str, message: Option<&str>, commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let output = build_git_tag_create_command(&cwd, name, message, commit, force)
        .output()
        .map_err(Error::Io)?;

    process_command_output(&output, "git tag")?;
    Output::success(&format!("Created tag: {}", name));
    Ok(())
}

pub fn list(pattern: Option<&str>, sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let output = build_git_tag_list_command(&cwd, pattern, sort)
        .output()
        .map_err(Error::Io)?;

    process_command_output(&output, "git tag list")?;
    print_tags(&String::from_utf8_lossy(&output.stdout));
    Ok(())
}

pub fn delete(tag: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let output = if remote {
        build_git_tag_delete_remote_command(&cwd, tag)
    } else {
        build_git_tag_delete_command(&cwd, tag)
    }
    .output()
    .map_err(Error::Io)?;

    process_command_output(&output, "git tag delete")?;
    let scope = if remote { "remote" } else { "local" };
    Output::success(&format!("Deleted {} tag: {}", scope, tag));
    Ok(())
}

pub fn push(tag: Option<&str>, remote: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let output = match tag {
        Some(t) => build_git_tag_push_command(&cwd, remote, t, force),
        None => build_git_tag_push_all_command(&cwd, remote, force),
    }
    .output()
    .map_err(Error::Io)?;

    if output.status.success() {
        Output::success(&format!("Pushed tags to {}", remote));
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::VcsPushFailed(stderr.to_string()))
    }
}

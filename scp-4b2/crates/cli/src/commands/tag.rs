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
    Command::new("git")
        .arg("tag")
        .tap(|cmd| {
            if force {
                cmd.arg("-f");
            }
        })
        .tap(|cmd| {
            if let Some(msg) = message {
                cmd.args(["-a", name, "-m", msg]);
            } else {
                let commit_ref = commit.unwrap_or("HEAD");
                cmd.arg(name).arg(commit_ref);
            }
        })
        .current_dir(cwd)
}

fn build_git_tag_list_command(
    cwd: &std::path::Path,
    pattern: Option<&str>,
    sort: Option<&str>,
) -> Command {
    Command::new("git")
        .arg("tag")
        .args(match pattern {
            Some(_) => ["-l"],
            None => ["-l"],
        })
        .tap(|cmd| {
            if let Some(pat) = pattern {
                cmd.arg(pat);
            }
        })
        .tap(|cmd| {
            if let Some(sort_key) = sort {
                cmd.args(["--sort", sort_key]);
            }
        })
        .current_dir(cwd)
}

fn build_git_tag_delete_command(cwd: &std::path::Path, tag: &str) -> Command {
    Command::new("git")
        .args(["tag", "-d", tag])
        .current_dir(cwd)
}

fn build_git_tag_delete_remote_command(cwd: &std::path::Path, tag: &str) -> Command {
    Command::new("git")
        .args(["push", "origin", "--delete", tag])
        .current_dir(cwd)
}

fn build_git_tag_push_command(
    cwd: &std::path::Path,
    remote: &str,
    tag: &str,
    force: bool,
) -> Command {
    Command::new("git")
        .arg("push")
        .arg(remote)
        .arg(tag)
        .tap(|cmd| {
            if force {
                cmd.arg("--force");
            }
        })
        .current_dir(cwd)
}

fn build_git_tag_push_all_command(cwd: &std::path::Path, remote: &str, force: bool) -> Command {
    Command::new("git")
        .arg("push")
        .arg(remote)
        .arg("--tags")
        .tap(|cmd| {
            if force {
                cmd.arg("--force");
            }
        })
        .current_dir(cwd)
}

/// Validates the current directory is a Git repository.
/// Returns the VCS type if valid, otherwise an error.
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

    build_git_tag_create_command(&cwd, name, message, commit, force)
        .output()
        .map_err(Error::Io)
        .and_then(|output| process_command_output(&output, "git tag"))
        .map(|_| Output::success(&format!("Created tag: {}", name)))
}

pub fn list(pattern: Option<&str>, sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    build_git_tag_list_command(&cwd, pattern, sort)
        .output()
        .map_err(Error::Io)
        .and_then(|output| {
            process_command_output(&output, "git tag list")
                .map(|_| String::from_utf8_lossy(&output.stdout).to_string())
        })
        .map(print_tags)
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

    process_command_output(&output, "git tag delete").map(|_| {
        let scope = if remote { "remote" } else { "local" };
        Output::success(&format!("Deleted {} tag: {}", scope, tag));
    })
}

pub fn push(tag: Option<&str>, remote: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let cmd = tag.map_or_else(
        || build_git_tag_push_all_command(&cwd, remote, force),
        |t| build_git_tag_push_command(&cwd, remote, t, force),
    );

    cmd.output()
        .map_err(Error::Io)
        .and_then(|output| {
            output.status.success().then_some(()).ok_or_else(|| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Error::VcsPushFailed(stderr.to_string())
            })
        })
        .map(|_| Output::success(&format!("Pushed tags to {}", remote)))
}

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
    let commit_ref = commit.unwrap_or("HEAD");
    let mut cmd = Command::new("git");
    cmd.arg("tag");

    if force {
        cmd.arg("-f");
    }

    match message {
        Some(msg) => {
            cmd.args(["-a", name, "-m", msg]);
        }
        None => {
            cmd.arg(name).arg(commit_ref);
        }
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_git_tag_list_command(
    cwd: &std::path::Path,
    pattern: Option<&str>,
    sort: Option<&str>,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("tag").arg("-l");

    pattern.iter().for_each(|pat| {
        cmd.arg(pat);
    });

    sort.iter().for_each(|sort_key| {
        cmd.args(["--sort", sort_key]);
    });

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

/// Extracts stderr from command output as a formatted string.
fn extract_stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Processes command output, printing success message or returning error.
fn process_command_output(output: &std::process::Output, context: &str) -> Result<()> {
    output
        .status
        .success()
        .then_some(())
        .ok_or_else(|| Error::VcsConflict(context.to_string(), extract_stderr(output)))
}

fn format_success_message(prefix: &str, name: &str) -> String {
    format!("{}: {}", prefix, name)
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
    Output::success(&format_success_message("Created tag", name));
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
    Output::success(&format_success_message(
        &format!("Deleted {} tag", scope),
        tag,
    ));
    Ok(())
}

pub fn push(tag: Option<&str>, remote: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    validate_git_vcs(&cwd)?;

    let mut cmd = match tag {
        Some(t) => build_git_tag_push_command(&cwd, remote, t, force),
        None => build_git_tag_push_all_command(&cwd, remote, force),
    };

    let output = cmd.output().map_err(Error::Io)?;

    output
        .status
        .success()
        .then_some(())
        .ok_or_else(|| Error::VcsPushFailed(extract_stderr(&output)))?;

    Output::success(&format_success_message("Pushed tags to", remote));
    Ok(())
}

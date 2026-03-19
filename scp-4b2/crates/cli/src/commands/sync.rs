//! Fetch and sync commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};

fn build_git_fetch_command(
    cwd: &std::path::Path,
    remote: Option<&str>,
    prune: bool,
    tags: bool,
    all: bool,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("fetch");

    if all {
        cmd.arg("--all");
    } else if let Some(r) = remote {
        cmd.arg(r);
    }

    if prune {
        cmd.arg("--prune");
    }

    if tags {
        cmd.arg("--tags");
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_jj_fetch_command(cwd: &std::path::Path, remote: Option<&str>, all: bool) -> Command {
    let mut cmd = Command::new("jj");
    cmd.arg("git");

    if all || remote.is_none() {
        cmd.arg("fetch");
    } else if let Some(r) = remote {
        cmd.arg("fetch").arg("--remote").arg(r);
    } else {
        cmd.arg("fetch");
    }

    cmd.current_dir(cwd);
    cmd
}

/// Execute fetch command for either Git or Jujutsu
fn execute_fetch(
    vcs_type: scp_core::vcs::VcsType,
    cwd: &std::path::Path,
    remote: Option<&str>,
    prune: bool,
    tags: bool,
    all: bool,
) -> Result<std::process::Output> {
    match vcs_type {
        scp_core::vcs::VcsType::Git => build_git_fetch_command(cwd, remote, prune, tags, all)
            .output()
            .map_err(Error::Io),
        scp_core::vcs::VcsType::Jujutsu => build_jj_fetch_command(cwd, remote, all)
            .output()
            .map_err(Error::Io),
    }
}

pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    let output = execute_fetch(vcs_type, &cwd, remote, prune, tags, all)?;

    output
        .status
        .success()
        .then(|| {
            stdout_if_present(&output.stdout);
            Output::success("Fetched from remote(s)");
        })
        .ok_or_else(|| Error::VcsPullFailed(stderr_to_string(&output.stderr)))
}

fn build_git_pull_command(cwd: &std::path::Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("pull").current_dir(cwd);
    cmd
}

fn build_jj_pull_commands(cwd: &std::path::Path) -> (Command, Command) {
    let mut cmd1 = Command::new("jj");
    cmd1.args(["git", "fetch"]).current_dir(cwd);
    let mut cmd2 = Command::new("jj");
    cmd2.args(["rebase", "-d", "@-"]).current_dir(cwd);
    (cmd1, cmd2)
}

/// Handle successful command output, printing stdout if non-empty
fn handle_success_output(output: &std::process::Output, success_msg: &str) -> Result<()> {
    output
        .status
        .success()
        .then(|| {
            stdout_if_present(&output.stdout);
            Output::success(success_msg);
        })
        .ok_or_else(|| Error::VcsPullFailed(stderr_to_string(&output.stderr)))
}

fn stdout_if_present(stdout: &[u8]) {
    let trimmed = String::from_utf8_lossy(stdout).trim();
    if !trimmed.is_empty() {
        print!("{}", trimmed);
    }
}

fn stderr_to_string(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr).to_string()
}

/// Execute jj fetch and rebase, handling errors at each step
fn execute_jj_pull(cwd: &std::path::Path) -> Result<()> {
    let (fetch_cmd, rebase_cmd) = build_jj_pull_commands(cwd);

    let fetch_output = fetch_cmd.output().map_err(Error::Io)?;
    if !fetch_output.status.success() {
        return Err(Error::VcsPullFailed(stderr_to_string(&fetch_output.stderr)));
    }

    let rebase_output = rebase_cmd.output().map_err(Error::Io)?;
    rebase_output
        .status
        .success()
        .then(|| Output::success("Pulled and rebased"))
        .ok_or_else(|| Error::VcsRebaseFailed(stderr_to_string(&rebase_output.stderr)))
}

pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => handle_success_output(
            &build_git_pull_command(&cwd).output().map_err(Error::Io)?,
            "Pulled from remote",
        ),
        scp_core::vcs::VcsType::Jujutsu => execute_jj_pull(&cwd),
    }
}

fn build_git_push_command(
    cwd: &std::path::Path,
    remote: &str,
    branch: Option<&str>,
    set_upstream: bool,
    force: bool,
    force_with_lease: bool,
    tags: bool,
    delete: bool,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("push");

    cmd.arg(remote);

    if let Some(b) = branch {
        cmd.arg(b);
    }

    if set_upstream {
        cmd.arg("-u");
    }

    if force {
        cmd.arg("--force");
    } else if force_with_lease {
        cmd.arg("--force-with-lease");
    }

    if tags {
        cmd.arg("--tags");
    }

    if delete {
        cmd.arg("--delete");
    }

    cmd.current_dir(cwd);
    cmd
}

fn build_jj_push_command(
    cwd: &std::path::Path,
    branch: Option<&str>,
    force: bool,
    delete: bool,
) -> Command {
    let mut cmd = Command::new("jj");
    cmd.arg("git");

    if delete {
        cmd.arg("push").arg("--deleted-branch");
        if let Some(b) = branch {
            cmd.arg(b);
        }
    } else {
        cmd.arg("git").arg("push");

        if force {
            cmd.arg("--force-push");
        }

        if let Some(b) = branch {
            cmd.arg("--branch").arg(b);
        }
    }

    cmd.current_dir(cwd);
    cmd
}

/// Handle push command output with remote name in message
fn handle_push_output(output: &std::process::Output, remote: &str) -> Result<()> {
    output
        .status
        .success()
        .then(|| Output::success(&format!("Pushed to {}", remote)))
        .ok_or_else(|| Error::VcsPushFailed(stderr_to_string(&output.stderr)))
}

/// Execute git push command
fn execute_git_push(
    cwd: &std::path::Path,
    remote: &str,
    branch: Option<&str>,
    set_upstream: bool,
    force: bool,
    force_with_lease: bool,
    tags: bool,
    delete: bool,
) -> Result<()> {
    let output = build_git_push_command(
        cwd,
        remote,
        branch,
        set_upstream,
        force,
        force_with_lease,
        tags,
        delete,
    )
    .output()
    .map_err(Error::Io)?;
    handle_push_output(&output, remote)
}

/// Execute jj push command
fn execute_jj_push(
    cwd: &std::path::Path,
    remote: &str,
    branch: Option<&str>,
    force: bool,
    delete: bool,
) -> Result<()> {
    let output = build_jj_push_command(cwd, branch, force, delete)
        .output()
        .map_err(Error::Io)?;
    handle_push_output(&output, remote)
}

pub fn push(
    remote: &str,
    branch: Option<&str>,
    set_upstream: bool,
    force: bool,
    force_with_lease: bool,
    tags: bool,
    delete: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => execute_git_push(
            &cwd,
            remote,
            branch,
            set_upstream,
            force,
            force_with_lease,
            tags,
            delete,
        ),
        scp_core::vcs::VcsType::Jujutsu => execute_jj_push(&cwd, remote, branch, force, delete),
    }
}

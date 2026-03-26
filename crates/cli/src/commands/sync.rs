//! Fetch and sync commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{remote, repository};

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

pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| Error::vcs_pull_failed(e.to_string()))?;
            let _output = remote::fetch(&repo, remote, prune, tags, all)
                .map_err(|e| Error::vcs_pull_failed(e.to_string()))?;
            Output::success("Fetched from remote(s)");
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let output = build_jj_fetch_command(&cwd, remote, all)
                .output()
                .map_err(|e| Error::io_error(e.to_string()))?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().is_empty() {
                    print!("{}", stdout);
                }
                Output::success("Fetched from remote(s)");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::vcs_pull_failed(stderr.to_string()));
            }
        }
    }

    Ok(())
}

fn build_jj_pull_commands(cwd: &std::path::Path) -> (Command, Command) {
    let mut cmd1 = Command::new("jj");
    cmd1.args(["git", "fetch"]).current_dir(cwd);
    let mut cmd2 = Command::new("jj");
    cmd2.args(["rebase", "-d", "@-"]).current_dir(cwd);
    (cmd1, cmd2)
}

pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| Error::vcs_pull_failed(e.to_string()))?;
            let _output = remote::pull(&repo, None, false)
                .map_err(|e| Error::vcs_pull_failed(e.to_string()))?;
            Output::success("Pulled from remote");
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let (mut fetch_cmd, mut rebase_cmd) = build_jj_pull_commands(&cwd);

            let output = fetch_cmd
                .output()
                .map_err(|e| Error::io_error(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::vcs_pull_failed(stderr.to_string()));
            }

            let output = rebase_cmd
                .output()
                .map_err(|e| Error::io_error(e.to_string()))?;

            if output.status.success() {
                Output::success("Pulled and rebased");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::vcs_rebase_failed(stderr.to_string()));
            }
        }
    }

    Ok(())
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

pub fn push(
    remote: &str,
    branch: Option<&str>,
    _set_upstream: bool,
    force: bool,
    _force_with_lease: bool,
    tags: bool,
    delete: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| Error::vcs_push_failed(e.to_string()))?;
            remote::push(&repo, remote, branch, force, tags, delete)
                .map_err(|e| Error::vcs_push_failed(e.to_string()))?;
            Output::success(&format!("Pushed to {}", remote));
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let output = build_jj_push_command(&cwd, branch, force, delete)
                .output()
                .map_err(|e| Error::io_error(e.to_string()))?;

            if output.status.success() {
                Output::success(&format!("Pushed to {}", remote));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::vcs_push_failed(stderr.to_string()));
            }
        }
    }

    Ok(())
}

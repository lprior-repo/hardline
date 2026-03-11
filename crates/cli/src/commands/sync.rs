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

pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    let output = match vcs_type {
        scp_core::vcs::VcsType::Git => build_git_fetch_command(&cwd, remote, prune, tags, all)
            .output()
            .map_err(Error::Io)?,
        scp_core::vcs::VcsType::Jujutsu => build_jj_fetch_command(&cwd, remote, all)
            .output()
            .map_err(Error::Io)?,
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            print!("{}", stdout);
        }
        Output::success("Fetched from remote(s)");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VcsPullFailed(stderr.to_string()));
    }

    Ok(())
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

pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let mut cmd = build_git_pull_command(&cwd);
            let output = cmd.output().map_err(Error::Io)?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().is_empty() {
                    print!("{}", stdout);
                }
                Output::success("Pulled from remote");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPullFailed(stderr.to_string()));
            }
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let (mut fetch_cmd, mut rebase_cmd) = build_jj_pull_commands(&cwd);

            let output = fetch_cmd.output().map_err(Error::Io)?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPullFailed(stderr.to_string()));
            }

            let output = rebase_cmd.output().map_err(Error::Io)?;

            if output.status.success() {
                Output::success("Pulled and rebased");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsRebaseFailed(stderr.to_string()));
            }
        }
    }

    Ok(())
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
        scp_core::vcs::VcsType::Git => {
            let output = build_git_push_command(
                &cwd,
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

            if output.status.success() {
                Output::success(&format!("Pushed to {}", remote));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPushFailed(stderr.to_string()));
            }
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let output = build_jj_push_command(&cwd, branch, force, delete)
                .output()
                .map_err(Error::Io)?;

            if output.status.success() {
                Output::success(&format!("Pushed to {}", remote));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPushFailed(stderr.to_string()));
            }
        }
    }

    Ok(())
}

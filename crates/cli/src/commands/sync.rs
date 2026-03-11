//! Fetch and sync commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};

pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    let output = match vcs_type {
        scp_core::vcs::VcsType::Git => {
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

            cmd.current_dir(&cwd).output().map_err(Error::Io)?
        }
        scp_core::vcs::VcsType::Jujutsu => {
            let mut cmd = Command::new("jj");
            cmd.arg("git");

            if all || remote.is_none() {
                cmd.arg("fetch");
            } else if let Some(r) = remote {
                cmd.arg("fetch").arg("--remote").arg(r);
            } else {
                cmd.arg("fetch");
            }

            cmd.current_dir(&cwd).output().map_err(Error::Io)?
        }
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

pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let output = Command::new("git")
                .arg("pull")
                .current_dir(&cwd)
                .output()
                .map_err(Error::Io)?;

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
            let output = Command::new("jj")
                .args(["git", "fetch"])
                .current_dir(&cwd)
                .output()
                .map_err(Error::Io)?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPullFailed(stderr.to_string()));
            }

            let output = Command::new("jj")
                .args(["rebase", "-d", "@-"])
                .current_dir(&cwd)
                .output()
                .map_err(Error::Io)?;

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

            let output = cmd.current_dir(&cwd).output().map_err(Error::Io)?;

            if output.status.success() {
                Output::success(&format!("Pushed to {}", remote));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VcsPushFailed(stderr.to_string()));
            }
        }
        scp_core::vcs::VcsType::Jujutsu => {
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

            let output = cmd.current_dir(&cwd).output().map_err(Error::Io)?;

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

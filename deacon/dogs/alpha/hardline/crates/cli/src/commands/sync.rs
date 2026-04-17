//! Fetch and sync commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{remote, repository};

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
    }

    Ok(())
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
    }

    Ok(())
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
    }

    Ok(())
}

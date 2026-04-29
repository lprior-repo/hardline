//! Fetch and sync commands (ported from stak CLI)

use std::process::Command;

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{remote, repository};

/// Fetch from one or all remotes.
///
/// Optionally prunes stale remote-tracking branches and fetches tags.
pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

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

/// Pull from the default remote, fast-forwarding the current branch.
pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

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

/// Arguments for [`push`].
pub struct PushArgs<'a> {
    pub remote: &'a str,
    pub branch: Option<&'a str>,
    pub set_upstream: bool,
    pub force: bool,
    pub force_with_lease: bool,
    pub tags: bool,
    pub delete: bool,
}

/// Push to a remote, optionally forcing, pushing tags, or deleting a remote branch.
pub fn push(args: PushArgs<'_>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = repository::open(&cwd).map_err(|e| Error::vcs_push_failed(e.to_string()))?;
            remote::push(&repo, args.remote, args.branch, args.force, args.tags, args.delete)
                .map_err(|e| Error::vcs_push_failed(e.to_string()))?;
            Output::success(&format!("Pushed to {}", args.remote));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_accepts_all_none_params() {
        let _fn: fn(Option<&str>, bool, bool, bool) -> Result<()> = fetch;
    }

    #[test]
    fn pull_has_no_params() {
        let _fn: fn() -> Result<()> = pull;
    }

    #[test]
    fn push_accepts_all_params() {
        let _fn: fn(PushArgs<'_>) -> Result<()> = push;
    }

    #[test]
    fn fetch_in_non_vcs_dir_fails() {
        // Skip if cwd doesn't exist (can happen in parallel test env)
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = fetch(None, false, false, false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn pull_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = pull();

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn push_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = push(PushArgs {
            remote: "origin",
            branch: None,
            set_upstream: false,
            force: false,
            force_with_lease: false,
            tags: false,
            delete: false,
        });

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn vcs_type_git_exists() {
        let _ = scp_core::vcs::VcsType::Git;
    }

    #[test]
    fn error_constructors_used_in_sync() {
        let _ = Error::io_error("test");
        let _ = Error::vcs_not_initialized();
        let _ = Error::vcs_pull_failed("test");
        let _ = Error::vcs_push_failed("test");
    }
}

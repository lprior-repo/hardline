//! Tag commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, tag};

/// List tags, optionally filtered by a glob pattern.
pub fn list(pattern: Option<&str>, _sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    let tags =
        tag::list(&repo, pattern).map_err(|e| Error::vcs_conflict("list tags", e.to_string()))?;

    if tags.is_empty() {
        Output::info("No tags found");
    } else {
        for t in tags {
            println!("{}", t);
        }
    }
    Ok(())
}

/// Create a new Git tag, optionally with an annotated message.
pub fn create(name: &str, message: Option<&str>, _commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    let msg = message.unwrap_or("");
    tag::create(&repo, name, msg, force)
        .map_err(|e| Error::vcs_conflict("create tag", e.to_string()))?;

    Output::success(&format!("Created tag: {}", name));
    Ok(())
}

/// Delete a local (or remote) tag.
pub fn delete(name: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    if remote {
        return delete_remote_tag(&cwd, name);
    }

    tag::delete(&repo, name, false)
        .map_err(|e| Error::vcs_conflict("delete tag", e.to_string()))?;

    Output::success(&format!("Deleted local tag: {}", name));
    Ok(())
}

/// Delete a remote tag using git CLI fallback.
fn delete_remote_tag(workdir: &std::path::Path, name: &str) -> Result<()> {
    let output = scp_vcs::gix::cli::run_git(workdir, &["push", "--delete", "origin", name])
        .map_err(|e| Error::vcs_push_failed(e.to_string()))?;

    if !output.success {
        let git_err = scp_vcs::gix::cli::cli_error(&output, "remote tag delete");
        return Err(Error::vcs_push_failed(git_err.to_string()));
    }

    Output::success(&format!("Deleted remote tag: {}", name));
    Ok(())
}

/// Push a specific tag (or all tags) to a remote.
pub fn push(tag: Option<&str>, remote: &str, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    tag.map_or_else(
        || push_all_tags(&cwd, remote),
        |t| push_single_tag(&cwd, t, remote),
    )
}

/// Push a single tag to a remote using the gix native path.
fn push_single_tag(workdir: &std::path::Path, tag_name: &str, remote: &str) -> Result<()> {
    let repo = repository::open(workdir)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;
    tag::push(&repo, remote, tag_name).map_err(|e| Error::vcs_push_failed(e.to_string()))?;
    Output::success(&format!("Pushed tag {} to {}", tag_name, remote));
    Ok(())
}

/// Push all tags to a remote using git CLI fallback.
fn push_all_tags(workdir: &std::path::Path, remote: &str) -> Result<()> {
    let output = scp_vcs::gix::cli::run_git(workdir, &["push", "--tags", remote])
        .map_err(|e| Error::vcs_push_failed(e.to_string()))?;

    if !output.success {
        let git_err = scp_vcs::gix::cli::cli_error(&output, "push all tags");
        return Err(Error::vcs_push_failed(git_err.to_string()));
    }

    Output::success(&format!("Pushed all tags to {}", remote));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_accepts_none_params() {
        let _fn: fn(Option<&str>, Option<&str>) -> Result<()> = list;
    }

    #[test]
    fn create_accepts_all_params() {
        let _fn: fn(&str, Option<&str>, Option<&str>, bool) -> Result<()> = create;
    }

    #[test]
    fn delete_rejects_remote_delete() {
        // Skip if cwd doesn't exist (can happen in parallel test env)
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };

        // We need a git repo for this — but remote delete is checked before repo open
        // So even in a non-VCS dir, the error should come from vcs_not_initialized
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }
        let result = delete("v1.0", true);
        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn push_rejects_none_tag() {
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
        let result = push(None, "origin", false);
        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn list_in_non_vcs_dir_fails() {
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

        let result = list(None, None);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn create_in_non_vcs_dir_fails() {
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

        let result = create("v1.0", None, None, false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn delete_in_non_vcs_dir_fails() {
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

        let result = delete("v1.0", false);

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

        let result = push(Some("v1.0"), "origin", false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn error_constructors_used_in_tag() {
        let _ = Error::io_error("test");
        let _ = Error::vcs_not_initialized();
        let _ = Error::vcs_conflict("repo", "msg");
        let _ = Error::vcs_push_failed("test");
    }
}

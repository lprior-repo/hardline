//! Tag commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, tag};

pub fn list(pattern: Option<&str>, _sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

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

pub fn create(name: &str, message: Option<&str>, _commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    let msg = message.unwrap_or("");
    tag::create(&repo, name, msg, force)
        .map_err(|e| Error::vcs_conflict("create tag", e.to_string()))?;

    Output::success(&format!("Created tag: {}", name));
    Ok(())
}

pub fn delete(name: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    if remote {
        return Err(Error::vcs_conflict(
            "Remote tag delete not yet implemented",
            "remote".to_string(),
        ));
    }

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    tag::delete(&repo, name, false)
        .map_err(|e| Error::vcs_conflict("delete tag", e.to_string()))?;

    Output::success(&format!("Deleted local tag: {}", name));
    Ok(())
}

pub fn push(tag: Option<&str>, remote: &str, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or(Error::vcs_not_initialized())?;

    if tag.is_none() {
        return Err(Error::vcs_conflict(
            "Push all tags not yet implemented",
            "all tags".to_string(),
        ));
    }

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    let t = tag.unwrap();
    tag::push(&repo, remote, t).map_err(|e| Error::vcs_push_failed(e.to_string()))?;
    Output::success(&format!("Pushed tag {} to {}", t, remote));
    Ok(())
}

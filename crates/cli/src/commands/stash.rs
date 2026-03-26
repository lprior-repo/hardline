//! Stash commands (ported from stak CLI)

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, stash};

pub fn save(message: Option<&str>, include_untracked: bool, _patch: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::invalid_state(
            "stash is only supported for Git repositories",
        ));
    }

    let repo =
        repository::open(&cwd).map_err(|e| Error::vcs_conflict("git stash", e.to_string()))?;

    stash::save(&repo, message, include_untracked)
        .map_err(|e| Error::vcs_conflict("git stash", e.to_string()))?;

    let msg = message.unwrap_or("changes");
    Output::success(&format!("Stashed: {}", msg));

    Ok(())
}

pub fn pop(stash_ref: Option<&str>, _restore_index: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::invalid_state(
            "stash is only supported for Git repositories",
        ));
    }

    let repo =
        repository::open(&cwd).map_err(|e| Error::vcs_conflict("git stash pop", e.to_string()))?;

    let index = stash_ref
        .and_then(|s| s.strip_prefix("stash@{").and_then(|s| s.strip_suffix('}')))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    stash::pop(&repo, index).map_err(|e| Error::vcs_conflict("git stash pop", e.to_string()))?;

    Output::success("Applied stash and removed from stash list");

    Ok(())
}

pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::invalid_state(
            "stash is only supported for Git repositories",
        ));
    }

    let repo =
        repository::open(&cwd).map_err(|e| Error::vcs_conflict("git stash list", e.to_string()))?;

    let entries =
        stash::list(&repo).map_err(|e| Error::vcs_conflict("git stash list", e.to_string()))?;

    if entries.is_empty() {
        Output::info("No stashed changes");
    } else {
        for entry in entries {
            println!("{}: {}", entry.index, entry.message);
        }
    }

    Ok(())
}

pub fn drop(stash_ref: &str, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::invalid_state(
            "stash is only supported for Git repositories",
        ));
    }

    let repo =
        repository::open(&cwd).map_err(|e| Error::vcs_conflict("git stash drop", e.to_string()))?;

    let index = stash_ref
        .strip_prefix("stash@{")
        .and_then(|s| s.strip_suffix('}'))
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| Error::vcs_conflict("git stash drop", "Invalid stash reference"))?;

    stash::drop(&repo, index).map_err(|e| Error::vcs_conflict("git stash drop", e.to_string()))?;

    Output::success(&format!("Dropped stash: {}", stash_ref));

    Ok(())
}

pub fn show(stash_ref: Option<&str>, _stat: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    let vcs_type = detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if vcs_type != scp_core::vcs::VcsType::Git {
        return Err(Error::invalid_state(
            "stash is only supported for Git repositories",
        ));
    }

    let repo =
        repository::open(&cwd).map_err(|e| Error::vcs_conflict("git stash show", e.to_string()))?;

    let index = stash_ref
        .and_then(|s| s.strip_prefix("stash@{").and_then(|s| s.strip_suffix('}')))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let output = stash::show(&repo, index)
        .map_err(|e| Error::vcs_conflict("git stash show", e.to_string()))?;

    print!("{}", output);

    Ok(())
}

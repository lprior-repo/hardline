//! Stash commands (ported from stak CLI)

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, stash};

/// Save current changes to the stash.
///
/// Stashes both tracked and (optionally) untracked files so the working
/// directory can be restored to a clean state.
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

/// Apply the most recent (or specified) stash and remove it from the stash list.
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

/// List all stashed changes with their index and message.
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

/// Remove a stash entry by its reference (e.g., `stash@{0}`).
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

/// Show the diff contents of a stash entry.
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

/// Parse a stash reference string (e.g., "stash@{0}") into a numeric index.
///
/// Returns `Some(index)` for valid stash references, `None` otherwise.
/// Pure function extracted for testability.
fn parse_stash_ref(stash_ref: &str) -> Option<usize> {
    stash_ref
        .strip_prefix("stash@{")
        .and_then(|s| s.strip_suffix('}'))
        .and_then(|s| s.parse::<usize>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stash_ref_zero() {
        assert_eq!(parse_stash_ref("stash@{0}"), Some(0));
    }

    #[test]
    fn parse_stash_ref_one() {
        assert_eq!(parse_stash_ref("stash@{1}"), Some(1));
    }

    #[test]
    fn parse_stash_ref_large_number() {
        assert_eq!(parse_stash_ref("stash@{999}"), Some(999));
    }

    #[test]
    fn parse_stash_ref_missing_braces() {
        assert_eq!(parse_stash_ref("stash@0"), None);
    }

    #[test]
    fn parse_stash_ref_missing_prefix() {
        assert_eq!(parse_stash_ref("0"), None);
    }

    #[test]
    fn parse_stash_ref_empty_string() {
        assert_eq!(parse_stash_ref(""), None);
    }

    #[test]
    fn parse_stash_ref_non_numeric() {
        assert_eq!(parse_stash_ref("stash@{abc}"), None);
    }

    #[test]
    fn parse_stash_ref_negative_number() {
        assert_eq!(parse_stash_ref("stash@{-1}"), None);
    }

    #[test]
    fn parse_stash_ref_with_spaces() {
        assert_eq!(parse_stash_ref("stash@{ 0 }"), None);
    }

    #[test]
    fn parse_stash_ref_missing_closing_brace() {
        assert_eq!(parse_stash_ref("stash@{0"), None);
    }

    #[test]
    fn parse_stash_ref_missing_opening_brace() {
        assert_eq!(parse_stash_ref("stash@0}"), None);
    }

    #[test]
    fn parse_stash_ref_float() {
        assert_eq!(parse_stash_ref("stash@{1.5}"), None);
    }

    #[test]
    fn parse_stash_ref_leading_zeros() {
        assert_eq!(parse_stash_ref("stash@{007}"), Some(7));
    }

    #[test]
    fn parse_stash_ref_just_braces() {
        assert_eq!(parse_stash_ref("stash@{}"), None);
    }

    #[test]
    fn parse_stash_ref_garbage() {
        assert_eq!(parse_stash_ref("totally wrong"), None);
    }

    #[test]
    fn parse_stash_ref_almost_valid() {
        assert_eq!(parse_stash_ref("stash{0}"), None);
    }
}

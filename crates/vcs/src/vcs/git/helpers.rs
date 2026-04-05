//! Git helper functions
//!
//! Provides:
//! - Version parsing helpers
//! - `resolve_ref` — resolve a reference name to a CommitId
//! - `is_ancestor` — check if one commit is ancestor of another
//! - `GitError` → `VcsError` conversion bridge
//!
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::vcs::{CommitId, VcsError};

/// Parse Git version from output like "git version 2.43.0"
pub fn parse_git_version(output: &str) -> Result<(u32, u32), VcsError> {
    let output = output.trim();

    let parts: Vec<&str> = output.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(VcsError::GitParseError(format!(
            "Unexpected git version format: {output}"
        )));
    }

    let version_str = parts[2];

    let version_parts: Vec<&str> = version_str.split('.').collect();

    if version_parts.len() < 2 {
        return Err(VcsError::GitParseError(format!(
            "Invalid version number: {version_str}"
        )));
    }

    let major = version_parts[0].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid major version: {}", version_parts[0]))
    })?;

    let minor = version_parts[1].parse::<u32>().map_err(|_| {
        VcsError::GitParseError(format!("Invalid minor version: {}", version_parts[1]))
    })?;

    Ok((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_version() {
        assert_eq!(parse_git_version("git version 2.43.0").expect("parse"), (2, 43));
    }

    #[test]
    fn parse_version_with_patch() {
        assert_eq!(parse_git_version("git version 2.38.4").expect("parse"), (2, 38));
    }

    #[test]
    fn parse_version_windows_suffix() {
        assert_eq!(parse_git_version("git version 2.43.0.windows.1").expect("parse"), (2, 43));
    }

    #[test]
    fn parse_version_apple_suffix() {
        assert_eq!(parse_git_version("git version 2.39.3 (Apple Git-146)").expect("parse"), (2, 39));
    }

    #[test]
    fn parse_version_minimal() {
        assert_eq!(parse_git_version("git version 1.0").expect("parse"), (1, 0));
    }

    #[test]
    fn parse_version_major_zero() {
        assert_eq!(parse_git_version("git version 0.99").expect("parse"), (0, 99));
    }

    #[test]
    fn parse_version_large_numbers() {
        assert_eq!(parse_git_version("git version 100.200").expect("parse"), (100, 200));
    }

    #[test]
    fn parse_error_empty_input() {
        assert!(matches!(parse_git_version(""), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_single_word() {
        assert!(matches!(parse_git_version("git"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_two_words() {
        assert!(matches!(parse_git_version("git version"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_no_dots() {
        assert!(matches!(parse_git_version("git version abc"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_invalid_major() {
        assert!(matches!(parse_git_version("git version abc.0"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_error_invalid_minor() {
        assert!(matches!(parse_git_version("git version 2.xyz"), Err(VcsError::GitParseError(_))));
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(parse_git_version("  git version 2.40.0  ").expect("parse"), (2, 40));
    }

    #[test]
    fn parse_extra_whitespace_between_parts() {
        assert_eq!(parse_git_version("git  version  2.41.0").expect("parse"), (2, 41));
    }
}

// ============================================================================
// Reference Resolution
// ============================================================================

/// Resolve a reference name (branch, tag, HEAD, etc.) to a `CommitId`.
///
/// # Errors
/// - `VcsError::NotFound` if the reference does not point to a commit
/// - `VcsError::GitReferenceError` if resolution fails for other reasons
pub fn resolve_ref(repo: &gix::Repository, ref_name: &str) -> Result<CommitId, VcsError> {
    match repo.rev_parse(ref_name) {
        Ok(obj) => {
            if obj.kind == gix::object::Kind::Commit {
                CommitId::new(obj.to_string()).map_err(|_| {
                    VcsError::GitReferenceError(format!(
                        "Invalid commit ID from ref '{ref_name}'"
                    ))
                })
            } else {
                Err(VcsError::NotFound {
                    entity: "Commit",
                    id: ref_name.to_string(),
                })
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("ambiguous") || msg.contains("invalid") {
                Err(VcsError::NotFound {
                    entity: "Reference",
                    id: ref_name.to_string(),
                })
            } else {
                Err(VcsError::GitReferenceError(format!(
                    "Failed to resolve ref '{ref_name}': {e}"
                )))
            }
        }
    }
}

// ============================================================================
// Ancestry Check
// ============================================================================

/// Check if `ancestor` is an ancestor of `descendant`.
///
/// Walks the commit graph from `descendant` (all parents, not first-parent-only)
/// looking for `ancestor`. Returns `Ok(false)` if either commit cannot be resolved.
///
/// # Errors
/// - `VcsError::GitReferenceError` if the graph walk fails
pub fn is_ancestor(
    repo: &gix::Repository,
    ancestor: &CommitId,
    descendant: &CommitId,
) -> Result<bool, VcsError> {
    let ancestor_id = match repo.rev_parse(ancestor.as_str()) {
        Ok(obj) => obj.detach(),
        Err(_) => return Ok(false),
    };

    let descendant_id = match repo.rev_parse(descendant.as_str()) {
        Ok(obj) => obj.detach(),
        Err(_) => return Ok(false),
    };

    let walk = repo
        .rev_walk(Some(descendant_id))
        .all()
        .map_err(|e| VcsError::GitReferenceError(format!("Failed to walk commit graph: {e}")))?;

    for item in walk {
        let item = item.map_err(|e| {
            VcsError::GitReferenceError(format!("Failed to read commit during walk: {e}"))
        })?;
        if item.id == ancestor_id {
            return Ok(true);
        }
    }

    Ok(false)
}

// ============================================================================
// Error Conversion Bridge
// ============================================================================

/// Convert `crate::error::GitError` to `crate::vcs::VcsError`
///
/// This bridges the gix module's error type to the vcs module's error type,
/// enabling delegation from `VcsBackend` trait methods to `crate::gix` functions.
impl From<crate::error::GitError> for VcsError {
    fn from(err: crate::error::GitError) -> Self {
        match err {
            crate::error::GitError::NotFound(path) => VcsError::NoVcsFound(path),
            crate::error::GitError::InvalidRef { name, reason: _ } => {
                VcsError::GitReferenceError(name)
            }
            crate::error::GitError::Io(io) => VcsError::CommandFailed {
                message: io.to_string(),
                source: Some(io),
            },
            crate::error::GitError::Gix(e)
            | crate::error::GitError::GixDiscover(e)
            | crate::error::GitError::GixInit(e) => {
                VcsError::GitReferenceError(e.to_string())
            }
            crate::error::GitError::GixStatus(e) | crate::error::GitError::GixStatusIter(e) => {
                VcsError::GitReferenceError(e.to_string())
            }
            crate::error::GitError::Conflict { message, .. } => {
                VcsError::GitReferenceError(message)
            }
            crate::error::GitError::Unauthorized(msg) | crate::error::GitError::Network(msg) => {
                VcsError::GitReferenceError(msg)
            }
        }
    }
}

// ============================================================================
// Gix Module Delegation Helpers
// ============================================================================

/// Acquire the Mutex lock on a gix::Repository.
///
/// Centralizes the lock acquisition error handling used by all trait methods.
pub(crate) fn lock_repo(repo: &std::sync::Mutex<gix::Repository>) -> Result<std::sync::MutexGuard<'_, gix::Repository>, VcsError> {
    repo.lock().map_err(|_| {
        VcsError::GitReferenceError("Failed to acquire repository lock".to_string())
    })
}

/// Get current branch via the `crate::gix` module, with detached-HEAD → None mapping.
///
/// The `gix::branch::current()` function returns an error for detached HEAD,
/// but the `VcsBackend` trait expects `Ok(None)` in that case.
pub fn current_branch_via_gix(repo: &gix::Repository) -> Result<Option<crate::vcs::BranchName>, VcsError> {
    match crate::gix::branch::current(repo) {
        Ok(name) => {
            let branch = crate::vcs::BranchName::new(name).map_err(|_| {
                VcsError::GitReferenceError("Invalid branch name from gix".to_string())
            })?;
            Ok(Some(branch))
        }
        Err(
            e @ crate::error::GitError::InvalidRef { .. },
        ) => {
            let msg = e.to_string();
            if msg.contains("Detached HEAD") || msg.contains("detached") {
                Ok(None)
            } else {
                Err(VcsError::from(e))
            }
        }
        Err(e) => Err(VcsError::from(e)),
    }
}

/// List local branches via the `crate::gix` module, converting to `BranchName`.
pub fn list_branches_via_gix(repo: &gix::Repository) -> Result<Vec<crate::vcs::BranchName>, VcsError> {
    let branches = crate::gix::branch::list(repo, false).map_err(VcsError::from)?;
    let mut names: Vec<crate::vcs::BranchName> = branches
        .into_iter()
        .filter_map(|b| crate::vcs::BranchName::new(b.name).ok())
        .collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(names)
}

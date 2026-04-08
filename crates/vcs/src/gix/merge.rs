//! Gitoxide Merge and Ancestor Operations
//!
//! Port of stax/src/git/repo.rs merge/ancestor-related operations.
//! Pure gitoxide implementation — no CLI spawning.

use crate::error::{GitError, GitResult};

/// Find merge-base commit between any two refs (branch names, remote refs, or SHAs).
///
/// Returns the OID of the merge base as a string.
pub fn merge_base_refs(repo: &gix::Repository, left: &str, right: &str) -> GitResult<String> {
    let left_oid = super::commit::resolve_to_oid_export(repo, left)?;
    let right_oid = super::commit::resolve_to_oid_export(repo, right)?;

    let base = repo
        .merge_base(left_oid, right_oid)
        .map_err(|e| GitError::InvalidRef {
            name: format!("{left}..{right}"),
            reason: format!("Failed to find merge base: {e}"),
        })?;

    Ok(base.detach().to_string())
}

/// Check whether `ancestor` is an ancestor of `descendant`.
///
/// Returns `Ok(true)` if `ancestor` is a direct ancestor of `descendant`,
/// or if they point to the same commit.
///
/// Uses merge-base check: if the merge base of both equals `ancestor`,
/// then `ancestor` is an ancestor of `descendant`.
pub fn is_ancestor(repo: &gix::Repository, ancestor: &str, descendant: &str) -> GitResult<bool> {
    let ancestor_oid = super::commit::resolve_to_oid_export(repo, ancestor)?;
    let descendant_oid = super::commit::resolve_to_oid_export(repo, descendant)?;

    // Same commit → trivially ancestor
    if ancestor_oid == descendant_oid {
        return Ok(true);
    }

    // merge_base(ancestor, descendant) == ancestor → ancestor is ancestor of descendant
    match repo.merge_base(ancestor_oid, descendant_oid) {
        Ok(base) => Ok(base.detach() == ancestor_oid),
        Err(_) => Ok(false),
    }
}

/// Check if a branch is merged into another branch (ancestor check).
///
/// A branch is merged if its tip commit is an ancestor of the target branch.
pub fn is_branch_merged(
    repo: &gix::Repository,
    branch: &str,
    into_branch: &str,
) -> GitResult<bool> {
    let branch_oid = super::commit::resolve_to_oid_export(repo, branch)?;
    let into_oid = super::commit::resolve_to_oid_export(repo, into_branch)?;

    // If the branch tip IS the merge base with the target, it's merged
    match repo.merge_base(into_oid, branch_oid) {
        Ok(base) => Ok(base.detach() == branch_oid),
        Err(_) => Ok(false),
    }
}

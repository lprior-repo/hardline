//! Gitoxide Merge Detection and Patch-Id Analysis
//!
//! Higher-level merge detection built on the core merge-base module.
//! Provides patch-id computation, branch patch-id collection, and
//! already-merged detection via patch-id comparison.

use crate::error::{GitError, GitResult};
use crate::gix::merge::{
    collect_commit_oids, compute_patch_id, find_merge_base, is_ancestor, PatchId,
};
use std::collections::HashSet;

/// Check whether a branch is fully merged into a target branch.
///
/// A branch is merged if its tip commit is reachable from the target branch's
/// tip. This uses `is_ancestor` under the hood.
///
/// # Parameters
/// - `repo`: The git repository
/// - `branch`: The branch to check (e.g., "feature")
/// - `into_branch`: The target branch (e.g., "main")
///
/// # Returns
/// - `Ok(true)` if `branch` tip is an ancestor of `into_branch` tip
/// - `Ok(false)` if `branch` has commits not in `into_branch`
/// - `Err` if either branch cannot be resolved
pub fn is_branch_merged(
    repo: &gix::Repository,
    branch: &str,
    into_branch: &str,
) -> GitResult<bool> {
    let branch_tip = resolve_branch_tip(repo, branch)?;
    let into_tip = resolve_branch_tip(repo, into_branch)?;

    is_ancestor(repo, branch_tip, into_tip)
}

/// Compute patch-ids for all commits from `branch_tip` to `merge_base`.
///
/// Returns a set of patch-ids that uniquely identify the diff content of
/// each commit on the branch. Used for cherry-pick detection.
pub fn branch_patch_ids(
    repo: &gix::Repository,
    branch_tip: gix::ObjectId,
    merge_base: gix::ObjectId,
) -> GitResult<HashSet<PatchId>> {
    let oids = collect_commit_oids(repo, branch_tip, merge_base)?;
    let mut ids = HashSet::new();

    for oid in oids {
        if let Some(pid) = compute_patch_id(repo, oid)? {
            ids.insert(pid);
        }
    }

    Ok(ids)
}

/// Detect which commits on `branch` have already been merged into
/// `into_branch` by comparing patch-ids.
///
/// Returns the list of commit OIDs from `branch` whose diffs are already
/// present in `into_branch` (i.e., they were cherry-picked).
pub fn find_already_merged(
    repo: &gix::Repository,
    branch: &str,
    into_branch: &str,
) -> GitResult<Vec<gix::ObjectId>> {
    let branch_tip = resolve_branch_tip(repo, branch)?;
    let into_tip = resolve_branch_tip(repo, into_branch)?;

    let base = find_merge_base(repo, branch_tip, into_tip)?;
    let base = match base {
        Some(b) => b,
        None => return Ok(Vec::new()),
    };

    let into_oids = collect_commit_oids(repo, into_tip, base)?;
    let mut into_patch_ids = HashSet::new();
    for oid in &into_oids {
        if let Some(pid) = compute_patch_id(repo, *oid)? {
            into_patch_ids.insert(pid);
        }
    }

    let branch_oids = collect_commit_oids(repo, branch_tip, base)?;
    let mut already_merged = Vec::new();

    for oid in &branch_oids {
        if let Some(pid) = compute_patch_id(repo, *oid)? {
            if into_patch_ids.contains(&pid) {
                already_merged.push(*oid);
            }
        }
    }

    Ok(already_merged)
}

/// Resolve a branch name to its tip commit OID.
fn resolve_branch_tip(repo: &gix::Repository, branch: &str) -> GitResult<gix::ObjectId> {
    let ref_name = format!("refs/heads/{branch}");
    let reference = repo
        .find_reference(&ref_name)
        .map_err(|e| GitError::InvalidRef {
            name: branch.to_string(),
            reason: format!("Branch not found: {e}"),
        })?;

    let oid = reference
        .try_id()
        .ok_or_else(|| GitError::InvalidRef {
            name: branch.to_string(),
            reason: "Reference does not point to an object".to_string(),
        })?
        .detach();

    Ok(oid)
}

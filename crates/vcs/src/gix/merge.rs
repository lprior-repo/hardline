//! Gitoxide Merge-Base and Merge Detection Operations
//!
//! Pure gitoxide implementation for merge-base computation, branch merge
//! detection, and patch-id analysis. No CLI spawning.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: `MergeBaseInfo`, `PatchId` — pure value types
//! - **Calc**: `find_merge_base`, `is_ancestor`, `collect_commit_oids`,
//!   `compute_patch_id`, `is_branch_merged` — pure computation
//! - **Actions**: None (callers perform side effects)

use crate::error::{GitError, GitResult};
use gix::prelude::ObjectIdExt;
use sha1::{Digest, Sha1};

// Re-export merge_detect functions for backward compatibility
pub use crate::gix::merge_detect::{branch_patch_ids, find_already_merged, is_branch_merged};

// ============================================================================
// Types (Data layer)
// ============================================================================

/// Result of a merge-base query between two commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeBaseInfo {
    /// The merge-base commit OID (common ancestor)
    pub oid: gix::ObjectId,
    /// Distance in commits from `a` to the merge base
    pub distance_a: usize,
    /// Distance in commits from `b` to the merge base
    pub distance_b: usize,
}

/// A patch-id representing the semantic content of a diff.
///
/// Two commits with the same patch-id produce identical diffs — useful for
/// detecting cherry-picks and already-merged work.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatchId {
    /// The SHA-1 hash of the commit's diff content
    pub hash: gix::ObjectId,
}

// ============================================================================
// Core Operations (Calc layer)
// ============================================================================

/// Find the merge base of two commits.
///
/// Returns `Ok(Some(oid))` when a common ancestor exists, `Ok(None)` when
/// the two commits share no history (disconnected DAGs).
///
/// This reuses the same `repo.merge_base()` call that the rebase module uses,
/// but provides a standalone entry-point for callers that only need the base.
pub fn find_merge_base(
    repo: &gix::Repository,
    a: gix::ObjectId,
    b: gix::ObjectId,
) -> GitResult<Option<gix::ObjectId>> {
    match repo.merge_base(a, b) {
        Ok(id) => Ok(Some(id.detach())),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("no merge base")
                || msg.contains("not found")
                || msg.contains("could not find a merge-base")
            {
                Ok(None)
            } else {
                Err(GitError::InvalidRef {
                    name: "merge_base".to_string(),
                    reason: format!("Failed to find merge base: {e}"),
                })
            }
        }
    }
}

/// Find the merge base and compute distances from each commit to the base.
///
/// Useful for understanding how far two branches have diverged.
pub fn find_merge_base_info(
    repo: &gix::Repository,
    a: gix::ObjectId,
    b: gix::ObjectId,
) -> GitResult<Option<MergeBaseInfo>> {
    let base = find_merge_base(repo, a, b)?;
    let base = match base {
        Some(b) => b,
        None => return Ok(None),
    };

    let distance_a = count_ancestors_to(repo, a, base)?;
    let distance_b = count_ancestors_to(repo, b, base)?;

    Ok(Some(MergeBaseInfo {
        oid: base,
        distance_a,
        distance_b,
    }))
}

/// Check whether `maybe_ancestor` is an ancestor of `descendant`.
///
/// Returns `Ok(true)` if `maybe_ancestor` is reachable from `descendant`
/// through the commit graph (including the trivial case where they are equal).
pub fn is_ancestor(
    repo: &gix::Repository,
    maybe_ancestor: gix::ObjectId,
    descendant: gix::ObjectId,
) -> GitResult<bool> {
    if maybe_ancestor == descendant {
        return Ok(true);
    }

    let base = find_merge_base(repo, maybe_ancestor, descendant)?;
    Ok(base == Some(maybe_ancestor))
}

/// Compute a patch-id for a commit by hashing its tree diff against its parent.
///
/// The patch-id is derived from the tree change (parent-tree-id + commit-tree-id),
/// providing a stable fingerprint for cherry-pick detection. Two commits that
/// produce the same tree change (same parent tree, same commit tree) will have
/// the same patch-id.
///
/// For merge commits (multiple parents), the first parent is used as the diff
/// base (matching `git patch-id` behavior).
///
/// Returns `Ok(None)` for root commits (no parent to diff against).
pub fn compute_patch_id(
    repo: &gix::Repository,
    commit_oid: gix::ObjectId,
) -> GitResult<Option<PatchId>> {
    let commit = commit_oid
        .attach(repo)
        .object()
        .map_err(|e| GitError::InvalidRef {
            name: commit_oid.to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?
        .peel_to_commit()
        .map_err(|e| GitError::InvalidRef {
            name: commit_oid.to_string(),
            reason: format!("Not a commit: {e}"),
        })?;

    // Root commits have no parent → no diff → no patch-id
    let first_parent = match commit.parent_ids().next() {
        Some(p) => p.detach(),
        None => return Ok(None),
    };

    let parent_tree_id = first_parent
        .attach(repo)
        .object()
        .map_err(|e| GitError::InvalidRef {
            name: first_parent.to_string(),
            reason: format!("Failed to read parent: {e}"),
        })?
        .peel_to_commit()
        .map_err(|e| GitError::InvalidRef {
            name: first_parent.to_string(),
            reason: format!("Parent is not a commit: {e}"),
        })?
        .tree_id()
        .map_err(|e| GitError::InvalidRef {
            name: first_parent.to_string(),
            reason: format!("Failed to get tree: {e}"),
        })?
        .detach();

    let commit_tree_id = commit
        .tree_id()
        .map_err(|e| GitError::InvalidRef {
            name: commit_oid.to_string(),
            reason: format!("Failed to get tree: {e}"),
        })?
        .detach();

    // Hash the tree pair: parent_tree_id || commit_tree_id
    let hash = hash_tree_pair(parent_tree_id, commit_tree_id);

    Ok(Some(PatchId { hash }))
}

/// Collect commit OIDs from `tip` back to `base` (exclusive).
///
/// Uses first-parent walk. Returns commits in chronological order (oldest first).
pub fn collect_commit_oids(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    base: gix::ObjectId,
) -> GitResult<Vec<gix::ObjectId>> {
    let walk = repo
        .rev_walk(Some(tip))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: format!("Failed to walk commits: {e}"),
        })?;

    let mut oids = Vec::new();

    for item in walk {
        let info = item.map_err(|e| GitError::InvalidRef {
            name: "walk".to_string(),
            reason: format!("Failed to read commit during walk: {e}"),
        })?;

        if info.id == base {
            break;
        }

        oids.push(info.id);
    }

    oids.reverse();
    Ok(oids)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Count commits from `tip` back to (but not including) `base`.
fn count_ancestors_to(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    base: gix::ObjectId,
) -> GitResult<usize> {
    let walk = repo
        .rev_walk(Some(tip))
        .first_parent_only()
        .all()
        .map_err(|e| GitError::InvalidRef {
            name: "rev_walk".to_string(),
            reason: format!("Failed to walk commits: {e}"),
        })?;

    let mut count = 0;
    for item in walk {
        let info = item.map_err(|e| GitError::InvalidRef {
            name: "walk".to_string(),
            reason: format!("Failed to read commit: {e}"),
        })?;
        if info.id == base {
            break;
        }
        count += 1;
    }

    Ok(count)
}

/// Compute a deterministic hash from a pair of tree OIDs.
///
/// Hashes the concatenation of `parent_tree || commit_tree` to produce
/// a patch-id. Two commits that introduce the same tree change
/// (same parent tree, same resulting tree) will produce the same patch-id.
fn hash_tree_pair(parent_tree: gix::ObjectId, commit_tree: gix::ObjectId) -> gix::ObjectId {
    use std::io::Write;

    let mut hasher = Sha1::new();
    let _ = hasher.write(parent_tree.as_bytes());
    let _ = hasher.write(commit_tree.as_bytes());
    let result = hasher.finalize();

    // Convert 20-byte SHA-1 digest to gix::ObjectId
    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(&result);
    gix::ObjectId::from(bytes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn merge_base_info_debug_format() {
        let info = MergeBaseInfo {
            oid: "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap(),
            distance_a: 3,
            distance_b: 5,
        };
        let debug = format!("{info:?}");
        assert!(debug.contains("MergeBaseInfo"));
        assert!(debug.contains("distance_a"));
    }

    #[test]
    fn merge_base_info_equality() {
        let oid: gix::ObjectId = "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap();
        let a = MergeBaseInfo {
            oid,
            distance_a: 3,
            distance_b: 5,
        };
        let b = MergeBaseInfo {
            oid,
            distance_a: 3,
            distance_b: 5,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn merge_base_info_inequality_distance() {
        let oid: gix::ObjectId = "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap();
        let a = MergeBaseInfo {
            oid,
            distance_a: 3,
            distance_b: 5,
        };
        let b = MergeBaseInfo {
            oid,
            distance_a: 3,
            distance_b: 7,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn patch_id_debug_format() {
        let pid = PatchId {
            hash: "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap(),
        };
        let debug = format!("{pid:?}");
        assert!(debug.contains("PatchId"));
    }

    #[test]
    fn patch_id_equality() {
        let hash: gix::ObjectId = "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap();
        let a = PatchId { hash };
        let b = PatchId { hash };
        assert_eq!(a, b);
    }

    #[test]
    fn patch_id_hash_in_set() {
        let hash: gix::ObjectId = "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap();
        let mut set = HashSet::new();
        set.insert(PatchId { hash });
        assert!(set.contains(&PatchId { hash }));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn patch_id_different_hashes_not_equal() {
        let a = PatchId {
            hash: "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap(),
        };
        let b = PatchId {
            hash: "1234567890abcdef1234567890abcdef12345678".parse().unwrap(),
        };
        assert_ne!(a, b);
    }
}

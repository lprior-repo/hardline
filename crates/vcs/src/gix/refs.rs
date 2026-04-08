//! Gitoxide Ref Operations
//!
//! Port of stax/src/git/refs.rs to pure gix — no CLI spawning.
//!
//! Manages custom git refs for metadata storage:
//! - `refs/branch-metadata/<branch>` — per-branch JSON metadata
//! - `refs/stax/trunk` — configured trunk branch name
//! - `refs/stax/prev-branch` — previous branch for `stax prev`

use crate::error::{GitError, GitResult};
use gix::bstr::ByteSlice;

// -- Ref name constants --

const METADATA_REF_PREFIX: &str = "refs/branch-metadata/";
const STAX_TRUNK_REF: &str = "refs/stax/trunk";
const STAX_PREV_BRANCH_REF: &str = "refs/stax/prev-branch";

// ============================================================================
// Branch Metadata
// ============================================================================

/// Read metadata JSON for a branch from git refs.
///
/// Looks up `refs/branch-metadata/<branch>` and returns the blob content.
/// Returns `Ok(None)` if the ref does not exist.
pub fn read_metadata(repo: &gix::Repository, branch: &str) -> GitResult<Option<String>> {
    let ref_name = format!("{METADATA_REF_PREFIX}{branch}");
    read_blob_ref(repo, &ref_name)
}

/// Write metadata JSON for a branch to git refs.
///
/// Creates a blob with the JSON content and points `refs/branch-metadata/<branch>` at it.
pub fn write_metadata(repo: &gix::Repository, branch: &str, json: &str) -> GitResult<()> {
    let ref_name = format!("{METADATA_REF_PREFIX}{branch}");
    write_blob_ref(repo, &ref_name, json)
}

/// Delete metadata ref for a branch.
pub fn delete_metadata(repo: &gix::Repository, branch: &str) -> GitResult<()> {
    let ref_name = format!("{METADATA_REF_PREFIX}{branch}");
    delete_ref(repo, &ref_name)
}

/// List all branches that have metadata refs.
///
/// Returns branch names (stripped of the `refs/branch-metadata/` prefix).
pub fn list_metadata_branches(repo: &gix::Repository) -> GitResult<Vec<String>> {
    list_refs_by_prefix(repo, METADATA_REF_PREFIX)
}

// ============================================================================
// Stax Initialization
// ============================================================================

/// Check if stax has been initialized in this repo.
///
/// Returns `true` if `refs/stax/trunk` exists.
pub fn is_initialized(repo: &gix::Repository) -> bool {
    repo.find_reference(STAX_TRUNK_REF).is_ok()
}

// ============================================================================
// Trunk Branch
// ============================================================================

/// Read the configured trunk branch from `refs/stax/trunk`.
///
/// Returns `Ok(None)` if not initialized (ref does not exist).
pub fn read_trunk(repo: &gix::Repository) -> GitResult<Option<String>> {
    let content = read_blob_ref(repo, STAX_TRUNK_REF)?;
    Ok(content.map(|s| s.trim().to_string()))
}

/// Write the trunk branch setting to `refs/stax/trunk`.
pub fn write_trunk(repo: &gix::Repository, trunk: &str) -> GitResult<()> {
    write_blob_ref(repo, STAX_TRUNK_REF, trunk)
}

// ============================================================================
// Previous Branch
// ============================================================================

/// Read the previous branch from `refs/stax/prev-branch`.
///
/// Returns `Ok(None)` if the ref does not exist.
pub fn read_prev_branch(repo: &gix::Repository) -> GitResult<Option<String>> {
    let content = read_blob_ref(repo, STAX_PREV_BRANCH_REF)?;
    Ok(content.map(|s| s.trim().to_string()))
}

/// Write the previous branch to `refs/stax/prev-branch`.
pub fn write_prev_branch(repo: &gix::Repository, branch: &str) -> GitResult<()> {
    write_blob_ref(repo, STAX_PREV_BRANCH_REF, branch)
}

// ============================================================================
// Generic Ref Operations
// ============================================================================

/// Update a ref to point to a specific OID string.
///
/// Resolves the OID string and updates the ref. Creates the ref if it doesn't exist.
pub fn update_ref(repo: &gix::Repository, refname: &str, oid: &str) -> GitResult<()> {
    let target: gix::ObjectId = oid
        .parse()
        .map_err(|e| GitError::InvalidRef {
            name: refname.to_string(),
            reason: format!("Invalid OID '{oid}': {e}"),
        })?;

    repo.reference(
        refname,
        target,
        gix::refs::transaction::PreviousValue::Any,
        format!("update {refname}"),
    )
    .map_err(|e| GitError::InvalidRef {
        name: refname.to_string(),
        reason: format!("Failed to update ref: {e}"),
    })?;

    Ok(())
}

/// Delete a ref by name.
pub fn delete_ref(repo: &gix::Repository, refname: &str) -> GitResult<()> {
    let reference = repo.find_reference(refname).map_err(|e| GitError::InvalidRef {
        name: refname.to_string(),
        reason: format!("Ref not found: {e}"),
    })?;

    reference.delete().map_err(|e| GitError::InvalidRef {
        name: refname.to_string(),
        reason: format!("Failed to delete ref: {e}"),
    })?;

    Ok(())
}

/// Resolve any refspec (branch name, remote ref, SHA) to an OID string.
///
/// Tries in order: local branch, remote branch, direct reference, rev-parse.
pub fn resolve_ref(repo: &gix::Repository, refspec: &str) -> GitResult<String> {
    // Try as local branch
    let local_ref = format!("refs/heads/{refspec}");
    if let Ok(reference) = repo.find_reference(&local_ref) {
        if let Some(oid) = reference.try_id() {
            return Ok(oid.detach().to_string());
        }
    }

    // Try as remote branch (e.g., "origin/main")
    let remote_ref = format!("refs/remotes/{refspec}");
    if let Ok(reference) = repo.find_reference(&remote_ref) {
        if let Some(oid) = reference.try_id() {
            return Ok(oid.detach().to_string());
        }
    }

    // Try as direct reference (e.g., "refs/stax/trunk")
    if let Ok(reference) = repo.find_reference(refspec) {
        if let Some(oid) = reference.try_id() {
            return Ok(oid.detach().to_string());
        }
    }

    // Try rev-parse
    let oid = repo
        .rev_parse_single(refspec)
        .map_err(|e| GitError::InvalidRef {
            name: refspec.to_string(),
            reason: format!("Cannot resolve refspec: {e}"),
        })?
        .detach();

    Ok(oid.to_string())
}

/// List ref names under a given prefix, stripping the prefix.
pub fn list_refs_by_prefix(repo: &gix::Repository, prefix: &str) -> GitResult<Vec<String>> {
    let refs = repo.references().map_err(|e| GitError::InvalidRef {
        name: prefix.to_string(),
        reason: format!("Failed to list references: {e}"),
    })?;

    let prefixed = refs.prefixed(prefix).map_err(|e| GitError::InvalidRef {
        name: prefix.to_string(),
        reason: format!("Failed to filter references: {e}"),
    })?;

    let mut names = Vec::new();
    for reference_result in prefixed {
        let reference = reference_result.map_err(|e| GitError::InvalidRef {
            name: prefix.to_string(),
            reason: format!("Failed to read reference: {e}"),
        })?;

        let full_name = reference.name().as_bstr().to_str_lossy().to_string();
        if let Some(name) = full_name.strip_prefix(prefix) {
            names.push(name.to_string());
        }
    }

    Ok(names)
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Read a blob-backed ref and return its string content.
///
/// Returns `Ok(None)` if the ref does not exist.
fn read_blob_ref(repo: &gix::Repository, ref_name: &str) -> GitResult<Option<String>> {
    let reference = match repo.find_reference(ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let oid = reference
        .try_id()
        .ok_or_else(|| GitError::InvalidRef {
            name: ref_name.to_string(),
            reason: "Reference has no target".to_string(),
        })?
        .detach();

    let obj = repo.find_object(oid).map_err(|e| GitError::InvalidRef {
        name: ref_name.to_string(),
        reason: format!("Failed to find object: {e}"),
    })?;

    let blob = obj.try_into_blob().map_err(|e| GitError::InvalidRef {
        name: ref_name.to_string(),
        reason: format!("Not a blob: {e}"),
    })?;

    let content =
        String::from_utf8(blob.data.to_vec()).map_err(|e| GitError::InvalidRef {
            name: ref_name.to_string(),
            reason: format!("Invalid UTF-8 in blob: {e}"),
        })?;

    Ok(Some(content))
}

/// Write content as a blob and point a ref at it.
fn write_blob_ref(repo: &gix::Repository, ref_name: &str, content: &str) -> GitResult<()> {
    // Create a blob with the content
    let oid = repo
        .write_blob(content.as_bytes())
        .map_err(|e| GitError::InvalidRef {
            name: ref_name.to_string(),
            reason: format!("Failed to create blob: {e}"),
        })?;

    // Update the ref to point to the blob
    repo.reference(
        ref_name,
        oid,
        gix::refs::transaction::PreviousValue::Any,
        format!("update {ref_name}"),
    )
    .map_err(|e| GitError::InvalidRef {
        name: ref_name.to_string(),
        reason: format!("Failed to update ref: {e}"),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_name_constants() {
        assert_eq!(METADATA_REF_PREFIX, "refs/branch-metadata/");
        assert_eq!(STAX_TRUNK_REF, "refs/stax/trunk");
        assert_eq!(STAX_PREV_BRANCH_REF, "refs/stax/prev-branch");
    }

    #[test]
    fn metadata_ref_format() {
        let branch = "feature/foo";
        let ref_name = format!("{METADATA_REF_PREFIX}{branch}");
        assert_eq!(ref_name, "refs/branch-metadata/feature/foo");
    }

    #[test]
    fn trunk_ref_format() {
        assert!(STAX_TRUNK_REF.starts_with("refs/stax/"));
    }

    #[test]
    fn prev_branch_ref_format() {
        assert!(STAX_PREV_BRANCH_REF.starts_with("refs/stax/"));
    }

    // -- resolve_ref unit tests (no repo needed) --

    #[test]
    fn local_ref_format() {
        let branch = "main";
        let local_ref = format!("refs/heads/{branch}");
        assert_eq!(local_ref, "refs/heads/main");
    }

    #[test]
    fn remote_ref_format() {
        let spec = "origin/main";
        let remote_ref = format!("refs/remotes/{spec}");
        assert_eq!(remote_ref, "refs/remotes/origin/main");
    }
}

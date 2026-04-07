//! Git ref-based metadata storage.
//!
//! Stores per-branch metadata as JSON blobs under `refs/branch-metadata/<branch>`,
//! and the trunk setting under `refs/stax/trunk`. Compatible with stax/freephite
//! metadata format.

use crate::application::traits::MetadataStore;
use crate::domain::metadata::BranchMetadata;
use crate::error::{Result, StackError};

/// Metadata storage using git references.
///
/// Uses `gix` (libgit2) for all git operations. Metadata is stored as:
/// - `refs/branch-metadata/<branch>` — JSON blob for each tracked branch
/// - `refs/stax/trunk` — the configured trunk branch name
pub struct GitRefMetadataStore {
    /// Path to the git repository.
    repo_path: std::path::PathBuf,
}

const METADATA_REF_PREFIX: &str = "refs/branch-metadata/";
const STAX_TRUNK_REF: &str = "refs/stax/trunk";

impl GitRefMetadataStore {
    pub fn new(repo_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    fn metadata_ref_name(branch: &str) -> String {
        format!("{METADATA_REF_PREFIX}{branch}")
    }

    fn open_repo(&self) -> Result<gix::Repository> {
        gix::open(&self.repo_path)
            .map_err(|e| StackError::GitError(format!("failed to open repo: {e}")))
    }
}

impl MetadataStore for GitRefMetadataStore {
    fn read(&self, branch: &str) -> Result<Option<BranchMetadata>> {
        let repo = self.open_repo()?;
        let ref_name = Self::metadata_ref_name(branch);

        let reference = match repo.try_find_reference(&ref_name) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(None),
            Err(e) => {
                return Err(StackError::GitError(format!(
                    "failed to find reference {ref_name}: {e}"
                )));
            }
        };

        let Peeled { target, .. } = reference
            .peeled()
            .map_err(|e| StackError::GitError(format!("failed to peel reference: {e}")))?; // Use .target().object() approach instead

        drop(Peeled);

        // Try to read the blob content
        let obj = reference
            .target()
            .ok_or_else(|| StackError::GitError("reference has no target".to_string()))?
            .object()
            .map_err(|e| StackError::GitError(format!("failed to read ref target: {e}")))?;

        let blob = obj
            .try_to_blob()
            .map_err(|e| StackError::GitError(format!("ref target is not a blob: {e}")))?;

        let json = std::str::from_utf8(&blob.data)
            .map_err(|e| StackError::GitError(format!("metadata is not valid UTF-8: {e}")))?;

        BranchMetadata::from_json(json).map(Some)
    }

    fn write(&self, branch: &str, metadata: &BranchMetadata) -> Result<()> {
        let repo = self.open_repo()?;
        let json = metadata.to_json()?;
        let ref_name = Self::metadata_ref_name(branch);

        // Create a blob with the JSON content
        let blob_id = repo
            .write_object(gix::objs::Blob {
                data: json.into_bytes().into(),
            })
            .map_err(|e| StackError::GitError(format!("failed to write blob: {e}")))?;

        // Update the reference to point to the new blob
        repo.reference(
            &ref_name,
            blob_id.detach(),
            gix::refs::transaction::PreviousValue::Any,
            "hardline: update branch metadata",
            None,
        )
        .map_err(|e| StackError::GitError(format!("failed to update ref {ref_name}: {e}")))?;

        Ok(())
    }

    fn delete(&self, branch: &str) -> Result<()> {
        let repo = self.open_repo()?;
        let ref_name = Self::metadata_ref_name(branch);

        repo.find_reference(&ref_name)
            .and_then(|mut r| r.delete())
            .map_err(|e| StackError::GitError(format!("failed to delete ref {ref_name}: {e}")))?;

        Ok(())
    }

    fn list_branches(&self) -> Result<Vec<String>> {
        let repo = self.open_repo()?;

        let mut branches = Vec::new();
        let refs = repo
            .references()
            .map_err(|e| StackError::GitError(format!("failed to list references: {e}")))?;

        for reference in refs {
            let reference = reference
                .map_err(|e| StackError::GitError(format!("failed to read reference: {e}")))?;

            let name = reference
                .name()
                .as_bstr()
                .to_string();

            if let Some(branch) = name.strip_prefix(METADATA_REF_PREFIX) {
                branches.push(branch.to_string());
            }
        }

        Ok(branches)
    }

    fn read_trunk(&self) -> Result<Option<String>> {
        let repo = self.open_repo()?;

        let reference = match repo.try_find_reference(STAX_TRUNK_REF) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(None),
            Err(e) => {
                return Err(StackError::GitError(format!(
                    "failed to find trunk ref: {e}"
                )));
            }
        };

        let obj = reference
            .target()
            .ok_or_else(|| StackError::GitError("trunk ref has no target".to_string()))?
            .object()
            .map_err(|e| StackError::GitError(format!("failed to read trunk ref: {e}")))?;

        let blob = obj
            .try_to_blob()
            .map_err(|e| StackError::GitError(format!("trunk ref is not a blob: {e}")))?;

        let trunk = std::str::from_utf8(&blob.data)
            .map_err(|e| StackError::GitError(format!("trunk ref is not valid UTF-8: {e}")))?
            .trim()
            .to_string();

        Ok(Some(trunk))
    }

    fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
        let repo = self.open_repo()?;

        let reference = match repo.try_find_reference(&format!("refs/heads/{branch}")) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(None),
            Err(_) => return Ok(None),
        };

        let id = reference
            .target()
            .ok_or_else(|| StackError::GitError("branch ref has no target".to_string()))?;

        let commit = id
            .object()
            .map_err(|e| StackError::GitError(format!("failed to read commit: {e}")))?
            .try_to_commit()
            .map_err(|e| StackError::GitError(format!("ref target is not a commit: {e}")))?;

        Ok(Some(commit.id.to_hex().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_metadata_ref_name() {
        assert_eq!(
            GitRefMetadataStore::metadata_ref_name("feature-a"),
            "refs/branch-metadata/feature-a"
        );
        assert_eq!(
            GitRefMetadataStore::metadata_ref_name("main"),
            "refs/branch-metadata/main"
        );
    }

    #[test]
    fn test_new_store() {
        let store = GitRefMetadataStore::new("/tmp/test-repo");
        assert_eq!(store.repo_path, PathBuf::from("/tmp/test-repo"));
    }
}

//! `StackMetadata` query operations

use super::entities::StackMetadata;
use crate::dag::BranchId;

impl StackMetadata {
    /// Get parent of branch
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    pub fn get_parent(&self, branch: &BranchId) -> Result<Option<BranchId>, crate::Error> {
        self.parents.get(branch).map_or_else(
            || {
                Err(crate::Error::not_found(format!(
                    "Branch not found: {branch}"
                )))
            },
            |parent| Ok(parent.clone()),
        )
    }

    /// Get children of branch
    ///
    /// # Errors
    /// Returns `MetadataError::ParentNotFound` if parent doesn't exist.
    pub fn get_children(&self, parent: &BranchId) -> Result<Vec<BranchId>, crate::Error> {
        self.children.get(parent).map_or_else(
            || {
                if self.parents.contains_key(parent) {
                    Ok(Vec::new())
                } else {
                    Err(crate::Error::not_found(format!(
                        "Parent not found: {parent}"
                    )))
                }
            },
            |children| Ok(children.clone()),
        )
    }

    /// Check if branch exists
    #[must_use]
    pub fn has_branch(&self, branch: &BranchId) -> bool {
        self.parents.contains_key(branch)
    }

    /// Get all branch IDs
    #[must_use]
    pub fn branch_ids(&self) -> Vec<BranchId> {
        self.parents.keys().cloned().collect()
    }

    /// Get the number of branches
    #[must_use]
    pub fn len(&self) -> usize {
        self.parents.len()
    }

    /// Check if metadata is empty (only trunk)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parents.len() == 1
    }
}

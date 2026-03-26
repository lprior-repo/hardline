//! StackMetadata mutation operations

use crate::dag::BranchId;
use crate::Error;

use super::entities::StackMetadata;

impl StackMetadata {
    /// Set parent relationship
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    /// Returns `MetadataError::ParentNotFound` if parent doesn't exist.
    /// Returns `MetadataError::CircularReference` if setting parent would create a cycle.
    pub fn set_parent(&mut self, branch: BranchId, parent: BranchId) -> Result<(), Error> {
        // Check if branch exists
        if !self.parents.contains_key(&branch) {
            return Err(Error::not_found(format!("Branch not found: {}", branch)));
        }

        // Check if parent exists
        if !self.parents.contains_key(&parent) {
            return Err(Error::not_found(format!("Parent not found: {}", parent)));
        }

        // Check if parent is the same (no change needed)
        if self.parents.get(&branch) == Some(&Some(parent.clone())) {
            return Ok(());
        }

        // Check if setting this parent would create a cycle
        if self.would_create_cycle(&branch, &parent) {
            return Err(Error::invalid_state(format!(
                "Circular reference would be created for branch {}",
                branch
            )));
        }

        // Get old parent if exists
        let old_parent = self.parents.get(&branch).cloned().flatten();

        // Update parent mapping
        self.parents.insert(branch.clone(), Some(parent.clone()));

        // Update children mapping for old parent
        if let Some(ref old_p) = old_parent {
            if let Some(children) = self.children.get_mut(old_p) {
                children.retain(|c| c != &branch);
            }
        }

        // Update children mapping for new parent
        self.children
            .entry(parent.clone())
            .or_default()
            .push(branch.clone());

        // Save to backend
        self.save()?;

        Ok(())
    }

    /// Add a new branch to metadata
    ///
    /// # Errors
    /// Returns `MetadataError::BranchAlreadyExists` if branch already exists.
    /// Returns `MetadataError::ParentNotFound` if parent doesn't exist.
    pub fn add_branch(&mut self, branch: BranchId, parent: Option<&BranchId>) -> Result<(), Error> {
        if self.parents.contains_key(&branch) {
            return Err(Error::invalid_state(format!(
                "Branch already exists: {}",
                branch
            )));
        }

        // If parent is specified, check it exists
        if let Some(parent_id) = parent {
            if !self.parents.contains_key(parent_id) {
                return Err(Error::not_found(format!("Parent not found: {}", parent_id)));
            }
        }

        // Update parent mapping
        let parent = parent.cloned();
        self.parents.insert(branch.clone(), parent.clone());

        // Update children mapping for parent
        if let Some(ref parent_id) = parent {
            self.children
                .entry(parent_id.clone())
                .or_default()
                .push(branch.clone());
        }

        // Save to backend
        self.save()?;

        Ok(())
    }

    /// Remove branch from metadata
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    pub fn remove_branch(&mut self, branch: BranchId) -> Result<(), Error> {
        // Check if branch exists
        if !self.parents.contains_key(&branch) {
            return Err(Error::not_found(format!("Branch not found: {}", branch)));
        }

        // Get parent if exists
        let parent = self.parents.get(&branch).cloned().flatten();

        // Remove from parents
        self.parents.remove(&branch);

        // Remove from parent's children
        if let Some(ref parent_id) = parent {
            if let Some(children) = self.children.get_mut(parent_id) {
                children.retain(|c| c != &branch);
            }
        }

        // Remove from children (if this branch has children)
        self.children.remove(&branch);

        // Save to backend
        self.save()?;

        Ok(())
    }
}

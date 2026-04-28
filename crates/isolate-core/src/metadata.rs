//! `StackMetadata` - Storage layer for branch metadata with backend delegation
//!
//! Manages parent-child relationships between branches with persistence.

use std::{collections::BTreeMap, rc::Rc};

use petgraph::{
    algo::has_path_connecting,
    graph::{DiGraph, NodeIndex},
};

use crate::dag::BranchId;
use crate::Error;

/// Backend trait for metadata persistence
pub trait MetadataBackend {
    /// Load metadata from backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to load data.
    fn load(&self) -> Result<Vec<u8>, Error>;
    /// Save metadata to backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to save data.
    fn save(&self, data: &[u8]) -> Result<(), Error>;
}

/// Error types for metadata operations
#[derive(Debug)]
pub enum MetadataError {
    /// Branch not found in metadata
    BranchNotFound(BranchId),
    /// Branch already exists in metadata
    BranchAlreadyExists(BranchId),
    /// Parent branch not found
    ParentNotFound(BranchId),
    /// Setting parent would create circular reference
    CircularReference(BranchId),
    /// Backend operation failed
    Backend(String),
    /// Metadata is corrupted
    Corrupted,
}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BranchNotFound(id) => write!(f, "Branch not found: {}", id),
            Self::BranchAlreadyExists(id) => write!(f, "Branch already exists: {}", id),
            Self::ParentNotFound(id) => write!(f, "Parent not found: {}", id),
            Self::CircularReference(id) => {
                write!(f, "Circular reference would be created for branch {}", id)
            }
            Self::Backend(msg) => write!(f, "Backend error: {}", msg),
            Self::Corrupted => write!(f, "Metadata corrupted"),
        }
    }
}

impl std::error::Error for MetadataError {}

/// Metadata storage with backend delegation
#[derive(Clone)]
pub struct StackMetadata {
    /// `BranchId` -> Option<BranchId> (parent, None for trunk)
    parents: BTreeMap<BranchId, Option<BranchId>>,
    /// `BranchId` -> Vec<BranchId> (children)
    children: BTreeMap<BranchId, Vec<BranchId>>,
    /// Backend for persistence
    backend: Rc<dyn MetadataBackend>,
}

impl std::fmt::Debug for StackMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackMetadata")
            .field("parents", &self.parents)
            .field("children", &self.children)
            .field("backend", &"<backend trait object>")
            .finish()
    }
}

impl StackMetadata {
    fn build_graph(&self) -> (DiGraph<BranchId, ()>, BTreeMap<BranchId, NodeIndex>) {
        let (graph, indices) = self.parents.keys().cloned().fold(
            (DiGraph::new(), BTreeMap::new()),
            |(mut graph, mut indices), branch| {
                let node_idx = graph.add_node(branch.clone());
                indices.insert(branch, node_idx);
                (graph, indices)
            },
        );

        let graph = self
            .parents
            .iter()
            .filter_map(|(branch, maybe_parent)| {
                maybe_parent
                    .as_ref()
                    .and_then(|parent| indices.get(parent).copied())
                    .zip(indices.get(branch).copied())
            })
            .fold(graph, |mut graph, (parent_idx, branch_idx)| {
                graph.add_edge(parent_idx, branch_idx, ());
                graph
            });

        (graph, indices)
    }

    /// Create new metadata with backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to save initial metadata.
    pub fn new(backend: Rc<dyn MetadataBackend>) -> Result<Self, Error> {
        let trunk = BranchId::new("trunk").map_err(|e| Error::InvalidState(format!("{}", e)))?;
        let parents = BTreeMap::from_iter([(trunk, None)]);
        let children = BTreeMap::new();

        let metadata = Self {
            parents,
            children,
            backend,
        };

        metadata.save()?;
        Ok(metadata)
    }

    /// Load metadata from backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to load or if metadata is corrupted.
    pub fn load(backend: Rc<dyn MetadataBackend>) -> Result<Self, Error> {
        let data = backend.load()?;

        if data.is_empty() {
            return Self::new(backend);
        }

        let (parents, children) = Self::parse_metadata(&data)?;

        Ok(Self {
            parents,
            children,
            backend,
        })
    }

    /// Parse metadata from bytes
    #[allow(clippy::type_complexity)]
    fn parse_metadata(
        data: &[u8],
    ) -> Result<
        (
            BTreeMap<BranchId, Option<BranchId>>,
            BTreeMap<BranchId, Vec<BranchId>>,
        ),
        Error,
    > {
        let text = String::from_utf8(data.to_vec())
            .map_err(|_| Error::InvalidState("Metadata corrupted: invalid UTF-8".to_string()))?;

        text.lines().try_fold(
            (BTreeMap::new(), BTreeMap::<BranchId, Vec<BranchId>>::new()),
            |(mut parents, mut children), raw_line| {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return Ok((parents, children));
                }

                let parts: Vec<&str> = line.split('|').map(str::trim).collect();
                if parts.len() != 2 {
                    return Err(Error::InvalidState(
                        "Metadata corrupted: invalid format".to_string(),
                    ));
                }

                let branch = BranchId::new(parts[0])
                    .map_err(|e| Error::InvalidState(format!("Invalid branch: {}", e)))?;
                let parent = if parts[1] != "none" {
                    Some(
                        BranchId::new(parts[1])
                            .map_err(|e| Error::InvalidState(format!("Invalid parent: {}", e)))?,
                    )
                } else {
                    None
                };

                parents.insert(branch.clone(), parent.clone());
                if let Some(parent_id) = &parent {
                    children
                        .entry(parent_id.clone())
                        .or_default()
                        .push(branch.clone());
                }

                Ok((parents, children))
            },
        )
    }

    /// Serialize metadata to bytes
    fn serialize_metadata(&self) -> Vec<u8> {
        [
            "# StackMetadata - Branch parent relationships".to_string(),
            "# Format: branch|parent".to_string(),
        ]
        .into_iter()
        .chain(self.parents.iter().map(|(branch, parent)| {
            format!(
                "{}|{}",
                branch.as_str(),
                parent
                    .as_ref()
                    .map_or_else(|| "none".to_string(), |value| value.as_str().to_string())
            )
        }))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
    }

    /// Save metadata to backend
    ///
    /// # Errors
    /// Returns an error if the backend fails to save.
    pub fn save(&self) -> Result<(), Error> {
        let data = self.serialize_metadata();
        self.backend
            .save(&data)
            .map_err(|e| Error::InvalidState(format!("Metadata backend error: {}", e)))
    }

    /// Set parent relationship
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    /// Returns `MetadataError::ParentNotFound` if parent doesn't exist.
    /// Returns `MetadataError::CircularReference` if setting parent would create a cycle.
    pub fn set_parent(&mut self, branch: BranchId, parent: BranchId) -> Result<(), Error> {
        // Check if branch exists
        if !self.parents.contains_key(&branch) {
            return Err(Error::NotFound(format!("Branch not found: {}", branch)));
        }

        // Check if parent exists
        if !self.parents.contains_key(&parent) {
            return Err(Error::NotFound(format!("Parent not found: {}", parent)));
        }

        // Check if parent is the same (no change needed)
        if self.parents.get(&branch) == Some(&Some(parent.clone())) {
            return Ok(());
        }

        // Check if setting this parent would create a cycle
        if self.would_create_cycle(&branch, &parent) {
            return Err(Error::InvalidState(format!(
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

    /// Get parent of branch
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    pub fn get_parent(&self, branch: BranchId) -> Result<Option<BranchId>, Error> {
        match self.parents.get(&branch) {
            Some(parent) => Ok(parent.clone()),
            None => Err(Error::NotFound(format!("Branch not found: {}", branch))),
        }
    }

    /// Get children of branch
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if parent doesn't exist.
    pub fn get_children(&self, parent: BranchId) -> Result<Vec<BranchId>, Error> {
        match self.children.get(&parent) {
            Some(children) => Ok(children.clone()),
            None => {
                // If parent exists in parents but not in children, return empty list
                if self.parents.contains_key(&parent) {
                    Ok(Vec::new())
                } else {
                    Err(Error::NotFound(format!("Parent not found: {}", parent)))
                }
            }
        }
    }

    /// Check if branch exists
    #[must_use]
    pub fn has_branch(&self, branch: &BranchId) -> bool {
        self.parents.contains_key(branch)
    }

    /// Remove branch from metadata
    ///
    /// # Errors
    /// Returns `MetadataError::BranchNotFound` if branch doesn't exist.
    pub fn remove_branch(&mut self, branch: BranchId) -> Result<(), Error> {
        // Check if branch exists
        if !self.parents.contains_key(&branch) {
            return Err(Error::NotFound(format!("Branch not found: {}", branch)));
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

    /// Check if adding parent would create a cycle
    fn would_create_cycle(&self, branch: &BranchId, parent: &BranchId) -> bool {
        // Can't set parent to self
        if branch == parent {
            return true;
        }

        // Can't set trunk as child of anything
        if branch.as_str() == "trunk" {
            return true;
        }

        let (graph, indices) = self.build_graph();

        indices
            .get(branch)
            .copied()
            .zip(indices.get(parent).copied())
            .is_some_and(|(branch_idx, parent_idx)| {
                has_path_connecting(&graph, branch_idx, parent_idx, None)
            })
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

    /// Add a new branch to metadata
    ///
    /// # Errors
    /// Returns `MetadataError::BranchAlreadyExists` if branch already exists.
    /// Returns `MetadataError::ParentNotFound` if parent doesn't exist.
    pub fn add_branch(&mut self, branch: BranchId, parent: Option<&BranchId>) -> Result<(), Error> {
        if self.parents.contains_key(&branch) {
            return Err(Error::InvalidState(format!(
                "Branch already exists: {}",
                branch
            )));
        }

        // If parent is specified, check it exists
        if let Some(parent_id) = parent {
            if !self.parents.contains_key(parent_id) {
                return Err(Error::NotFound(format!("Parent not found: {}", parent_id)));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockBackend {
        data: std::cell::RefCell<Vec<u8>>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                data: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl MetadataBackend for MockBackend {
        fn load(&self) -> Result<Vec<u8>, Error> {
            Ok(self.data.borrow().clone())
        }

        fn save(&self, data: &[u8]) -> Result<(), Error> {
            *self.data.borrow_mut() = data.to_vec();
            Ok(())
        }
    }

    #[test]
    fn test_new_metadata_has_trunk_with_no_parent() {
        let backend = MockBackend::new();
        let metadata = StackMetadata::new(Rc::new(backend)).expect("Should create metadata");

        let trunk = BranchId::new("trunk").unwrap();
        assert!(metadata.has_branch(&trunk));
        assert_eq!(
            metadata
                .get_parent(trunk.clone())
                .expect("Should get parent"),
            None
        );
        assert_eq!(metadata.len(), 1);
    }

    #[test]
    fn test_set_parent_creates_parent_child_relationship() {
        let backend = MockBackend::new();
        let mut metadata = StackMetadata::new(Rc::new(backend)).expect("Should create metadata");

        let feature = BranchId::new("feature").unwrap();
        let trunk = BranchId::new("trunk").unwrap();

        metadata
            .add_branch(feature.clone(), None)
            .expect("Should add branch");
        metadata
            .set_parent(feature.clone(), trunk.clone())
            .expect("Should set parent");

        assert_eq!(
            metadata
                .get_parent(feature.clone())
                .expect("Should get parent"),
            Some(trunk.clone())
        );
        assert_eq!(
            metadata.get_children(trunk).expect("Should get children"),
            vec![feature]
        );
    }

    #[test]
    fn test_add_branch_fails_for_existing_branch() {
        let backend = MockBackend::new();
        let mut metadata = StackMetadata::new(Rc::new(backend)).expect("Should create metadata");

        // Adding trunk again should fail
        let trunk = BranchId::new("trunk").unwrap();
        let result = metadata.add_branch(trunk, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_branch() {
        let backend = MockBackend::new();
        let mut metadata = StackMetadata::new(Rc::new(backend)).expect("Should create metadata");

        let feature = BranchId::new("feature").unwrap();
        let trunk = BranchId::new("trunk").unwrap();

        metadata
            .add_branch(feature.clone(), Some(&trunk))
            .expect("Should add branch");

        metadata
            .remove_branch(feature.clone())
            .expect("Should remove branch");

        assert!(!metadata.has_branch(&feature));
    }

    #[test]
    fn test_circular_reference_prevented() {
        let backend = MockBackend::new();
        let mut metadata = StackMetadata::new(Rc::new(backend)).expect("Should create metadata");

        let feature = BranchId::new("feature").unwrap();
        let trunk = BranchId::new("trunk").unwrap();

        metadata
            .add_branch(feature.clone(), None)
            .expect("Should add branch");
        metadata
            .set_parent(feature.clone(), trunk.clone())
            .expect("Should set parent");

        // Try to set trunk's parent to feature - should fail
        let result = metadata.set_parent(trunk, feature);
        assert!(result.is_err());
    }
}

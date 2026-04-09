//! Domain service for stack operations.
//!
//! Ported from stax `engine/stack.rs`. Provides the core `load_stack` operation
//! that constructs the stack graph from persisted branch metadata, plus
//! read-only queries (branch ancestry, restack detection).
//!
//! The actual mutating VCS operations (restack, create branch, delete branch)
//! live in `TransactionalStackOps`; this service provides read-only stack
//! loading and query capabilities.

use super::transactional_engine::MetadataStore;
use crate::domain::metadata::BranchMetadata;
use crate::error::{Result, StackError};

/// Re-export StackGraph from transactional_engine for use in query results.
pub use super::transactional_engine::StackGraph;

/// Domain service for stack graph construction and queries.
///
/// Wraps a `MetadataStore` to build the stack graph from persisted
/// per-branch metadata. All mutating VCS operations (restack, create branch,
/// delete branch) live in `TransactionalStackOps`; this service is read-only.
pub struct StackEngine<M: MetadataStore> {
    metadata: M,
}

impl<M: MetadataStore> StackEngine<M> {
    /// Create a new engine backed by the given metadata store.
    pub fn new(metadata: M) -> Self {
        Self { metadata }
    }

    /// Load the stack graph from persisted metadata.
    ///
    /// Ported from stax `Stack::load`. Reads all branch metadata refs,
    /// builds the parent-child graph, handles orphaned branches, and
    /// prunes stale metadata for deleted branches.
    pub fn load_stack(&self) -> Result<StackGraph> {
        StackGraph::load(&self.metadata)
    }

    /// Get the ancestors of a branch (up to trunk).
    pub fn ancestors(&self, branch: &str) -> Result<Vec<String>> {
        let graph = self.load_stack()?;
        Ok(graph.ancestors(branch))
    }

    /// Get all descendants of a branch.
    pub fn descendants(&self, branch: &str) -> Result<Vec<String>> {
        let graph = self.load_stack()?;
        Ok(graph.descendants(branch))
    }

    /// Get the current stack (ancestors + current + descendants).
    pub fn current_stack(&self, branch: &str) -> Result<Vec<String>> {
        let graph = self.load_stack()?;
        Ok(graph.current_stack(branch))
    }

    /// Get branches that need restacking.
    pub fn needs_restack(&self) -> Result<Vec<String>> {
        let graph = self.load_stack()?;
        Ok(graph.needs_restack())
    }

    /// Get siblings of a branch (other branches with the same parent).
    pub fn siblings(&self, branch: &str) -> Result<Vec<String>> {
        let graph = self.load_stack()?;
        Ok(graph.get_siblings(branch))
    }

    /// Read metadata for a specific branch.
    pub fn read_branch_metadata(&self, branch: &str) -> Result<Option<BranchMetadata>> {
        self.metadata.read(branch)
    }

    /// Write metadata for a branch.
    pub fn write_branch_metadata(&self, branch: &str, metadata: &BranchMetadata) -> Result<()> {
        metadata.validate()?;
        self.metadata.write(branch, metadata)
    }

    /// Delete metadata for a branch.
    pub fn delete_branch_metadata(&self, branch: &str) -> Result<()> {
        self.metadata.delete(branch)
    }

    /// Get the trunk branch name.
    pub fn trunk(&self) -> Result<String> {
        Ok(self
            .metadata
            .read_trunk()?
            .unwrap_or_else(|| "main".to_string()))
    }

    /// Check if a branch exists (has a revision).
    pub fn branch_exists(&self, branch: &str) -> Result<bool> {
        Ok(self.metadata.branch_revision(branch)?.is_some())
    }

    /// Get the current revision of a branch.
    pub fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
        self.metadata.branch_revision(branch)
    }

    /// Create metadata for a new branch.
    ///
    /// Records the parent branch and its current revision so restack
    /// detection works correctly.
    pub fn create_branch(&self, name: &str, parent: &str) -> Result<()> {
        let parent_revision = self
            .metadata
            .branch_revision(parent)?
            .ok_or_else(|| StackError::BranchNotFound(parent.to_string()))?;

        let metadata = BranchMetadata::new(parent, &parent_revision);
        self.metadata.write(name, &metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metadata::BranchMetadata;
    use std::collections::HashMap;

    /// In-memory metadata store for testing.
    struct MockStore {
        metadata: HashMap<String, BranchMetadata>,
        trunk: Option<String>,
        revisions: HashMap<String, String>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                metadata: HashMap::new(),
                trunk: None,
                revisions: HashMap::new(),
            }
        }

        fn with_trunk(mut self, trunk: &str) -> Self {
            self.trunk = Some(trunk.to_string());
            self.revisions
                .insert(trunk.to_string(), "trunk-rev".to_string());
            self
        }

        fn add_branch(
            mut self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
        ) -> Self {
            self.metadata
                .insert(name.to_string(), BranchMetadata::new(parent, parent_rev));
            self.revisions
                .insert(name.to_string(), current_rev.to_string());
            self
        }

        fn add_branch_with_pr(
            mut self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
            pr_number: u64,
            pr_state: &str,
        ) -> Self {
            self.metadata.insert(
                name.to_string(),
                BranchMetadata::new(parent, parent_rev).with_pr(pr_number, pr_state, None),
            );
            self.revisions
                .insert(name.to_string(), current_rev.to_string());
            self
        }
    }

    impl MetadataStore for MockStore {
        fn read(&self, branch: &str) -> Result<Option<BranchMetadata>> {
            Ok(self.metadata.get(branch).cloned())
        }

        fn write(&self, _branch: &str, _metadata: &BranchMetadata) -> Result<()> {
            Ok(())
        }

        fn delete(&self, _branch: &str) -> Result<()> {
            Ok(())
        }

        fn list_branches(&self) -> Result<Vec<String>> {
            Ok(self.metadata.keys().cloned().collect())
        }

        fn read_trunk(&self) -> Result<Option<String>> {
            Ok(self.trunk.clone())
        }

        fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
            Ok(self.revisions.get(branch).cloned())
        }
    }

    fn create_test_store() -> MockStore {
        MockStore::new()
            .with_trunk("main")
            .add_branch("feature-a", "main", "trunk-rev", "rev-a")
            .add_branch_with_pr("feature-a-1", "feature-a", "rev-a", "rev-a1", 2, "OPEN")
            .add_branch("feature-a-2", "feature-a-1", "rev-a1", "rev-a2")
            .add_branch_with_pr("feature-b", "main", "trunk-rev-old", "rev-b", 3, "MERGED")
    }

    #[test]
    fn test_engine_load_stack() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let graph = engine.load_stack().expect("load");
        assert!(graph.branches.contains_key("main"));
        assert!(graph.branches.contains_key("feature-a"));
        assert!(graph.branches.contains_key("feature-a-1"));
        assert!(graph.branches.contains_key("feature-a-2"));
        assert!(graph.branches.contains_key("feature-b"));
        assert_eq!(graph.trunk, "main");
    }

    #[test]
    fn test_engine_ancestors_from_leaf() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let ancestors = engine.ancestors("feature-a-2").expect("ancestors");
        assert_eq!(ancestors, vec!["feature-a-1", "feature-a", "main"]);
    }

    #[test]
    fn test_engine_ancestors_from_trunk() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let ancestors = engine.ancestors("main").expect("ancestors");
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_engine_descendants_from_trunk() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let mut descendants = engine.descendants("main").expect("descendants");
        descendants.sort();
        assert_eq!(
            descendants,
            vec!["feature-a", "feature-a-1", "feature-a-2", "feature-b"]
        );
    }

    #[test]
    fn test_engine_descendants_from_leaf() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let descendants = engine.descendants("feature-a-2").expect("descendants");
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_engine_current_stack_from_leaf() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let current = engine.current_stack("feature-a-2").expect("current_stack");
        assert_eq!(
            current,
            vec!["main", "feature-a", "feature-a-1", "feature-a-2"]
        );
    }

    #[test]
    fn test_engine_current_stack_from_first_level() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let current = engine.current_stack("feature-b").expect("current_stack");
        assert_eq!(current, vec!["main", "feature-b"]);
    }

    #[test]
    fn test_engine_needs_restack() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let mut needs = engine.needs_restack().expect("needs_restack");
        needs.sort();
        // feature-b has mismatched parent revision (trunk-rev-old vs trunk-rev)
        assert_eq!(needs, vec!["feature-b"]);
    }

    #[test]
    fn test_engine_siblings() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let siblings = engine.siblings("feature-a").expect("siblings");
        assert!(siblings.contains(&"feature-a".to_string()));
        assert!(siblings.contains(&"feature-b".to_string()));
    }

    #[test]
    fn test_engine_read_branch_metadata() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let meta = engine
            .read_branch_metadata("feature-a")
            .expect("read")
            .expect("some");
        assert_eq!(meta.parent_branch_name, "main");
        assert_eq!(meta.parent_branch_revision, "trunk-rev");
    }

    #[test]
    fn test_engine_read_branch_metadata_not_found() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let result = engine.read_branch_metadata("nonexistent").expect("read");
        assert!(result.is_none());
    }

    #[test]
    fn test_engine_trunk() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        assert_eq!(engine.trunk().expect("trunk"), "main");
    }

    #[test]
    fn test_engine_trunk_default() {
        let store = MockStore::new();
        let engine = StackEngine::new(store);
        assert_eq!(engine.trunk().expect("trunk"), "main");
    }

    #[test]
    fn test_engine_branch_exists() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        assert!(engine.branch_exists("feature-a").expect("exists"));
        assert!(!engine.branch_exists("nonexistent").expect("exists"));
    }

    #[test]
    fn test_engine_branch_revision() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        assert_eq!(
            engine.branch_revision("feature-a").expect("revision"),
            Some("rev-a".to_string())
        );
        assert_eq!(
            engine.branch_revision("nonexistent").expect("revision"),
            None
        );
    }

    #[test]
    fn test_engine_create_branch_parent_not_found() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let result = engine.create_branch("new-branch", "nonexistent-parent");
        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::BranchNotFound(_)));
    }

    #[test]
    fn test_engine_empty_stack() {
        let store = MockStore::new().with_trunk("main");
        let engine = StackEngine::new(store);
        let graph = engine.load_stack().expect("load");
        assert_eq!(graph.branches.len(), 1);
        assert!(graph.branches.contains_key("main"));
    }

    #[test]
    fn test_engine_ancestors_nonexistent() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let ancestors = engine.ancestors("nonexistent").expect("ancestors");
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_engine_descendants_nonexistent() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let descendants = engine.descendants("nonexistent").expect("descendants");
        assert!(descendants.is_empty());
    }
}

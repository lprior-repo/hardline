//! Domain service for stack operations.
//!
//! Ported from stax `engine/stack.rs`. Provides the core `load_stack` operation
//! that constructs a `Stack` entity from persisted branch metadata, plus
//! read-only queries (branch ancestry, restack detection).

use std::collections::HashMap;

use crate::domain::entities::{Draft, PrInfo, PrState, Stack, StackBranch};
use crate::domain::metadata::BranchMetadata;
use crate::domain::value_objects::BranchName;
use crate::engine::transactional_engine::MetadataStore;
use crate::error::{Result, StackError};

/// Domain service for stack graph construction and queries.
///
/// Wraps a `MetadataStore` to build the `Stack` entity from persisted
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

    /// Load the full stack from persisted metadata.
    ///
    /// Ported from stax `Stack::load`. Walks all branches that have metadata,
    /// prunes branches whose git refs no longer exist, populates parent/child
    /// links, handles orphaned branches, and returns a `Stack<Draft>`.
    pub fn load_stack(&self) -> Result<Stack<Draft>> {
        let trunk = self
            .metadata
            .read_trunk()?
            .ok_or_else(|| StackError::NotFound("trunk branch not configured".to_string()))?;

        let tracked = self.metadata.list_branches()?;

        // First pass: load metadata for each tracked branch.
        let mut branch_map: HashMap<String, BranchMetadata> = HashMap::new();
        for name in &tracked {
            // Prune metadata for branches that no longer exist on disk.
            if self.metadata.branch_revision(name)?.is_none() {
                continue;
            }

            if let Some(meta) = self.metadata.read(name)? {
                branch_map.insert(name.clone(), meta);
            }
        }

        // Build StackBranch entries and track children.
        let mut branches: Vec<StackBranch> = Vec::new();
        let mut children_map: HashMap<String, Vec<BranchName>> = HashMap::new();
        let mut orphaned: Vec<BranchName> = Vec::new();

        for (name, meta) in &branch_map {
            let parent_name = &meta.parent_branch_name;
            let parent_rev = match self.metadata.branch_revision(parent_name) {
                Ok(Some(rev)) => rev,
                _ => String::new(),
            };
            let needs_restack = meta.needs_restack(&parent_rev);

            let pr_info = meta.pr_info.as_ref().map(|p| PrInfo {
                number: u32::try_from(p.number).unwrap_or(u32::MAX),
                url: String::new(),
                title: String::new(),
                state: parse_pr_state(&p.state),
                is_draft: p.is_draft,
            });

            let parent_bn = BranchName::new(parent_name.clone());

            // Track children for the parent.
            if *parent_name == trunk {
                // direct child of trunk — handled below
            } else if branch_map.contains_key(parent_name) {
                children_map
                    .entry(parent_name.clone())
                    .or_default()
                    .push(BranchName::new(name.clone()));
            } else {
                // Parent not tracked — orphaned; treat as direct child of trunk.
                orphaned.push(BranchName::new(name.clone()));
            }

            branches.push(StackBranch {
                name: BranchName::new(name.clone()),
                parent: Some(parent_bn),
                children: Vec::new(), // populated in second pass
                needs_restack,
                pr_info,
            });
        }

        // Collect direct children of trunk (including orphaned branches).
        let mut trunk_children: Vec<BranchName> = branches
            .iter()
            .filter(|b| b.parent.as_ref().is_some_and(|p| p.as_str() == trunk))
            .map(|b| b.name.clone())
            .collect();
        trunk_children.extend(orphaned);

        // Second pass: populate children arrays.
        for branch in &mut branches {
            if let Some(children) = children_map.get(branch.name.as_str()) {
                branch.children = children.clone();
            }
        }

        // Add trunk root node.
        let trunk_bn = BranchName::new(&trunk);
        branches.push(StackBranch {
            name: trunk_bn.clone(),
            parent: None,
            children: trunk_children,
            needs_restack: false,
            pr_info: None,
        });

        let mut stack = Stack::<Draft>::new(trunk_bn);
        for branch in branches {
            stack.branches.push(branch);
        }

        Ok(stack)
    }
}

/// Parse a PR state string from metadata into a domain `PrState`.
///
/// The metadata format uses uppercase strings ("OPEN", "CLOSED", "MERGED").
/// Unknown values default to `PrState::Open`.
fn parse_pr_state(state: &str) -> PrState {
    match state.to_uppercase().as_str() {
        "CLOSED" => PrState::Closed,
        "MERGED" => PrState::Merged,
        _ => PrState::Open,
    }
}

/// In-memory `MetadataStore` for testing.
#[cfg(test)]
pub(crate) struct InMemoryMetadataStore {
    trunk: String,
    branches: HashMap<String, BranchMetadata>,
    revisions: HashMap<String, String>,
}

#[cfg(test)]
impl InMemoryMetadataStore {
    pub(crate) fn new(trunk: &str) -> Self {
        Self {
            trunk: trunk.to_string(),
            branches: HashMap::new(),
            revisions: HashMap::new(),
        }
    }

    pub(crate) fn add_branch(
        &mut self,
        name: &str,
        meta: BranchMetadata,
        revision: &str,
    ) {
        self.branches.insert(name.to_string(), meta);
        self.revisions.insert(name.to_string(), revision.to_string());
    }
}

#[cfg(test)]
impl MetadataStore for InMemoryMetadataStore {
    fn read(&self, branch: &str) -> Result<Option<BranchMetadata>> {
        Ok(self.branches.get(branch).cloned())
    }

    fn branch_revision(&self, branch: &str) -> Result<Option<String>> {
        Ok(self.revisions.get(branch).cloned())
    }

    fn list_branches(&self) -> Result<Vec<String>> {
        Ok(self.branches.keys().cloned().collect())
    }

    fn read_trunk(&self) -> Result<Option<String>> {
        Ok(Some(self.trunk.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> InMemoryMetadataStore {
        let mut store = InMemoryMetadataStore::new("main");

        // main at revision "aaa"
        store.revisions.insert("main".to_string(), "aaa".to_string());

        // feature-a: parent=main@aaa, revision=bbb
        store.add_branch(
            "feature-a",
            BranchMetadata::new("main", "aaa").with_pr(1, "OPEN", Some(false)),
            "bbb",
        );

        // feature-a-1: parent=feature-a@bbb, revision=ccc
        store.add_branch(
            "feature-a-1",
            BranchMetadata::new("feature-a", "bbb").with_pr(2, "OPEN", Some(true)),
            "ccc",
        );

        // feature-a-2: parent=feature-a-1@ccc, revision=ddd
        store.add_branch(
            "feature-a-2",
            BranchMetadata::new("feature-a-1", "ccc"),
            "ddd",
        );

        // feature-b: parent=main@aaa (parent has moved to "eee"), needs restack
        store.add_branch(
            "feature-b",
            BranchMetadata::new("main", "old").with_pr(3, "MERGED", None),
            "fff",
        );
        // Simulate main having moved
        store.revisions.insert("main".to_string(), "eee".to_string());

        store
    }

    #[test]
    fn test_load_stack_basic() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        // Should have: main, feature-a, feature-a-1, feature-a-2, feature-b = 5
        assert_eq!(stack.branches.len(), 5);
        assert_eq!(stack.main_branch, BranchName::new("main"));
    }

    #[test]
    fn test_load_stack_trunk_children() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let trunk = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "main")
            .expect("trunk should exist");
        let mut children: Vec<&str> = trunk.children.iter().map(|c| c.as_str()).collect();
        children.sort();
        assert_eq!(children, vec!["feature-a", "feature-b"]);
    }

    #[test]
    fn test_load_stack_parent_child_links() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let feature_a = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "feature-a")
            .expect("feature-a should exist");
        assert_eq!(feature_a.children.len(), 1);
        assert_eq!(feature_a.children[0].as_str(), "feature-a-1");
    }

    #[test]
    fn test_load_stack_deep_chain() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let ancestors = stack.ancestors(&BranchName::new("feature-a-2"));
        let ancestor_strs: Vec<&str> = ancestors.iter().map(|a| a.as_str()).collect();
        assert!(ancestor_strs.contains(&"feature-a-1"));
        assert!(ancestor_strs.contains(&"feature-a"));
        assert!(ancestor_strs.contains(&"main"));
    }

    #[test]
    fn test_load_stack_needs_restack() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let needs = stack.needs_restack();
        // feature-b's parent (main) has moved from "old" to "eee"
        let needs_strs: Vec<&str> = needs.iter().map(|n| n.as_str()).collect();
        assert!(
            needs_strs.contains(&"feature-b"),
            "feature-b should need restack, got: {needs_strs:?}"
        );
    }

    #[test]
    fn test_load_stack_pr_info_preserved() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let feature_a = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "feature-a")
            .expect("feature-a should exist");
        let pr = feature_a.pr_info.as_ref().expect("should have PR info");
        assert_eq!(pr.number, 1);
        assert!(matches!(pr.state, PrState::Open));
        assert_eq!(pr.is_draft, Some(false));
    }

    #[test]
    fn test_load_stack_pr_merged_state() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let feature_b = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "feature-b")
            .expect("feature-b should exist");
        let pr = feature_b.pr_info.as_ref().expect("should have PR info");
        assert!(matches!(pr.state, PrState::Merged));
    }

    #[test]
    fn test_load_stack_pr_draft_state() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let feature_a1 = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "feature-a-1")
            .expect("feature-a-1 should exist");
        let pr = feature_a1.pr_info.as_ref().expect("should have PR info");
        assert_eq!(pr.is_draft, Some(true));
    }

    #[test]
    fn test_load_stack_no_pr_info() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let feature_a2 = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "feature-a-2")
            .expect("feature-a-2 should exist");
        assert!(feature_a2.pr_info.is_none());
    }

    #[test]
    fn test_load_stack_empty() {
        let mut store = InMemoryMetadataStore::new("main");
        store.revisions.insert("main".to_string(), "aaa".to_string());
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        // Only trunk should exist
        assert_eq!(stack.branches.len(), 1);
        assert_eq!(stack.main_branch, BranchName::new("main"));
    }

    #[test]
    fn test_load_stack_no_trunk() {
        let mut store = InMemoryMetadataStore::new("main");
        store.trunk = String::new(); // no trunk
        // MetadataStore::read_trunk returns Some("") which is still Some
        // Actually our impl returns Some(self.trunk), so trunk = "" means Some("")
        // This should still "work" — it'll use "" as trunk name.
        // Let's make read_trunk return None instead.
        struct NoTrunkStore;
        impl MetadataStore for NoTrunkStore {
            fn read(&self, _branch: &str) -> Result<Option<BranchMetadata>> {
                Ok(None)
            }
            fn branch_revision(&self, _branch: &str) -> Result<Option<String>> {
                Ok(None)
            }
            fn list_branches(&self) -> Result<Vec<String>> {
                Ok(vec![])
            }
            fn read_trunk(&self) -> Result<Option<String>> {
                Ok(None)
            }
        }
        let engine = StackEngine::new(NoTrunkStore);
        let result = engine.load_stack();
        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(matches!(err, StackError::NotFound(_)));
    }

    #[test]
    fn test_load_stack_prunes_deleted_branches() {
        let mut store = InMemoryMetadataStore::new("main");
        // Add metadata for "deleted-branch" but don't add a revision
        // (simulating a branch whose git ref was deleted but metadata remains)
        store.branches.insert(
            "deleted-branch".to_string(),
            BranchMetadata::new("main", "aaa"),
        );
        // No revision for "deleted-branch" — branch_revision returns None
        store.revisions.insert("main".to_string(), "aaa".to_string());

        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        // deleted-branch should be pruned; only trunk remains
        assert_eq!(stack.branches.len(), 1);
        assert_eq!(stack.branches[0].name.as_str(), "main");
    }

    #[test]
    fn test_load_stack_orphaned_branch_becomes_trunk_child() {
        let mut store = InMemoryMetadataStore::new("main");
        store.revisions.insert("main".to_string(), "aaa".to_string());

        // orphan-branch's parent is "nonexistent" which is not tracked
        store.add_branch(
            "orphan-branch",
            BranchMetadata::new("nonexistent", "zzz"),
            "bbb",
        );

        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let trunk = stack
            .branches
            .iter()
            .find(|b| b.name.as_str() == "main")
            .expect("trunk");
        assert!(trunk.children.contains(&BranchName::new("orphan-branch")));
    }

    #[test]
    fn test_load_stack_current_stack() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let current = stack.current_stack(&BranchName::new("feature-a-2"));
        let names: Vec<&str> = current.iter().map(|n| n.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"feature-a"));
        assert!(names.contains(&"feature-a-1"));
        assert!(names.contains(&"feature-a-2"));
    }

    #[test]
    fn test_parse_pr_state_open() {
        assert!(matches!(parse_pr_state("OPEN"), PrState::Open));
    }

    #[test]
    fn test_parse_pr_state_closed() {
        assert!(matches!(parse_pr_state("CLOSED"), PrState::Closed));
    }

    #[test]
    fn test_parse_pr_state_merged() {
        assert!(matches!(parse_pr_state("MERGED"), PrState::Merged));
    }

    #[test]
    fn test_parse_pr_state_case_insensitive() {
        assert!(matches!(parse_pr_state("open"), PrState::Open));
        assert!(matches!(parse_pr_state("merged"), PrState::Merged));
    }

    #[test]
    fn test_parse_pr_state_unknown_defaults_open() {
        assert!(matches!(parse_pr_state("UNKNOWN"), PrState::Open));
    }

    #[test]
    fn test_load_stack_descendants() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let desc = stack.descendants(&BranchName::new("main"));
        let mut names: Vec<&str> = desc.iter().map(|n| n.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["feature-a", "feature-a-1", "feature-a-2", "feature-b"]);
    }

    #[test]
    fn test_load_stack_descendants_from_middle() {
        let store = create_test_store();
        let engine = StackEngine::new(store);
        let stack = engine.load_stack().expect("load_stack should succeed");

        let desc = stack.descendants(&BranchName::new("feature-a"));
        let mut names: Vec<&str> = desc.iter().map(|n| n.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["feature-a-1", "feature-a-2"]);
    }
}

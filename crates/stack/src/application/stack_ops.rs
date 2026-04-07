//! Stack loading and traversal operations.
//!
//! Ported from stax `engine/stack.rs`. Loads the branch graph from
//! per-branch metadata stored in git refs, then provides graph-traversal
//! operations (ancestors, descendants, current_stack, etc.).

use std::collections::{HashMap, HashSet};

use crate::application::traits::MetadataStore;
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};

/// A branch within the loaded stack graph.
#[derive(Debug, Clone)]
pub struct StackNode {
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub needs_restack: bool,
    pub pr_number: Option<u64>,
    pub pr_state: Option<String>,
    pub pr_is_draft: Option<bool>,
}

/// The full stack structure loaded from git metadata.
#[derive(Debug, Clone)]
pub struct StackGraph {
    pub branches: HashMap<String, StackNode>,
    pub trunk: String,
}

impl StackGraph {
    /// Load the stack from git metadata.
    ///
    /// Ported from stax `Stack::load`. Reads all branch metadata refs,
    /// builds the parent-child graph, handles orphaned branches, and
    /// prunes stale metadata for deleted branches.
    pub fn load<M: MetadataStore>(store: &M) -> Result<Self> {
        let trunk = store
            .read_trunk()?
            .unwrap_or_else(|| "main".to_string());

        let tracked_branches = store.list_branches()?;
        let mut branches: HashMap<String, StackNode> = HashMap::new();

        // First pass: load all metadata, prune stale entries for deleted branches.
        for branch_name in &tracked_branches {
            // Prune metadata for branches that no longer exist.
            if store.branch_revision(branch_name)?.is_none() {
                let _ = store.delete(branch_name);
                continue;
            }

            if let Some(meta) = store.read(branch_name)? {
                let needs_restack = store
                    .branch_revision(&meta.parent_branch_name)
                    .ok()
                    .flatten()
                    .map_or(false, |rev| meta.needs_restack(&rev));

                branches.insert(
                    branch_name.clone(),
                    StackNode {
                        name: branch_name.clone(),
                        parent: Some(meta.parent_branch_name.clone()),
                        children: Vec::new(),
                        needs_restack,
                        pr_number: meta.pr_info.as_ref().map(|p| p.number),
                        pr_state: meta.pr_info.as_ref().map(|p| p.state.clone()),
                        pr_is_draft: meta.pr_info.as_ref().and_then(|p| p.is_draft),
                    },
                );
            }
        }

        // Second pass: populate children and find orphans.
        let branch_names: Vec<String> = branches.keys().cloned().collect();
        let mut orphaned_branches: Vec<String> = Vec::new();

        for name in branch_names {
            if let Some(parent_name) = branches.get(&name).and_then(|b| b.parent.clone()) {
                if parent_name == trunk {
                    continue;
                }
                if let Some(parent) = branches.get_mut(&parent_name) {
                    parent.children.push(name);
                } else {
                    orphaned_branches.push(name);
                }
            }
        }

        // Collect direct children of trunk (including orphaned branches).
        let mut trunk_children: Vec<String> = branches
            .values()
            .filter(|b| b.parent.as_ref() == Some(&trunk))
            .map(|b| b.name.clone())
            .collect();
        trunk_children.extend(orphaned_branches);

        // Add trunk as root.
        branches.insert(
            trunk.clone(),
            StackNode {
                name: trunk.clone(),
                parent: None,
                children: trunk_children,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
                pr_is_draft: None,
            },
        );

        Ok(Self { branches, trunk })
    }

    /// Get the ancestors of a branch (up to trunk).
    pub fn ancestors(&self, branch: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = branch.to_string();
        let mut visited = HashSet::from([current.clone()]);

        while let Some(b) = self.branches.get(&current) {
            if let Some(parent) = &b.parent {
                if !visited.insert(parent.clone()) {
                    break;
                }
                result.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        result
    }

    /// Get all descendants of a branch.
    pub fn descendants(&self, branch: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut to_visit = vec![branch.to_string()];
        let mut visited = HashSet::from([branch.to_string()]);

        while let Some(current) = to_visit.pop() {
            if let Some(b) = self.branches.get(&current) {
                for child in &b.children {
                    if !visited.insert(child.clone()) {
                        continue;
                    }
                    result.push(child.clone());
                    to_visit.push(child.clone());
                }
            }
        }

        result
    }

    /// Get the current stack (ancestors + current + descendants).
    pub fn current_stack(&self, branch: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        let mut ancestors = self.ancestors(branch);
        ancestors.reverse();

        for name in ancestors {
            if seen.insert(name.clone()) {
                result.push(name);
            }
        }

        if seen.insert(branch.to_string()) {
            result.push(branch.to_string());
        }

        for name in self.descendants(branch) {
            if seen.insert(name.clone()) {
                result.push(name);
            }
        }

        result
    }

    /// Get branches that need restacking.
    pub fn needs_restack(&self) -> Vec<String> {
        self.branches
            .values()
            .filter(|b| b.needs_restack)
            .map(|b| b.name.clone())
            .collect()
    }

    /// Get siblings of a branch (other branches with the same parent).
    pub fn get_siblings(&self, branch: &str) -> Vec<String> {
        let branch_info = match self.branches.get(branch) {
            Some(b) => b,
            None => return vec![branch.to_string()],
        };

        let parent = match &branch_info.parent {
            Some(p) => p,
            None => return vec![branch.to_string()],
        };

        let parent_info = match self.branches.get(parent) {
            Some(p) => p,
            None => {
                let mut siblings: Vec<String> = self
                    .branches
                    .values()
                    .filter(|b| b.parent.as_ref() == Some(&parent.to_string()))
                    .map(|b| b.name.clone())
                    .collect();
                siblings.sort();
                return siblings;
            }
        };

        let mut siblings = parent_info.children.clone();
        siblings.sort();
        siblings
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
            self.revisions.insert(trunk.to_string(), "trunk-rev".to_string());
            self
        }

        fn add_branch(
            mut self,
            name: &str,
            parent: &str,
            parent_rev: &str,
            current_rev: &str,
        ) -> Self {
            self.metadata.insert(
                name.to_string(),
                BranchMetadata::new(parent, parent_rev),
            );
            self.revisions.insert(name.to_string(), current_rev.to_string());
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
            self.revisions.insert(name.to_string(), current_rev.to_string());
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

        fn delete(&self, branch: &str) -> Result<()> {
            // In real impl this would delete, but for tests we just ignore
            let _ = branch;
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
    fn test_load_stack() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        assert!(graph.branches.contains_key("main"));
        assert!(graph.branches.contains_key("feature-a"));
        assert!(graph.branches.contains_key("feature-a-1"));
        assert!(graph.branches.contains_key("feature-a-2"));
        assert!(graph.branches.contains_key("feature-b"));
        assert_eq!(graph.trunk, "main");
    }

    #[test]
    fn test_ancestors_from_leaf() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let ancestors = graph.ancestors("feature-a-2");
        assert_eq!(ancestors, vec!["feature-a-1", "feature-a", "main"]);
    }

    #[test]
    fn test_ancestors_from_middle() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let ancestors = graph.ancestors("feature-a-1");
        assert_eq!(ancestors, vec!["feature-a", "main"]);
    }

    #[test]
    fn test_ancestors_from_first_level() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let ancestors = graph.ancestors("feature-a");
        assert_eq!(ancestors, vec!["main"]);
    }

    #[test]
    fn test_ancestors_from_trunk() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let ancestors = graph.ancestors("main");
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_ancestors_nonexistent() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let ancestors = graph.ancestors("nonexistent");
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_descendants_from_trunk() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let mut descendants = graph.descendants("main");
        descendants.sort();
        assert_eq!(
            descendants,
            vec!["feature-a", "feature-a-1", "feature-a-2", "feature-b"]
        );
    }

    #[test]
    fn test_descendants_from_middle() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let mut descendants = graph.descendants("feature-a");
        descendants.sort();
        assert_eq!(descendants, vec!["feature-a-1", "feature-a-2"]);
    }

    #[test]
    fn test_descendants_from_leaf() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let descendants = graph.descendants("feature-a-2");
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_current_stack_from_leaf() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let current = graph.current_stack("feature-a-2");
        assert_eq!(
            current,
            vec!["main", "feature-a", "feature-a-1", "feature-a-2"]
        );
    }

    #[test]
    fn test_current_stack_from_middle() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let current = graph.current_stack("feature-a-1");
        assert_eq!(
            current,
            vec!["main", "feature-a", "feature-a-1", "feature-a-2"]
        );
    }

    #[test]
    fn test_current_stack_from_first_level() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let current = graph.current_stack("feature-b");
        assert_eq!(current, vec!["main", "feature-b"]);
    }

    #[test]
    fn test_ancestors_breaks_parent_cycles() {
        let store = MockStore::new()
            .with_trunk("main")
            .add_branch("a", "b", "rev-b", "rev-a")
            .add_branch("b", "a", "rev-a", "rev-b");

        let graph = StackGraph::load(&store).expect("load");
        assert_eq!(graph.ancestors("a"), vec!["b"]);
        assert_eq!(graph.current_stack("a"), vec!["b", "a"]);
    }

    #[test]
    fn test_descendants_breaks_child_cycles() {
        let store = MockStore::new()
            .with_trunk("main")
            .add_branch("a", "main", "trunk-rev", "rev-a")
            .add_branch("b", "a", "rev-a", "rev-b");

        let graph = StackGraph::load(&store).expect("load");

        // Manually create a cycle in children for testing
        if let Some(a) = graph.branches.get_mut("a") {
            a.children.push("b".to_string());
        }
        if let Some(b) = graph.branches.get_mut("b") {
            b.children.push("a".to_string());
        }

        assert_eq!(graph.descendants("a"), vec!["b"]);
    }

    #[test]
    fn test_needs_restack() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let mut needs = graph.needs_restack();
        needs.sort();
        // feature-b has mismatched parent revision (trunk-rev-old vs trunk-rev)
        assert_eq!(needs, vec!["feature-b"]);
    }

    #[test]
    fn test_get_siblings_with_sibling() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let siblings = graph.get_siblings("feature-a");
        assert!(siblings.contains(&"feature-a".to_string()));
        assert!(siblings.contains(&"feature-b".to_string()));
        assert_eq!(siblings.len(), 2);
    }

    #[test]
    fn test_get_siblings_only_child() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let siblings = graph.get_siblings("feature-a-1");
        assert_eq!(siblings, vec!["feature-a-1"]);
    }

    #[test]
    fn test_get_siblings_trunk() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let siblings = graph.get_siblings("main");
        assert_eq!(siblings, vec!["main"]);
    }

    #[test]
    fn test_get_siblings_nonexistent() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let siblings = graph.get_siblings("nonexistent");
        assert_eq!(siblings, vec!["nonexistent"]);
    }

    #[test]
    fn test_pr_info_preserved() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");

        let a1 = graph.branches.get("feature-a-1").expect("branch");
        assert_eq!(a1.pr_number, Some(2));
        assert_eq!(a1.pr_state, Some("OPEN".to_string()));

        let b = graph.branches.get("feature-b").expect("branch");
        assert_eq!(b.pr_number, Some(3));
        assert_eq!(b.pr_state, Some("MERGED".to_string()));
    }

    #[test]
    fn test_empty_stack() {
        let store = MockStore::new().with_trunk("main");
        let graph = StackGraph::load(&store).expect("load");
        // Only trunk exists
        assert_eq!(graph.branches.len(), 1);
        assert!(graph.branches.contains_key("main"));
    }

    #[test]
    fn test_default_trunk_when_not_set() {
        let store = MockStore::new();
        let graph = StackGraph::load(&store).expect("load");
        assert_eq!(graph.trunk, "main");
    }

    #[test]
    fn test_orphaned_branch_becomes_trunk_child() {
        let store = MockStore::new()
            .with_trunk("main")
            .add_branch("orphan", "deleted-parent", "rev-dp", "rev-orphan");

        let graph = StackGraph::load(&store).expect("load");
        let trunk = graph.branches.get("main").expect("trunk");
        assert!(trunk.children.contains(&"orphan".to_string()));
    }

    #[test]
    fn test_stack_node_clone() {
        let node = StackNode {
            name: "test".to_string(),
            parent: Some("parent".to_string()),
            children: vec!["child".to_string()],
            needs_restack: true,
            pr_number: Some(42),
            pr_state: Some("OPEN".to_string()),
            pr_is_draft: Some(false),
        };
        let cloned = node.clone();
        assert_eq!(cloned.name, node.name);
        assert_eq!(cloned.pr_number, node.pr_number);
    }

    #[test]
    fn test_stack_graph_clone() {
        let store = create_test_store();
        let graph = StackGraph::load(&store).expect("load");
        let cloned = graph.clone();
        assert_eq!(cloned.trunk, graph.trunk);
        assert_eq!(cloned.branches.len(), graph.branches.len());
    }

    #[test]
    fn test_branch_revision_none_for_deleted_branch() {
        let store = MockStore::new()
            .with_trunk("main");
        // No revision for "ghost" — should be pruned during load
        assert!(store.branch_revision("ghost").expect("ok").is_none());
    }
}

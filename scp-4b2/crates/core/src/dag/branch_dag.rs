//! `BranchDag` - Directed Acyclic Graph of branch relationships

use std::collections::{BTreeMap, BTreeSet};

use petgraph::{
    algo::{has_path_connecting, toposort},
    graph::{DiGraph, NodeIndex},
    visit::{Bfs, Reversed},
};

use crate::dag::types::{BranchId, DagError};

/// Directed Acyclic Graph of branch relationships
///
/// # Invariants
/// - DAG is acyclic (no circular dependencies exist)
/// - Every branch except trunk has at least one parent
/// - Every branch has a path to trunk (transitively through parents)
/// - Parent and child relationships are bidirectionally consistent
#[derive(Debug, Clone)]
pub struct BranchDag {
    /// `BranchId` -> Vec<BranchId> (parents of this branch)
    parents: BTreeMap<BranchId, Vec<BranchId>>,
    /// `BranchId` -> Vec<BranchId> (children of this branch)
    children: BTreeMap<BranchId, Vec<BranchId>>,
    /// Set of all branch IDs in the DAG
    branches: BTreeSet<BranchId>,
}

impl BranchDag {
    fn build_graph(&self) -> (DiGraph<BranchId, ()>, BTreeMap<BranchId, NodeIndex>) {
        let (graph, indices) = self.branches.iter().cloned().fold(
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
            .flat_map(|(child, parents)| {
                parents.iter().filter_map(|parent| {
                    indices
                        .get(parent)
                        .copied()
                        .zip(indices.get(child).copied())
                })
            })
            .fold(graph, |mut graph, (parent_idx, child_idx)| {
                graph.add_edge(parent_idx, child_idx, ());
                graph
            });

        (graph, indices)
    }

    /// Create a new empty `BranchDag` with trunk
    ///
    /// # Postconditions
    /// - Returns DAG with trunk branch (no parents)
    #[must_use]
    pub fn new() -> Self {
        let trunk = BranchId::new("trunk");
        let parents = BTreeMap::from_iter([(trunk.clone(), Vec::new())]);
        let children = BTreeMap::new();
        let branches = BTreeSet::from_iter([trunk]);

        Self {
            parents,
            children,
            branches,
        }
    }

    /// Add a branch with optional parents
    ///
    /// # Preconditions
    /// - `id` is not already present in the DAG
    /// - All `parent_ids` exist in the DAG
    /// - Adding these parents does not create a cycle
    ///
    /// # Postconditions
    /// - Branch added with specified parents
    /// - Children relationships updated consistently
    ///
    /// # Errors
    /// Returns `DagError::BranchAlreadyExists` if branch already exists.
    /// Returns `DagError::InvalidParent` if any parent doesn't exist.
    /// Returns `DagError::CycleDetected` if adding would create a cycle.
    pub fn add_branch(&mut self, id: BranchId, parent_ids: Vec<BranchId>) -> Result<(), DagError> {
        if parent_ids.iter().any(|p| p == &id) {
            return Err(DagError::CycleDetected(id));
        }

        if let Some(invalid_parent) = parent_ids
            .iter()
            .find(|parent_id| !self.branches.contains(*parent_id))
        {
            return Err(DagError::InvalidParent(invalid_parent.clone()));
        }

        if self.would_create_cycle(&id, &parent_ids) {
            return Err(DagError::CycleDetected(id));
        }

        if self.branches.contains(&id) {
            return Err(DagError::BranchAlreadyExists(id));
        }

        if !self.is_trunk(&id) && parent_ids.is_empty() {
            return Err(DagError::NoParentForBranch(id));
        }

        self.parents.insert(id.clone(), parent_ids.clone());
        self.branches.insert(id.clone());

        for parent_id in parent_ids {
            self.children.entry(parent_id).or_default().push(id.clone());
        }

        Ok(())
    }

    /// Remove a branch from the DAG
    ///
    /// # Preconditions
    /// - `id` exists in the DAG
    /// - Branch has no descendants
    ///
    /// # Postconditions
    /// - Branch removed from DAG
    /// - Parent relationships updated consistently
    ///
    /// # Errors
    /// Returns `DagError::BranchNotFound` if branch doesn't exist.
    /// Returns `DagError::HasDescendants` if branch has descendants.
    pub fn remove_branch(&mut self, id: BranchId) -> Result<(), DagError> {
        if !self.branches.contains(&id) {
            return Err(DagError::BranchNotFound(id));
        }

        let descendants = self.descendants(&id)?;
        if !descendants.is_empty() {
            return Err(DagError::HasDescendants(id, descendants.len()));
        }

        let parent_ids = self
            .parents
            .get(&id)
            .cloned()
            .map_or_else(Vec::new, std::convert::identity);

        self.parents.remove(&id);
        self.branches.remove(&id);

        for parent_id in parent_ids {
            if let Some(children) = self.children.get_mut(&parent_id) {
                children.retain(|c| c != &id);
            }
        }

        Ok(())
    }

    /// Get all ancestors of a branch (from current to trunk)
    ///
    /// # Precondition
    /// - `id` exists in the DAG
    ///
    /// # Postcondition
    /// - Returns all reachable ancestors (transitive closure of parents)
    ///
    /// # Errors
    /// Returns `DagError::BranchNotFound` if branch doesn't exist.
    pub fn ancestors(&self, id: &BranchId) -> Result<Vec<BranchId>, DagError> {
        if !self.branches.contains(id) {
            return Err(DagError::BranchNotFound(id.clone()));
        }

        let (graph, indices) = self.build_graph();
        let start = indices
            .get(id)
            .copied()
            .ok_or_else(|| DagError::BranchNotFound(id.clone()))?;
        let reversed = Reversed(&graph);
        let mut bfs = Bfs::new(reversed, start);

        Ok(std::iter::from_fn(|| bfs.next(reversed))
            .skip(1)
            .filter_map(|node_idx| graph.node_weight(node_idx).cloned())
            .collect())
    }

    /// Get all descendants of a branch (recursive)
    ///
    /// # Precondition
    /// - `id` exists in the DAG
    ///
    /// # Postcondition
    /// - Returns all reachable descendants (transitive closure of children)
    ///
    /// # Errors
    /// Returns `DagError::BranchNotFound` if branch doesn't exist.
    pub fn descendants(&self, id: &BranchId) -> Result<Vec<BranchId>, DagError> {
        if !self.branches.contains(id) {
            return Err(DagError::BranchNotFound(id.clone()));
        }

        let (graph, indices) = self.build_graph();
        let start = indices
            .get(id)
            .copied()
            .ok_or_else(|| DagError::BranchNotFound(id.clone()))?;
        let mut bfs = Bfs::new(&graph, start);

        Ok(std::iter::from_fn(|| bfs.next(&graph))
            .skip(1)
            .filter_map(|node_idx| graph.node_weight(node_idx).cloned())
            .collect())
    }

    /// Get path from branch to trunk
    ///
    /// # Precondition
    /// - `id` exists in the DAG
    ///
    /// # Postcondition
    /// - Returns chain from branch to trunk (empty if branch is trunk)
    ///
    /// # Errors
    /// Returns `DagError::BranchNotFound` if branch doesn't exist.
    pub fn path_to_root(&self, id: &BranchId) -> Result<Vec<BranchId>, DagError> {
        if !self.branches.contains(id) {
            return Err(DagError::BranchNotFound(id.clone()));
        }

        if self.is_trunk(id) {
            return Ok(Vec::new());
        }

        Ok(std::iter::successors(Some(id.clone()), |current| {
            (!self.is_trunk(current)).then(|| {
                self.parents
                    .get(current)
                    .and_then(|parents| parents.first().cloned())
            })?
        })
        .collect())
    }

    /// Get deterministic topological ordering
    ///
    /// # Postcondition
    /// - Returns branches in dependency order (parents before children)
    ///
    /// # Errors
    /// Returns `DagError::EmptyDag` if DAG is empty (shouldn't happen with trunk).
    pub fn topological_sort(&self) -> Result<Vec<BranchId>, DagError> {
        if self.branches.is_empty() {
            return Err(DagError::EmptyDag);
        }

        let (graph, _indices) = self.build_graph();
        let ordered = toposort(&graph, None).map_err(|cycle| {
            graph
                .node_weight(cycle.node_id())
                .cloned()
                .map_or(DagError::EmptyDag, DagError::CycleDetected)
        })?;

        Ok(ordered
            .into_iter()
            .filter_map(|node_idx| graph.node_weight(node_idx).cloned())
            .collect())
    }

    /// Check if branch exists in DAG
    #[must_use]
    pub fn contains(&self, id: &BranchId) -> bool {
        self.branches.contains(id)
    }

    /// Check if branch is trunk
    #[must_use]
    pub fn is_trunk(&self, id: &BranchId) -> bool {
        id.as_str() == "trunk"
    }

    /// Get the number of branches in the DAG
    #[must_use]
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Check if the DAG is empty (only trunk)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.branches.len() == 1
    }

    /// Get all branch IDs in the DAG (sorted)
    #[must_use]
    pub fn branch_ids(&self) -> Vec<BranchId> {
        self.branches.iter().cloned().collect()
    }

    /// Check if adding these parents would create a cycle
    /// Note: Self-reference is checked before calling this function
    fn would_create_cycle(&self, branch_id: &BranchId, parent_ids: &[BranchId]) -> bool {
        let (graph, indices) = self.build_graph();

        indices.get(branch_id).copied().is_some_and(|branch_idx| {
            parent_ids.iter().any(|parent_id| {
                indices.get(parent_id).copied().is_some_and(|parent_idx| {
                    has_path_connecting(&graph, branch_idx, parent_idx, None)
                })
            })
        })
    }
}

impl Default for BranchDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_dag_with_trunk_branch() {
        let dag = BranchDag::new();
        assert!(dag.contains(&BranchId::new("trunk")));
        assert!(dag.is_trunk(&BranchId::new("trunk")));
        assert_eq!(dag.len(), 1);
    }

    #[test]
    fn test_add_branch_creates_branch_with_parents() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add branch");
        assert!(dag.contains(&BranchId::new("feature")));
        assert_eq!(dag.len(), 2);
    }

    #[test]
    fn test_remove_branch_removes_branch_successfully() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.remove_branch(BranchId::new("feature"))
            .expect("Should remove");
        assert!(!dag.contains(&BranchId::new("feature")));
        assert_eq!(dag.len(), 1);
    }

    #[test]
    fn test_ancestors_returns_all_upstream_branches() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
            .expect("Should add hotfix");

        let ancestors = dag
            .ancestors(&BranchId::new("hotfix"))
            .expect("Should get ancestors");

        assert_eq!(
            ancestors,
            vec![BranchId::new("feature"), BranchId::new("trunk")]
        );
    }

    #[test]
    fn test_descendants_returns_all_downstream_branches() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
            .expect("Should add hotfix");

        let descendants = dag
            .descendants(&BranchId::new("trunk"))
            .expect("Should get descendants");

        assert_eq!(
            descendants,
            vec![BranchId::new("feature"), BranchId::new("hotfix")]
        );
    }

    #[test]
    fn test_path_to_root_returns_chain_to_trunk() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
            .expect("Should add hotfix");

        let path = dag
            .path_to_root(&BranchId::new("hotfix"))
            .expect("Should get path");

        assert_eq!(
            path,
            vec![
                BranchId::new("hotfix"),
                BranchId::new("feature"),
                BranchId::new("trunk")
            ]
        );
    }

    #[test]
    fn test_path_to_root_of_trunk_returns_empty() {
        let dag = BranchDag::new();
        let path = dag
            .path_to_root(&BranchId::new("trunk"))
            .expect("Should get path");
        assert!(path.is_empty());
    }

    #[test]
    fn test_topological_sort_returns_dependency_order() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("feature")])
            .expect("Should add hotfix");

        let sorted = dag.topological_sort().expect("Should sort");

        let trunk_idx = sorted
            .iter()
            .position(|b| b == &BranchId::new("trunk"))
            .unwrap();
        let feature_idx = sorted
            .iter()
            .position(|b| b == &BranchId::new("feature"))
            .unwrap();
        let hotfix_idx = sorted
            .iter()
            .position(|b| b == &BranchId::new("hotfix"))
            .unwrap();

        assert!(trunk_idx < feature_idx);
        assert!(feature_idx < hotfix_idx);
    }

    #[test]
    fn test_contains_returns_true_for_existing_branch() {
        let dag = BranchDag::new();
        assert!(dag.contains(&BranchId::new("trunk")));
    }

    #[test]
    fn test_is_trunk_returns_true_for_root_branch() {
        let dag = BranchDag::new();
        assert!(dag.is_trunk(&BranchId::new("trunk")));
    }

    #[test]
    fn test_add_branch_returns_error_when_branch_already_exists() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");

        let result = dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")]);
        assert!(matches!(result, Err(DagError::BranchAlreadyExists(_))));
    }

    #[test]
    fn test_add_branch_returns_error_when_parent_not_found() {
        let mut dag = BranchDag::new();
        let result = dag.add_branch(BranchId::new("feature"), vec![BranchId::new("nonexistent")]);
        assert!(matches!(result, Err(DagError::InvalidParent(_))));
    }

    #[test]
    fn test_add_branch_returns_error_when_cycle_detected() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
            .expect("Should add a");
        dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
            .expect("Should add b");

        let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("b")]);
        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_add_branch_returns_error_when_self_reference() {
        let mut dag = BranchDag::new();
        let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("a")]);
        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_add_branch_returns_error_when_indirect_cycle() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("a"), vec![BranchId::new("trunk")])
            .expect("Should add a");
        dag.add_branch(BranchId::new("b"), vec![BranchId::new("a")])
            .expect("Should add b");
        dag.add_branch(BranchId::new("c"), vec![BranchId::new("b")])
            .expect("Should add c");

        let result = dag.add_branch(BranchId::new("a"), vec![BranchId::new("c")]);
        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_remove_branch_returns_error_when_branch_not_found() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");

        let result = dag.remove_branch(BranchId::new("nonexistent"));
        assert!(matches!(result, Err(DagError::BranchNotFound(_))));
    }

    #[test]
    fn test_remove_branch_returns_error_when_has_descendants() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("main")])
            .expect("Should add feature");

        let result = dag.remove_branch(BranchId::new("main"));
        assert!(matches!(result, Err(DagError::HasDescendants(_, _))));
    }

    #[test]
    fn test_ancestors_returns_error_when_branch_not_found() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");

        let result = dag.ancestors(&BranchId::new("nonexistent"));
        assert!(matches!(result, Err(DagError::BranchNotFound(_))));
    }

    #[test]
    fn test_descendants_returns_error_when_branch_not_found() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");

        let result = dag.descendants(&BranchId::new("nonexistent"));
        assert!(matches!(result, Err(DagError::BranchNotFound(_))));
    }

    #[test]
    fn test_ancestors_of_trunk_returns_empty() {
        let dag = BranchDag::new();
        let ancestors = dag
            .ancestors(&BranchId::new("trunk"))
            .expect("Should get ancestors");
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_descendants_of_leaf_returns_empty() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");

        let descendants = dag
            .descendants(&BranchId::new("feature"))
            .expect("Should get descendants");
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_multiple_branches_same_parent() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature-a"), vec![BranchId::new("trunk")])
            .expect("Should add feature-a");
        dag.add_branch(BranchId::new("feature-b"), vec![BranchId::new("trunk")])
            .expect("Should add feature-b");

        assert_eq!(dag.len(), 3);
    }

    #[test]
    fn test_branch_with_multiple_parents() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("feature"), vec![BranchId::new("trunk")])
            .expect("Should add feature");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("trunk")])
            .expect("Should add hotfix");
        dag.add_branch(
            BranchId::new("release"),
            vec![BranchId::new("feature"), BranchId::new("hotfix")],
        )
        .expect("Should add release");

        let parents = dag
            .parents
            .get(&BranchId::new("release"))
            .cloned()
            .unwrap_or_default();
        assert_eq!(parents.len(), 2);
    }

    #[test]
    fn test_topological_sort_empty_dag() {
        let dag = BranchDag::new();
        let result = dag.topological_sort();
        let sorted = result.expect("Should sort");
        assert_eq!(sorted, vec![BranchId::new("trunk")]);
    }

    #[test]
    fn test_complex_dag_with_multiple_levels() {
        let mut dag = BranchDag::new();
        dag.add_branch(BranchId::new("main"), vec![BranchId::new("trunk")])
            .expect("Should add main");
        dag.add_branch(BranchId::new("feature-a"), vec![BranchId::new("main")])
            .expect("Should add feature-a");
        dag.add_branch(BranchId::new("feature-b"), vec![BranchId::new("main")])
            .expect("Should add feature-b");
        dag.add_branch(
            BranchId::new("release"),
            vec![BranchId::new("feature-a"), BranchId::new("feature-b")],
        )
        .expect("Should add release");
        dag.add_branch(BranchId::new("hotfix"), vec![BranchId::new("release")])
            .expect("Should add hotfix");

        let ancestors = dag
            .ancestors(&BranchId::new("hotfix"))
            .expect("Should get ancestors");
        assert!(ancestors.contains(&BranchId::new("release")));
        assert!(ancestors.contains(&BranchId::new("feature-a")));
        assert!(ancestors.contains(&BranchId::new("feature-b")));
        assert!(ancestors.contains(&BranchId::new("main")));
        assert!(ancestors.contains(&BranchId::new("trunk")));

        let descendants = dag
            .descendants(&BranchId::new("main"))
            .expect("Should get descendants");
        assert!(descendants.contains(&BranchId::new("feature-a")));
        assert!(descendants.contains(&BranchId::new("feature-b")));
        assert!(descendants.contains(&BranchId::new("release")));
        assert!(descendants.contains(&BranchId::new("hotfix")));
    }
}

//! `BranchDag` data structures and core implementation

use std::collections::{BTreeMap, BTreeSet};

use petgraph::{
    algo::has_path_connecting,
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
    pub(crate) parents: BTreeMap<BranchId, Vec<BranchId>>,
    /// `BranchId` -> Vec<BranchId> (children of this branch)
    pub(crate) children: BTreeMap<BranchId, Vec<BranchId>>,
    /// Set of all branch IDs in the DAG
    pub(crate) branches: BTreeSet<BranchId>,
}

impl BranchDag {
    /// Build the internal graph representation
    pub(crate) fn build_graph(&self) -> (DiGraph<BranchId, ()>, BTreeMap<BranchId, NodeIndex>) {
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
                    indices.get(parent).copied().zip(indices.get(child).copied())
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
}

impl Default for BranchDag {
    fn default() -> Self {
        Self::new()
    }
}

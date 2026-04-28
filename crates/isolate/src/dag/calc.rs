//! `BranchDag` Calc layer — pure computation operations.
//!
//! Add/remove branches, cycle detection, traversal, topological sort.
//! All operations return `Result` — zero panic in source code.

use std::collections::BTreeMap;

use petgraph::{
    algo::{has_path_connecting, toposort},
    graph::{DiGraph, NodeIndex},
    visit::{Bfs, Reversed},
};

use super::{
    data::BranchDag,
    types::{BranchId, DagError},
};

impl BranchDag {
    /// Build the internal petgraph representation.
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

    /// Add a branch with optional parents.
    ///
    /// # Errors
    ///
    /// Returns `DagError` if the branch already exists, a parent is invalid,
    /// the operation would create a cycle, or a non-trunk branch has no parents.
    pub fn add_branch(&mut self, id: BranchId, parent_ids: Vec<BranchId>) -> Result<(), DagError> {
        if parent_ids.iter().any(|p| p == &id) {
            return Err(DagError::CycleDetected(id));
        }

        if let Some(invalid) = parent_ids.iter().find(|pid| !self.branches.contains(*pid)) {
            return Err(DagError::InvalidParent(invalid.clone()));
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

    /// Remove a branch from the DAG.
    ///
    /// # Errors
    ///
    /// Returns `DagError` if the branch doesn't exist or has descendants.
    pub fn remove_branch(&mut self, id: BranchId) -> Result<(), DagError> {
        if !self.branches.contains(&id) {
            return Err(DagError::BranchNotFound(id));
        }

        let descendants = self.descendants(&id)?;
        if !descendants.is_empty() {
            return Err(DagError::HasDescendants(id, descendants.len()));
        }

        let parent_ids = self.parents.get(&id).cloned().unwrap_or_else(Vec::new);

        self.parents.remove(&id);
        self.branches.remove(&id);

        for parent_id in parent_ids {
            if let Some(children) = self.children.get_mut(&parent_id) {
                children.retain(|c| c != &id);
            }
        }

        Ok(())
    }

    /// Get all ancestors of a branch (from current to trunk).
    ///
    /// # Errors
    ///
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

    /// Get all descendants of a branch (BFS traversal).
    ///
    /// # Errors
    ///
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

    /// Check if adding parents would create a cycle.
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

    /// Get path from branch to trunk.
    ///
    /// # Errors
    ///
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

    /// Get deterministic topological ordering (parents before children).
    ///
    /// # Errors
    ///
    /// Returns `DagError::EmptyDag` if DAG is empty.
    /// Returns `DagError::CycleDetected` if a cycle is detected.
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
}

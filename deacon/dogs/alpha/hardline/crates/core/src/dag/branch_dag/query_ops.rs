//! `BranchDag` query operations for paths and sorting

use crate::dag::types::{BranchId, DagError};

impl crate::dag::branch_dag::BranchDag {
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
        let ordered = petgraph::algo::toposort(&graph, None).map_err(|cycle| {
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

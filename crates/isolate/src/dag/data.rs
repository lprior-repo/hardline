//! `BranchDag` data structure — inert struct definition and constructors.
//!
//! This is the **Data** layer: the struct, its fields, constructors, and
//! read-only accessors. No business logic lives here.

use std::collections::{BTreeMap, BTreeSet};

use super::types::BranchId;

/// Directed Acyclic Graph of branch relationships.
///
/// # Invariants
///
/// - DAG is acyclic (no circular dependencies exist)
/// - Every branch except trunk has at least one parent
/// - Every branch has a path to trunk (transitively through parents)
/// - Parent and child relationships are bidirectionally consistent
#[derive(Debug, Clone)]
pub struct BranchDag {
    /// `BranchId` → `Vec<BranchId>` (parents of this branch).
    pub(crate) parents: BTreeMap<BranchId, Vec<BranchId>>,
    /// `BranchId` → `Vec<BranchId>` (children of this branch).
    pub(crate) children: BTreeMap<BranchId, Vec<BranchId>>,
    /// Set of all branch IDs in the DAG.
    pub(crate) branches: BTreeSet<BranchId>,
}

impl BranchDag {
    /// Create a new empty `BranchDag` with trunk.
    ///
    /// # Postconditions
    ///
    /// - Returns DAG with trunk branch (no parents).
    #[must_use]
    pub fn new() -> Self {
        let trunk = BranchId::new("trunk");
        Self {
            parents: BTreeMap::from_iter([(trunk.clone(), Vec::new())]),
            children: BTreeMap::new(),
            branches: BTreeSet::from_iter([trunk]),
        }
    }

    /// Check if branch exists in DAG.
    #[must_use]
    pub fn contains(&self, id: &BranchId) -> bool {
        self.branches.contains(id)
    }

    /// Check if branch is trunk.
    #[must_use]
    pub fn is_trunk(&self, id: &BranchId) -> bool {
        id.as_str() == "trunk"
    }

    /// Get the number of branches in the DAG.
    #[must_use]
    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Check if the DAG is empty (only trunk).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.branches.len() == 1
    }

    /// Get all branch IDs in the DAG (sorted).
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

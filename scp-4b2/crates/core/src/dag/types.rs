//! `BranchDag` types

use std::fmt;

use thiserror::Error;

/// Unique identifier for a branch in the DAG
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct BranchId(String);

impl BranchId {
    /// Create a new branch ID
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the branch ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BranchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors for `BranchDag` operations
#[derive(Debug, Error)]
pub enum DagError {
    /// `BranchId` already exists in DAG
    #[error("branch already exists: {0}")]
    BranchAlreadyExists(BranchId),

    /// `BranchId` not found in DAG
    #[error("branch not found: {0}")]
    BranchNotFound(BranchId),

    /// Adding parent would create a cycle in the DAG
    #[error("adding parent would create cycle for branch {0}")]
    CycleDetected(BranchId),

    /// Cannot remove branch that has descendants
    #[error("cannot remove branch {0} with {1} descendants")]
    HasDescendants(BranchId, usize),

    /// Invalid parent relationship (parent ID doesn't exist)
    #[error("invalid parent: {0}")]
    InvalidParent(BranchId),

    /// Operation requires non-empty DAG
    #[error("DAG is empty")]
    EmptyDag,

    /// Non-trunk branch must have at least one parent (invariant I2)
    #[error("branch {0} requires at least one parent")]
    NoParentForBranch(BranchId),
}

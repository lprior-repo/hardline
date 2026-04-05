//! Value types for the Branch DAG domain.

use std::fmt;

use thiserror::Error;

/// Unique identifier for a branch in the DAG.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct BranchId(String);

impl BranchId {
    /// Create a new branch ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the branch ID as a string slice.
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

/// Errors for `BranchDag` operations.
#[derive(Debug, Error)]
pub enum DagError {
    /// `BranchId` already exists in DAG.
    #[error("branch already exists: {0}")]
    BranchAlreadyExists(BranchId),

    /// `BranchId` not found in DAG.
    #[error("branch not found: {0}")]
    BranchNotFound(BranchId),

    /// Adding parent would create a cycle in the DAG.
    #[error("adding parent would create cycle for branch {0}")]
    CycleDetected(BranchId),

    /// Cannot remove branch that has descendants.
    #[error("cannot remove branch {0} with {1} descendants")]
    HasDescendants(BranchId, usize),

    /// Invalid parent relationship (parent ID doesn't exist).
    #[error("invalid parent: {0}")]
    InvalidParent(BranchId),

    /// Operation requires non-empty DAG.
    #[error("DAG is empty")]
    EmptyDag,

    /// Non-trunk branch must have at least one parent.
    #[error("branch {0} requires at least one parent")]
    NoParentForBranch(BranchId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_id_new() {
        let id = BranchId::new("feature-branch");
        assert_eq!(id.as_str(), "feature-branch");
    }

    #[test]
    fn branch_id_display() {
        assert_eq!(format!("{}", BranchId::new("main")), "main");
    }

    #[test]
    fn branch_id_equality() {
        assert_eq!(BranchId::new("trunk"), BranchId::new("trunk"));
        assert_ne!(BranchId::new("trunk"), BranchId::new("develop"));
    }

    #[test]
    fn branch_id_ord() {
        assert!(BranchId::new("alpha") < BranchId::new("beta"));
    }

    #[test]
    fn dag_error_displays() {
        assert!(format!("{}", DagError::BranchAlreadyExists(BranchId::new("x"))).contains("x"));
        assert!(format!("{}", DagError::BranchNotFound(BranchId::new("y"))).contains("y"));
        assert!(format!("{}", DagError::CycleDetected(BranchId::new("z"))).contains("z"));
        assert!(format!("{}", DagError::HasDescendants(BranchId::new("p"), 3)).contains("3"));
        assert!(format!("{}", DagError::InvalidParent(BranchId::new("q"))).contains("q"));
        assert!(format!("{}", DagError::EmptyDag).contains("empty"));
        assert!(format!("{}", DagError::NoParentForBranch(BranchId::new("r"))).contains("r"));
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── BranchId construction ────────────────────────────────────────────────

    #[test]
    fn test_branch_id_new() {
        let id = BranchId::new("feature-branch");
        assert_eq!(id.as_str(), "feature-branch");
    }

    #[test]
    fn test_branch_id_display() {
        let id = BranchId::new("main");
        assert_eq!(format!("{id}"), "main");
    }

    #[test]
    fn test_branch_id_equality() {
        let a = BranchId::new("trunk");
        let b = BranchId::new("trunk");
        let c = BranchId::new("develop");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_branch_id_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        let id = BranchId::new("feature");
        map.insert(id.clone(), 42);
        assert_eq!(map.get(&BranchId::new("feature")), Some(&42));
    }

    #[test]
    fn test_branch_id_ord() {
        let a = BranchId::new("alpha");
        let b = BranchId::new("beta");
        assert!(a < b);
        // Ord is based on string lexicographic order
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_branch_id_clone() {
        let original = BranchId::new("my-branch");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ── DagError Display ─────────────────────────────────────────────────────

    #[test]
    fn test_dag_error_branch_already_exists_display() {
        let err = DagError::BranchAlreadyExists(BranchId::new("feature"));
        let msg = format!("{err}");
        assert!(msg.contains("branch already exists"));
        assert!(msg.contains("feature"));
    }

    #[test]
    fn test_dag_error_branch_not_found_display() {
        let err = DagError::BranchNotFound(BranchId::new("missing"));
        let msg = format!("{err}");
        assert!(msg.contains("branch not found"));
        assert!(msg.contains("missing"));
    }

    #[test]
    fn test_dag_error_cycle_detected_display() {
        let err = DagError::CycleDetected(BranchId::new("cyclic"));
        let msg = format!("{err}");
        assert!(msg.contains("cycle"));
        assert!(msg.contains("cyclic"));
    }

    #[test]
    fn test_dag_error_has_descendants_display() {
        let err = DagError::HasDescendants(BranchId::new("parent"), 3);
        let msg = format!("{err}");
        assert!(msg.contains("cannot remove"));
        assert!(msg.contains("parent"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_dag_error_invalid_parent_display() {
        let err = DagError::InvalidParent(BranchId::new("nonexistent"));
        let msg = format!("{err}");
        assert!(msg.contains("invalid parent"));
        assert!(msg.contains("nonexistent"));
    }

    #[test]
    fn test_dag_error_empty_dag_display() {
        let err = DagError::EmptyDag;
        let msg = format!("{err}");
        assert!(msg.contains("DAG is empty"));
    }

    #[test]
    fn test_dag_error_no_parent_for_branch_display() {
        let err = DagError::NoParentForBranch(BranchId::new("orphan"));
        let msg = format!("{err}");
        assert!(msg.contains("requires at least one parent"));
        assert!(msg.contains("orphan"));
    }

    // ── DagError is std::error::Error ────────────────────────────────────────

    #[test]
    fn test_dag_error_is_error() {
        let err: Box<dyn std::error::Error> = Box::new(DagError::EmptyDag);
        let _ = format!("{err}");
    }
}

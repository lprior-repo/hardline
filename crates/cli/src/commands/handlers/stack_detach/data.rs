//! Data layer for stack detach - inert, serializable types.
//!
//! No business logic. Types only.

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack detach command.
#[derive(Debug, Clone)]
pub struct StackDetachOptions {
    /// Branch to detach from the stack.
    pub branch: BranchName,
    /// Trunk branch name (e.g., "main").
    pub trunk: BranchName,
    /// Skip confirmation prompts (polecat mode).
    pub force: bool,
    /// Delete the local git branch after detaching.
    pub delete_branch: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a stack detach operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackDetachResult {
    /// Branch that was detached.
    pub branch: BranchName,
    /// Previous parent of the detached branch.
    pub previous_parent: BranchName,
    /// Children that were reparented to the detached branch's parent.
    pub reparented_children: Vec<BranchName>,
    /// Whether the local branch was deleted.
    pub branch_deleted: bool,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack detach operations.
#[derive(Debug, thiserror::Error)]
pub enum DetachError {
    /// Branch has no metadata (not tracked in the stack).
    #[error("Branch '{0}' is not tracked in any stack")]
    NotTracked(BranchName),
    /// Branch is the trunk and cannot be detached.
    #[error("Cannot detach trunk branch '{0}'")]
    CannotDetachTrunk(BranchName),
    /// Branch has no parent (already a root).
    #[error("Branch '{0}' has no parent to detach from")]
    NoParent(BranchName),
    /// Workspace has uncommitted changes.
    #[error("Workspace has uncommitted changes. Stash or commit first.")]
    DirtyWorkspace,
    /// Failed to delete the branch.
    #[error("Failed to delete branch '{branch}': {reason}")]
    BranchDeleteFailed { branch: BranchName, reason: String },
    /// Metadata operation failed.
    #[error("Metadata error: {0}")]
    MetadataError(String),
    /// I/O error.
    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_detach_options_fields() {
        let opts = StackDetachOptions {
            branch: BranchName::new("feature-a".to_string()),
            trunk: BranchName::new("main".to_string()),
            force: false,
            delete_branch: false,
        };
        assert_eq!(opts.branch.as_str(), "feature-a");
        assert_eq!(opts.trunk.as_str(), "main");
        assert!(!opts.force);
        assert!(!opts.delete_branch);
    }

    #[test]
    fn stack_detach_result_serialization() {
        let result = StackDetachResult {
            branch: BranchName::new("feature-a".to_string()),
            previous_parent: BranchName::new("main".to_string()),
            reparented_children: vec![BranchName::new("feature-a-1".to_string())],
            branch_deleted: true,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: StackDetachResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.branch.as_str(), "feature-a");
        assert_eq!(back.previous_parent.as_str(), "main");
        assert_eq!(back.reparented_children.len(), 1);
        assert!(back.branch_deleted);
    }

    #[test]
    fn detach_error_display_not_tracked() {
        let err = DetachError::NotTracked(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("not tracked"));
    }

    #[test]
    fn detach_error_display_cannot_detach_trunk() {
        let err = DetachError::CannotDetachTrunk(BranchName::new("main".to_string()));
        assert!(err.to_string().contains("trunk"));
    }

    #[test]
    fn detach_error_display_no_parent() {
        let err = DetachError::NoParent(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("no parent"));
    }

    #[test]
    fn detach_error_display_dirty_workspace() {
        let err = DetachError::DirtyWorkspace;
        assert!(err.to_string().contains("uncommitted"));
    }

    #[test]
    fn detach_error_display_branch_delete_failed() {
        let err = DetachError::BranchDeleteFailed {
            branch: BranchName::new("feat".to_string()),
            reason: "checked out".to_string(),
        };
        assert!(err.to_string().contains("feat"));
        assert!(err.to_string().contains("checked out"));
    }
}

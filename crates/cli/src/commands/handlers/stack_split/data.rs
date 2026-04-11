//! Data layer for stack split - inert, serializable types.
//!
//! No business logic. Types only.

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack split command.
#[derive(Debug, Clone)]
pub struct StackSplitOptions {
    /// Branch to split.
    pub branch: BranchName,
    /// Trunk branch name (e.g., "main").
    pub trunk: BranchName,
    /// Commit hash at which to split the branch.
    /// Commits up to and including this hash go into the lower branch.
    /// Commits after this hash go into the upper branch.
    pub at_commit: String,
    /// Name for the lower branch (commits before/at split point).
    /// If None, the original branch name is reused with a "-1" suffix.
    pub lower_name: Option<BranchName>,
    /// Name for the upper branch (commits after split point).
    /// If None, the original branch name is reused with a "-2" suffix.
    pub upper_name: Option<BranchName>,
    /// Skip confirmation prompts (polecat mode).
    pub force: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a stack split operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackSplitResult {
    /// Original branch that was split.
    pub source_branch: BranchName,
    /// Lower branch created by the split (commits up to split point).
    pub lower_branch: BranchName,
    /// Upper branch created by the split (commits after split point).
    pub upper_branch: BranchName,
    /// Commit at which the split occurred.
    pub split_commit: String,
    /// Children that were reparented to the upper branch.
    pub reparented_children: Vec<BranchName>,
}

/// Plan computed before executing the split.
#[derive(Debug, Clone)]
pub struct SplitPlan {
    /// Lower branch name.
    pub lower_branch: BranchName,
    /// Upper branch name.
    pub upper_branch: BranchName,
    /// Parent of the lower branch (original branch's parent).
    pub lower_parent: BranchName,
    /// Revision of the lower parent (for metadata).
    pub lower_parent_revision: String,
    /// Commit hash at the split point (becomes tip of lower branch).
    pub split_commit: String,
    /// Current tip of the source branch (becomes tip of upper branch).
    pub source_tip: String,
    /// Children of the original branch that need reparenting to upper.
    pub children_to_reparent: Vec<BranchName>,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack split operations.
#[derive(Debug, thiserror::Error)]
pub enum SplitError {
    /// Branch has no metadata (not tracked in the stack).
    #[error("Branch '{0}' is not tracked in any stack")]
    NotTracked(BranchName),
    /// Branch is the trunk and cannot be split.
    #[error("Cannot split trunk branch '{0}'")]
    CannotSplitTrunk(BranchName),
    /// Split commit not found on the branch.
    #[error("Commit '{commit}' is not found on branch '{branch}'")]
    SplitCommitNotFound { branch: BranchName, commit: String },
    /// Split commit is the tip — nothing to split after it.
    #[error("Commit '{commit}' is the tip of branch '{branch}'. Nothing to split.")]
    SplitCommitIsTip { branch: BranchName, commit: String },
    /// Split commit is the base — nothing to split before it.
    #[error("Commit '{commit}' is the base of branch '{branch}'. Nothing to split before it.")]
    SplitCommitIsBase { branch: BranchName, commit: String },
    /// Branch has no parent.
    #[error("Branch '{0}' has no parent in the stack")]
    NoParent(BranchName),
    /// Workspace has uncommitted changes.
    #[error("Workspace has uncommitted changes. Stash or commit first.")]
    DirtyWorkspace,
    /// A target branch name already exists.
    #[error("Branch '{0}' already exists")]
    BranchAlreadyExists(BranchName),
    /// Metadata operation failed.
    #[error("Metadata error: {0}")]
    MetadataError(String),
    /// I/O error.
    #[error("IO error: {0}")]
    IoError(String),
    /// Git operation failed.
    #[error("Git error: {0}")]
    GitError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_split_options_fields() {
        let opts = StackSplitOptions {
            branch: BranchName::new("feature-a".to_string()),
            trunk: BranchName::new("main".to_string()),
            at_commit: "abc123".to_string(),
            lower_name: Some(BranchName::new("feature-a-1".to_string())),
            upper_name: Some(BranchName::new("feature-a-2".to_string())),
            force: false,
        };
        assert_eq!(opts.branch.as_str(), "feature-a");
        assert_eq!(opts.at_commit, "abc123");
        assert!(opts.lower_name.is_some());
        assert!(opts.upper_name.is_some());
        assert!(!opts.force);
    }

    #[test]
    fn stack_split_options_default_names() {
        let opts = StackSplitOptions {
            branch: BranchName::new("feature".to_string()),
            trunk: BranchName::new("main".to_string()),
            at_commit: "def456".to_string(),
            lower_name: None,
            upper_name: None,
            force: true,
        };
        assert!(opts.lower_name.is_none());
        assert!(opts.upper_name.is_none());
        assert!(opts.force);
    }

    #[test]
    fn stack_split_result_serialization() {
        let result = StackSplitResult {
            source_branch: BranchName::new("feature-a".to_string()),
            lower_branch: BranchName::new("feature-a-1".to_string()),
            upper_branch: BranchName::new("feature-a-2".to_string()),
            split_commit: "abc123".to_string(),
            reparented_children: vec![BranchName::new("feature-a-child".to_string())],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: StackSplitResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.source_branch.as_str(), "feature-a");
        assert_eq!(back.lower_branch.as_str(), "feature-a-1");
        assert_eq!(back.upper_branch.as_str(), "feature-a-2");
        assert_eq!(back.split_commit, "abc123");
        assert_eq!(back.reparented_children.len(), 1);
    }

    #[test]
    fn split_plan_fields() {
        let plan = SplitPlan {
            lower_branch: BranchName::new("feature-a-1".to_string()),
            upper_branch: BranchName::new("feature-a-2".to_string()),
            lower_parent: BranchName::new("main".to_string()),
            lower_parent_revision: "rev-main".to_string(),
            split_commit: "abc123".to_string(),
            source_tip: "def456".to_string(),
            children_to_reparent: vec![],
        };
        assert_eq!(plan.lower_branch.as_str(), "feature-a-1");
        assert_eq!(plan.upper_branch.as_str(), "feature-a-2");
        assert!(plan.children_to_reparent.is_empty());
    }

    #[test]
    fn split_error_display_not_tracked() {
        let err = SplitError::NotTracked(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("not tracked"));
    }

    #[test]
    fn split_error_display_cannot_split_trunk() {
        let err = SplitError::CannotSplitTrunk(BranchName::new("main".to_string()));
        assert!(err.to_string().contains("trunk"));
    }

    #[test]
    fn split_error_display_split_commit_not_found() {
        let err = SplitError::SplitCommitNotFound {
            branch: BranchName::new("feat".to_string()),
            commit: "abc".to_string(),
        };
        assert!(err.to_string().contains("abc"));
        assert!(err.to_string().contains("feat"));
    }

    #[test]
    fn split_error_display_split_commit_is_tip() {
        let err = SplitError::SplitCommitIsTip {
            branch: BranchName::new("feat".to_string()),
            commit: "abc".to_string(),
        };
        assert!(err.to_string().contains("tip"));
    }

    #[test]
    fn split_error_display_split_commit_is_base() {
        let err = SplitError::SplitCommitIsBase {
            branch: BranchName::new("feat".to_string()),
            commit: "abc".to_string(),
        };
        assert!(err.to_string().contains("base"));
    }

    #[test]
    fn split_error_display_no_parent() {
        let err = SplitError::NoParent(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("no parent"));
    }

    #[test]
    fn split_error_display_dirty_workspace() {
        let err = SplitError::DirtyWorkspace;
        assert!(err.to_string().contains("uncommitted"));
    }

    #[test]
    fn split_error_display_branch_already_exists() {
        let err = SplitError::BranchAlreadyExists(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn split_error_display_metadata_error() {
        let err = SplitError::MetadataError("read failed".to_string());
        assert!(err.to_string().contains("read failed"));
    }

    #[test]
    fn split_error_display_io_error() {
        let err = SplitError::IoError("disk full".to_string());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn split_error_display_git_error() {
        let err = SplitError::GitError("merge conflict".to_string());
        assert!(err.to_string().contains("merge conflict"));
    }
}

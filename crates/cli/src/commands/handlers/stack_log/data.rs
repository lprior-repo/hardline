//! Data layer for stack log - inert, serializable types.
//!
//! No business logic. Types only.

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack log command (parsed from CLI or API).
#[derive(Debug, Clone)]
pub struct StackLogOptions {
    /// Maximum commits to show per branch.
    pub limit: Option<usize>,
    /// Output format: "tree", "linear", or "json".
    pub format: String,
    /// Include commit messages in output.
    pub include_messages: bool,
    /// Show ahead/behind counts relative to parent.
    pub show_ahead_behind: bool,
    /// Filter to a specific branch and its ancestors/descendants.
    pub branch_filter: Option<BranchName>,
}

impl Default for StackLogOptions {
    fn default() -> Self {
        Self {
            limit: None,
            format: "tree".to_string(),
            include_messages: true,
            show_ahead_behind: false,
            branch_filter: None,
        }
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// A single branch entry in the stack log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackLogBranchEntry {
    /// Branch name.
    pub branch: BranchName,
    /// Parent branch (None for trunk or root branches).
    pub parent: Option<BranchName>,
    /// Depth in the stack tree (0 for trunk, 1 for direct children, etc.).
    pub depth: usize,
    /// Commits unique to this branch (not in parent).
    pub commits: Vec<StackLogCommit>,
    /// Number of commits ahead of parent.
    pub ahead: usize,
    /// Number of commits behind parent.
    pub behind: usize,
    /// Whether this branch needs restacking.
    pub needs_restack: bool,
    /// Associated PR number, if any.
    pub pr_number: Option<u64>,
    /// PR state, if any.
    pub pr_state: Option<String>,
}

/// A single commit in the stack log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackLogCommit {
    /// Short commit hash (10 chars).
    pub short_hash: String,
    /// Full commit hash.
    pub hash: String,
    /// Commit message (first line).
    pub message: String,
    /// Author name.
    pub author: String,
    /// Commit timestamp in ISO 8601 format.
    pub datetime: String,
}

/// Complete stack log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackLogOutput {
    /// All branch entries in topological order.
    pub branches: Vec<StackLogBranchEntry>,
    /// The trunk/base branch name.
    pub trunk: BranchName,
    /// Total number of branches in the stack.
    pub total_branches: usize,
    /// Total number of commits across all branches.
    pub total_commits: usize,
    /// Branches that need restacking.
    pub needs_restack: Vec<BranchName>,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack log operations.
#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("Not in a git repository")]
    NotGitRepo,
    #[error("No stack metadata found. Run 'scp workspace sync' first.")]
    NoStackMetadata,
    #[error("Branch not found in stack: {0}")]
    BranchNotFound(BranchName),
    #[error("VCS error: {0}")]
    VcsError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<LogError> for scp_core::Error {
    fn from(err: LogError) -> Self {
        scp_core::Error::vcs_conflict("stack-log", err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_log_options_default() {
        let opts = StackLogOptions::default();
        assert!(opts.limit.is_none());
        assert_eq!(opts.format, "tree");
        assert!(opts.include_messages);
        assert!(!opts.show_ahead_behind);
        assert!(opts.branch_filter.is_none());
    }

    #[test]
    fn stack_log_commit_serialization() {
        let commit = StackLogCommit {
            short_hash: "abc1234567".to_string(),
            hash: "abc1234567890def".to_string(),
            message: "Fix bug".to_string(),
            author: "alice".to_string(),
            datetime: "2026-04-09T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&commit).expect("serialize");
        let back: StackLogCommit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.short_hash, "abc1234567");
        assert_eq!(back.message, "Fix bug");
    }

    #[test]
    fn stack_log_branch_entry_serialization() {
        let entry = StackLogBranchEntry {
            branch: BranchName::new("feature-a".to_string()),
            parent: Some(BranchName::new("main".to_string())),
            depth: 1,
            commits: vec![],
            ahead: 3,
            behind: 0,
            needs_restack: false,
            pr_number: Some(42),
            pr_state: Some("open".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: StackLogBranchEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.depth, 1);
        assert_eq!(back.ahead, 3);
    }

    #[test]
    fn stack_log_output_serialization() {
        let output = StackLogOutput {
            branches: vec![],
            trunk: BranchName::new("main".to_string()),
            total_branches: 0,
            total_commits: 0,
            needs_restack: vec![],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let back: StackLogOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.total_branches, 0);
        assert_eq!(back.trunk, BranchName::new("main".to_string()));
    }

    #[test]
    fn log_error_display() {
        let err = LogError::NotGitRepo;
        assert!(err.to_string().contains("git"));

        let err = LogError::NoStackMetadata;
        assert!(err.to_string().contains("sync"));

        let err = LogError::BranchNotFound(BranchName::new("feat".to_string()));
        assert!(err.to_string().contains("feat"));
    }
}

//! Data layer for stack diff - inert, serializable types.
//!
//! No business logic. Types only.

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack diff command (parsed from CLI or API).
#[derive(Debug, Clone, Default)]
pub struct StackDiffOptions {
    /// Only diff branches within this range (start..end branch names).
    /// None means diff the entire stack.
    pub range: Option<DiffRange>,
    /// Show stat summary only (not full diff).
    pub stat_only: bool,
    /// Show color in diff output.
    pub color: bool,
}

/// A range of branches within the stack to diff.
#[derive(Debug, Clone)]
pub struct DiffRange {
    /// Start branch (inclusive). If None, start from trunk.
    pub start: Option<BranchName>,
    /// End branch (inclusive). If None, go to the tip.
    pub end: Option<BranchName>,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a stack diff operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackDiffResult {
    /// Per-branch diff results, in topological order.
    pub branch_diffs: Vec<BranchDiff>,
    /// Total additions across all branches.
    pub total_additions: usize,
    /// Total deletions across all branches.
    pub total_deletions: usize,
    /// Total files changed across all branches.
    pub total_files_changed: usize,
}

/// Diff result for a single branch within the stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDiff {
    /// Branch name.
    pub branch: BranchName,
    /// Parent branch name (or trunk).
    pub parent: BranchName,
    /// File-level diff stats.
    pub file_stats: Vec<FileStat>,
    /// Full diff output (empty when stat_only is true).
    pub diff_lines: Vec<String>,
    /// Total additions in this branch.
    pub additions: usize,
    /// Total deletions in this branch.
    pub deletions: usize,
}

/// File-level diff statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    /// File path.
    pub path: String,
    /// Number of lines added.
    pub additions: usize,
    /// Number of lines deleted.
    pub deletions: usize,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack diff operations.
#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("No stack branches tracked")]
    NoStackBranches,
    #[error("Branch not found in stack: {0}")]
    BranchNotFound(String),
    #[error("Range start branch '{start}' is not an ancestor of end branch '{end}'")]
    InvalidRange { start: String, end: String },
    #[error("VCS backend error: {0}")]
    BackendError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

// ============================================================================
// Helper constructors for tests
// ============================================================================

impl StackDiffResult {
    /// Create an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

impl BranchDiff {
    /// Create a new branch diff with computed totals from file_stats.
    #[must_use]
    pub fn new(branch: BranchName, parent: BranchName, file_stats: Vec<FileStat>) -> Self {
        let additions: usize = file_stats.iter().map(|f| f.additions).sum();
        let deletions: usize = file_stats.iter().map(|f| f.deletions).sum();
        Self {
            branch,
            parent,
            file_stats,
            diff_lines: Vec::new(),
            additions,
            deletions,
        }
    }

    /// Create a branch diff with full diff output.
    #[must_use]
    pub fn with_diff_lines(mut self, lines: Vec<String>) -> Self {
        self.diff_lines = lines;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_diff_options_default() {
        let opts = StackDiffOptions::default();
        assert!(opts.range.is_none());
        assert!(!opts.stat_only);
        assert!(!opts.color);
    }

    #[test]
    fn stack_diff_result_default() {
        let result = StackDiffResult::default();
        assert!(result.branch_diffs.is_empty());
        assert_eq!(result.total_additions, 0);
        assert_eq!(result.total_deletions, 0);
        assert_eq!(result.total_files_changed, 0);
    }

    #[test]
    fn stack_diff_result_empty() {
        let result = StackDiffResult::empty();
        assert!(result.branch_diffs.is_empty());
    }

    #[test]
    fn branch_diff_new_computes_totals() {
        let stats = vec![
            FileStat {
                path: "a.rs".to_string(),
                additions: 10,
                deletions: 2,
            },
            FileStat {
                path: "b.rs".to_string(),
                additions: 5,
                deletions: 3,
            },
        ];
        let bd = BranchDiff::new(
            BranchName::new("feat".to_string()),
            BranchName::new("main".to_string()),
            stats,
        );
        assert_eq!(bd.additions, 15);
        assert_eq!(bd.deletions, 5);
        assert!(bd.diff_lines.is_empty());
    }

    #[test]
    fn branch_diff_with_diff_lines() {
        let bd = BranchDiff::new(
            BranchName::new("feat".to_string()),
            BranchName::new("main".to_string()),
            vec![],
        )
        .with_diff_lines(vec!["+hello".to_string()]);
        assert_eq!(bd.diff_lines.len(), 1);
    }

    #[test]
    fn diff_error_display_no_stack_branches() {
        let err = DiffError::NoStackBranches;
        assert!(err.to_string().contains("No stack"));
    }

    #[test]
    fn diff_error_display_branch_not_found() {
        let err = DiffError::BranchNotFound("ghost".to_string());
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn diff_error_display_invalid_range() {
        let err = DiffError::InvalidRange {
            start: "a".to_string(),
            end: "b".to_string(),
        };
        assert!(err.to_string().contains("a"));
        assert!(err.to_string().contains("b"));
    }

    #[test]
    fn diff_error_display_backend_error() {
        let err = DiffError::BackendError("fail".to_string());
        assert!(err.to_string().contains("fail"));
    }

    #[test]
    fn file_stat_fields() {
        let fs = FileStat {
            path: "src/main.rs".to_string(),
            additions: 42,
            deletions: 7,
        };
        assert_eq!(fs.path, "src/main.rs");
        assert_eq!(fs.additions, 42);
        assert_eq!(fs.deletions, 7);
    }
}

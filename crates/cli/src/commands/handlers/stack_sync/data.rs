//! Data layer for stack sync - inert, serializable types.
//!
//! No business logic. Types only.

use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack sync command (parsed from CLI or API).
#[derive(Debug, Clone)]
pub struct StackSyncOptions {
    /// Auto-restack branches after sync.
    pub restack: bool,
    /// Fetch all refs (vs trunk-only).
    pub full_fetch: bool,
    /// Delete branches detected as merged.
    pub delete_merged: bool,
    /// Delete branches whose upstream tracking is gone.
    pub delete_upstream_gone: bool,
    /// Skip confirmation prompts (polecat mode).
    pub force: bool,
    /// Avoid hard reset when trunk update fails.
    pub safe: bool,
    /// Remote name to fetch from.
    pub remote_name: String,
    /// Trunk branch name (e.g., "main").
    pub trunk_branch: BranchName,
}

impl Default for StackSyncOptions {
    fn default() -> Self {
        Self {
            restack: false,
            full_fetch: false,
            delete_merged: true,
            delete_upstream_gone: false,
            force: false,
            safe: false,
            remote_name: "origin".to_string(),
            trunk_branch: BranchName::new("main".to_string()),
        }
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a stack sync operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackSyncResult {
    /// Remote branches that were fetched.
    pub branches_fetched: Vec<BranchName>,
    /// Whether trunk was updated.
    pub trunk_updated: bool,
    /// Merged branches detected and optionally deleted.
    pub merged_branches: Vec<MergedBranch>,
    /// Per-branch restack outcomes.
    pub restack_results: Vec<RestackOutcome>,
    /// Whether any conflicts were encountered.
    pub had_conflicts: bool,
    /// Whether stash was used.
    pub stash_used: bool,
    /// Per-step timing information.
    pub timings: Vec<(String, Duration)>,
}

/// A branch detected as merged, with its detection method and deletion status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedBranch {
    /// Branch name.
    pub name: BranchName,
    /// How the merge was detected.
    pub detection_method: MergedDetectionMethod,
    /// Whether it was deleted locally.
    pub deleted_locally: bool,
    /// Whether it was deleted remotely.
    pub deleted_remotely: bool,
    /// Children that were reparented to this branch's parent.
    pub reparented_children: Vec<BranchName>,
}

/// Method used to detect a merged branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergedDetectionMethod {
    /// `git branch --merged <trunk>` reported it.
    GitBranchMerged,
    /// `git branch --merged origin/<trunk>` reported it.
    GitBranchMergedRemote,
    /// PR state is "merged".
    PrStateMerged,
    /// PR state is "closed" (cancelled).
    PrStateClosed,
    /// Remote branch was deleted (GitHub post-merge cleanup).
    RemoteBranchDeleted,
    /// Branch exists neither locally nor remotely.
    OrphanedBranch,
}

/// Outcome of restacking a single branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestackOutcome {
    /// Branch that was restacked.
    pub branch: BranchName,
    /// Status of the restack operation.
    pub status: RestackStatus,
}

/// Status of a single branch restack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestackStatus {
    /// Successfully rebased.
    Success,
    /// Skipped because already up to date.
    SkippedUpToDate,
    /// Conflict during rebase; remaining branches not attempted.
    Conflict {
        /// Number of remaining branches that were not attempted.
        remaining: usize,
    },
}

/// Report of what changed between local and remote state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftReport {
    /// Whether trunk SHA advanced on remote.
    pub trunk_advanced: bool,
    /// New branches on remote not previously tracked.
    pub new_remote_branches: Vec<BranchName>,
    /// Remote branches that disappeared.
    pub removed_remote_branches: Vec<BranchName>,
    /// Stack branches that need restacking.
    pub branches_needing_restack: Vec<BranchName>,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack sync operations.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    // Precondition failures
    #[error("Workspace has uncommitted changes. Stash or commit first.")]
    DirtyWorkspace,
    #[error("No stack branches tracked")]
    NoStackBranches,
    #[error("Not on a tracked branch")]
    NotOnTrackedBranch,

    // Fetch failures
    #[error("Fetch from '{remote}' failed: {stderr}")]
    FetchFailed { remote: String, stderr: String },
    #[error("Remote '{0}' not found")]
    RemoteNotFound(String),

    // Trunk update failures
    #[error("Trunk '{trunk}' update failed: {stderr}")]
    TrunkUpdateFailed { trunk: String, stderr: String },
    #[error("Trunk diverged: local={local}, remote={remote}")]
    TrunkDiverged { local: String, remote: String },

    // Branch operations
    #[error("Branch not found: {0}")]
    BranchNotFound(BranchName),
    #[error("Failed to delete branch '{branch}': {reason}")]
    BranchDeleteFailed { branch: BranchName, reason: String },
    #[error("Branch '{branch}' is checked out in worktree '{worktree}'")]
    BranchInWorktree { branch: BranchName, worktree: String },

    // Restack failures
    #[error("Rebase conflict on '{branch}' against '{parent}'")]
    RebaseConflict { branch: BranchName, parent: BranchName },
    #[error("Rebase failed on '{branch}': {reason}")]
    RebaseFailed { branch: BranchName, reason: String },

    // I/O
    #[error("VCS backend error: {0}")]
    BackendError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

// ============================================================================
// Helper constructors for tests
// ============================================================================

/// Input for merged detection (collected from VCS queries).
#[derive(Debug, Clone, Default)]
pub struct MergedDetectionInput {
    /// Branches tracked in the stack (excluding trunk).
    pub tracked_branches: Vec<BranchName>,
    /// Branches reported as merged by `git branch --merged <trunk>`.
    pub local_merged: HashSet<BranchName>,
    /// Branches reported as merged by `git branch --merged origin/<trunk>`.
    pub remote_merged: HashSet<BranchName>,
    /// PR states keyed by branch name.
    pub pr_states: std::collections::HashMap<BranchName, scp_stack::PrState>,
    /// Branches that exist on the remote.
    pub remote_branches: HashSet<BranchName>,
    /// Branches that exist locally.
    pub local_branches: HashSet<BranchName>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_sync_options_default() {
        let opts = StackSyncOptions::default();
        assert!(!opts.restack);
        assert!(!opts.full_fetch);
        assert!(opts.delete_merged);
        assert!(!opts.delete_upstream_gone);
        assert!(!opts.force);
        assert!(!opts.safe);
        assert_eq!(opts.remote_name, "origin");
    }

    #[test]
    fn stack_sync_result_default() {
        let result = StackSyncResult::default();
        assert!(result.branches_fetched.is_empty());
        assert!(!result.trunk_updated);
        assert!(result.merged_branches.is_empty());
        assert!(result.restack_results.is_empty());
        assert!(!result.had_conflicts);
        assert!(!result.stash_used);
        assert!(result.timings.is_empty());
    }

    #[test]
    fn drift_report_default() {
        let report = DriftReport::default();
        assert!(!report.trunk_advanced);
        assert!(report.new_remote_branches.is_empty());
        assert!(report.removed_remote_branches.is_empty());
        assert!(report.branches_needing_restack.is_empty());
    }

    #[test]
    fn merged_detection_method_equality() {
        assert_eq!(
            MergedDetectionMethod::GitBranchMerged,
            MergedDetectionMethod::GitBranchMerged
        );
        assert_ne!(
            MergedDetectionMethod::GitBranchMerged,
            MergedDetectionMethod::PrStateMerged
        );
    }

    #[test]
    fn restack_status_conflict_remaining() {
        let status = RestackStatus::Conflict { remaining: 3 };
        assert_eq!(status, RestackStatus::Conflict { remaining: 3 });
        assert_ne!(status, RestackStatus::Conflict { remaining: 0 });
    }

    #[test]
    fn sync_error_display_dirty_workspace() {
        let err = SyncError::DirtyWorkspace;
        assert!(err.to_string().contains("uncommitted"));
    }

    #[test]
    fn sync_error_display_fetch_failed() {
        let err = SyncError::FetchFailed {
            remote: "origin".to_string(),
            stderr: "timeout".to_string(),
        };
        assert!(err.to_string().contains("origin"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn sync_error_display_rebase_conflict() {
        let err = SyncError::RebaseConflict {
            branch: BranchName::new("feat".to_string()),
            parent: BranchName::new("main".to_string()),
        };
        assert!(err.to_string().contains("feat"));
        assert!(err.to_string().contains("main"));
    }
}

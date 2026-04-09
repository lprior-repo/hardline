//! Data layer for stack merge-remote — inert, serializable types.
//!
//! No business logic. Types only.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use scp_stack::{BranchName, PrInfo};
use scp_stack::infrastructure::forge::MergeMethod;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack merge-remote command (parsed from CLI or API).
#[derive(Debug, Clone)]
pub struct MergeRemoteOptions {
    /// Merge all branches in the stack (not just ancestors + current).
    pub all: bool,
    /// Merge strategy to use.
    pub method: MergeMethod,
    /// Maximum time to wait for CI on each PR.
    pub timeout: Duration,
    /// Interval between CI status polls.
    pub poll_interval: Duration,
    /// Skip deleting local metadata after merge.
    pub no_delete: bool,
    /// Skip the sync hint at the end.
    pub no_sync_hint: bool,
    /// Skip confirmation prompts (polecat/CI mode).
    pub force: bool,
    /// Suppress progress output.
    pub quiet: bool,
    /// Remote name (e.g., "origin").
    pub remote_name: String,
}

impl Default for MergeRemoteOptions {
    fn default() -> Self {
        Self {
            all: false,
            method: MergeMethod::Squash,
            timeout: Duration::from_secs(600),
            poll_interval: Duration::from_secs(30),
            no_delete: false,
            no_sync_hint: false,
            force: false,
            quiet: false,
            remote_name: "origin".to_string(),
        }
    }
}

// ============================================================================
// Scope Types
// ============================================================================

/// The calculated merge scope — which branches to merge and which remain.
#[derive(Debug, Clone)]
pub struct MergeRemoteScope {
    /// Branches to merge (bottom-up order: ancestors first, then current).
    pub to_merge: Vec<BranchName>,
    /// Branches that remain in the stack after merge (descendants if `all` is false).
    pub remaining: Vec<BranchName>,
    /// The trunk branch name.
    pub trunk: BranchName,
}

// ============================================================================
// PR Resolution Types
// ============================================================================

/// A branch with its resolved PR number, ready for remote merge.
#[derive(Debug, Clone)]
pub struct PrBranchInfo {
    /// Branch name.
    pub branch: BranchName,
    /// PR number (must be present for merge).
    pub pr_number: u64,
}

/// A remaining branch that may or may not have a PR.
#[derive(Debug, Clone)]
pub struct RemainingBranchInfo {
    /// Branch name.
    pub branch: BranchName,
    /// PR number, if the branch has one.
    pub pr_number: Option<u64>,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a merge-remote operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeRemoteOutput {
    /// PRs that were successfully merged.
    pub merged_prs: Vec<MergedPr>,
    /// The failure that stopped the merge, if any.
    pub failure: Option<MergeFailure>,
    /// Remaining branches that were retargeted.
    pub retargeted_remaining: Vec<BranchName>,
    /// Branches whose local metadata was cleaned up.
    pub cleaned_branches: Vec<BranchName>,
}

/// A successfully merged PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedPr {
    /// Branch name.
    pub branch: BranchName,
    /// PR number.
    pub pr_number: u64,
}

/// A failure that stopped the merge process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeFailure {
    /// Branch where the failure occurred.
    pub branch: BranchName,
    /// PR number where the failure occurred.
    pub pr_number: u64,
    /// Human-readable failure reason.
    pub reason: String,
}

// ============================================================================
// Wait/Polling Types
// ============================================================================

/// Outcome of waiting for a PR to become ready for merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitOutcome {
    /// PR is ready (CI passed, approved if required).
    Ready,
    /// PR is blocked (merge conflict, failing CI, needs review).
    Blocked(String),
    /// Polling timed out before the PR became ready.
    Timeout,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for merge-remote operations.
#[derive(Debug, thiserror::Error)]
pub enum MergeRemoteError {
    // Precondition failures
    #[error("Currently on trunk. Checkout a branch in a stack to merge.")]
    OnTrunk,
    #[error("Branch '{0}' is not tracked in any stack")]
    NotTracked(BranchName),
    #[error("No branches to merge")]
    NothingToMerge,

    // PR resolution
    #[error("Branch '{branch}' has no PR. Submit first.")]
    NoPr { branch: BranchName },

    // Forge restrictions
    #[error("Remote merge is only supported for GitHub remotes (found {found})")]
    ForgeNotSupported { found: String },

    // API failures
    #[error("Failed to check if PR #{pr_number} is merged: {reason}")]
    IsMergedFailed { pr_number: u64, reason: String },
    #[error("Failed to wait for PR #{pr_number}: {reason}")]
    WaitFailed { pr_number: u64, reason: String },
    #[error("Failed to merge PR #{pr_number}: {reason}")]
    MergeFailed { pr_number: u64, reason: String },
    #[error("Failed to retarget PR #{pr_number} to {target}: {reason}")]
    RetargetFailed {
        pr_number: u64,
        target: String,
        reason: String,
    },
    #[error("Failed to update branch for PR #{pr_number}: {reason}")]
    UpdateBranchFailed { pr_number: u64, reason: String },
    #[error("Failed to delete metadata for '{branch}': {reason}")]
    DeleteMetadataFailed { branch: BranchName, reason: String },

    // I/O
    #[error("Forge client error: {0}")]
    ForgeClientError(String),
    #[error("Remote info error: {0}")]
    RemoteInfoError(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_remote_options_default() {
        let opts = MergeRemoteOptions::default();
        assert!(!opts.all);
        assert_eq!(opts.method, MergeMethod::Squash);
        assert!(!opts.no_delete);
        assert!(!opts.no_sync_hint);
        assert!(!opts.force);
        assert!(!opts.quiet);
        assert_eq!(opts.remote_name, "origin");
    }

    #[test]
    fn merge_remote_output_default() {
        let output = MergeRemoteOutput::default();
        assert!(output.merged_prs.is_empty());
        assert!(output.failure.is_none());
        assert!(output.retargeted_remaining.is_empty());
        assert!(output.cleaned_branches.is_empty());
    }

    #[test]
    fn wait_outcome_equality() {
        assert_eq!(WaitOutcome::Ready, WaitOutcome::Ready);
        assert_eq!(
            WaitOutcome::Blocked("conflict".to_string()),
            WaitOutcome::Blocked("conflict".to_string())
        );
        assert_eq!(WaitOutcome::Timeout, WaitOutcome::Timeout);
        assert_ne!(WaitOutcome::Ready, WaitOutcome::Timeout);
    }

    #[test]
    fn merged_pr_serialization() {
        let pr = MergedPr {
            branch: BranchName::new("feat-a".to_string()),
            pr_number: 42,
        };
        let json = serde_json::to_string(&pr).expect("serialize");
        let back: MergedPr = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.branch, pr.branch);
        assert_eq!(back.pr_number, 42);
    }

    #[test]
    fn merge_failure_serialization() {
        let failure = MergeFailure {
            branch: BranchName::new("feat-b".to_string()),
            pr_number: 7,
            reason: "CI failed".to_string(),
        };
        let json = serde_json::to_string(&failure).expect("serialize");
        let back: MergeFailure = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.branch, failure.branch);
        assert_eq!(back.pr_number, 7);
        assert_eq!(back.reason, "CI failed");
    }

    #[test]
    fn merge_remote_error_display() {
        let err = MergeRemoteError::OnTrunk;
        assert!(err.to_string().contains("trunk"));

        let err = MergeRemoteError::NoPr {
            branch: BranchName::new("feat".to_string()),
        };
        assert!(err.to_string().contains("feat"));
        assert!(err.to_string().contains("Submit"));

        let err = MergeRemoteError::MergeFailed {
            pr_number: 42,
            reason: "conflict".to_string(),
        };
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("conflict"));
    }

    #[test]
    fn merge_remote_error_forge_not_supported() {
        let err = MergeRemoteError::ForgeNotSupported {
            found: "GitLab".to_string(),
        };
        assert!(err.to_string().contains("GitLab"));
        assert!(err.to_string().contains("GitHub"));
    }

    #[test]
    fn pr_branch_info_fields() {
        let info = PrBranchInfo {
            branch: BranchName::new("feat-a".to_string()),
            pr_number: 99,
        };
        assert_eq!(info.branch.as_str(), "feat-a");
        assert_eq!(info.pr_number, 99);
    }

    #[test]
    fn remaining_branch_info_optional_pr() {
        let with_pr = RemainingBranchInfo {
            branch: BranchName::new("feat".to_string()),
            pr_number: Some(10),
        };
        assert_eq!(with_pr.pr_number, Some(10));

        let without_pr = RemainingBranchInfo {
            branch: BranchName::new("no-pr".to_string()),
            pr_number: None,
        };
        assert!(without_pr.pr_number.is_none());
    }

    #[test]
    fn merge_remote_scope_fields() {
        let scope = MergeRemoteScope {
            to_merge: vec![BranchName::new("a".to_string())],
            remaining: vec![BranchName::new("b".to_string())],
            trunk: BranchName::new("main".to_string()),
        };
        assert_eq!(scope.to_merge.len(), 1);
        assert_eq!(scope.remaining.len(), 1);
        assert_eq!(scope.trunk.as_str(), "main");
    }
}

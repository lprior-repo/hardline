//! Data layer for stack range-diff - inert, serializable types.
//!
//! No business logic. Types only.
//!
//! Range-diff compares two commit ranges and shows how patches changed
//! between them. In a stack context, this is used to compare how a branch
//! evolved relative to its base (e.g., before and after a rebase).

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

// ============================================================================
// Input Types
// ============================================================================

/// Options for the stack range-diff command (parsed from CLI or API).
#[derive(Debug, Clone)]
pub struct RangeDiffOptions {
    /// Base commit (or branch) for range A.
    pub base_a: String,
    /// Tip commit (or branch) for range A.
    pub tip_a: String,
    /// Base commit (or branch) for range B.
    pub base_b: String,
    /// Tip commit (or branch) for range B.
    pub tip_b: String,
    /// Output format.
    pub format: RangeDiffFormat,
    /// Creation/deletion cost factor (default: 1).
    pub creation_factor: Option<u8>,
    /// Compare against the other side's diff, not the common ancestor.
    pub dual: bool,
}

impl Default for RangeDiffOptions {
    fn default() -> Self {
        Self {
            base_a: String::new(),
            tip_a: String::new(),
            base_b: String::new(),
            tip_b: String::new(),
            format: RangeDiffFormat::default(),
            creation_factor: None,
            dual: false,
        }
    }
}

/// Output format for range-diff.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeDiffFormat {
    /// Default: show commit pairing with diffstat.
    #[default]
    Default,
    /// Show only commit pairing, no diff.
    Stat,
    /// Show full patch diff.
    Patch,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a range-diff operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RangeDiffResult {
    /// Raw output from git range-diff.
    pub output: String,
    /// Parsed commit pairings (old commit -> new commit).
    pub pairings: Vec<CommitPairing>,
    /// Whether any differences were found.
    pub has_changes: bool,
}

/// A pairing between commits in range A and range B.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitPairing {
    /// Commit in range A (None if new in B).
    pub commit_a: Option<CommitSummary>,
    /// Commit in range B (None if removed in B).
    pub commit_b: Option<CommitSummary>,
    /// Change status.
    pub status: PairingStatus,
}

/// Summary of a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    /// Short commit hash.
    pub short_hash: String,
    /// Commit subject line.
    pub subject: String,
}

/// Status of a commit pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingStatus {
    /// Commit is unchanged between ranges.
    Unchanged,
    /// Commit was modified (content changed).
    Modified,
    /// Commit was added in range B.
    Added,
    /// Commit was removed in range B.
    Removed,
}

/// A single range specification (base..tip).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeSpec {
    /// Base ref (exclusive).
    pub base: String,
    /// Tip ref (inclusive).
    pub tip: String,
}

// ============================================================================
// Error Types
// ============================================================================

/// Error taxonomy for stack range-diff operations.
#[derive(Debug, thiserror::Error)]
pub enum RangeDiffError {
    /// One or more refs could not be resolved.
    #[error("Invalid ref: {ref_name} — {reason}")]
    InvalidRef {
        ref_name: String,
        reason: String,
    },
    /// git range-diff command failed.
    #[error("git range-diff failed: {stderr}")]
    CommandFailed { stderr: String },
    /// One or both commit ranges are empty.
    #[error("Empty commit range: {range}")]
    EmptyRange { range: String },
    /// I/O error.
    #[error("IO error: {0}")]
    IoError(String),
}

// ============================================================================
// Helper constructors for tests
// ============================================================================

/// Build a RangeSpec from base and tip refs.
#[must_use]
pub fn range_spec(base: impl Into<String>, tip: impl Into<String>) -> RangeSpec {
    RangeSpec {
        base: base.into(),
        tip: tip.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_diff_options_default() {
        let opts = RangeDiffOptions::default();
        assert!(opts.base_a.is_empty());
        assert!(opts.tip_a.is_empty());
        assert!(opts.base_b.is_empty());
        assert!(opts.tip_b.is_empty());
        assert_eq!(opts.format, RangeDiffFormat::Default);
        assert!(opts.creation_factor.is_none());
        assert!(!opts.dual);
    }

    #[test]
    fn range_diff_format_default_is_default() {
        assert_eq!(RangeDiffFormat::default(), RangeDiffFormat::Default);
    }

    #[test]
    fn range_diff_result_default() {
        let result = RangeDiffResult::default();
        assert!(result.output.is_empty());
        assert!(result.pairings.is_empty());
        assert!(!result.has_changes);
    }

    #[test]
    fn range_spec_constructor() {
        let spec = range_spec("main", "feat-a");
        assert_eq!(spec.base, "main");
        assert_eq!(spec.tip, "feat-a");
    }

    #[test]
    fn pairing_status_equality() {
        assert_eq!(PairingStatus::Unchanged, PairingStatus::Unchanged);
        assert_ne!(PairingStatus::Added, PairingStatus::Removed);
    }

    #[test]
    fn range_diff_error_display() {
        let err = RangeDiffError::InvalidRef {
            ref_name: "abc123".to_string(),
            reason: "not found".to_string(),
        };
        assert!(err.to_string().contains("abc123"));
        assert!(err.to_string().contains("not found"));

        let err = RangeDiffError::CommandFailed {
            stderr: "fatal: bad revision".to_string(),
        };
        assert!(err.to_string().contains("fatal"));

        let err = RangeDiffError::EmptyRange {
            range: "main..main".to_string(),
        };
        assert!(err.to_string().contains("main..main"));
    }

    #[test]
    fn commit_summary_fields() {
        let summary = CommitSummary {
            short_hash: "abc1234".to_string(),
            subject: "feat: add thing".to_string(),
        };
        assert_eq!(summary.short_hash, "abc1234");
        assert_eq!(summary.subject, "feat: add thing");
    }

    #[test]
    fn commit_pairing_both_present() {
        let pairing = CommitPairing {
            commit_a: Some(CommitSummary {
                short_hash: "aaa1111".to_string(),
                subject: "old".to_string(),
            }),
            commit_b: Some(CommitSummary {
                short_hash: "bbb2222".to_string(),
                subject: "new".to_string(),
            }),
            status: PairingStatus::Modified,
        };
        assert!(pairing.commit_a.is_some());
        assert!(pairing.commit_b.is_some());
        assert_eq!(pairing.status, PairingStatus::Modified);
    }

    #[test]
    fn commit_pairing_added() {
        let pairing = CommitPairing {
            commit_a: None,
            commit_b: Some(CommitSummary {
                short_hash: "bbb2222".to_string(),
                subject: "new commit".to_string(),
            }),
            status: PairingStatus::Added,
        };
        assert!(pairing.commit_a.is_none());
        assert_eq!(pairing.status, PairingStatus::Added);
    }

    #[test]
    fn commit_pairing_removed() {
        let pairing = CommitPairing {
            commit_a: Some(CommitSummary {
                short_hash: "aaa1111".to_string(),
                subject: "old commit".to_string(),
            }),
            commit_b: None,
            status: PairingStatus::Removed,
        };
        assert!(pairing.commit_b.is_none());
        assert_eq!(pairing.status, PairingStatus::Removed);
    }
}

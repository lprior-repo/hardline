//! Data types for the recover command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the recover command,
//! adapted from the isolate recover command but pivoted from JJ to Git.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the recover command (parsed from CLI).
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // CLI flags: independent options
pub struct RecoverOptions {
    /// Just diagnose without fixing
    pub diagnose_only: bool,

    /// Session or workspace to recover (None = auto-detect)
    pub target: Option<String>,

    /// Dry run - show what would be done without making changes
    pub dry_run: bool,

    /// Verbose output
    pub verbose: bool,
}

/// Options for the rollback command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct RollbackOptions {
    /// Session/workspace to rollback
    pub session: String,

    /// Commit hash to rollback to
    pub commit: String,

    /// Dry run - show what would happen without executing
    pub dry_run: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the recover command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoverOutput {
    /// Issues found during diagnosis
    pub issues: Vec<Issue>,

    /// Number of issues fixed
    pub fixed_count: usize,

    /// Number of issues remaining (excluding info-severity)
    pub remaining_count: usize,

    /// Overall status: "healthy", "partially_fixed", "issues_remaining"
    pub status: String,
}

/// A single issue found during recovery diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Machine-readable issue code (e.g., "GIT_NOT_INITIALIZED")
    pub code: String,

    /// Human-readable description
    pub description: String,

    /// Severity: "critical", "warning", "info"
    pub severity: String,

    /// Suggested fix command (if available)
    pub fix_command: Option<String>,

    /// Whether the issue was fixed during recovery
    pub fixed: bool,
}

/// Output from the rollback command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOutput {
    /// Session/workspace that was rolled back
    pub session: String,

    /// Commit hash rolled back to
    pub commit: String,

    /// Whether this was a dry run
    pub dry_run: bool,

    /// Whether the rollback operation succeeded
    pub succeeded: bool,

    /// Human-readable message
    pub message: String,
}

/// Phase of the recover operation where an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverPhase {
    /// Diagnosing issues
    Diagnosing,
    /// Fixing issues
    Fixing,
    /// Rolling back to a commit
    RollingBack,
}

impl RecoverPhase {
    /// Returns the snake_case name of this phase.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Diagnosing => "diagnosing",
            Self::Fixing => "fixing",
            Self::RollingBack => "rolling_back",
        }
    }
}

// ============================================================================
// Pure computation functions (Tier 2)
// ============================================================================

/// Compute overall status from a list of issues.
///
/// Returns:
/// - "healthy" if all non-info issues are fixed or no issues exist
/// - "partially_fixed" if some issues were fixed but others remain
/// - "issues_remaining" if no issues were fixed and unfixed ones remain
#[must_use]
pub fn compute_status(issues: &[Issue]) -> String {
    let remaining = issues
        .iter()
        .filter(|i| !i.fixed && i.severity != "info")
        .count();

    if remaining == 0 {
        "healthy".to_string()
    } else if issues.iter().any(|i| i.fixed) {
        "partially_fixed".to_string()
    } else {
        "issues_remaining".to_string()
    }
}

/// Count fixed issues.
#[must_use]
pub fn count_fixed(issues: &[Issue]) -> usize {
    issues.iter().filter(|i| i.fixed).count()
}

/// Count remaining unfixed non-info issues.
#[must_use]
pub fn count_remaining(issues: &[Issue]) -> usize {
    issues
        .iter()
        .filter(|i| !i.fixed && i.severity != "info")
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- RecoverOptions ----

    #[test]
    fn recover_options_default_diagnose_only_is_false() {
        let opts = RecoverOptions::default();
        assert!(!opts.diagnose_only);
    }

    #[test]
    fn recover_options_default_target_is_none() {
        let opts = RecoverOptions::default();
        assert!(opts.target.is_none());
    }

    #[test]
    fn recover_options_default_dry_run_is_false() {
        let opts = RecoverOptions::default();
        assert!(!opts.dry_run);
    }

    #[test]
    fn recover_options_default_verbose_is_false() {
        let opts = RecoverOptions::default();
        assert!(!opts.verbose);
    }

    #[test]
    fn recover_options_with_explicit_fields() {
        let opts = RecoverOptions {
            diagnose_only: true,
            target: Some("my-session".to_string()),
            dry_run: true,
            verbose: true,
        };
        assert!(opts.diagnose_only);
        assert_eq!(opts.target.as_deref(), Some("my-session"));
        assert!(opts.dry_run);
        assert!(opts.verbose);
    }

    // ---- Issue ----

    #[test]
    fn issue_construction_and_field_access() {
        let issue = Issue {
            code: "GIT_NOT_INITIALIZED".to_string(),
            description: "Git is not initialized".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("git init".to_string()),
            fixed: false,
        };
        assert_eq!(issue.code, "GIT_NOT_INITIALIZED");
        assert_eq!(issue.description, "Git is not initialized");
        assert_eq!(issue.severity, "critical");
        assert_eq!(issue.fix_command.as_deref(), Some("git init"));
        assert!(!issue.fixed);
    }

    #[test]
    fn issue_serialization_roundtrip() {
        let issue = Issue {
            code: "ORPHANED_WORKTREE".to_string(),
            description: "Worktree has missing directory".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("scp workspace remove orphan --force".to_string()),
            fixed: true,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let deserialized: Issue = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.code, "ORPHANED_WORKTREE");
        assert_eq!(deserialized.severity, "warning");
        assert!(deserialized.fixed);
        assert_eq!(
            deserialized.fix_command.as_deref(),
            Some("scp workspace remove orphan --force")
        );
    }

    #[test]
    fn issue_without_fix_command_serializes() {
        let issue = Issue {
            code: "INFO_ONLY".to_string(),
            description: "Info note".to_string(),
            severity: "info".to_string(),
            fix_command: None,
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let deserialized: Issue = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert!(deserialized.fix_command.is_none());
    }

    // ---- RecoverOutput ----

    #[test]
    fn recover_output_default_has_empty_issues() {
        let output = RecoverOutput::default();
        assert!(output.issues.is_empty());
        assert_eq!(output.fixed_count, 0);
        assert_eq!(output.remaining_count, 0);
        assert!(output.status.is_empty());
    }

    #[test]
    fn recover_output_serialization_roundtrip() {
        let output = RecoverOutput {
            issues: vec![Issue {
                code: "TEST".to_string(),
                description: "test issue".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: false,
            }],
            fixed_count: 0,
            remaining_count: 1,
            status: "issues_remaining".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RecoverOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.issues.len(), 1);
        assert_eq!(deserialized.remaining_count, 1);
        assert_eq!(deserialized.status, "issues_remaining");
    }

    // ---- RollbackOutput ----

    #[test]
    fn rollback_output_construction() {
        let output = RollbackOutput {
            session: "feature-x".to_string(),
            commit: "abc123def".to_string(),
            dry_run: false,
            succeeded: true,
            message: "Rolled back successfully".to_string(),
        };
        assert_eq!(output.session, "feature-x");
        assert_eq!(output.commit, "abc123def");
        assert!(!output.dry_run);
        assert!(output.succeeded);
    }

    #[test]
    fn rollback_output_serialization_roundtrip() {
        let output = RollbackOutput {
            session: "ws".to_string(),
            commit: "deadbeef".to_string(),
            dry_run: true,
            succeeded: false,
            message: "Preview: would reset".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: RollbackOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.dry_run);
        assert!(!deserialized.succeeded);
        assert!(deserialized.message.contains("Preview"));
    }

    // ---- RollbackOptions ----

    #[test]
    fn rollback_options_construction() {
        let opts = RollbackOptions {
            session: "my-session".to_string(),
            commit: "abc123".to_string(),
            dry_run: true,
        };
        assert_eq!(opts.session, "my-session");
        assert_eq!(opts.commit, "abc123");
        assert!(opts.dry_run);
    }

    // ---- RecoverPhase ----

    #[test]
    fn recover_phase_diagnosing_name() {
        assert_eq!(RecoverPhase::Diagnosing.name(), "diagnosing");
    }

    #[test]
    fn recover_phase_fixing_name() {
        assert_eq!(RecoverPhase::Fixing.name(), "fixing");
    }

    #[test]
    fn recover_phase_rolling_back_name() {
        assert_eq!(RecoverPhase::RollingBack.name(), "rolling_back");
    }

    #[test]
    fn recover_phase_equality() {
        assert_eq!(RecoverPhase::Diagnosing, RecoverPhase::Diagnosing);
        assert_ne!(RecoverPhase::Diagnosing, RecoverPhase::Fixing);
    }

    #[test]
    fn recover_phase_all_variants_exhaustive_match() {
        let phases = [
            RecoverPhase::Diagnosing,
            RecoverPhase::Fixing,
            RecoverPhase::RollingBack,
        ];
        for phase in &phases {
            let name = phase.name();
            assert!(!name.is_empty());
            assert!(!name.contains(' '));
        }
    }

    // ---- compute_status ----

    #[test]
    fn compute_status_no_issues_is_healthy() {
        let status = compute_status(&[]);
        assert_eq!(status, "healthy");
    }

    #[test]
    fn compute_status_all_fixed_is_healthy() {
        let issues = vec![Issue {
            code: "X".to_string(),
            description: "d".to_string(),
            severity: "warning".to_string(),
            fix_command: None,
            fixed: true,
        }];
        assert_eq!(compute_status(&issues), "healthy");
    }

    #[test]
    fn compute_status_some_fixed_some_remaining_is_partially_fixed() {
        let issues = vec![
            Issue {
                code: "A".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: true,
            },
            Issue {
                code: "B".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: false,
            },
        ];
        assert_eq!(compute_status(&issues), "partially_fixed");
    }

    #[test]
    fn compute_status_none_fixed_is_issues_remaining() {
        let issues = vec![Issue {
            code: "X".to_string(),
            description: "d".to_string(),
            severity: "critical".to_string(),
            fix_command: None,
            fixed: false,
        }];
        assert_eq!(compute_status(&issues), "issues_remaining");
    }

    #[test]
    fn compute_status_info_severity_ignored_in_remaining() {
        let issues = vec![Issue {
            code: "X".to_string(),
            description: "d".to_string(),
            severity: "info".to_string(),
            fix_command: None,
            fixed: false,
        }];
        // Info-severity unfixed issues don't count as "remaining"
        assert_eq!(compute_status(&issues), "healthy");
    }

    // ---- count_fixed / count_remaining ----

    #[test]
    fn count_fixed_empty() {
        assert_eq!(count_fixed(&[]), 0);
    }

    #[test]
    fn count_fixed_mixed() {
        let issues = vec![
            Issue {
                code: "A".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: true,
            },
            Issue {
                code: "B".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: false,
            },
            Issue {
                code: "C".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: true,
            },
        ];
        assert_eq!(count_fixed(&issues), 2);
    }

    #[test]
    fn count_remaining_excludes_info_and_fixed() {
        let issues = vec![
            Issue {
                code: "A".to_string(),
                description: "d".to_string(),
                severity: "warning".to_string(),
                fix_command: None,
                fixed: true,
            },
            Issue {
                code: "B".to_string(),
                description: "d".to_string(),
                severity: "critical".to_string(),
                fix_command: None,
                fixed: false,
            },
            Issue {
                code: "C".to_string(),
                description: "d".to_string(),
                severity: "info".to_string(),
                fix_command: None,
                fixed: false,
            },
        ];
        // Only B counts as remaining (A is fixed, C is info)
        assert_eq!(count_remaining(&issues), 1);
    }

    // ---- JSON behavior tests (AI-actionable) ----

    #[test]
    fn issue_json_has_all_fields_for_automation() {
        let issue = Issue {
            code: "ORPHANED_WORKTREE".to_string(),
            description: "Worktree directory missing".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("scp workspace remove orphan --force".to_string()),
            fixed: false,
        };
        let json = serde_json::to_string(&issue).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");

        assert!(
            parsed.get("code").is_some(),
            "AI needs code for categorization"
        );
        assert!(
            parsed.get("severity").is_some(),
            "AI needs severity for prioritization"
        );
        assert!(
            parsed.get("fix_command").is_some(),
            "AI needs fix_command for automation"
        );
        assert!(
            parsed.get("fixed").is_some(),
            "AI needs fixed for status tracking"
        );
    }
}

//! Validation result type
//!
//! Result of a workspace validation check.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{IntegrityIssue, RepairStrategy, Severity};

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a workspace validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Name of the workspace
    pub workspace: String,
    /// Path to the workspace
    pub path: PathBuf,
    /// Whether the workspace is valid
    pub is_valid: bool,
    /// List of issues found
    pub issues: Vec<IntegrityIssue>,
    /// Maximum severity among all issues
    pub max_severity: Option<Severity>,
    /// Duration of check in milliseconds
    pub duration_ms: u64,
    /// When the validation was performed
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl ValidationResult {
    /// Create a valid result
    #[must_use]
    pub fn valid(workspace: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: true,
            issues: Vec::new(),
            max_severity: None,
            duration_ms: 0,
            validated_at: chrono::Utc::now(),
        }
    }

    /// Create an invalid result with issues
    #[must_use]
    pub fn invalid(
        workspace: impl Into<String>,
        path: impl Into<PathBuf>,
        issues: Vec<IntegrityIssue>,
    ) -> Self {
        let max_severity = issues.iter().map(|i| i.severity).max();

        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: issues.is_empty(),
            issues,
            max_severity,
            duration_ms: 0,
            validated_at: chrono::Utc::now(),
        }
    }

    /// Set the check duration
    #[must_use]
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Check if any issues are auto-repairable
    #[must_use]
    pub fn has_auto_repairable_issues(&self) -> bool {
        self.issues.iter().any(|i| {
            !matches!(
                i.recommended_strategy,
                RepairStrategy::NoRepair | RepairStrategy::NoRepairPossible
            )
        })
    }

    /// Get the most severe issue found
    #[must_use]
    pub fn most_severe_issue(&self) -> Option<&IntegrityIssue> {
        self.issues.iter().max_by_key(|i| i.severity)
    }
}

//! Health check and doctor command types
//!
//! This module provides types for system health diagnostics,
//! auto-fix operations, and troubleshooting recommendations.

use serde::{Deserialize, Serialize};

/// System health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Status message
    pub message: String,
    /// Suggestion for fixing issues
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Whether this issue can be auto-fixed
    pub auto_fixable: bool,
    /// Additional details about the check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a health check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Warning - non-critical issue
    Warn,
    /// Failure - critical issue
    Fail,
}

/// Overall health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOutput {
    /// Whether the system is healthy overall
    pub healthy: bool,
    /// Individual check results
    pub checks: Vec<DoctorCheck>,
    /// Count of warnings
    pub warnings: usize,
    /// Count of errors
    pub errors: usize,
    /// Number of issues that can be auto-fixed
    pub auto_fixable_issues: usize,
}

impl DoctorOutput {
    /// Calculate summary statistics from checks
    ///
    /// # Returns
    ///
    /// Returns a summary with calculated statistics. The result should be used
    /// as this performs analysis and aggregates check results.
    #[must_use]
    pub fn from_checks(checks: Vec<DoctorCheck>) -> Self {
        let warnings = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();
        let errors = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let auto_fixable_issues = checks.iter().filter(|c| c.auto_fixable).count();
        let healthy = errors == 0;

        Self {
            healthy,
            checks,
            warnings,
            errors,
            auto_fixable_issues,
        }
    }
}

/// Result of auto-fix operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixOutput {
    /// Issues that were fixed
    pub fixed: Vec<FixResult>,
    /// Issues that could not be fixed
    pub unable_to_fix: Vec<UnfixableIssue>,
}

/// Result of fixing a single issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Issue that was fixed
    pub issue: String,
    /// Action taken
    pub action: String,
    /// Whether the fix succeeded
    pub success: bool,
}

/// Issue that could not be auto-fixed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfixableIssue {
    /// Issue name
    pub issue: String,
    /// Reason why it couldn't be fixed
    pub reason: String,
    /// Manual fix suggestion
    pub suggestion: String,
}

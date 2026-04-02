//! Integrity issue type
//!
//! Represents a detected workspace integrity issue.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{CorruptionType, RepairStrategy, Severity};

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY ISSUE
// ═══════════════════════════════════════════════════════════════════════════

/// A specific integrity issue detected in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    /// Type of corruption
    pub corruption_type: CorruptionType,
    /// Severity level
    pub severity: Severity,
    /// Description of the issue
    pub description: String,
    /// Path affected by the issue
    pub affected_path: Option<PathBuf>,
    /// Contextual information (e.g. error message)
    pub context: Option<String>,
    /// Recommended repair strategy
    pub recommended_strategy: RepairStrategy,
}

impl IntegrityIssue {
    /// Create a new integrity issue
    #[must_use]
    pub fn new(corruption_type: CorruptionType, description: impl Into<String>) -> Self {
        let strategy = Self::recommended_strategy_for_type(corruption_type);
        let severity = Self::default_severity(corruption_type);

        Self {
            corruption_type,
            severity,
            description: description.into(),
            affected_path: None,
            context: None,
            recommended_strategy: strategy,
        }
    }

    /// Set the affected path
    #[must_use]
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.affected_path = Some(path.into());
        self
    }

    /// Set contextual information
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set a custom repair strategy
    #[must_use]
    pub const fn with_strategy(mut self, strategy: RepairStrategy) -> Self {
        self.recommended_strategy = strategy;
        self
    }

    /// Determine the default severity for a corruption type
    const fn default_severity(corruption_type: CorruptionType) -> Severity {
        match corruption_type {
            CorruptionType::MissingDirectory => Severity::Critical,
            CorruptionType::StaleLocks => Severity::Warn,
            CorruptionType::MissingGitDir
            | CorruptionType::CorruptedGitDir
            | CorruptionType::PermissionDenied
            | CorruptionType::CorruptedGitIndex => Severity::Fail,
        }
    }

    /// Determine the recommended repair strategy for a corruption type
    #[must_use]
    pub const fn recommended_strategy_for_type(corruption_type: CorruptionType) -> RepairStrategy {
        match corruption_type {
            CorruptionType::MissingDirectory | CorruptionType::CorruptedGitDir => {
                RepairStrategy::RemoveAndReclone
            }
            CorruptionType::MissingGitDir => RepairStrategy::RecreateWorkspace,
            CorruptionType::StaleLocks => RepairStrategy::ClearLocks,
            CorruptionType::PermissionDenied | CorruptionType::CorruptedGitIndex => {
                RepairStrategy::NoRepairPossible
            }
        }
    }
}

//! Conflict resolution output types
//!
//! Provides conflict detection and resolution reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// CONFLICT RESOLUTION TYPES
// ============================================================================

/// Type of conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Files modified on both branches
    Overlapping,
    /// Conflict already exists in workspace
    Existing,
    /// File deleted on one branch, modified on other
    DeleteModify,
    /// File renamed on one branch, modified on other
    RenameModify,
    /// Binary file conflict
    Binary,
}

/// Strategy for resolving a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    AcceptOurs,
    AcceptTheirs,
    JjResolve,
    ManualMerge,
    Rebase,
    Abort,
    Skip,
}

/// Risk level of a resolution option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionRisk {
    Safe,
    Moderate,
    Destructive,
}

/// A resolution option for a conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionOption {
    pub strategy: ResolutionStrategy,
    pub description: String,
    pub risk: ResolutionRisk,
    pub automatic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ResolutionOption {
    #[must_use]
    pub fn accept_ours() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptOurs,
            description: "Accept workspace version".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj resolve --with workspace".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn accept_theirs() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptTheirs,
            description: "Accept main version".to_string(),
            risk: ResolutionRisk::Destructive,
            automatic: true,
            command: Some("jj resolve --with main".to_string()),
            notes: Some("Will discard workspace changes".to_string()),
        }
    }

    #[must_use]
    pub fn manual_merge() -> Self {
        Self {
            strategy: ResolutionStrategy::ManualMerge,
            description: "Manually resolve conflicts".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: false,
            command: None,
            notes: Some("Open file in editor".to_string()),
        }
    }

    #[must_use]
    pub fn jj_resolve(file: &str) -> Self {
        Self {
            strategy: ResolutionStrategy::JjResolve,
            description: "Use jj resolve tool".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: Some(format!("jj resolve {file}")),
            notes: None,
        }
    }

    #[must_use]
    pub fn rebase() -> Self {
        Self {
            strategy: ResolutionStrategy::Rebase,
            description: "Rebase onto fresh main".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj rebase -d main".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn abort() -> Self {
        Self {
            strategy: ResolutionStrategy::Abort,
            description: "Abort the operation".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: Some("jj abort".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn skip() -> Self {
        Self {
            strategy: ResolutionStrategy::Skip,
            description: "Skip this file".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: None,
            notes: Some("File will remain conflicted".to_string()),
        }
    }
}

/// Details about a specific conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictDetail {
    pub file: String,
    pub conflict_type: ConflictType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_additions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_deletions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_additions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_deletions: Option<u32>,
    pub resolutions: Vec<ResolutionOption>,
    pub recommended: ResolutionStrategy,
}

impl ConflictDetail {
    #[must_use]
    pub fn overlapping(file: &str) -> Self {
        Self {
            file: file.to_string(),
            conflict_type: ConflictType::Overlapping,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(file),
                ResolutionOption::manual_merge(),
                ResolutionOption::accept_ours(),
                ResolutionOption::accept_theirs(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }

    #[must_use]
    pub fn existing(file: &str) -> Self {
        Self {
            file: file.to_string(),
            conflict_type: ConflictType::Existing,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(file),
                ResolutionOption::manual_merge(),
                ResolutionOption::rebase(),
                ResolutionOption::abort(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }
}

/// Analysis of all conflicts in a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    #[serde(rename = "type")]
    pub type_field: String,
    pub session: String,
    pub merge_safe: bool,
    pub total_conflicts: usize,
    pub conflicts: Vec<ConflictDetail>,
    pub existing_conflicts: usize,
    pub overlapping_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_time_ms: Option<u64>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

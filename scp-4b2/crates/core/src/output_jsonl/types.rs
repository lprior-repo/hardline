//! JSONL output types for AI-first CLI design
//!
//! Each output line is a self-describing JSON object with a "type" field.
//! This enables streaming JSONL output where each line can be parsed independently.
//!
//! # Domain-Driven Design Refactoring
//!
//! This module follows Scott Wlaschin's DDD principles:
//! - Parse at boundaries, validate once (semantic newtypes in `domain_types`)
//! - Make illegal states unrepresentable (enums instead of `bool`/`Option`)
//! - Railway-oriented programming with `Result<T, E>`

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Import domain types for semantic validation
use super::domain_types::{
    ActionResult, ActionTarget, ActionVerb, IssueId, IssueScope, IssueTitle, Message, Outcome,
    PlanDescription, PlanTitle, RecoveryCapability, RecoveryExecution, SessionName, WarningCode,
};
use crate::{types::SessionStatus, WorkspaceState};

#[derive(Debug, Clone, Error)]
pub enum OutputLineError {
    #[error("message is required but was empty")]
    EmptyMessage,
    #[error("title is required but was empty")]
    EmptyTitle,
    #[error("description is required but was empty")]
    EmptyDescription,
    #[error("session name is required but was empty")]
    EmptySessionName,
    #[error("at least one action is required")]
    NoActions,
    #[error("plan step count exceeds u32::MAX")]
    PlanStepOverflow,
    #[error("recovery action count exceeds u32::MAX")]
    RecoveryActionOverflow,
    #[error("workspace path must be absolute")]
    RelativePath,
    #[error("invalid warning code: {0}")]
    InvalidWarningCode(String),
    #[error("invalid action verb: {0}")]
    InvalidActionVerb(String),
    #[error("invalid action target: {0}")]
    InvalidActionTarget(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputLine {
    Summary(Summary),
    Session(SessionOutput),
    Issue(Issue),
    Plan(Plan),
    Action(Action),
    Warning(Warning),
    Result(ResultOutput),
    ConflictDetail(ConflictAnalysis),
    ConflictAnalysis(ConflictAnalysis),
}

impl OutputLine {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Summary(_) => "summary",
            Self::Session(_) => "session",
            Self::Issue(_) => "issue",
            Self::Plan(_) => "plan",
            Self::Action(_) => "action",
            Self::Warning(_) => "warning",
            Self::Result(_) => "result",
            Self::ConflictDetail(_) => "conflictdetail",
            Self::ConflictAnalysis(_) => "conflict_analysis",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
    #[serde(rename = "type")]
    pub type_field: SummaryType,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Status,
    Count,
    Info,
}

impl Summary {
    /// Create a new summary line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(type_field: SummaryType, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            type_field,
            message,
            details: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_details(self, details: String) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOutput {
    pub name: String,
    pub status: SessionStatus,
    pub state: WorkspaceState,
    pub workspace_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

impl SessionOutput {
    /// Create a new session output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptySessionName` if `name` is blank.
    /// Returns `OutputLineError::RelativePath` if `workspace_path` is not absolute.
    pub fn new(
        name: String,
        status: SessionStatus,
        state: WorkspaceState,
        workspace_path: PathBuf,
    ) -> Result<Self, OutputLineError> {
        if name.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        if !workspace_path.is_absolute() {
            return Err(OutputLineError::RelativePath);
        }
        let now = Utc::now();
        Ok(Self {
            name,
            status,
            state,
            workspace_path,
            branch: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }

    #[must_use]
    pub fn with_branch(self, branch: String) -> Self {
        Self {
            branch: Some(branch),
            ..self
        }
    }

    #[must_use]
    pub fn with_metadata(self, metadata: serde_json::Value) -> Self {
        Self {
            metadata: Some(metadata),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: IssueId,
    pub title: IssueTitle,
    pub kind: IssueKind,
    pub severity: IssueSeverity,
    #[serde(skip_serializing_if = "IssueScope::is_standalone")]
    #[serde(default = "IssueScope::standalone")]
    pub scope: IssueScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

// Helper function for serde skip condition
impl IssueScope {
    #[must_use]
    pub const fn is_standalone(&self) -> bool {
        matches!(self, Self::Standalone)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    Validation,
    StateConflict,
    ResourceNotFound,
    PermissionDenied,
    Timeout,
    Configuration,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Hint,
    Warning,
    Error,
    Critical,
}

impl Issue {
    /// Create a new issue output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if `title` is blank.
    pub const fn new(
        id: IssueId,
        title: IssueTitle,
        kind: IssueKind,
        severity: IssueSeverity,
    ) -> Result<Self, OutputLineError> {
        Ok(Self {
            id,
            title,
            kind,
            severity,
            scope: IssueScope::Standalone,
            suggestion: None,
        })
    }

    #[must_use]
    pub fn with_session(self, session: SessionName) -> Self {
        Self {
            scope: IssueScope::InSession { session },
            ..self
        }
    }

    #[must_use]
    pub fn with_suggestion(self, suggestion: String) -> Self {
        Self {
            suggestion: Some(suggestion),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub title: PlanTitle,
    pub description: PlanDescription,
    pub steps: Vec<PlanStep>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStep {
    pub order: u32,
    pub description: String,
    pub status: ActionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl Plan {
    /// Create a new plan output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyTitle` if `title` is blank.
    /// Returns `OutputLineError::EmptyDescription` if `description` is blank.
    pub fn new(title: PlanTitle, description: PlanDescription) -> Result<Self, OutputLineError> {
        Ok(Self {
            title,
            description,
            steps: Vec::new(),
            created_at: Utc::now(),
        })
    }

    /// Append a step to this plan.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::PlanStepOverflow` when the number of steps
    /// cannot be represented as `u32`.
    pub fn with_step(
        self,
        description: String,
        status: ActionStatus,
    ) -> Result<Self, OutputLineError> {
        let order =
            u32::try_from(self.steps.len()).map_err(|_| OutputLineError::PlanStepOverflow)?;
        Ok(Self {
            steps: self
                .steps
                .into_iter()
                .chain(std::iter::once(PlanStep {
                    order,
                    description,
                    status,
                }))
                .collect(),
            ..self
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub verb: ActionVerb,
    pub target: ActionTarget,
    pub status: ActionStatus,
    #[serde(skip_serializing_if = "ActionResult::is_pending")]
    #[serde(default = "ActionResult::pending")]
    pub result: ActionResult,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

// Helper for serde skip condition
impl ActionResult {
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

impl Action {
    #[must_use]
    pub fn new(verb: ActionVerb, target: ActionTarget, status: ActionStatus) -> Self {
        Self {
            verb,
            target,
            status,
            result: ActionResult::Pending,
            timestamp: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_result(self, result: String) -> Self {
        Self {
            result: ActionResult::Completed { result },
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Warning {
    pub code: WarningCode,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub session: String,
    pub workspace: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<serde_json::Value>,
}

impl Warning {
    /// Create a new warning output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn new(code: WarningCode, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            code,
            message,
            context: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_context(self, session: String, workspace: PathBuf) -> Self {
        Self {
            context: Some(Context {
                session,
                workspace,
                additional: None,
            }),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultOutput {
    pub kind: ResultKind,
    pub outcome: Outcome,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Command,
    Operation,
    Assessment,
    Recovery,
}

impl ResultOutput {
    /// Create a successful result output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn success(kind: ResultKind, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            kind,
            outcome: Outcome::Success,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    /// Create a failed result output line.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if `message` is blank.
    pub fn failure(kind: ResultKind, message: Message) -> Result<Self, OutputLineError> {
        Ok(Self {
            kind,
            outcome: Outcome::Failure,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_data(self, data: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recovery {
    pub issue_id: IssueId,
    pub assessment: Assessment,
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Assessment {
    pub severity: ErrorSeverity,
    pub capability: RecoveryCapability,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryAction {
    pub order: u32,
    pub description: String,
    pub execution: RecoveryExecution,
}

impl Recovery {
    #[must_use]
    pub const fn new(issue_id: IssueId, assessment: Assessment) -> Self {
        Self {
            issue_id,
            assessment,
            actions: Vec::new(),
        }
    }

    /// Append a recovery action.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::RecoveryActionOverflow` when the number of
    /// actions cannot be represented as `u32`.
    pub fn with_action(
        self,
        description: String,
        command: Option<String>,
        automatic: bool,
    ) -> Result<Self, OutputLineError> {
        let order = u32::try_from(self.actions.len())
            .map_err(|_| OutputLineError::RecoveryActionOverflow)?;

        let execution = if automatic {
            let cmd = command.unwrap_or_else(|| {
                // Default command if none provided for automatic action
                "echo 'No command specified'".to_string()
            });
            RecoveryExecution::automatic(cmd)
        } else {
            RecoveryExecution::manual()
        };

        Ok(Self {
            actions: self
                .actions
                .into_iter()
                .chain(std::iter::once(RecoveryAction {
                    order,
                    description,
                    execution,
                }))
                .collect(),
            ..self
        })
    }
}

// Backward compatibility helpers
impl Assessment {
    #[must_use]
    pub const fn from_parts(
        severity: ErrorSeverity,
        recoverable: bool,
        recommended_action: String,
    ) -> Self {
        let capability = if recoverable {
            RecoveryCapability::Recoverable { recommended_action }
        } else {
            RecoveryCapability::NotRecoverable {
                reason: recommended_action,
            }
        };
        Self {
            severity,
            capability,
        }
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self.capability, RecoveryCapability::Recoverable { .. })
    }

    #[must_use]
    pub const fn recommended_action(&self) -> Option<&str> {
        match &self.capability {
            RecoveryCapability::Recoverable { recommended_action } => {
                Some(recommended_action.as_str())
            }
            RecoveryCapability::NotRecoverable { .. } => None,
        }
    }
}

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

impl OutputLine {
    #[must_use]
    pub fn conflict_analysis(
        session: &str,
        merge_safe: bool,
        conflicts: Vec<ConflictDetail>,
    ) -> Self {
        let existing_conflicts = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Existing)
            .count();
        let overlapping_files = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Overlapping)
            .count();

        Self::ConflictAnalysis(ConflictAnalysis {
            type_field: "conflictdetail".to_string(),
            session: session.to_string(),
            merge_safe,
            total_conflicts: conflicts.len(),
            conflicts,
            existing_conflicts,
            overlapping_files,
            merge_base: None,
            analysis_time_ms: None,
            timestamp: Utc::now(),
        })
    }
}

/// Session state for output (mirrors `SessionStatus` for JSON output)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Active,
    Paused,
    Creating,
    Completed,
    Failed,
}

/// Type alias for backward compatibility
pub type Session = SessionOutput;

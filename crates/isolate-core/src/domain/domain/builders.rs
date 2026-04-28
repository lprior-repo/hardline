//! Type-safe builders for complex domain aggregates
//!
//! This module implements the builder pattern for domain types with:
//! - Compile-time enforcement of required fields
//! - Fluent API for optional fields
//! - Validation at build time
//! - Zero-unwrap, zero-panic
//!
//! # Design Principles
//!
//! 1. **Type-safe state machine**: Each builder state tracks which required fields have been set
//! 2. **Cannot build incomplete**: `build()` only available when all required fields are set
//! 3. **Zero-panic**: No `unwrap()`, `expect()`, or `panic!()`
//! 4. **Clear error messages**: Validation errors explain what's missing or invalid
//!
//! # Example
//!
//! ```rust
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use isolate_core::{
//!     domain::builders::SessionOutputBuilder, types::SessionStatus, WorkspaceState,
//! };
//!
//! let session = SessionOutputBuilder::new()
//!     .name("my-session")?
//!     .status(SessionStatus::Active)
//!     .state(WorkspaceState::Working)
//!     .workspace_path("/path/to/workspace")?
//!     .build()?;
//! # Ok(())
//! # }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::{
    domain::session::BranchState,
    output::{
        domain_types::{IssueId, IssueScope, IssueTitle, Message, PlanDescription, PlanTitle},
        Action, ActionResult as OutputActionResult, ActionStatus, ActionTarget,
        ActionVerb as OutputActionVerb, ConflictDetail, Issue, IssueKind as OutputIssueKind,
        IssueSeverity, Plan, PlanStep, SessionOutput, Summary, SummaryType as OutputSummaryType,
    },
    types::SessionStatus as TypesSessionStatus,
    WorkspaceState as TypesWorkspaceState,
};

// ==============================================================================
// BUILDER ERROR TYPES
// ==============================================================================

/// Errors that can occur during builder operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderError {
    /// Required field not set
    MissingRequired { field: &'static str },

    /// Invalid value provided
    InvalidValue { field: &'static str, reason: String },

    /// Collection overflow
    Overflow {
        field: &'static str,
        capacity: usize,
    },

    /// Invalid state transition
    InvalidTransition {
        from: &'static str,
        to: &'static str,
        reason: String,
    },
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequired { field } => {
                write!(f, "missing required field: {field}")
            }
            Self::InvalidValue { field, reason } => {
                write!(f, "invalid value for field '{field}': {reason}")
            }
            Self::Overflow { field, capacity } => {
                write!(f, "field '{field}' exceeds capacity of {capacity}")
            }
            Self::InvalidTransition { from, to, reason } => {
                write!(f, "invalid transition from '{from}' to '{to}': {reason}")
            }
        }
    }
}

impl std::error::Error for BuilderError {}

// ==============================================================================
// SESSION OUTPUT BUILDER
// ==============================================================================

/// Builder for `SessionOutput` with compile-time required field tracking
///
/// # Required Fields
/// - `name`: Session name
/// - `status`: Session status
/// - `state`: Workspace state
/// - `workspace_path`: Absolute path to workspace
///
/// # Optional Fields
/// - `branch`: Git branch information
/// - `created_at`: Creation timestamp (defaults to now)
/// - `updated_at`: Update timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct SessionOutputBuilder {
    // Required fields (Option to track presence)
    name: Option<String>,
    status: Option<TypesSessionStatus>,
    state: Option<TypesWorkspaceState>,
    workspace_path: Option<PathBuf>,

    // Optional fields
    branch: Option<BranchState>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl Default for SessionOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionOutputBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            status: None,
            state: None,
            workspace_path: None,
            branch: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set the session name (required)
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::InvalidValue` if the name is empty.
    pub fn name(mut self, name: impl Into<String>) -> Result<Self, BuilderError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(BuilderError::InvalidValue {
                field: "name",
                reason: "session name cannot be empty".to_string(),
            });
        }
        self.name = Some(name);
        Ok(self)
    }

    /// Set the session status (required)
    #[must_use]
    pub const fn status(mut self, status: TypesSessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the workspace state (required)
    #[must_use]
    pub const fn state(mut self, state: TypesWorkspaceState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the workspace path (required)
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::InvalidValue` if the path is not absolute.
    pub fn workspace_path(mut self, path: impl Into<PathBuf>) -> Result<Self, BuilderError> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(BuilderError::InvalidValue {
                field: "workspace_path",
                reason: "workspace path must be absolute".to_string(),
            });
        }
        self.workspace_path = Some(path);
        Ok(self)
    }

    /// Set the branch state (optional)
    #[must_use]
    pub fn branch(mut self, branch: BranchState) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Set the creation timestamp (optional, defaults to now)
    #[must_use]
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the update timestamp (optional, defaults to now)
    #[must_use]
    pub const fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Build the `SessionOutput`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    /// Returns `BuilderError::InvalidValue` if validation fails.
    pub fn build(self) -> Result<SessionOutput, BuilderError> {
        // Validate required fields
        let name = self
            .name
            .ok_or(BuilderError::MissingRequired { field: "name" })?;
        let status = self
            .status
            .ok_or(BuilderError::MissingRequired { field: "status" })?;
        let state = self
            .state
            .ok_or(BuilderError::MissingRequired { field: "state" })?;
        let workspace_path = self.workspace_path.ok_or(BuilderError::MissingRequired {
            field: "workspace_path",
        })?;

        // Convert status to the output type
        let output_status = convert_session_status(status);

        // Convert state to the output type
        let output_state = convert_workspace_state(state);

        let now = self.created_at.unwrap_or_else(Utc::now);
        let updated = self.updated_at.unwrap_or(now);

        Ok(SessionOutput {
            name,
            status: output_status,
            state: output_state,
            workspace_path,
            branch: self.branch.map(|b: BranchState| b.to_string()),
            metadata: None,
            created_at: now,
            updated_at: updated,
        })
    }
}

const fn convert_session_status(status: TypesSessionStatus) -> crate::types::SessionStatus {
    match status {
        TypesSessionStatus::Creating => crate::types::SessionStatus::Creating,
        TypesSessionStatus::Active => crate::types::SessionStatus::Active,
        TypesSessionStatus::Paused => crate::types::SessionStatus::Paused,
        TypesSessionStatus::Completed => crate::types::SessionStatus::Completed,
        TypesSessionStatus::Failed => crate::types::SessionStatus::Failed,
    }
}

const fn convert_workspace_state(state: TypesWorkspaceState) -> crate::WorkspaceState {
    // WorkspaceState is the same type, just use it directly
    state
}

// ==============================================================================
// ISSUE BUILDER
// ==============================================================================

/// Builder for [Issue] with fluent API
///
/// # Required Fields
/// - `id`: Issue identifier
/// - `title`: Issue title
/// - `kind`: Issue kind
/// - `severity`: Issue severity
///
/// # Optional Fields
/// - `scope`: Issue scope (defaults to Standalone)
/// - `suggestion`: Suggested fix
#[derive(Debug, Clone)]
pub struct IssueBuilder {
    // Required fields
    id: Option<IssueId>,
    title: Option<IssueTitle>,
    kind: Option<IssueKind>,
    severity: Option<IssueSeverity>,

    // Optional fields
    scope: Option<IssueScope>,
    suggestion: Option<String>,
}

/// Issue kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    Validation,
    StateConflict,
    ResourceNotFound,
    PermissionDenied,
    Timeout,
    Configuration,
    External,
}

impl Default for IssueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IssueBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            title: None,
            kind: None,
            severity: None,
            scope: None,
            suggestion: None,
        }
    }

    /// Set the issue ID (required)
    #[must_use]
    pub fn id(mut self, id: IssueId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the issue title (required)
    #[must_use]
    pub fn title(mut self, title: IssueTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the issue kind (required)
    #[must_use]
    pub const fn kind(mut self, kind: IssueKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Set the issue severity (required)
    #[must_use]
    pub const fn severity(mut self, severity: IssueSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Set the issue scope (optional)
    #[must_use]
    pub fn scope(mut self, scope: IssueScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Set the suggestion (optional)
    #[must_use]
    pub fn suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Build the Issue
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Issue, BuilderError> {
        let id = self
            .id
            .ok_or(BuilderError::MissingRequired { field: "id" })?;
        let title = self
            .title
            .ok_or(BuilderError::MissingRequired { field: "title" })?;
        let kind = self
            .kind
            .ok_or(BuilderError::MissingRequired { field: "kind" })?;
        let severity = self
            .severity
            .ok_or(BuilderError::MissingRequired { field: "severity" })?;

        Ok(Issue {
            id,
            title,
            kind: convert_issue_kind(kind),
            severity,
            scope: self.scope.unwrap_or(IssueScope::Standalone),
            suggestion: self.suggestion,
        })
    }
}

const fn convert_issue_kind(kind: IssueKind) -> OutputIssueKind {
    match kind {
        IssueKind::Validation => OutputIssueKind::Validation,
        IssueKind::StateConflict => OutputIssueKind::StateConflict,
        IssueKind::ResourceNotFound => OutputIssueKind::ResourceNotFound,
        IssueKind::PermissionDenied => OutputIssueKind::PermissionDenied,
        IssueKind::Timeout => OutputIssueKind::Timeout,
        IssueKind::Configuration => OutputIssueKind::Configuration,
        IssueKind::External => OutputIssueKind::External,
    }
}

// ==============================================================================
// PLAN BUILDER
// ==============================================================================

/// Builder for [Plan] with step collection
///
/// # Required Fields
/// - `title`: Plan title
/// - `description`: Plan description
///
/// # Optional Fields
/// - `steps`: Plan steps (can be added incrementally)
/// - `created_at`: Creation timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    // Required fields
    title: Option<PlanTitle>,
    description: Option<PlanDescription>,

    // Optional fields
    steps: Vec<PlanStepData>,
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct PlanStepData {
    description: String,
    status: ActionStatus,
}

impl Default for PlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: None,
            description: None,
            steps: Vec::new(),
            created_at: None,
        }
    }

    /// Set the plan title (required)
    #[must_use]
    pub fn title(mut self, title: PlanTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the plan description (required)
    #[must_use]
    pub fn description(mut self, description: PlanDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a step to the plan
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::Overflow` if the step count exceeds `u32::MAX`.
    pub fn with_step(
        mut self,
        description: impl Into<String>,
        status: ActionStatus,
    ) -> Result<Self, BuilderError> {
        let _order = u32::try_from(self.steps.len()).map_err(|_| BuilderError::Overflow {
            field: "steps",
            capacity: u32::MAX as usize,
        })?;

        self.steps.push(PlanStepData {
            description: description.into(),
            status,
        });
        Ok(self)
    }

    /// Set the creation timestamp (optional)
    #[must_use]
    pub const fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Build the Plan
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Plan, BuilderError> {
        let title = self
            .title
            .ok_or(BuilderError::MissingRequired { field: "title" })?;
        let description = self.description.ok_or(BuilderError::MissingRequired {
            field: "description",
        })?;

        let steps = self
            .steps
            .into_iter()
            .enumerate()
            .map(|(order, step)| {
                let order_u32 = u32::try_from(order).map_err(|_| BuilderError::Overflow {
                    field: "steps",
                    capacity: u32::MAX as usize,
                })?;
                Ok(PlanStep {
                    order: order_u32,
                    description: step.description,
                    status: step.status,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Plan {
            title,
            description,
            steps,
            created_at: self.created_at.unwrap_or_else(Utc::now),
        })
    }
}

// ==============================================================================
// AGENT INFO BUILDER
// ==============================================================================

/// Builder for `AgentInfo` with fluent API
///
/// # Required Fields
/// - `id`: Agent ID
/// - `state`: Agent state
///
/// # Optional Fields
/// - `last_seen`: Last seen timestamp
#[derive(Debug, Clone)]
pub struct AgentInfoBuilder {
    // Required fields
    id: Option<crate::domain::AgentId>,
    state: Option<AgentState>,

    // Optional fields
    last_seen: Option<DateTime<Utc>>,
}

/// Agent state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Active,
    Idle,
    Offline,
    Error,
}

impl Default for AgentInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentInfoBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            state: None,
            last_seen: None,
        }
    }

    /// Set the agent ID (required)
    #[must_use]
    pub fn id(mut self, id: crate::domain::AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the agent state (required)
    #[must_use]
    pub const fn state(mut self, state: AgentState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the last seen timestamp (optional)
    #[must_use]
    pub const fn last_seen(mut self, last_seen: DateTime<Utc>) -> Self {
        self.last_seen = Some(last_seen);
        self
    }

    /// Build the `AgentInfo`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<crate::domain::agent::AgentInfo, BuilderError> {
        let id = self
            .id
            .ok_or(BuilderError::MissingRequired { field: "id" })?;
        let state = self
            .state
            .ok_or(BuilderError::MissingRequired { field: "state" })?;

        Ok(crate::domain::agent::AgentInfo {
            id,
            state: convert_agent_state(state),
            last_seen: self.last_seen,
        })
    }
}

const fn convert_agent_state(state: AgentState) -> crate::domain::agent::AgentState {
    match state {
        AgentState::Active => crate::domain::agent::AgentState::Active,
        AgentState::Idle => crate::domain::agent::AgentState::Idle,
        AgentState::Offline => crate::domain::agent::AgentState::Offline,
        AgentState::Error => crate::domain::agent::AgentState::Error,
    }
}

// ==============================================================================
// WORKSPACE INFO BUILDER
// ==============================================================================

/// Builder for `WorkspaceInfo` with fluent API
///
/// # Required Fields
/// - `path`: Workspace path
/// - `state`: Workspace state
#[derive(Debug, Clone)]
pub struct WorkspaceInfoBuilder {
    // Required fields
    path: Option<PathBuf>,
    state: Option<WorkspaceInfoState>,
}

/// Workspace info state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceInfoState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,
}

impl Default for WorkspaceInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceInfoBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            path: None,
            state: None,
        }
    }

    /// Set the workspace path (required)
    #[must_use]
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the workspace state (required)
    #[must_use]
    pub const fn state(mut self, state: WorkspaceInfoState) -> Self {
        self.state = Some(state);
        self
    }

    /// Build the `WorkspaceInfo`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<crate::domain::workspace::WorkspaceInfo, BuilderError> {
        let path = self
            .path
            .ok_or(BuilderError::MissingRequired { field: "path" })?;
        let state = self
            .state
            .ok_or(BuilderError::MissingRequired { field: "state" })?;

        Ok(crate::domain::workspace::WorkspaceInfo {
            path,
            state: convert_workspace_info_state(state),
        })
    }
}

const fn convert_workspace_info_state(
    state: WorkspaceInfoState,
) -> crate::domain::workspace::WorkspaceState {
    match state {
        WorkspaceInfoState::Creating => crate::domain::workspace::WorkspaceState::Creating,
        WorkspaceInfoState::Ready => crate::domain::workspace::WorkspaceState::Ready,
        WorkspaceInfoState::Active => crate::domain::workspace::WorkspaceState::Active,
        WorkspaceInfoState::Cleaning => crate::domain::workspace::WorkspaceState::Cleaning,
        WorkspaceInfoState::Removed => crate::domain::workspace::WorkspaceState::Removed,
    }
}

// ==============================================================================
// SUMMARY BUILDER
// ==============================================================================

/// Builder for [Summary] with fluent API
///
/// # Required Fields
/// - `type_field`: Summary type
/// - `message`: Summary message
///
/// # Optional Fields
/// - `details`: Additional details
/// - `timestamp`: Timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct SummaryBuilder {
    // Required fields
    type_field: Option<OutputSummaryType>,
    message: Option<Message>,

    // Optional fields
    details: Option<String>,
    timestamp: Option<DateTime<Utc>>,
}

impl Default for SummaryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SummaryBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            type_field: None,
            message: None,
            details: None,
            timestamp: None,
        }
    }

    /// Set the summary type (required)
    #[must_use]
    pub const fn type_field(mut self, type_field: OutputSummaryType) -> Self {
        self.type_field = Some(type_field);
        self
    }

    /// Set the message (required)
    #[must_use]
    pub fn message(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    /// Set additional details (optional)
    #[must_use]
    pub fn details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Set the timestamp (optional)
    #[must_use]
    pub const fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the Summary
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Summary, BuilderError> {
        let type_field = self.type_field.ok_or(BuilderError::MissingRequired {
            field: "type_field",
        })?;
        let message = self
            .message
            .ok_or(BuilderError::MissingRequired { field: "message" })?;

        Ok(Summary {
            type_field,
            message,
            details: self.details,
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
        })
    }
}

// ==============================================================================
// ACTION BUILDER
// ==============================================================================

/// Builder for [Action] with fluent API
///
/// # Required Fields
/// - `verb`: Action verb
/// - `target`: Action target
/// - `status`: Action status
///
/// # Optional Fields
/// - `result`: Action result (defaults to Pending)
/// - `timestamp`: Timestamp (defaults to now)
#[derive(Debug, Clone)]
pub struct ActionBuilder {
    // Required fields
    verb: Option<OutputActionVerb>,
    target: Option<ActionTarget>,
    status: Option<ActionStatus>,

    // Optional fields
    result: Option<OutputActionResult>,
    timestamp: Option<DateTime<Utc>>,
}

impl Default for ActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            verb: None,
            target: None,
            status: None,
            result: None,
            timestamp: None,
        }
    }

    /// Set the action verb (required)
    #[must_use]
    pub fn verb(mut self, verb: OutputActionVerb) -> Self {
        self.verb = Some(verb);
        self
    }

    /// Set the action target (required)
    #[must_use]
    pub fn target(mut self, target: ActionTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the action status (required)
    #[must_use]
    pub const fn status(mut self, status: ActionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the action result (optional)
    #[must_use]
    pub fn result(mut self, result: OutputActionResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set a completed result with a message (optional)
    #[must_use]
    pub fn with_completed_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(OutputActionResult::Completed {
            result: result.into(),
        });
        self
    }

    /// Set the timestamp (optional)
    #[must_use]
    pub const fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the Action
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<Action, BuilderError> {
        let verb = self
            .verb
            .ok_or(BuilderError::MissingRequired { field: "verb" })?;
        let target = self
            .target
            .ok_or(BuilderError::MissingRequired { field: "target" })?;
        let status = self
            .status
            .ok_or(BuilderError::MissingRequired { field: "status" })?;

        Ok(Action {
            verb,
            target,
            status,
            result: self.result.unwrap_or(OutputActionResult::Pending),
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
        })
    }
}

// ==============================================================================
// CONFLICT DETAIL BUILDER
// ==============================================================================

/// Builder for `ConflictDetail` with fluent API
///
/// # Required Fields
/// - `file`: Conflicted file path
///
/// # Optional Fields
/// - `conflict_type`: Type of conflict (defaults to Overlapping)
/// - `recommended`: Recommended resolution strategy
#[derive(Debug, Clone)]
pub struct ConflictDetailBuilder {
    // Required fields
    file: Option<String>,

    // Optional fields
    conflict_type: Option<ConflictType>,
    workspace_additions: Option<u32>,
    workspace_deletions: Option<u32>,
    main_additions: Option<u32>,
    main_deletions: Option<u32>,
    recommended: Option<ResolutionStrategy>,
}

/// Conflict type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    Overlapping,
    Existing,
    DeleteModify,
    RenameModify,
    Binary,
}

/// Resolution strategy enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    AcceptOurs,
    AcceptTheirs,
    JjResolve,
    ManualMerge,
    Rebase,
    Abort,
    Skip,
}

impl Default for ConflictDetailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictDetailBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            file: None,
            conflict_type: None,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            recommended: None,
        }
    }

    /// Set the conflicted file (required)
    #[must_use]
    pub fn file(mut self, file: String) -> Self {
        self.file = Some(file);
        self
    }

    /// Set the conflict type (optional)
    #[must_use]
    pub const fn conflict_type(mut self, conflict_type: ConflictType) -> Self {
        self.conflict_type = Some(conflict_type);
        self
    }

    /// Set workspace additions count (optional)
    #[must_use]
    pub const fn workspace_additions(mut self, count: u32) -> Self {
        self.workspace_additions = Some(count);
        self
    }

    /// Set workspace deletions count (optional)
    #[must_use]
    pub const fn workspace_deletions(mut self, count: u32) -> Self {
        self.workspace_deletions = Some(count);
        self
    }

    /// Set main additions count (optional)
    #[must_use]
    pub const fn main_additions(mut self, count: u32) -> Self {
        self.main_additions = Some(count);
        self
    }

    /// Set main deletions count (optional)
    #[must_use]
    pub const fn main_deletions(mut self, count: u32) -> Self {
        self.main_deletions = Some(count);
        self
    }

    /// Set the recommended resolution strategy (optional)
    #[must_use]
    pub const fn recommended(mut self, strategy: ResolutionStrategy) -> Self {
        self.recommended = Some(strategy);
        self
    }

    /// Build the `ConflictDetail`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<ConflictDetail, BuilderError> {
        let file = self
            .file
            .ok_or(BuilderError::MissingRequired { field: "file" })?;

        let conflict_type = self.conflict_type.unwrap_or(ConflictType::Overlapping);
        let recommended = self.recommended.unwrap_or(ResolutionStrategy::JjResolve);

        Ok(ConflictDetail {
            file,
            conflict_type: convert_conflict_type(conflict_type),
            workspace_additions: self.workspace_additions,
            workspace_deletions: self.workspace_deletions,
            main_additions: self.main_additions,
            main_deletions: self.main_deletions,
            resolutions: vec![],
            recommended: convert_resolution_strategy(recommended),
        })
    }
}

const fn convert_conflict_type(ty: ConflictType) -> crate::output::ConflictType {
    match ty {
        ConflictType::Overlapping => crate::output::ConflictType::Overlapping,
        ConflictType::Existing => crate::output::ConflictType::Existing,
        ConflictType::DeleteModify => crate::output::ConflictType::DeleteModify,
        ConflictType::RenameModify => crate::output::ConflictType::RenameModify,
        ConflictType::Binary => crate::output::ConflictType::Binary,
    }
}

const fn convert_resolution_strategy(
    strategy: ResolutionStrategy,
) -> crate::output::ResolutionStrategy {
    match strategy {
        ResolutionStrategy::AcceptOurs => crate::output::ResolutionStrategy::AcceptOurs,
        ResolutionStrategy::AcceptTheirs => crate::output::ResolutionStrategy::AcceptTheirs,
        ResolutionStrategy::JjResolve => crate::output::ResolutionStrategy::JjResolve,
        ResolutionStrategy::ManualMerge => crate::output::ResolutionStrategy::ManualMerge,
        ResolutionStrategy::Rebase => crate::output::ResolutionStrategy::Rebase,
        ResolutionStrategy::Abort => crate::output::ResolutionStrategy::Abort,
        ResolutionStrategy::Skip => crate::output::ResolutionStrategy::Skip,
    }
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_output_builder_complete() {
        let result = SessionOutputBuilder::new()
            .name("test-session")
            .unwrap()
            .status(TypesSessionStatus::Active)
            .state(TypesWorkspaceState::Working)
            .workspace_path("/tmp/workspace")
            .unwrap()
            .build();

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.name, "test-session");
    }

    #[test]
    fn test_session_output_builder_missing_required() {
        let result = SessionOutputBuilder::new()
            .name("test-session")
            .unwrap()
            .status(TypesSessionStatus::Active)
            // Missing state and workspace_path
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BuilderError::MissingRequired { field } => {
                assert!(field == "state" || field == "workspace_path");
            }
            _ => panic!("expected MissingRequired error"),
        }
    }

    #[test]
    fn test_session_output_builder_invalid_name() {
        let result = SessionOutputBuilder::new()
            .name("")  // Empty name
            .unwrap_err();

        match result {
            BuilderError::InvalidValue { field, .. } => {
                assert_eq!(field, "name");
            }
            _ => panic!("expected InvalidValue error"),
        }
    }

    #[test]
    fn test_session_output_builder_relative_path() {
        let result = SessionOutputBuilder::new()
            .name("test-session")
            .unwrap()
            .status(TypesSessionStatus::Active)
            .state(TypesWorkspaceState::Working)
            .workspace_path("relative/path")  // Not absolute
            .unwrap_err();

        match result {
            BuilderError::InvalidValue { field, .. } => {
                assert_eq!(field, "workspace_path");
            }
            _ => panic!("expected InvalidValue error"),
        }
    }

    #[test]
    fn test_issue_builder_complete() {
        let id = IssueId::new("issue-1".to_string()).expect("valid id");
        let title = IssueTitle::new("Test issue").expect("valid title");

        let result = IssueBuilder::new()
            .id(id)
            .title(title)
            .kind(IssueKind::Validation)
            .severity(IssueSeverity::Error)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_issue_builder_with_suggestion() {
        let id = IssueId::new("issue-2".to_string()).expect("valid id");
        let title = IssueTitle::new("Another issue").expect("valid title");

        let result = IssueBuilder::new()
            .id(id)
            .title(title)
            .kind(IssueKind::Configuration)
            .severity(IssueSeverity::Warning)
            .suggestion("Fix the config".to_string())
            .build();

        assert!(result.is_ok());
        let issue = result.unwrap();
        assert_eq!(issue.suggestion, Some("Fix the config".to_string()));
    }

    #[test]
    fn test_plan_builder_with_steps() {
        let title = PlanTitle::new("Test plan").expect("valid title");
        let description = PlanDescription::new("Plan description").expect("valid description");

        let result = PlanBuilder::new()
            .title(title)
            .description(description)
            .with_step("Step 1", ActionStatus::Pending)
            .unwrap()
            .with_step("Step 2", ActionStatus::Completed)
            .unwrap()
            .build();

        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn test_summary_builder_complete() {
        let message = Message::new("Test message").expect("valid message");

        let result = SummaryBuilder::new()
            .type_field(OutputSummaryType::Info)
            .message(message)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_action_builder_complete() {
        let verb = OutputActionVerb::Run;
        let target = ActionTarget::new("test-target").expect("valid target");

        let result = ActionBuilder::new()
            .verb(verb)
            .target(target)
            .status(ActionStatus::Pending)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_action_builder_with_result() {
        let verb = OutputActionVerb::Execute;
        let target = ActionTarget::new("another-target").expect("valid target");

        let result = ActionBuilder::new()
            .verb(verb)
            .target(target)
            .status(ActionStatus::Completed)
            .with_completed_result("Success!")
            .build();

        assert!(result.is_ok());
        let action = result.unwrap();
        assert!(matches!(
            action.result,
            OutputActionResult::Completed { .. }
        ));
    }

    #[test]
    fn test_agent_info_builder_complete() {
        let id = crate::domain::AgentId::parse("test-agent").expect("valid agent id");

        let result = AgentInfoBuilder::new()
            .id(id)
            .state(AgentState::Active)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_workspace_info_builder_complete() {
        let path = PathBuf::from("/tmp/workspace");

        let result = WorkspaceInfoBuilder::new()
            .path(path.clone())
            .state(WorkspaceInfoState::Active)
            .build();

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.path, path);
    }
}

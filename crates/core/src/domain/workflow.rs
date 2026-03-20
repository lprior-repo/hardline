//! # Durable Workflow Execution
//!
//! Implements ADR-002: Durable Workflow Execution for multi-step operations
//! that survive crashes, with saga pattern and automatic compensation.
//!
//! ## Core Concept: Step Journal
//!
//! Every durable operation maintains a **journal** of steps. On restart,
//! the system replays the journal, skipping completed steps.
//!
//! ## Architecture
//!
//! - [`OperationState`] - State machine for durable operations
//! - [`OperationRecord`] - Record tracking a durable operation
//! - [`StepStatus`] - Status of individual steps
//! - [`StepRecord`] - Record of a single step in the journal
//! - [`CompensationState`] - Two-phase compensation state machine
//! - [`PipelineState`] - Orchestrator pipeline state machine

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// =============================================================================
// Operation State Machine
// =============================================================================

/// State of a durable operation (tracks multi-step AI workflows)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    /// Operation created, not yet running
    Started,
    /// Currently executing
    InProgress,
    /// Successfully finished
    Completed,
    /// Permanently failed (no more retries)
    Failed,
}

impl OperationState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "started" => Some(Self::Started),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// hardline-specific operation status with more states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Operation created, waiting to start
    Pending,
    /// Currently executing steps
    Running,
    /// All steps completed successfully
    Completed,
    /// Failed with error (may have partial compensation)
    Failed,
    /// Waiting on external input (promise/awakeable)
    Suspended,
    /// Compensation in progress (rolling back)
    Compensating,
}

impl OperationStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Suspended => "suspended",
            Self::Compensating => "compensating",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "suspended" => Some(Self::Suspended),
            "compensating" => Some(Self::Compensating),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

// =============================================================================
// Operation Record
// =============================================================================

/// Record of a durable operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation_id: String,
    pub state: OperationState,
    pub current_step: u32,
    pub total_steps: u32,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub final_revision: Option<i64>,
    pub error_message: Option<String>,
    pub author_id: String,
    pub description: String,
}

impl OperationRecord {
    pub fn new(
        operation_id: String,
        author_id: String,
        description: String,
        total_steps: u32,
    ) -> Self {
        Self {
            operation_id,
            state: OperationState::Started,
            current_step: 0,
            total_steps,
            started_at: Utc::now().timestamp(),
            completed_at: None,
            final_revision: None,
            error_message: None,
            author_id,
            description,
        }
    }

    pub fn with_state(mut self, state: OperationState) -> Self {
        self.state = state;
        self
    }

    pub fn advance_step(&mut self) {
        self.current_step += 1;
    }

    pub fn complete(&mut self) {
        self.state = OperationState::Completed;
        self.completed_at = Some(Utc::now().timestamp());
    }

    pub fn fail(&mut self, error: String) {
        self.state = OperationState::Failed;
        self.completed_at = Some(Utc::now().timestamp());
        self.error_message = Some(error);
    }

    #[must_use]
    pub fn progress_percentage(&self) -> f64 {
        if self.total_steps == 0 {
            0.0
        } else {
            (self.current_step as f64 / self.total_steps as f64) * 100.0
        }
    }
}

// =============================================================================
// Step Journal
// =============================================================================

/// Status of a single step within an operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step not yet started
    Pending,
    /// Step currently executing
    Running,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Skipped due to earlier failure (compensation)
    Skipped,
}

impl StepStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }
}

/// Record of a single step in the step journal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub status: StepStatus,
    pub event_revision: Option<i64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

impl StepRecord {
    pub fn new(operation_id: String, step_index: u32, step_name: String) -> Self {
        Self {
            operation_id,
            step_index,
            step_name,
            status: StepStatus::Pending,
            event_revision: None,
            created_at: Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }

    pub fn start(&mut self) {
        self.status = StepStatus::Running;
        self.started_at = Some(Utc::now().timestamp());
    }

    pub fn complete(&mut self) {
        self.status = StepStatus::Completed;
        self.completed_at = Some(Utc::now().timestamp());
    }

    pub fn fail(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.completed_at = Some(Utc::now().timestamp());
        self.error_message = Some(error);
    }

    pub fn skip(&mut self) {
        self.status = StepStatus::Skipped;
        self.completed_at = Some(Utc::now().timestamp());
    }

    #[must_use]
    pub fn duration_ms(&self) -> Option<i64> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}

// =============================================================================
// Journal Structure
// =============================================================================

/// Journal entry states for two-phase compensation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalState {
    /// Pending external work
    PendingExternal,
    /// Compensation in progress (rolling back)
    Compensating,
    /// Operation completed successfully
    Done,
    /// Compensation failed (needs manual intervention)
    FailedCompensation,
}

impl JournalState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PendingExternal => "pending_external",
            Self::Compensating => "compensating",
            Self::Done => "done",
            Self::FailedCompensation => "failed_compensation",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_external" => Some(Self::PendingExternal),
            "compensating" => Some(Self::Compensating),
            "done" => Some(Self::Done),
            "failed_compensation" => Some(Self::FailedCompensation),
            _ => None,
        }
    }
}

/// Journal entry for tracking operation progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub operation_id: String,
    pub state: JournalState,
    pub step_index: Option<u32>,
    pub error_message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl JournalEntry {
    pub fn new(operation_id: String, state: JournalState) -> Self {
        Self {
            operation_id,
            state,
            step_index: None,
            error_message: None,
            updated_at: Utc::now(),
        }
    }

    pub fn with_step(mut self, step_index: u32) -> Self {
        self.step_index = Some(step_index);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }
}

// =============================================================================
// Two-Phase Compensation
// =============================================================================

/// Compensation state machine for saga pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompensationState {
    /// Operation succeeded, no compensation needed
    NoCompensationNeeded,
    /// Currently rolling back
    CompensationInProgress,
    /// Rollback succeeded
    CompensationCompleted,
    /// Rollback failed (needs manual intervention)
    CompensationFailed,
}

impl CompensationState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoCompensationNeeded => "no_compensation_needed",
            Self::CompensationInProgress => "compensation_in_progress",
            Self::CompensationCompleted => "compensation_completed",
            Self::CompensationFailed => "compensation_failed",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "no_compensation_needed" => Some(Self::NoCompensationNeeded),
            "compensation_in_progress" => Some(Self::CompensationInProgress),
            "compensation_completed" => Some(Self::CompensationCompleted),
            "compensation_failed" => Some(Self::CompensationFailed),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::NoCompensationNeeded | Self::CompensationCompleted | Self::CompensationFailed
        )
    }
}

/// Compensation action for a single step
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationAction {
    pub step_index: u32,
    pub step_name: String,
    pub compensate_fn: String,
    pub status: StepStatus,
    pub error_message: Option<String>,
}

impl CompensationAction {
    pub fn new(step_index: u32, step_name: String, compensate_fn: String) -> Self {
        Self {
            step_index,
            step_name,
            compensate_fn,
            status: StepStatus::Pending,
            error_message: None,
        }
    }

    pub fn mark_compensated(&mut self) {
        self.status = StepStatus::Completed;
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.error_message = Some(error);
    }
}

// =============================================================================
// Pipeline State Machine (Orchestrator)
// =============================================================================

/// Unique identifier for a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineId(pub String);

impl PipelineId {
    #[must_use]
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        
        Self(format!("pipeline-{}", timestamp))
    }
}

impl Default for PipelineId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PipelineId({})", self.0)
    }
}

/// Pipeline state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    /// Initial state - pipeline created but not started
    Pending,
    /// Running linter on spec
    SpecReview,
    /// Deploying twin/universe
    UniverseSetup,
    /// Agent working (with iteration count)
    AgentDevelopment,
    /// Running scenarios for validation
    Validation,
    /// All scenarios passed - artifact ready for merge
    Accepted,
    /// Human intervention needed
    Escalated,
    /// Validation failed permanently
    Failed,
}

impl PipelineState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineState::Accepted | PipelineState::Escalated | PipelineState::Failed
        )
    }

    #[must_use]
    pub fn allows_iteration(&self) -> bool {
        matches!(self, PipelineState::AgentDevelopment)
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            PipelineState::Pending => "Pending - awaiting start",
            PipelineState::SpecReview => "Spec Review - running linter",
            PipelineState::UniverseSetup => "Universe Setup - deploying twin",
            PipelineState::AgentDevelopment => "Agent Development - working on task",
            PipelineState::Validation => "Validation - running scenarios",
            PipelineState::Accepted => "Accepted - all scenarios passed",
            PipelineState::Escalated => "Escalated - human intervention needed",
            PipelineState::Failed => "Failed - validation failed",
        }
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub max_iterations: u32,
    pub quality_threshold: u32,
    pub scenarios_path: String,
    pub linter_path: Option<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            quality_threshold: 80,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        }
    }
}

/// Pipeline instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub spec_path: String,
    pub state: PipelineState,
    pub iteration: u32,
    pub max_iterations: u32,
    pub quality_threshold: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

impl Pipeline {
    #[must_use]
    pub fn new(spec_path: String) -> Self {
        let now = Utc::now();
        Self {
            id: PipelineId::new(),
            spec_path,
            state: PipelineState::Pending,
            iteration: 0,
            max_iterations: 10,
            quality_threshold: 80,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    #[must_use]
    pub fn with_config(spec_path: String, config: &PipelineConfig) -> Self {
        let now = Utc::now();
        Self {
            id: PipelineId::new(),
            spec_path,
            state: PipelineState::Pending,
            iteration: 0,
            max_iterations: config.max_iterations,
            quality_threshold: config.quality_threshold,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    pub fn transition_to(&mut self, new_state: PipelineState) -> Result<(), PipelineTransitionError> {
        match (&self.state, &new_state) {
            (PipelineState::Pending, PipelineState::SpecReview) => {}
            (PipelineState::SpecReview, PipelineState::UniverseSetup) => {}
            (PipelineState::SpecReview, PipelineState::Failed) => {}
            (PipelineState::SpecReview, PipelineState::Escalated) => {}
            (PipelineState::UniverseSetup, PipelineState::AgentDevelopment) => {}
            (PipelineState::UniverseSetup, PipelineState::Failed) => {}
            (PipelineState::UniverseSetup, PipelineState::Escalated) => {}
            (PipelineState::AgentDevelopment, PipelineState::Validation) => {}
            (PipelineState::AgentDevelopment, PipelineState::AgentDevelopment) => {}
            (PipelineState::AgentDevelopment, PipelineState::Escalated) => {}
            (PipelineState::Validation, PipelineState::Accepted) => {}
            (PipelineState::Validation, PipelineState::AgentDevelopment) => {}
            (PipelineState::Validation, PipelineState::Failed) => {}
            (PipelineState::Validation, PipelineState::Escalated) => {}
            (state, _) if state.is_terminal() => {
                return Err(PipelineTransitionError::AlreadyTerminal { current: *state });
            }
            (_, PipelineState::Failed) => {}
            (_, PipelineState::Escalated) => {}
            _ => {
                return Err(PipelineTransitionError::InvalidTransition {
                    from: self.state,
                    to: new_state,
                });
            }
        }

        self.state = new_state;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn increment_iteration(&mut self) -> Result<u32, IterationLimitError> {
        if self.iteration >= self.max_iterations {
            return Err(IterationLimitError {
                current: self.iteration,
                max: self.max_iterations,
            });
        }
        self.iteration += 1;
        self.updated_at = Utc::now();
        Ok(self.iteration)
    }

    #[must_use]
    pub fn can_iterate(&self) -> bool {
        self.iteration < self.max_iterations && self.state.allows_iteration()
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.updated_at = Utc::now();
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
        self.updated_at = Utc::now();
    }
}

/// Error when transitioning pipeline states
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineTransitionError {
    InvalidTransition {
        from: PipelineState,
        to: PipelineState,
    },
    AlreadyTerminal {
        current: PipelineState,
    },
}

impl std::fmt::Display for PipelineTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineTransitionError::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {from:?} to {to:?}")
            }
            PipelineTransitionError::AlreadyTerminal { current } => {
                write!(f, "Pipeline already in terminal state: {current:?}")
            }
        }
    }
}

impl std::error::Error for PipelineTransitionError {}

/// Error during iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IterationLimitError {
    pub current: u32,
    pub max: u32,
}

impl std::fmt::Display for IterationLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Max iterations reached: {} of {}", self.current, self.max)
    }
}

impl std::error::Error for IterationLimitError {}

// =============================================================================
// Recovery
// =============================================================================

/// Recovery task for resuming incomplete operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryTask {
    pub operation_id: String,
    pub resume_from_step: u32,
    pub attempted_at: DateTime<Utc>,
}

impl RecoveryTask {
    pub fn new(operation_id: String, resume_from_step: u32) -> Self {
        Self {
            operation_id,
            resume_from_step,
            attempted_at: Utc::now(),
        }
    }
}

/// Recovery report after scanning and recovering operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub total_incomplete: usize,
    pub recovered: usize,
    pub failed: usize,
    pub recovered_operations: Vec<String>,
    pub failed_operations: Vec<(String, String)>,
}

impl RecoveryReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_recovered(&mut self, operation_id: String) {
        self.recovered += 1;
        self.total_incomplete += 1;
        self.recovered_operations.push(operation_id);
    }

    pub fn add_failed(&mut self, operation_id: String, error: String) {
        self.failed += 1;
        self.total_incomplete += 1;
        self.failed_operations.push((operation_id, error));
    }
}

// =============================================================================
// Domain Events
// =============================================================================

/// Domain events for workflow execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum WorkflowEvent {
    OperationStarted(OperationStartedEvent),
    OperationCompleted(OperationCompletedEvent),
    OperationFailed(OperationFailedEvent),
    StepStarted(StepStartedEvent),
    StepCompleted(StepCompletedEvent),
    StepFailed(StepFailedEvent),
    CompensationStarted(CompensationStartedEvent),
    CompensationCompleted(CompensationCompletedEvent),
    CompensationFailed(CompensationFailedEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationStartedEvent {
    pub operation_id: String,
    pub total_steps: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationCompletedEvent {
    pub operation_id: String,
    pub completed_steps: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationFailedEvent {
    pub operation_id: String,
    pub failed_step: u32,
    pub error: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepStartedEvent {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepCompletedEvent {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub duration_ms: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepFailedEvent {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub error: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationStartedEvent {
    pub operation_id: String,
    pub steps_to_compensate: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationCompletedEvent {
    pub operation_id: String,
    pub compensated_steps: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompensationFailedEvent {
    pub operation_id: String,
    pub failed_step: u32,
    pub error: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_state_is_terminal() {
        assert!(!OperationState::Started.is_terminal());
        assert!(!OperationState::InProgress.is_terminal());
        assert!(OperationState::Completed.is_terminal());
        assert!(OperationState::Failed.is_terminal());
    }

    #[test]
    fn test_operation_state_serialization() {
        assert_eq!(OperationState::Started.as_str(), "started");
        assert_eq!(OperationState::InProgress.as_str(), "in_progress");
        assert_eq!(OperationState::Completed.as_str(), "completed");
        assert_eq!(OperationState::Failed.as_str(), "failed");

        assert_eq!(OperationState::from_str("started"), Some(OperationState::Started));
        assert_eq!(OperationState::from_str("in_progress"), Some(OperationState::InProgress));
        assert_eq!(OperationState::from_str("completed"), Some(OperationState::Completed));
        assert_eq!(OperationState::from_str("failed"), Some(OperationState::Failed));
        assert_eq!(OperationState::from_str("invalid"), None);
    }

    #[test]
    fn test_step_status_is_terminal() {
        assert!(!StepStatus::Pending.is_terminal());
        assert!(!StepStatus::Running.is_terminal());
        assert!(StepStatus::Completed.is_terminal());
        assert!(StepStatus::Failed.is_terminal());
        assert!(StepStatus::Skipped.is_terminal());
    }

    #[test]
    fn test_step_record_lifecycle() {
        let mut step = StepRecord::new(
            "op-123".to_string(),
            0,
            "create-db-record".to_string(),
        );

        assert_eq!(step.status, StepStatus::Pending);
        assert!(step.started_at.is_none());

        step.start();
        assert_eq!(step.status, StepStatus::Running);
        assert!(step.started_at.is_some());

        step.complete();
        assert_eq!(step.status, StepStatus::Completed);
        assert!(step.completed_at.is_some());
        assert!(step.duration_ms().is_some());
    }

    #[test]
    fn test_step_record_fail() {
        let mut step = StepRecord::new(
            "op-123".to_string(),
            0,
            "create-workspace".to_string(),
        );

        step.start();
        step.fail("Directory already exists".to_string());

        assert_eq!(step.status, StepStatus::Failed);
        assert_eq!(step.error_message, Some("Directory already exists".to_string()));
    }

    #[test]
    fn test_operation_record_progress() {
        let mut op = OperationRecord::new(
            "op-123".to_string(),
            "author-1".to_string(),
            "Test operation".to_string(),
            5,
        );

        assert_eq!(op.progress_percentage(), 0.0);

        op.advance_step();
        assert_eq!(op.progress_percentage(), 20.0);

        op.advance_step();
        assert_eq!(op.progress_percentage(), 40.0);

        op.complete();
        assert!(op.completed_at.is_some());
        assert!(op.is_terminal());
    }

    #[test]
    fn test_pipeline_valid_transitions() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        
        assert!(pipeline.transition_to(PipelineState::SpecReview).is_ok());
        assert!(pipeline.transition_to(PipelineState::UniverseSetup).is_ok());
        assert!(pipeline.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert!(pipeline.transition_to(PipelineState::Validation).is_ok());
        assert!(pipeline.transition_to(PipelineState::Accepted).is_ok());
    }

    #[test]
    fn test_pipeline_invalid_transition() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        
        let result = pipeline.transition_to(PipelineState::Validation);
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_terminal_state_no_transition() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        
        pipeline.transition_to(PipelineState::SpecReview).ok();
        pipeline.transition_to(PipelineState::UniverseSetup).ok();
        pipeline.transition_to(PipelineState::AgentDevelopment).ok();
        pipeline.transition_to(PipelineState::Validation).ok();
        pipeline.transition_to(PipelineState::Accepted).ok();

        assert!(pipeline.state.is_terminal());
        
        let result = pipeline.transition_to(PipelineState::Failed);
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_iteration_limit() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.transition_to(PipelineState::SpecReview).ok();
        pipeline.transition_to(PipelineState::UniverseSetup).ok();
        pipeline.transition_to(PipelineState::AgentDevelopment).ok();

        for _ in 0..10 {
            assert!(pipeline.increment_iteration().is_ok());
        }

        assert!(pipeline.increment_iteration().is_err());
    }

    #[test]
    fn test_pipeline_can_iterate() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        
        assert!(!pipeline.can_iterate());
        
        pipeline.transition_to(PipelineState::SpecReview).ok();
        pipeline.transition_to(PipelineState::UniverseSetup).ok();
        pipeline.transition_to(PipelineState::AgentDevelopment).ok();

        assert!(pipeline.can_iterate());
        
        for _ in 0..10 {
            pipeline.increment_iteration().ok();
        }

        assert!(!pipeline.can_iterate());
    }

    #[test]
    fn test_compensation_state_is_terminal() {
        assert!(!CompensationState::CompensationInProgress.is_terminal());
        assert!(CompensationState::NoCompensationNeeded.is_terminal());
        assert!(CompensationState::CompensationCompleted.is_terminal());
        assert!(CompensationState::CompensationFailed.is_terminal());
    }

    #[test]
    fn test_recovery_report() {
        let mut report = RecoveryReport::new();
        
        report.add_recovered("op-1".to_string());
        report.add_recovered("op-2".to_string());
        report.add_failed("op-3".to_string(), "Database locked".to_string());

        assert_eq!(report.total_incomplete, 3);
        assert_eq!(report.recovered, 2);
        assert_eq!(report.failed, 1);
        assert_eq!(report.recovered_operations.len(), 2);
        assert_eq!(report.failed_operations.len(), 1);
    }

    #[test]
    fn test_journal_state_serialization() {
        assert_eq!(JournalState::PendingExternal.as_str(), "pending_external");
        assert_eq!(JournalState::Compensating.as_str(), "compensating");
        assert_eq!(JournalState::Done.as_str(), "done");
        assert_eq!(JournalState::FailedCompensation.as_str(), "failed_compensation");

        assert_eq!(JournalState::from_str("pending_external"), Some(JournalState::PendingExternal));
        assert_eq!(JournalState::from_str("compensating"), Some(JournalState::Compensating));
        assert_eq!(JournalState::from_str("done"), Some(JournalState::Done));
        assert_eq!(JournalState::from_str("failed_compensation"), Some(JournalState::FailedCompensation));
    }

    #[test]
    fn test_pipeline_id() {
        let id1 = PipelineId::new();
        let id2 = PipelineId::new();
        
        assert_ne!(id1, id2);
        assert!(id1.to_string().starts_with("PipelineId("));
    }

    #[test]
    fn test_pipeline_with_config() {
        let config = PipelineConfig {
            max_iterations: 5,
            quality_threshold: 90,
            scenarios_path: "test-scenarios".to_string(),
            linter_path: Some("/usr/bin/linter".to_string()),
        };

        let pipeline = Pipeline::with_config("spec.yaml".to_string(), &config);
        
        assert_eq!(pipeline.max_iterations, 5);
        assert_eq!(pipeline.quality_threshold, 90);
    }

    #[test]
    fn test_workflow_events() {
        let event = WorkflowEvent::StepCompleted(StepCompletedEvent {
            operation_id: "op-123".to_string(),
            step_index: 2,
            step_name: "create-workspace".to_string(),
            duration_ms: Some(150),
            timestamp: Utc::now(),
        });

        assert!(matches!(event, WorkflowEvent::StepCompleted(_)));
    }
}

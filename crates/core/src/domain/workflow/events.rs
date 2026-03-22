//! Domain events for workflow execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// =============================================================================
// Workflow Events
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

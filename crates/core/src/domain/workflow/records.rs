//! Record types for durable workflow execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::workflow::states::{JournalState, OperationState, StepStatus};

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
// Step Record
// =============================================================================

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
// Journal Entry
// =============================================================================

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
// Compensation Action
// =============================================================================

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

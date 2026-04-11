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

pub mod events;
pub mod pipeline;
pub mod records;
pub mod saga;
pub mod states;

// Tests
#[cfg(test)]
mod _tests;

// Re-exports for convenience
pub use events::{
    CompensationCompletedEvent, CompensationFailedEvent, CompensationStartedEvent,
    OperationCompletedEvent, OperationFailedEvent, OperationStartedEvent, StepCompletedEvent,
    StepFailedEvent, StepStartedEvent, WorkflowEvent,
};
pub use pipeline::{
    IterationLimitError, Pipeline, PipelineConfig, PipelineId, PipelineTransitionError,
};
pub use records::{
    CompensationAction, JournalEntry, OperationRecord, RecoveryReport, RecoveryTask, StepRecord,
};
pub use saga::{SagaDefinition, SagaExecutor, SagaJournal, SagaResult, SagaStep, StepExecutor};
pub use states::{
    CompensationState, JournalState, OperationState, OperationStatus, PipelineState, StepStatus,
};

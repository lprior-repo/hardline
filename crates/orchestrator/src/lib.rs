//! Orchestrator crate for pipeline state machine
//!
//! This crate provides the pipeline orchestration logic including:
//! - State machine for pipeline phases
//! - State persistence for crash recovery
//! - Phase execution
//! - Metrics collection

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod cleanup;
pub mod metrics;
pub mod parallel;
#[cfg(test)]
mod parallel_tests;
pub mod persistence;
pub mod phases;
pub mod policies;
pub mod queue;
pub mod state;

pub use cleanup::{
    CleanupContext, CleanupError, CleanupHandler, CleanupManager, CleanupResult, CleanupStatus,
    PhaseType, ResourceId,
};
pub use metrics::{AggregatedMetrics, Metrics, PhaseMetrics, PipelineMetrics, ScenarioResult};
pub use persistence::{StateStore, StoreError};
pub use phases::{PhaseError, PipelineExecutor};
pub use policies::{
    CircuitBreaker, CircuitBreakerError, CircuitBreakerState, ConfigError, Deadline,
    OrchestratorError, PhaseTimeout, PolicyConfig,
    PolicyError, RetryPolicy, RetryPolicyError, TimeoutError, TimeoutPolicy, TimeoutPolicyError,
};
pub use parallel::{ParallelError, PhaseStatus};
pub use queue::{
    InMemoryJobRepository, Job, JobOutcome, JobPayload, JobPriority, JobProcessor,
    JobProcessorConfig, JobRepository, JobResult, JobState, QueueError, QueueResult,
};
pub use state::{IterationError, Pipeline, PipelineConfig, PipelineId, PipelineState, TransitionError};

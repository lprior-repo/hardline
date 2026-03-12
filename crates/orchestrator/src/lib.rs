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
pub mod persistence;
pub mod phases;
pub mod policies;
pub mod state;

pub use cleanup::{
    CleanupContext, CleanupError, CleanupHandler, CleanupManager, CleanupResult, PhaseType,
    ResourceId,
};
pub use metrics::{Metrics, PhaseMetrics, ScenarioResult};
pub use persistence::StateStore;
pub use phases::{PhaseError, PipelineExecutor};
pub use policies::{
    CircuitBreaker, CircuitBreakerState, ConfigError, Deadline, OrchestratorError, PhaseTimeout,
    PolicyConfig, RetryPolicy,
};
pub use state::{Pipeline, PipelineId, PipelineState};

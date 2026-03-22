//! Phase types and error definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cleanup::PhaseType;
use crate::metrics::ScenarioResult;

/// Errors that can occur during phase execution
#[derive(Debug, Clone, Error)]
pub enum PhaseError {
    #[error("Spec review failed: {0}")]
    SpecReviewFailed(String),

    #[error("Universe setup failed: {0}")]
    SetupFailed(String),

    #[error("Agent development failed: {0}")]
    DevelopmentFailed(String),

    #[error("Scenario validation failed: {0}")]
    ValidationFailed(String),

    #[error("Cleanup/rollback failed: {0}")]
    CleanupFailed(String),

    #[error("State persistence failed: {0}")]
    PersistenceFailed(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Parallel execution failed: {0}")]
    ParallelExecutionFailed(String),

    #[error("Dependency not met for phase: {0:?}")]
    DependencyNotMet(PhaseType),
}

/// Result of a phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub success: bool,
    pub message: String,
    pub quality_score: Option<u32>,
    pub scenario_results: Vec<ScenarioResult>,
}

/// Decision made after validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Accept,
    Retry,
    Escalate,
    Fail,
}

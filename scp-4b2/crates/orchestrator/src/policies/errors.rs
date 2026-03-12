//! Policy: Error types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration errors for policy creation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigError {
    InvalidTimeout {
        duration_ms: u64,
    },
    InvalidBaseDelay {
        delay_ms: u64,
    },
    InvalidMaxDelay {
        max_delay_ms: u64,
        base_delay_ms: u64,
    },
    InvalidFailureThreshold {
        threshold: u32,
    },
    InvalidRecoveryTimeout {
        timeout_ms: u64,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidTimeout { duration_ms } => {
                write!(f, "Timeout duration must be positive, got {duration_ms}")
            }
            ConfigError::InvalidBaseDelay { delay_ms } => {
                write!(f, "Base delay must be positive, got {delay_ms}")
            }
            ConfigError::InvalidMaxDelay {
                max_delay_ms,
                base_delay_ms,
            } => {
                write!(
                    f,
                    "Max delay ({max_delay_ms}) must be >= base delay ({base_delay_ms})"
                )
            }
            ConfigError::InvalidFailureThreshold { threshold } => {
                write!(f, "Failure threshold must be positive, got {threshold}")
            }
            ConfigError::InvalidRecoveryTimeout { timeout_ms } => {
                write!(f, "Recovery timeout must be positive, got {timeout_ms}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Orchestrator errors during phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorError {
    /// Phase execution exceeded timeout duration
    PhaseTimeout {
        phase: String,
        timeout_ms: u64,
        elapsed_ms: u64,
    },
    /// All retry attempts exhausted
    RetriesExhausted {
        phase: String,
        attempts: u32,
        last_error: Box<OrchestratorError>,
    },
    /// Circuit breaker is open, request rejected
    CircuitBreakerOpen {
        phase: String,
        failure_count: u32,
        recovery_timeout_ms: u64,
    },
    /// Global pipeline deadline exceeded
    DeadlineExceeded {
        deadline: DateTime<Utc>,
        elapsed_ms: u64,
    },
    /// Generic phase execution error
    PhaseExecution { phase: String, message: String },
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestratorError::PhaseTimeout {
                phase,
                timeout_ms,
                elapsed_ms,
            } => {
                write!(
                    f,
                    "Phase '{phase}' timed out after {elapsed_ms}ms (limit: {timeout_ms}ms)"
                )
            }
            OrchestratorError::RetriesExhausted {
                phase, attempts, ..
            } => {
                write!(
                    f,
                    "Phase '{phase}' failed after {attempts} attempts (retries exhausted)"
                )
            }
            OrchestratorError::CircuitBreakerOpen {
                phase,
                failure_count,
                ..
            } => {
                write!(
                    f,
                    "Circuit breaker open for phase '{phase}' after {failure_count} failures"
                )
            }
            OrchestratorError::DeadlineExceeded { elapsed_ms, .. } => {
                write!(f, "Pipeline deadline exceeded after {elapsed_ms}ms")
            }
            OrchestratorError::PhaseExecution { phase, message } => {
                write!(f, "Phase '{phase}' execution failed: {message}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

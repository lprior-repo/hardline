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
            Self::InvalidTimeout { duration_ms } => {
                write!(f, "Timeout duration must be positive, got {duration_ms}")
            }
            Self::InvalidBaseDelay { delay_ms } => {
                write!(f, "Base delay must be positive, got {delay_ms}")
            }
            Self::InvalidMaxDelay {
                max_delay_ms,
                base_delay_ms,
            } => {
                write!(
                    f,
                    "Max delay ({max_delay_ms}) must be >= base delay ({base_delay_ms})"
                )
            }
            Self::InvalidFailureThreshold { threshold } => {
                write!(f, "Failure threshold must be positive, got {threshold}")
            }
            Self::InvalidRecoveryTimeout { timeout_ms } => {
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
        last_error: Box<Self>,
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
            Self::PhaseTimeout {
                phase,
                timeout_ms,
                elapsed_ms,
            } => {
                write!(
                    f,
                    "Phase '{phase}' timed out after {elapsed_ms}ms (limit: {timeout_ms}ms)"
                )
            }
            Self::RetriesExhausted {
                phase, attempts, ..
            } => {
                write!(
                    f,
                    "Phase '{phase}' failed after {attempts} attempts (retries exhausted)"
                )
            }
            Self::CircuitBreakerOpen {
                phase,
                failure_count,
                ..
            } => {
                write!(
                    f,
                    "Circuit breaker open for phase '{phase}' after {failure_count} failures"
                )
            }
            Self::DeadlineExceeded { elapsed_ms, .. } => {
                write!(f, "Pipeline deadline exceeded after {elapsed_ms}ms")
            }
            Self::PhaseExecution { phase, message } => {
                write!(f, "Phase '{phase}' execution failed: {message}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ConfigError display tests ---

    #[test]
    fn test_config_error_invalid_timeout_display() {
        let err = ConfigError::InvalidTimeout { duration_ms: 0 };
        let msg = format!("{err}");
        assert!(msg.contains("positive"));
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_config_error_invalid_base_delay_display() {
        let err = ConfigError::InvalidBaseDelay { delay_ms: 0 };
        let msg = format!("{err}");
        assert!(msg.contains("Base delay"));
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_config_error_invalid_max_delay_display() {
        let err = ConfigError::InvalidMaxDelay {
            max_delay_ms: 50,
            base_delay_ms: 100,
        };
        let msg = format!("{err}");
        assert!(msg.contains("50"));
        assert!(msg.contains("100"));
    }

    #[test]
    fn test_config_error_invalid_failure_threshold_display() {
        let err = ConfigError::InvalidFailureThreshold { threshold: 0 };
        let msg = format!("{err}");
        assert!(msg.contains("Failure threshold"));
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_config_error_invalid_recovery_timeout_display() {
        let err = ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 };
        let msg = format!("{err}");
        assert!(msg.contains("Recovery timeout"));
        assert!(msg.contains("0"));
    }

    // --- OrchestratorError display tests ---

    #[test]
    fn test_orchestrator_error_phase_timeout_display() {
        let err = OrchestratorError::PhaseTimeout {
            phase: "review".to_string(),
            timeout_ms: 1000,
            elapsed_ms: 2000,
        };
        let msg = format!("{err}");
        assert!(msg.contains("review"));
        assert!(msg.contains("2000ms"));
        assert!(msg.contains("1000ms"));
    }

    #[test]
    fn test_orchestrator_error_retries_exhausted_display() {
        let err = OrchestratorError::RetriesExhausted {
            phase: "dev".to_string(),
            attempts: 5,
            last_error: Box::new(OrchestratorError::PhaseExecution {
                phase: "dev".to_string(),
                message: "oom".to_string(),
            }),
        };
        let msg = format!("{err}");
        assert!(msg.contains("dev"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn test_orchestrator_error_circuit_breaker_open_display() {
        let err = OrchestratorError::CircuitBreakerOpen {
            phase: "validation".to_string(),
            failure_count: 10,
            recovery_timeout_ms: 30000,
        };
        let msg = format!("{err}");
        assert!(msg.contains("validation"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_orchestrator_error_deadline_exceeded_display() {
        let deadline = Utc::now();
        let err = OrchestratorError::DeadlineExceeded {
            deadline,
            elapsed_ms: 60000,
        };
        let msg = format!("{err}");
        assert!(msg.contains("deadline"));
        assert!(msg.contains("60000ms"));
    }

    #[test]
    fn test_orchestrator_error_phase_execution_display() {
        let err = OrchestratorError::PhaseExecution {
            phase: "setup".to_string(),
            message: "disk full".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("setup"));
        assert!(msg.contains("disk full"));
    }

    // --- ConfigError serde roundtrips ---

    #[test]
    fn test_config_error_serde_roundtrip_all_variants() {
        let errors = [
            ConfigError::InvalidTimeout { duration_ms: 0 },
            ConfigError::InvalidBaseDelay { delay_ms: 0 },
            ConfigError::InvalidMaxDelay {
                max_delay_ms: 50,
                base_delay_ms: 100,
            },
            ConfigError::InvalidFailureThreshold { threshold: 0 },
            ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 },
        ];
        for err in &errors {
            let json = serde_json::to_string(err).expect("serialize");
            let deserialized: ConfigError = serde_json::from_str(&json).expect("deserialize");
            // Compare via display since ConfigError derives PartialEq but not Eq
            assert_eq!(format!("{err}"), format!("{deserialized}"));
        }
    }

    #[test]
    fn test_config_error_uses_snake_case() {
        let err = ConfigError::InvalidTimeout { duration_ms: 0 };
        let json = serde_json::to_string(&err).expect("serialize");
        assert!(json.contains("invalid_timeout"));
    }

    // --- OrchestratorError serde roundtrips ---

    #[test]
    fn test_orchestrator_error_serde_roundtrip() {
        let errors = [
            OrchestratorError::PhaseTimeout {
                phase: "review".to_string(),
                timeout_ms: 1000,
                elapsed_ms: 2000,
            },
            OrchestratorError::DeadlineExceeded {
                deadline: Utc::now(),
                elapsed_ms: 60000,
            },
            OrchestratorError::PhaseExecution {
                phase: "setup".to_string(),
                message: "err".to_string(),
            },
        ];
        for err in &errors {
            let json = serde_json::to_string(err).expect("serialize");
            let deserialized: OrchestratorError = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(format!("{err}"), format!("{deserialized}"));
        }
    }

    // --- ConfigError implements Error ---

    #[test]
    fn test_config_error_implements_error() {
        use std::error::Error;
        let err = ConfigError::InvalidTimeout { duration_ms: 0 };
        assert!(err.source().is_none());
    }

    // --- OrchestratorError implements Error ---

    #[test]
    fn test_orchestrator_error_implements_error() {
        use std::error::Error;
        let err = OrchestratorError::PhaseExecution {
            phase: "p".to_string(),
            message: "m".to_string(),
        };
        assert!(err.source().is_none());
    }
}

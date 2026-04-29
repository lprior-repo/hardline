//! Timeout error types

use std::time::Instant;

/// Error variants for timeout scenarios
#[derive(Debug, Clone)]
pub enum TimeoutError {
    InvalidTimeout(String),
    TimeoutExceeded {
        phase_id: String,
        duration_ms: u64,
        timeout_ms: u64,
    },
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTimeout(msg) => {
                write!(f, "Invalid timeout: {msg}")
            }
            Self::TimeoutExceeded {
                phase_id,
                duration_ms,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Phase '{phase_id}' exceeded timeout of {timeout_ms}ms (took {duration_ms}ms)"
                )
            }
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Error type combining all orchestrator policy errors
#[derive(Debug, Clone)]
pub enum PolicyError {
    InvalidTimeout(String),
    InvalidRetryPolicy(String),
    TimeoutExceeded {
        phase_id: String,
        duration_ms: u64,
        timeout_ms: u64,
    },
    MaxRetriesExceeded {
        phase_id: String,
        attempts: u32,
        last_error: Box<Self>,
    },
    CircuitBreakerOpen {
        phase_id: String,
        open_until: Instant,
    },
    NonRetryableError {
        phase_id: String,
        cause: String,
    },
    PreconditionViolation(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTimeout(msg) => {
                write!(f, "Invalid timeout: {msg}")
            }
            Self::InvalidRetryPolicy(msg) => {
                write!(f, "Invalid retry policy: {msg}")
            }
            Self::TimeoutExceeded {
                phase_id,
                duration_ms,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Phase '{phase_id}' exceeded timeout of {timeout_ms}ms (took {duration_ms}ms)"
                )
            }
            Self::MaxRetriesExceeded {
                phase_id, attempts, ..
            } => {
                write!(
                    f,
                    "Phase '{phase_id}' failed after {attempts} attempts (retries exhausted)"
                )
            }
            Self::CircuitBreakerOpen { phase_id, .. } => {
                write!(f, "Circuit breaker open for phase '{phase_id}'")
            }
            Self::NonRetryableError { phase_id, cause } => {
                write!(f, "Non-retryable error in phase '{phase_id}': {cause}")
            }
            Self::PreconditionViolation(msg) => {
                write!(f, "Precondition violation: {msg}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_error_display() {
        let err = TimeoutError::InvalidTimeout("timeout must be > 0".into());
        assert!(err.to_string().contains("timeout must be > 0"));

        let err = TimeoutError::TimeoutExceeded {
            phase_id: "test".into(),
            duration_ms: 200,
            timeout_ms: 100,
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("200ms"));
        assert!(err.to_string().contains("100ms"));
    }

    #[test]
    fn test_policy_error_display() {
        let err = PolicyError::InvalidTimeout("timeout must be > 0".into());
        assert!(err.to_string().contains("Invalid timeout"));

        let err = PolicyError::MaxRetriesExceeded {
            phase_id: "test".into(),
            attempts: 3,
            last_error: Box::new(PolicyError::InvalidTimeout("boom".into())),
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("3 attempts"));
    }

    #[test]
    fn test_timeout_error_implements_error() {
        use std::error::Error;
        let err = TimeoutError::InvalidTimeout("test".into());
        assert!(err.source().is_none());

        let err = TimeoutError::TimeoutExceeded {
            phase_id: "p".into(),
            duration_ms: 100,
            timeout_ms: 50,
        };
        assert!(err.source().is_none());
    }

    #[test]
    fn test_policy_error_implements_error() {
        use std::error::Error;
        let errors = [
            PolicyError::InvalidTimeout("t".into()),
            PolicyError::InvalidRetryPolicy("r".into()),
            PolicyError::TimeoutExceeded {
                phase_id: "p".into(),
                duration_ms: 100,
                timeout_ms: 50,
            },
            PolicyError::MaxRetriesExceeded {
                phase_id: "p".into(),
                attempts: 3,
                last_error: Box::new(PolicyError::InvalidTimeout("x".into())),
            },
            PolicyError::CircuitBreakerOpen {
                phase_id: "p".into(),
                open_until: std::time::Instant::now(),
            },
            PolicyError::NonRetryableError {
                phase_id: "p".into(),
                cause: "c".into(),
            },
            PolicyError::PreconditionViolation("v".into()),
        ];
        for err in &errors {
            let _ = format!("{err}");
            assert!(err.source().is_none());
        }
    }

    #[test]
    fn test_policy_error_circuit_breaker_display() {
        let err = PolicyError::CircuitBreakerOpen {
            phase_id: "validation".into(),
            open_until: std::time::Instant::now(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Circuit breaker open"));
        assert!(msg.contains("validation"));
    }

    #[test]
    fn test_policy_error_non_retryable_display() {
        let err = PolicyError::NonRetryableError {
            phase_id: "setup".into(),
            cause: "disk full".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Non-retryable"));
        assert!(msg.contains("setup"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn test_policy_error_precondition_violation_display() {
        let err = PolicyError::PreconditionViolation("state must be Pending".into());
        let msg = format!("{err}");
        assert!(msg.contains("Precondition violation"));
        assert!(msg.contains("state must be Pending"));
    }

    #[test]
    fn test_policy_error_invalid_retry_policy_display() {
        let err = PolicyError::InvalidRetryPolicy("max_retries must be > 0".into());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid retry policy"));
        assert!(msg.contains("max_retries must be > 0"));
    }
}

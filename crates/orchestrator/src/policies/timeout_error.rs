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
            TimeoutError::InvalidTimeout(msg) => {
                write!(f, "Invalid timeout: {msg}")
            }
            TimeoutError::TimeoutExceeded {
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
        last_error: Box<PolicyError>,
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
            PolicyError::InvalidTimeout(msg) => {
                write!(f, "Invalid timeout: {msg}")
            }
            PolicyError::InvalidRetryPolicy(msg) => {
                write!(f, "Invalid retry policy: {msg}")
            }
            PolicyError::TimeoutExceeded {
                phase_id,
                duration_ms,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Phase '{phase_id}' exceeded timeout of {timeout_ms}ms (took {duration_ms}ms)"
                )
            }
            PolicyError::MaxRetriesExceeded {
                phase_id, attempts, ..
            } => {
                write!(
                    f,
                    "Phase '{phase_id}' failed after {attempts} attempts (retries exhausted)"
                )
            }
            PolicyError::CircuitBreakerOpen { phase_id, .. } => {
                write!(f, "Circuit breaker open for phase '{phase_id}'")
            }
            PolicyError::NonRetryableError { phase_id, cause } => {
                write!(f, "Non-retryable error in phase '{phase_id}': {cause}")
            }
            PolicyError::PreconditionViolation(msg) => {
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
}

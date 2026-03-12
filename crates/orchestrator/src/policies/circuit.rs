//! Policy: Circuit breaker to prevent cascading failures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerState {
    /// Normal operation, requests allowed
    Closed,
    /// Too many failures, requests rejected
    Open,
    /// Testing if recovery is possible
    HalfOpen,
}

/// Circuit breaker to prevent cascading failures
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Number of consecutive failures before opening
    failure_threshold: u32,
    /// Time in milliseconds before attempting recovery
    recovery_timeout_ms: u64,
    /// Current state
    state: CircuitBreakerState,
    /// Number of consecutive failures
    failure_count: u32,
    /// Timestamp of last failure
    last_failure_at: Option<DateTime<Utc>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(
        failure_threshold: u32,
        recovery_timeout_ms: u64,
    ) -> Result<Self, super::ConfigError> {
        if failure_threshold == 0 {
            return Err(super::ConfigError::InvalidFailureThreshold {
                threshold: failure_threshold,
            });
        }
        if recovery_timeout_ms == 0 {
            return Err(super::ConfigError::InvalidRecoveryTimeout {
                timeout_ms: recovery_timeout_ms,
            });
        }
        Ok(Self {
            failure_threshold,
            recovery_timeout_ms,
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_at: None,
        })
    }

    /// Get the current state
    #[must_use]
    pub fn state(&self) -> CircuitBreakerState {
        self.state
    }

    /// Get the failure count
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Record a successful execution
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Closed;
        }
    }

    /// Record a failed execution
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());

        // Transition to Open if in HalfOpen (failed recovery test) or threshold reached
        if self.state == CircuitBreakerState::HalfOpen
            || self.failure_count >= self.failure_threshold
        {
            self.state = CircuitBreakerState::Open;
        }
    }

    /// Check if a request can be executed
    /// Returns false if circuit breaker is open
    pub fn can_execute(&self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if we should transition to HalfOpen
                if let Some(last_failure) = self.last_failure_at {
                    let elapsed = Utc::now().signed_duration_since(last_failure);
                    elapsed.num_milliseconds() >= self.recovery_timeout_ms as i64
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Try to transition to HalfOpen state (for external callers)
    pub fn try_transition_to_half_open(&mut self) -> bool {
        if self.state == CircuitBreakerState::Open {
            if let Some(last_failure) = self.last_failure_at {
                let elapsed = Utc::now().signed_duration_since(last_failure);
                if elapsed.num_milliseconds() >= self.recovery_timeout_ms as i64 {
                    self.state = CircuitBreakerState::HalfOpen;
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_new_valid() {
        let cb = CircuitBreaker::new(3, 5000).expect("should create circuit breaker");
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_new_zero_threshold() {
        let result = CircuitBreaker::new(0, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new(3, 5000).expect("should create circuit breaker");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_record_success_clears_failures() {
        let mut cb = CircuitBreaker::new(3, 5000).expect("should create circuit breaker");
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_open_rejects() {
        let mut cb = CircuitBreaker::new(2, 5000).expect("should create circuit breaker");
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.can_execute());
    }
}

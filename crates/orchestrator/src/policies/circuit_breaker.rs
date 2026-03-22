//! Circuit breaker with proper state transitions

use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    HalfOpen,
    Open,
}

/// Circuit breaker configuration and state
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_threshold: NonZeroU32,
    success_threshold: NonZeroU32,
    open_duration: NonZeroU64,
    failure_count: u32,
    success_count: u32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        open_duration_ms: u64,
    ) -> Result<Self, CircuitBreakerError> {
        let failure_threshold = NonZeroU32::new(failure_threshold)
            .ok_or(CircuitBreakerError::InvalidFailureThreshold)?;
        let success_threshold = NonZeroU32::new(success_threshold)
            .ok_or(CircuitBreakerError::InvalidSuccessThreshold)?;
        let open_duration =
            NonZeroU64::new(open_duration_ms).ok_or(CircuitBreakerError::InvalidOpenDuration)?;
        Ok(Self {
            state: CircuitState::Closed,
            failure_threshold,
            success_threshold,
            open_duration,
            failure_count: 0,
            success_count: 0,
        })
    }

    /// Get the current state
    #[must_use]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Get the failure count
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Get the success count
    #[must_use]
    pub fn success_count(&self) -> u32 {
        self.success_count
    }

    /// Get the failure threshold
    #[must_use]
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold.get()
    }

    /// Get the success threshold
    #[must_use]
    pub fn success_threshold(&self) -> u32 {
        self.success_threshold.get()
    }

    /// Get the open duration
    #[must_use]
    pub fn open_duration(&self) -> Duration {
        Duration::from_millis(self.open_duration.get())
    }

    /// Records a successful phase execution
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => self.failure_count = 0,
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold.get() {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Records a failed phase execution
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold.get() {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Checks if execution is allowed under the current circuit state
    #[must_use]
    pub fn is_execution_allowed(&self) -> bool {
        matches!(self.state, CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// Attempts to transition from Open to HalfOpen based on elapsed time
    pub fn check_and_transition(&mut self, elapsed_ms: u64) -> bool {
        if self.state == CircuitState::Open && elapsed_ms >= self.open_duration.get() {
            self.state = CircuitState::HalfOpen;
            self.success_count = 0;
            return true;
        }
        false
    }

    /// Reset to closed state (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    InvalidFailureThreshold,
    InvalidSuccessThreshold,
    InvalidOpenDuration,
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::InvalidFailureThreshold => {
                write!(f, "failure_threshold must be positive")
            }
            CircuitBreakerError::InvalidSuccessThreshold => {
                write!(f, "success_threshold must be positive")
            }
            CircuitBreakerError::InvalidOpenDuration => {
                write!(f, "open_duration must be greater than 0")
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_in_closed_state() {
        let cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_execution_allowed());
    }

    #[test]
    fn test_circuit_breaker_transitions_to_open_after_failure_threshold() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_execution_allowed());
    }

    #[test]
    fn test_circuit_breaker_transitions_to_halfopen_after_open_duration() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        let transitioned = cb.check_and_transition(30001);
        assert!(transitioned);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_halfopen_allows_one_execution() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        cb.check_and_transition(30001);
        assert!(cb.is_execution_allowed());
    }

    #[test]
    fn test_circuit_breaker_closes_after_success_threshold_in_halfopen() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        cb.check_and_transition(30001);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_returns_to_open_from_halfopen_after_probe_failure() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        cb.check_and_transition(30001);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_execution_allowed());
    }

    #[test]
    fn test_multiple_successful_executions_decrease_failure_count() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_invalid_failure_threshold_zero() {
        let result = CircuitBreaker::new(0, 2, 30000);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CircuitBreakerError::InvalidFailureThreshold
        );
    }

    #[test]
    fn test_invalid_success_threshold_zero() {
        let result = CircuitBreaker::new(3, 0, 30000);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CircuitBreakerError::InvalidSuccessThreshold
        );
    }

    #[test]
    fn test_invalid_open_duration_zero() {
        let result = CircuitBreaker::new(3, 2, 0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CircuitBreakerError::InvalidOpenDuration
        );
    }

    #[test]
    fn test_invariant_i1_circuit_state_reflects_failure_rate() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}

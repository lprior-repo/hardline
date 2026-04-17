//! Policy configurations for orchestrator: timeouts, retries, circuit breakers

pub mod circuit;
pub mod circuit_breaker;
pub mod deadline;
pub mod errors;
pub mod retry_policy;
pub mod timeout;
pub mod timeout_error;
pub mod timeout_policy;

pub use circuit::{CircuitBreaker, CircuitBreakerState};
pub use circuit_breaker::{
    CircuitBreaker as NewCircuitBreaker, CircuitBreakerError as NewCircuitBreakerError,
    CircuitState,
};
pub use deadline::Deadline;
pub use errors::{ConfigError, OrchestratorError};
pub use retry_policy::{RetryPolicy, RetryPolicyError};
pub use timeout::PhaseTimeout;
pub use timeout_error::{PolicyError, TimeoutError};
pub use timeout_policy::{TimeoutPolicy, TimeoutPolicyError};

/// Combined policy configuration for pipeline execution
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// Timeout for individual phases
    pub timeout: PhaseTimeout,
    /// Retry policy
    pub retry: RetryPolicy,
    /// Circuit breaker
    pub circuit_breaker: CircuitBreaker,
    /// Optional global deadline
    pub deadline: Option<Deadline>,
}

impl PolicyConfig {
    /// Create a new policy configuration
    pub fn new(
        timeout_ms: u64,
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        failure_threshold: u32,
        recovery_timeout_ms: u64,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            timeout: PhaseTimeout::new(timeout_ms)?,
            retry: RetryPolicy::new(max_retries, base_delay_ms, 2.0, Some(max_delay_ms), vec![])
                .map_err(|_| ConfigError::InvalidBaseDelay { delay_ms: base_delay_ms })?,
            circuit_breaker: CircuitBreaker::new(failure_threshold, recovery_timeout_ms)?,
            deadline: None,
        })
    }

    /// Set a global deadline
    #[must_use]
    pub fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_config_new_valid() {
        let config = PolicyConfig::new(
            1000, // timeout
            3,    // max_retries
            100,  // base_delay
            1000, // max_delay
            3,    // failure_threshold
            5000, // recovery_timeout
        )
        .expect("should create config");
        assert_eq!(config.timeout.duration_ms(), 1000);
        assert_eq!(config.retry.max_retries(), 3);
        assert_eq!(config.circuit_breaker.failure_count(), 0);
    }

    #[test]
    fn test_policy_config_with_deadline() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000)
            .expect("should create config")
            .with_deadline(Deadline::from_now(60000));

        assert!(config.deadline.is_some());
        assert!(!config.deadline.as_ref().unwrap().is_exceeded());
    }

    #[test]
    fn test_policy_config_default_no_deadline() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("should create config");
        assert!(config.deadline.is_none());
    }

    #[test]
    fn test_policy_config_invalid_timeout_zero() {
        let result = PolicyConfig::new(0, 3, 100, 1000, 3, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_base_delay_zero() {
        let result = PolicyConfig::new(1000, 3, 0, 1000, 3, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_max_delay_less_than_base() {
        let result = PolicyConfig::new(1000, 3, 500, 100, 3, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_failure_threshold_zero() {
        let result = PolicyConfig::new(1000, 3, 100, 1000, 0, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_recovery_timeout_zero() {
        let result = PolicyConfig::new(1000, 3, 100, 1000, 3, 0);
        assert!(result.is_err());
    }
}

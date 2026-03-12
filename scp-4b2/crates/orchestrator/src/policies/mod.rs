//! Policy configurations for orchestrator: timeouts, retries, circuit breakers

pub mod circuit;
pub mod deadline;
pub mod errors;
pub mod retry;
pub mod timeout;

pub use circuit::{CircuitBreaker, CircuitBreakerState};
pub use deadline::Deadline;
pub use errors::{ConfigError, OrchestratorError};
pub use retry::RetryPolicy;
pub use timeout::PhaseTimeout;

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
            retry: RetryPolicy::new(max_retries, base_delay_ms, max_delay_ms)?,
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
}

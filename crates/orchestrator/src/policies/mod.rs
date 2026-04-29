//! Policy configurations for orchestrator: timeouts, retries, circuit breakers

pub mod circuit_breaker;
pub mod deadline;
pub mod errors;
pub mod retry_policy;
pub mod timeout;
pub mod timeout_error;
pub mod timeout_policy;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerError, CircuitBreakerState};
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

/// Options for creating a new policy configuration.
pub struct PolicyOpts {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub failure_threshold: u32,
    pub recovery_timeout_ms: u64,
}

impl PolicyConfig {
    /// Create a new policy configuration
    pub fn new(opts: PolicyOpts) -> Result<Self, ConfigError> {
        Ok(Self {
            timeout: PhaseTimeout::new(opts.timeout_ms)?,
            retry: RetryPolicy::new(opts.max_retries, opts.base_delay_ms, 2.0, Some(opts.max_delay_ms), vec![])
                .map_err(|_| ConfigError::InvalidBaseDelay {
                    delay_ms: opts.base_delay_ms,
                })?,
            circuit_breaker: CircuitBreaker::with_recovery_timeout(
                opts.failure_threshold,
                opts.recovery_timeout_ms,
            )?,
            deadline: None,
        })
    }

    /// Set a global deadline
    #[must_use]
    pub const fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_config_new_valid() {
        let config = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 5000 })
        .expect("should create config");
        assert_eq!(config.timeout.duration_ms(), 1000);
        assert_eq!(config.retry.max_retries(), 3);
        assert_eq!(config.circuit_breaker.failure_count(), 0);
    }

    #[test]
    fn test_policy_config_with_deadline() {
        let config = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 5000 })
            .expect("should create config")
            .with_deadline(Deadline::from_now(60000));

        assert!(config.deadline.is_some());
        assert!(!config.deadline.as_ref().unwrap().is_exceeded());
    }

    #[test]
    fn test_policy_config_default_no_deadline() {
        let config = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 5000 }).expect("should create config");
        assert!(config.deadline.is_none());
    }

    #[test]
    fn test_policy_config_invalid_timeout_zero() {
        let result = PolicyConfig::new(PolicyOpts { timeout_ms: 0, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 5000 });
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_base_delay_zero() {
        let result = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 0, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 5000 });
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_max_delay_less_than_base() {
        let result = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 500, max_delay_ms: 100, failure_threshold: 3, recovery_timeout_ms: 5000 });
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_failure_threshold_zero() {
        let result = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 0, recovery_timeout_ms: 5000 });
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_invalid_recovery_timeout_zero() {
        let result = PolicyConfig::new(PolicyOpts { timeout_ms: 1000, max_retries: 3, base_delay_ms: 100, max_delay_ms: 1000, failure_threshold: 3, recovery_timeout_ms: 0 });
        assert!(result.is_err());
    }
}

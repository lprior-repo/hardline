//! Policy configurations for orchestrator: timeouts, retries, circuit breakers

pub mod circuit;
pub mod circuit_breaker;
pub mod deadline;
pub mod errors;
pub mod retry;
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
pub use retry::RetryPolicy;
pub use retry_policy::{RetryPolicy as NewRetryPolicy, RetryPolicyError};
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

    // ── Loading: field access verification ─────────────────────────────────

    #[test]
    fn test_policy_config_timeout_field_accessible() {
        let config = PolicyConfig::new(5000, 3, 100, 1000, 3, 5000).expect("ok");
        assert_eq!(config.timeout.duration_ms(), 5000);
    }

    #[test]
    fn test_policy_config_retry_fields_accessible() {
        let config = PolicyConfig::new(1000, 7, 200, 5000, 3, 5000).expect("ok");
        assert_eq!(config.retry.max_retries(), 7);
        assert_eq!(config.retry.total_attempts(), 8);
        assert_eq!(config.retry.calculate_delay(0), 200);
    }

    #[test]
    fn test_policy_config_circuit_breaker_fields_accessible() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 5, 10000).expect("ok");
        assert_eq!(config.circuit_breaker.state(), CircuitBreakerState::Closed);
        assert_eq!(config.circuit_breaker.failure_count(), 0);
    }

    #[test]
    fn test_policy_config_minimum_valid_values() {
        // All params at their minimum valid values (1ms, 1 retry, threshold=1)
        let config = PolicyConfig::new(1, 0, 1, 1, 1, 1).expect("ok");
        assert_eq!(config.timeout.duration_ms(), 1);
        assert_eq!(config.retry.max_retries(), 0);
        assert_eq!(config.circuit_breaker.failure_count(), 0);
    }

    #[test]
    fn test_policy_config_large_valid_values() {
        let config =
            PolicyConfig::new(u64::MAX, u32::MAX, 1, u64::MAX, u32::MAX, u64::MAX).expect("ok");
        assert_eq!(config.timeout.duration_ms(), u64::MAX);
        assert_eq!(config.retry.max_retries(), u32::MAX);
    }

    #[test]
    fn test_policy_config_equal_base_and_max_delay() {
        let config = PolicyConfig::new(1000, 3, 500, 500, 3, 5000).expect("ok");
        assert_eq!(config.retry.calculate_delay(0), 500);
        assert_eq!(config.retry.calculate_delay(5), 500);
    }

    // ── Validation: error variant matching ────────────────────────────────

    #[test]
    fn test_policy_config_invalid_timeout_returns_correct_variant() {
        let err = PolicyConfig::new(0, 3, 100, 1000, 3, 5000).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidTimeout { duration_ms: 0 }),
            "expected InvalidTimeout(0), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_invalid_base_delay_returns_correct_variant() {
        let err = PolicyConfig::new(1000, 3, 0, 1000, 3, 5000).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidBaseDelay { delay_ms: 0 }),
            "expected InvalidBaseDelay(0), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_invalid_max_delay_returns_correct_variant() {
        let err = PolicyConfig::new(1000, 3, 500, 100, 3, 5000).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidMaxDelay {
                    max_delay_ms: 100,
                    base_delay_ms: 500
                }
            ),
            "expected InvalidMaxDelay(100, 500), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_invalid_failure_threshold_returns_correct_variant() {
        let err = PolicyConfig::new(1000, 3, 100, 1000, 0, 5000).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidFailureThreshold { threshold: 0 }
            ),
            "expected InvalidFailureThreshold(0), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_invalid_recovery_timeout_returns_correct_variant() {
        let err = PolicyConfig::new(1000, 3, 100, 1000, 3, 0).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 }
            ),
            "expected InvalidRecoveryTimeout(0), got {err:?}"
        );
    }

    // ── Validation: first-error-wins ordering ─────────────────────────────

    #[test]
    fn test_policy_config_timeout_validated_before_retry() {
        // Both timeout and base_delay invalid — timeout should error first
        let err = PolicyConfig::new(0, 3, 0, 1000, 3, 5000).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidTimeout { .. }),
            "expected InvalidTimeout (checked first), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_retry_validated_before_circuit_breaker() {
        // Valid timeout, invalid retry (base_delay=0), invalid circuit breaker (threshold=0)
        let err = PolicyConfig::new(1000, 3, 0, 1000, 0, 5000).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidBaseDelay { .. }),
            "expected InvalidBaseDelay (retry checked before CB), got {err:?}"
        );
    }

    #[test]
    fn test_policy_config_base_delay_checked_before_max_delay() {
        // base_delay=0 is caught before max_delay < base_delay
        let err = PolicyConfig::new(1000, 3, 0, 0, 3, 5000).unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidBaseDelay { .. }),
            "expected InvalidBaseDelay, got {err:?}"
        );
    }

    // ── Validation: error messages ────────────────────────────────────────

    #[test]
    fn test_policy_config_error_messages_contain_context() {
        let cases: Vec<(Result<PolicyConfig, ConfigError>, &str)> = vec![
            (PolicyConfig::new(0, 3, 100, 1000, 3, 5000), "positive"),
            (PolicyConfig::new(1000, 3, 0, 1000, 3, 5000), "Base delay"),
            (PolicyConfig::new(1000, 3, 500, 100, 3, 5000), "Max delay"),
            (PolicyConfig::new(1000, 3, 100, 1000, 0, 5000), "Failure threshold"),
            (PolicyConfig::new(1000, 3, 100, 1000, 3, 0), "Recovery timeout"),
        ];
        for (result, expected_fragment) in cases {
            let err = result.unwrap_err();
            let msg = format!("{err}");
            assert!(
                msg.contains(expected_fragment),
                "Error message '{msg}' should contain '{expected_fragment}'"
            );
        }
    }

    // ── Defaults: initial state verification ──────────────────────────────

    #[test]
    fn test_policy_config_circuit_breaker_starts_closed() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        assert_eq!(config.circuit_breaker.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_policy_config_circuit_breaker_starts_with_zero_failures() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        assert_eq!(config.circuit_breaker.failure_count(), 0);
    }

    #[test]
    fn test_policy_config_retry_total_attempts_includes_initial() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        // 3 retries means 4 total attempts (initial + 3)
        assert_eq!(config.retry.total_attempts(), 4);
    }

    #[test]
    fn test_policy_config_zero_retries_means_single_attempt() {
        let config = PolicyConfig::new(1000, 0, 100, 1000, 3, 5000).expect("ok");
        assert_eq!(config.retry.total_attempts(), 1);
        assert_eq!(config.retry.max_retries(), 0);
    }

    // ── Builder: with_deadline ────────────────────────────────────────────

    #[test]
    fn test_policy_config_with_deadline_is_consuming() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        let with_dl = config.with_deadline(Deadline::from_now(60000));
        // Original consumed by with_deadline — can't use config after this
        assert!(with_dl.deadline.is_some());
    }

    #[test]
    fn test_policy_config_with_deadline_preserves_other_fields() {
        let config = PolicyConfig::new(2000, 5, 200, 2000, 4, 10000)
            .expect("ok")
            .with_deadline(Deadline::from_now(30000));

        assert_eq!(config.timeout.duration_ms(), 2000);
        assert_eq!(config.retry.max_retries(), 5);
        assert_eq!(config.circuit_breaker.failure_count(), 0);
        assert!(config.deadline.is_some());
    }

    #[test]
    fn test_policy_config_without_deadline_has_none() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        assert!(config.deadline.is_none());
    }

    // ── Clone semantics ───────────────────────────────────────────────────

    #[test]
    fn test_policy_config_clone_preserves_fields() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        let cloned = config.clone();
        assert_eq!(config.timeout.duration_ms(), cloned.timeout.duration_ms());
        assert_eq!(config.retry.max_retries(), cloned.retry.max_retries());
    }

    #[test]
    fn test_policy_config_clone_with_deadline() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000)
            .expect("ok")
            .with_deadline(Deadline::from_now(60000));
        let cloned = config.clone();
        assert!(cloned.deadline.is_some());
        assert_eq!(
            config.deadline.as_ref().unwrap().deadline_at(),
            cloned.deadline.as_ref().unwrap().deadline_at()
        );
    }

    #[test]
    fn test_policy_config_clone_without_deadline() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        let cloned = config.clone();
        assert!(config.deadline.is_none());
        assert!(cloned.deadline.is_none());
    }

    // ── Debug format ──────────────────────────────────────────────────────

    #[test]
    fn test_policy_config_debug_format() {
        let config = PolicyConfig::new(1000, 3, 100, 1000, 3, 5000).expect("ok");
        let debug = format!("{config:?}");
        assert!(debug.contains("PolicyConfig"));
        assert!(debug.contains("timeout"));
        assert!(debug.contains("retry"));
        assert!(debug.contains("circuit_breaker"));
        assert!(debug.contains("deadline"));
    }

    // ── ConfigError: Display and Error trait ──────────────────────────────

    #[test]
    fn test_config_error_display_all_variants() {
        let errors = [
            ConfigError::InvalidTimeout { duration_ms: 0 },
            ConfigError::InvalidBaseDelay { delay_ms: 0 },
            ConfigError::InvalidMaxDelay {
                max_delay_ms: 10,
                base_delay_ms: 100,
            },
            ConfigError::InvalidFailureThreshold { threshold: 0 },
            ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 },
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Display should not be empty for {err:?}");
        }
    }

    #[test]
    fn test_config_error_implements_std_error() {
        use std::error::Error;
        let err = ConfigError::InvalidTimeout { duration_ms: 0 };
        assert!(err.source().is_none());
    }
}

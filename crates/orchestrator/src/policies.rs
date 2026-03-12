//! Policy configurations for orchestrator: timeouts, retries, circuit breakers

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

/// Timeout configuration for a phase
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseTimeout {
    /// Maximum duration in milliseconds
    duration_ms: u64,
}

impl PhaseTimeout {
    /// Create a new timeout with the given duration in milliseconds
    pub fn new(duration_ms: u64) -> Result<Self, ConfigError> {
        if duration_ms == 0 {
            return Err(ConfigError::InvalidTimeout { duration_ms });
        }
        Ok(Self { duration_ms })
    }

    /// Get the timeout duration in milliseconds
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// Check if the timeout has elapsed since the given start time
    pub fn is_expired(&self, started_at: DateTime<Utc>) -> bool {
        let elapsed = Utc::now().signed_duration_since(started_at);
        elapsed.num_milliseconds() >= self.duration_ms as i64
    }

    /// Get elapsed time in milliseconds since the given start time
    pub fn elapsed_ms(&self, started_at: DateTime<Utc>) -> u64 {
        let elapsed = Utc::now().signed_duration_since(started_at);
        elapsed.num_milliseconds().max(0) as u64
    }
}

/// Retry policy configuration with exponential backoff
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    max_retries: u32,
    /// Base delay in milliseconds (before exponential multiplier)
    base_delay_ms: u64,
    /// Maximum delay cap in milliseconds
    max_delay_ms: u64,
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<Self, ConfigError> {
        if base_delay_ms == 0 {
            return Err(ConfigError::InvalidBaseDelay {
                delay_ms: base_delay_ms,
            });
        }
        if max_delay_ms < base_delay_ms {
            return Err(ConfigError::InvalidMaxDelay {
                max_delay_ms,
                base_delay_ms,
            });
        }
        Ok(Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
        })
    }

    /// Get the maximum number of retries
    #[must_use]
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Calculate the delay for a given retry attempt using exponential backoff
    /// Formula: min(base_delay_ms * 2^attempt, max_delay_ms)
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let exponential = self
            .base_delay_ms
            .saturating_mul(2_u64.saturating_pow(attempt));
        exponential.min(self.max_delay_ms)
    }

    /// Get the total number of attempts (initial + retries)
    #[must_use]
    pub fn total_attempts(&self) -> u32 {
        self.max_retries + 1
    }
}

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
    pub fn new(failure_threshold: u32, recovery_timeout_ms: u64) -> Result<Self, ConfigError> {
        if failure_threshold == 0 {
            return Err(ConfigError::InvalidFailureThreshold {
                threshold: failure_threshold,
            });
        }
        if recovery_timeout_ms == 0 {
            return Err(ConfigError::InvalidRecoveryTimeout {
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

        if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Open;
        } else if self.failure_count >= self.failure_threshold {
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

/// Deadline configuration for global pipeline timeout
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Deadline {
    /// The absolute deadline timestamp
    deadline_at: DateTime<Utc>,
}

impl Deadline {
    /// Create a deadline from now plus the given duration
    pub fn from_now(duration_ms: u64) -> Self {
        Self {
            deadline_at: Utc::now() + chrono::Duration::milliseconds(duration_ms as i64),
        }
    }

    /// Create a deadline at a specific timestamp
    #[must_use]
    pub fn at(deadline_at: DateTime<Utc>) -> Self {
        Self { deadline_at }
    }

    /// Get the deadline timestamp
    #[must_use]
    pub fn deadline_at(&self) -> DateTime<Utc> {
        self.deadline_at
    }

    /// Check if the deadline has been exceeded
    pub fn is_exceeded(&self) -> bool {
        Utc::now() > self.deadline_at
    }

    /// Get remaining time in milliseconds (0 if exceeded)
    pub fn remaining_ms(&self) -> i64 {
        let remaining = self.deadline_at.signed_duration_since(Utc::now());
        remaining.num_milliseconds().max(0)
    }
}

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
    fn test_phase_timeout_new_valid() {
        let timeout = PhaseTimeout::new(1000).expect("should create timeout");
        assert_eq!(timeout.duration_ms(), 1000);
    }

    #[test]
    fn test_phase_timeout_new_zero() {
        let result = PhaseTimeout::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_timeout_is_expired() {
        let timeout = PhaseTimeout::new(50).expect("should create timeout");
        let started = Utc::now() - chrono::Duration::milliseconds(100);
        assert!(timeout.is_expired(started));
    }

    #[test]
    fn test_retry_policy_new_valid() {
        let policy = RetryPolicy::new(3, 100, 1000).expect("should create policy");
        assert_eq!(policy.max_retries(), 3);
    }

    #[test]
    fn test_retry_policy_new_zero_base_delay() {
        let result = RetryPolicy::new(3, 0, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_policy_new_max_less_than_base() {
        let result = RetryPolicy::new(3, 100, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_policy_calculate_delay() {
        let policy = RetryPolicy::new(3, 100, 1000).expect("should create policy");
        assert_eq!(policy.calculate_delay(0), 100);
        assert_eq!(policy.calculate_delay(1), 200);
        assert_eq!(policy.calculate_delay(2), 400);
        assert_eq!(policy.calculate_delay(3), 800);
    }

    #[test]
    fn test_retry_policy_calculate_delay_capped() {
        let policy = RetryPolicy::new(10, 100, 500).expect("should create policy");
        assert_eq!(policy.calculate_delay(10), 500); // Capped at max
    }

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

    #[test]
    fn test_deadline_is_exceeded() {
        let deadline = Deadline::from_now(1000);
        // Should not be exceeded immediately
        assert!(!deadline.is_exceeded());
    }

    #[test]
    fn test_deadline_remaining_ms() {
        let deadline = Deadline::from_now(5000);
        let remaining = deadline.remaining_ms();
        // Should be approximately 5000 (within small margin)
        assert!(remaining >= 4900 && remaining <= 5100);
    }

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

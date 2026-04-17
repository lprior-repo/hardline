//! Policy: Circuit breaker to prevent cascading failures
//!
//! Consolidated implementation combining:
//! - NonZeroU* types for compile-time validated configuration
//! - chrono-based time transitions for real-world usage
//! - serde support for serialization
//! - success_threshold for configurable HalfOpen → Closed recovery

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;

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

/// Circuit breaker configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Current state
    state: CircuitBreakerState,
    /// Number of consecutive failures before opening
    failure_threshold: NonZeroU32,
    /// Number of consecutive successes in HalfOpen before closing
    success_threshold: NonZeroU32,
    /// Time in milliseconds before attempting recovery (Open → HalfOpen)
    open_duration_ms: NonZeroU64,
    /// Number of consecutive failures
    failure_count: u32,
    /// Number of consecutive successes (tracked in HalfOpen)
    success_count: u32,
    /// Timestamp of last failure (for chrono-based transitions)
    last_failure_at: Option<DateTime<Utc>>,
}

/// Circuit breaker specific errors
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

impl CircuitBreaker {
    /// Create a new circuit breaker with full configuration.
    ///
    /// # Arguments
    /// * `failure_threshold` - Consecutive failures before opening (must be > 0)
    /// * `success_threshold` - Consecutive successes in HalfOpen before closing (must be > 0)
    /// * `open_duration_ms` - Milliseconds in Open before transitioning to HalfOpen (must be > 0)
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        open_duration_ms: u64,
    ) -> Result<Self, CircuitBreakerError> {
        let failure_threshold = NonZeroU32::new(failure_threshold)
            .ok_or(CircuitBreakerError::InvalidFailureThreshold)?;
        let success_threshold = NonZeroU32::new(success_threshold)
            .ok_or(CircuitBreakerError::InvalidSuccessThreshold)?;
        let open_duration_ms = NonZeroU64::new(open_duration_ms)
            .ok_or(CircuitBreakerError::InvalidOpenDuration)?;
        Ok(Self {
            state: CircuitBreakerState::Closed,
            failure_threshold,
            success_threshold,
            open_duration_ms,
            failure_count: 0,
            success_count: 0,
            last_failure_at: None,
        })
    }

    /// Create a circuit breaker with sensible defaults (success_threshold=1).
    ///
    /// Compatibility constructor matching the original 2-arg API.
    pub fn with_recovery_timeout(
        failure_threshold: u32,
        recovery_timeout_ms: u64,
    ) -> Result<Self, super::ConfigError> {
        Self::new(failure_threshold, 1, recovery_timeout_ms).map_err(|e| match e {
            CircuitBreakerError::InvalidFailureThreshold => {
                super::ConfigError::InvalidFailureThreshold {
                    threshold: failure_threshold,
                }
            }
            CircuitBreakerError::InvalidSuccessThreshold => {
                super::ConfigError::InvalidFailureThreshold {
                    threshold: 0, // shouldn't happen with default 1
                }
            }
            CircuitBreakerError::InvalidOpenDuration => {
                super::ConfigError::InvalidRecoveryTimeout {
                    timeout_ms: recovery_timeout_ms,
                }
            }
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
        Duration::from_millis(self.open_duration_ms.get())
    }

    /// Record a successful execution
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold.get() {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitBreakerState::Open => {}
        }
    }

    /// Record a failed execution
    pub fn record_failure(&mut self) {
        self.last_failure_at = Some(Utc::now());
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold.get() {
                    self.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.success_count = 0;
            }
            CircuitBreakerState::Open => {}
        }
    }

    /// Check if a request can be executed (chrono-based, real-time check).
    ///
    /// Returns true for Closed and HalfOpen states.
    /// For Open state, checks if enough time has elapsed and auto-transitions to HalfOpen.
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_at {
                    let elapsed = Utc::now().signed_duration_since(last_failure);
                    if elapsed.num_milliseconds() >= self.open_duration_ms.get() as i64 {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.success_count = 0;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Check if execution is allowed (state-only check, no time-based transition).
    ///
    /// Returns true for Closed and HalfOpen, false for Open.
    /// Use `can_execute()` for auto-transitioning behavior.
    #[must_use]
    pub fn is_execution_allowed(&self) -> bool {
        matches!(
            self.state,
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen
        )
    }

    /// Attempts to transition from Open to HalfOpen based on elapsed time.
    ///
    /// Use when the caller tracks elapsed time externally (deterministic).
    pub fn check_and_transition(&mut self, elapsed_ms: u64) -> bool {
        if self.state == CircuitBreakerState::Open && elapsed_ms >= self.open_duration_ms.get() {
            self.state = CircuitBreakerState::HalfOpen;
            self.success_count = 0;
            return true;
        }
        false
    }

    /// Try to transition from Open to HalfOpen (chrono-based, real-time).
    ///
    /// Returns true if transition occurred.
    pub fn try_transition_to_half_open(&mut self) -> bool {
        if self.state == CircuitBreakerState::Open {
            if let Some(last_failure) = self.last_failure_at {
                let elapsed = Utc::now().signed_duration_since(last_failure);
                if elapsed.num_milliseconds() >= self.open_duration_ms.get() as i64 {
                    self.state = CircuitBreakerState::HalfOpen;
                    self.success_count = 0;
                    return true;
                }
            }
        }
        false
    }

    /// Reset to closed state (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction ---

    #[test]
    fn test_new_valid() {
        let cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.is_execution_allowed());
    }

    #[test]
    fn test_with_recovery_timeout_valid() {
        let cb = CircuitBreaker::with_recovery_timeout(3, 5000).expect("should create");
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_new_rejects_zero_failure_threshold() {
        assert_eq!(
            CircuitBreaker::new(0, 2, 30000).unwrap_err(),
            CircuitBreakerError::InvalidFailureThreshold
        );
    }

    #[test]
    fn test_new_rejects_zero_success_threshold() {
        assert_eq!(
            CircuitBreaker::new(3, 0, 30000).unwrap_err(),
            CircuitBreakerError::InvalidSuccessThreshold
        );
    }

    #[test]
    fn test_new_rejects_zero_open_duration() {
        assert_eq!(
            CircuitBreaker::new(3, 2, 0).unwrap_err(),
            CircuitBreakerError::InvalidOpenDuration
        );
    }

    #[test]
    fn test_with_recovery_timeout_rejects_zero_threshold() {
        let result = CircuitBreaker::with_recovery_timeout(0, 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_recovery_timeout_rejects_zero_timeout() {
        let result = CircuitBreaker::with_recovery_timeout(3, 0);
        assert!(result.is_err());
    }

    // --- State transitions: Closed -> Open ---

    #[test]
    fn test_opens_after_failure_threshold() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_single_failure_threshold_opens_immediately() {
        let mut cb = CircuitBreaker::new(1, 1, 30000).expect("should create");
        assert!(cb.is_execution_allowed());
        cb.record_failure();
        assert!(!cb.is_execution_allowed());
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_high_threshold_does_not_open_prematurely() {
        let mut cb = CircuitBreaker::new(100, 2, 30000).expect("should create");
        for _ in 0..99 {
            cb.record_failure();
            assert_eq!(cb.state(), CircuitBreakerState::Closed);
            assert!(cb.is_execution_allowed());
        }
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_failure_count_increments() {
        let mut cb = CircuitBreaker::with_recovery_timeout(5, 5000).expect("should create");
        for i in 1..5 {
            cb.record_failure();
            assert_eq!(cb.failure_count(), i);
        }
    }

    // --- Closed state: success resets failures ---

    #[test]
    fn test_closed_success_resets_failure_count() {
        let mut cb = CircuitBreaker::new(5, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 3);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    // --- Open state behavior ---

    #[test]
    fn test_open_rejects_execution() {
        let mut cb = CircuitBreaker::new(2, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_execution_allowed());
    }

    #[test]
    fn test_open_state_ignores_success() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_open_state_ignores_failure() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), 1);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), 1);
    }

    // --- HalfOpen transitions (deterministic via check_and_transition) ---

    #[test]
    fn test_check_and_transition_after_duration() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        let transitioned = cb.check_and_transition(30001);
        assert!(transitioned);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_check_and_transition_not_yet_elapsed() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        let transitioned = cb.check_and_transition(29999);
        assert!(!transitioned);
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_check_and_transition_already_half_open() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        let transitioned = cb.check_and_transition(50000);
        assert!(!transitioned);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_check_and_transition_already_closed() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        let transitioned = cb.check_and_transition(100000);
        assert!(!transitioned);
    }

    #[test]
    fn test_check_and_transition_resets_success_count() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        assert_eq!(cb.success_count(), 0);
    }

    // --- HalfOpen -> Closed (success threshold) ---

    #[test]
    fn test_halfopen_to_closed_with_threshold_1() {
        let mut cb = CircuitBreaker::new(1, 1, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
    }

    #[test]
    fn test_halfopen_to_closed_with_high_threshold() {
        let mut cb = CircuitBreaker::new(1, 5, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        for i in 0..4 {
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::HalfOpen,
                "Still half-open at success {}",
                i + 1
            );
        }
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    // --- HalfOpen -> Open (failure) ---

    #[test]
    fn test_halfopen_failure_returns_to_open() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        for _ in 0..3 {
            cb.record_failure();
        }
        cb.check_and_transition(30001);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.is_execution_allowed());
    }

    #[test]
    fn test_halfopen_failure_resets_success_count() {
        let mut cb = CircuitBreaker::new(1, 3, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        cb.record_success();
        cb.record_success();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.success_count(), 0);
    }

    // --- Chrono-based transitions (try_transition_to_half_open / can_execute) ---

    #[test]
    fn test_try_transition_to_half_open_after_timeout() {
        let mut cb = CircuitBreaker::with_recovery_timeout(1, 1).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_try_transition_to_half_open_before_timeout() {
        let mut cb = CircuitBreaker::with_recovery_timeout(1, 60000).expect("should create");
        cb.record_failure();
        assert!(!cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_try_transition_when_not_open() {
        let mut cb = CircuitBreaker::with_recovery_timeout(3, 5000).expect("should create");
        assert!(!cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_can_execute_auto_transitions_open_to_halfopen() {
        let mut cb = CircuitBreaker::with_recovery_timeout(1, 1).expect("should create");
        cb.record_failure();
        assert!(!cb.is_execution_allowed());

        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_half_open_transitions_to_closed_on_success() {
        let mut cb = CircuitBreaker::with_recovery_timeout(1, 1).expect("should create");
        cb.record_failure();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.try_transition_to_half_open());
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_half_open_to_open_on_failure() {
        let mut cb = CircuitBreaker::with_recovery_timeout(1, 1).expect("should create");
        cb.record_failure();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.try_transition_to_half_open());
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    // --- last_failure_at ---

    #[test]
    fn test_last_failure_at_set_on_failure() {
        let mut cb = CircuitBreaker::with_recovery_timeout(3, 5000).expect("should create");
        assert!(cb.last_failure_at.is_none());
        cb.record_failure();
        assert!(cb.last_failure_at.is_some());
    }

    // --- Getters ---

    #[test]
    fn test_getters_after_construction() {
        let cb = CircuitBreaker::new(7, 3, 5000).expect("should create");
        assert_eq!(cb.failure_threshold(), 7);
        assert_eq!(cb.success_threshold(), 3);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
        assert_eq!(cb.open_duration(), Duration::from_millis(5000));
    }

    // --- is_execution_allowed ---

    #[test]
    fn test_is_execution_allowed_states() {
        let mut cb = CircuitBreaker::new(2, 2, 30000).expect("should create");
        assert!(cb.is_execution_allowed()); // Closed
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_execution_allowed()); // Open
        cb.check_and_transition(30001);
        assert!(cb.is_execution_allowed()); // HalfOpen
    }

    // --- reset (test-only) ---

    #[test]
    fn test_reset_clears_all_state() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.reset();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
    }

    // --- Serde roundtrips ---

    #[test]
    fn test_state_serde_roundtrip() {
        let states = [
            CircuitBreakerState::Closed,
            CircuitBreakerState::Open,
            CircuitBreakerState::HalfOpen,
        ];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: CircuitBreakerState =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*state, deserialized);
        }
    }

    #[test]
    fn test_state_serde_uses_snake_case() {
        assert_eq!(
            serde_json::to_string(&CircuitBreakerState::Closed).expect("serialize"),
            "\"closed\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitBreakerState::Open).expect("serialize"),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitBreakerState::HalfOpen).expect("serialize"),
            "\"half_open\""
        );
    }

    #[test]
    fn test_circuit_breaker_serde_roundtrip() {
        let cb = CircuitBreaker::new(5, 2, 10000).expect("should create");
        let json = serde_json::to_string(&cb).expect("serialize");
        let deserialized: CircuitBreaker = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cb.failure_threshold(), deserialized.failure_threshold());
        assert_eq!(cb.success_threshold(), deserialized.success_threshold());
        assert_eq!(cb.state(), deserialized.state());
    }

    // --- Error display ---

    #[test]
    fn test_error_display() {
        let errors = [
            CircuitBreakerError::InvalidFailureThreshold,
            CircuitBreakerError::InvalidSuccessThreshold,
            CircuitBreakerError::InvalidOpenDuration,
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_error_implements_error() {
        use std::error::Error;
        let err = CircuitBreakerError::InvalidFailureThreshold;
        assert!(err.source().is_none());
    }

    // --- Invariant tests ---

    #[test]
    fn test_invariant_state_reflects_failure_rate() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    // --- Proptests ---

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_closed_never_transitions_to_open_before_threshold(
            threshold in 1u32..20u32,
            failures in 0u32..20u32,
        ) {
            let mut cb = CircuitBreaker::new(threshold, 1, 30000).expect("should create");
            for _ in 0..failures {
                cb.record_failure();
            }
            let should_be_open = failures >= threshold;
            prop_assert_eq!(cb.state() == CircuitBreakerState::Open, should_be_open);
        }

        #[test]
        fn prop_success_resets_failure_count_when_closed(
            num_failures in 0u32..10u32,
        ) {
            let mut cb = CircuitBreaker::new(100, 1, 30000).expect("should create");
            for _ in 0..num_failures {
                cb.record_failure();
            }
            cb.record_success();
            prop_assert_eq!(cb.failure_count(), 0);
        }
    }
}

//! Policy: Circuit breaker to prevent cascading failures
//!
//! Consolidated implementation combining:
//! - NonZeroU* types for compile-time validated configuration
//! - chrono-based time transitions for real-world usage
//! - serde support for serialization
//! - success_threshold for configurable HalfOpen → Closed recovery

use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Duration,
};

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
            Self::InvalidFailureThreshold => {
                write!(f, "failure_threshold must be positive")
            }
            Self::InvalidSuccessThreshold => {
                write!(f, "success_threshold must be positive")
            }
            Self::InvalidOpenDuration => {
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
        let open_duration_ms =
            NonZeroU64::new(open_duration_ms).ok_or(CircuitBreakerError::InvalidOpenDuration)?;
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
    pub const fn state(&self) -> CircuitBreakerState {
        self.state
    }

    /// Get the failure count
    #[must_use]
    pub const fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Get the success count
    #[must_use]
    pub const fn success_count(&self) -> u32 {
        self.success_count
    }

    /// Get the failure threshold
    #[must_use]
    pub const fn failure_threshold(&self) -> u32 {
        self.failure_threshold.get()
    }

    /// Get the success threshold
    #[must_use]
    pub const fn success_threshold(&self) -> u32 {
        self.success_threshold.get()
    }

    /// Get the open duration
    #[must_use]
    pub const fn open_duration(&self) -> Duration {
        Duration::from_millis(self.open_duration_ms.get())
    }

    /// Record a successful execution
    pub const fn record_success(&mut self) {
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
    pub const fn is_execution_allowed(&self) -> bool {
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

    // --- Full end-to-end lifecycle test ---

    #[test]
    fn test_full_lifecycle_closed_open_halfopen_closed() {
        // Thresholds: 3 failures to open, 2 successes to close from half-open, 30s open duration
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");

        // Phase 1: Closed — requests allowed
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.is_execution_allowed());

        // Phase 2: Accumulate failures BELOW threshold — stays Closed
        cb.record_failure();
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Closed,
            "2 failures < threshold 3"
        );
        assert!(cb.is_execution_allowed());
        assert_eq!(cb.failure_count(), 2);

        // Phase 3: Hit failure threshold — transitions to Open
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Open,
            "3 failures >= threshold 3"
        );
        assert!(!cb.is_execution_allowed(), "Open rejects requests");

        // Phase 3b: Open ignores successes and failures
        cb.record_success();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Open,
            "Open ignores success"
        );
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Open,
            "Open ignores failure"
        );

        // Phase 4: Not enough time elapsed — stays Open
        assert!(
            !cb.check_and_transition(29999),
            "29999ms < 30000ms open duration"
        );
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Phase 5: Enough time elapsed — transitions to HalfOpen (probe)
        assert!(cb.check_and_transition(30000));
        assert_eq!(
            cb.state(),
            CircuitBreakerState::HalfOpen,
            "elapsed >= open_duration"
        );
        assert!(cb.is_execution_allowed(), "HalfOpen allows probe requests");
        assert_eq!(cb.success_count(), 0, "success_count reset on transition");

        // Phase 6: Partial successes — stays HalfOpen
        cb.record_success();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::HalfOpen,
            "1 success < success_threshold 2"
        );
        assert_eq!(cb.success_count(), 1);

        // Phase 7: Hit success threshold — transitions to Closed
        cb.record_success();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Closed,
            "recovered after 2 successes"
        );
        assert!(cb.is_execution_allowed());
        assert_eq!(cb.failure_count(), 0, "failure_count reset on close");
        assert_eq!(cb.success_count(), 0, "success_count reset on close");
    }

    #[test]
    fn test_full_lifecycle_halfopen_failure_reopens() {
        let mut cb = CircuitBreaker::new(2, 2, 30000).expect("should create");

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Transition to HalfOpen
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        // Partial success then failure — reopens immediately
        cb.record_success();
        assert_eq!(cb.success_count(), 1);
        cb.record_failure();
        assert_eq!(
            cb.state(),
            CircuitBreakerState::Open,
            "failure in HalfOpen reopens"
        );
        assert_eq!(cb.success_count(), 0, "success_count reset on reopen");

        // Can transition to HalfOpen again
        cb.check_and_transition(60001);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        // This time, succeed fully
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
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

        /// Arbitrary operation sequences never violate state machine invariants:
        /// - Closed: failure_count <= failure_threshold, execution always allowed
        /// - Open: execution never allowed, only exit via check_and_transition
        /// - HalfOpen: success_count < success_threshold unless transitioning to Closed
        #[test]
        fn prop_arbitrary_operations_maintain_invariants(
            ops in proptest::collection::vec(
                prop_oneof![
                    Just(0u8), // record_failure
                    Just(1u8), // record_success
                    Just(2u8), // check_and_transition with large elapsed
                ],
                0..200usize,
            ),
            failure_threshold in 1u32..10u32,
            success_threshold in 1u32..10u32,
        ) {
            let mut cb = CircuitBreaker::new(failure_threshold, success_threshold, 1000)
                .expect("should create");

            for op in ops {
                match op {
                    0 => cb.record_failure(),
                    1 => cb.record_success(),
                    2 => { cb.check_and_transition(5000); }
                    _ => {}
                }

                match cb.state() {
                    CircuitBreakerState::Closed => {
                        prop_assert!(cb.is_execution_allowed());
                        prop_assert!(cb.failure_count() < failure_threshold);
                    }
                    CircuitBreakerState::Open => {
                        prop_assert!(!cb.is_execution_allowed());
                    }
                    CircuitBreakerState::HalfOpen => {
                        prop_assert!(cb.is_execution_allowed());
                        prop_assert!(cb.success_count() < success_threshold);
                    }
                }
            }
        }

        /// Full lifecycle completes deterministically with any valid thresholds
        #[test]
        fn prop_full_lifecycle_with_random_thresholds(
            failure_threshold in 1u32..20u32,
            success_threshold in 1u32..20u32,
            open_duration_ms in 1u64..60000u64,
        ) {
            let mut cb = CircuitBreaker::new(failure_threshold, success_threshold, open_duration_ms)
                .expect("should create");

            // Phase 1: Start Closed
            prop_assert_eq!(cb.state(), CircuitBreakerState::Closed);

            // Phase 2: Trip to Open
            for _ in 0..failure_threshold {
                cb.record_failure();
            }
            prop_assert_eq!(cb.state(), CircuitBreakerState::Open);

            // Phase 3: Transition to HalfOpen
            prop_assert!(cb.check_and_transition(open_duration_ms));
            prop_assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

            // Phase 4: Recover to Closed
            for _ in 0..success_threshold {
                cb.record_success();
            }
            prop_assert_eq!(cb.state(), CircuitBreakerState::Closed);
            prop_assert_eq!(cb.failure_count(), 0);
            prop_assert_eq!(cb.success_count(), 0);
        }

        /// In Closed state, failure_count never exceeds failure_threshold
        #[test]
        fn prop_closed_failure_count_bounded(
            failure_threshold in 2u32..50u32,
        ) {
            let mut cb = CircuitBreaker::new(failure_threshold, 1, 30000)
                .expect("should create");

            // Record failures up to threshold
            for _ in 0..(failure_threshold - 1) {
                cb.record_failure();
                prop_assert!(cb.failure_count() < failure_threshold);
                prop_assert_eq!(cb.state(), CircuitBreakerState::Closed);
            }

            // One more failure trips to Open
            cb.record_failure();
            prop_assert_eq!(cb.state(), CircuitBreakerState::Open);
        }

        /// check_and_transition never transitions from Closed or HalfOpen
        #[test]
        fn prop_check_and_transition_only_works_from_open(
            initial_state_op in 0u8..6u8,
            elapsed in 0u64..100_000u64,
        ) {
            let mut cb = CircuitBreaker::new(1, 1, 1000).expect("should create");

            // Set up various states
            match initial_state_op {
                0 => { /* Closed (default) */ }
                1 | 2 | 3 => {
                    cb.record_failure(); // Open
                    if initial_state_op >= 2 {
                        cb.check_and_transition(5000); // HalfOpen
                    }
                }
                _ => { /* Closed */ }
            }

            let state_before = cb.state();
            let transitioned = cb.check_and_transition(elapsed);

            match state_before {
                CircuitBreakerState::Open => {
                    // May or may not transition depending on elapsed
                }
                CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                    prop_assert!(!transitioned);
                    prop_assert_eq!(cb.state(), state_before);
                }
            }
        }
    }

    // --- Exhaustive lifecycle tests (ha-g1t4) ---

    #[test]
    fn test_exact_boundary_elapsed_equals_open_duration() {
        let mut cb = CircuitBreaker::new(1, 1, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Exactly at the boundary — must transition
        let transitioned = cb.check_and_transition(30000);
        assert!(
            transitioned,
            "must transition when elapsed == open_duration"
        );
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_failure_count_clamps_at_threshold() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), 3, "failure_count stays at threshold");

        // Additional failures in Open state don't increment
        cb.record_failure();
        cb.record_failure();
        assert_eq!(
            cb.failure_count(),
            3,
            "failure_count unchanged in Open state"
        );
    }

    #[test]
    fn test_multiple_complete_lifecycle_cycles() {
        let mut cb = CircuitBreaker::new(2, 2, 1000).expect("should create");

        for cycle in 0..5 {
            // Closed → Open
            cb.record_failure();
            cb.record_failure();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::Open,
                "cycle {}: should be Open after 2 failures",
                cycle
            );

            // Open → HalfOpen
            assert!(cb.check_and_transition(1001));
            assert_eq!(
                cb.state(),
                CircuitBreakerState::HalfOpen,
                "cycle {}: should be HalfOpen after transition",
                cycle
            );

            // HalfOpen → Closed
            cb.record_success();
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::Closed,
                "cycle {}: should be Closed after 2 successes",
                cycle
            );
            assert_eq!(
                cb.failure_count(),
                0,
                "cycle {}: failure_count reset",
                cycle
            );
            assert_eq!(
                cb.success_count(),
                0,
                "cycle {}: success_count reset",
                cycle
            );
        }
    }

    #[test]
    fn test_repeated_open_halfopen_cycles_without_recovery() {
        let mut cb = CircuitBreaker::new(1, 3, 1000).expect("should create");
        cb.record_failure(); // Open

        for cycle in 0..4 {
            // Open → HalfOpen
            assert!(cb.check_and_transition(1001 + (cycle as u64 * 2000)));
            assert_eq!(cb.state(), CircuitBreakerState::HalfOpen, "cycle {}", cycle);

            // Partial success (not enough to close)
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::HalfOpen,
                "cycle {} after 1 success",
                cycle
            );

            // Failure reopens
            cb.record_failure();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::Open,
                "cycle {} after failure",
                cycle
            );
            assert_eq!(cb.success_count(), 0);
        }

        // Finally recover
        assert!(cb.check_and_transition(20000));
        cb.record_success();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_success_in_closed_resets_accumulated_failures() {
        let mut cb = CircuitBreaker::new(5, 2, 30000).expect("should create");

        // Build up to 4 failures (one below threshold)
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.failure_count(), 4);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        // Success resets
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);

        // Build up again
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.failure_count(), 4);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        // One more failure trips
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_halfopen_to_closed_resets_both_counters() {
        let mut cb = CircuitBreaker::new(1, 2, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        cb.record_success(); // success_count = 1
        assert_eq!(cb.success_count(), 1);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.record_success(); // trips to Closed
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0, "failure_count must be 0");
        assert_eq!(cb.success_count(), 0, "success_count must be 0");
    }

    #[test]
    fn test_large_threshold_many_failures_before_open() {
        let mut cb = CircuitBreaker::new(1000, 1, 30000).expect("should create");

        for i in 0..999 {
            cb.record_failure();
            assert_eq!(cb.state(), CircuitBreakerState::Closed, "failure {}", i);
        }
        cb.record_failure(); // 1000th
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_success_threshold_1_single_success_closes_from_halfopen() {
        let mut cb = CircuitBreaker::new(1, 1, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        cb.record_success(); // Immediately closes
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_halfopen_partial_success_then_failure_resets_progress() {
        let mut cb = CircuitBreaker::new(1, 5, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        // Accumulate 4 successes
        for i in 0..4 {
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::HalfOpen,
                "after success {}",
                i + 1
            );
            assert_eq!(cb.success_count(), i as u32 + 1);
        }

        // Single failure blows all progress
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.success_count(), 0);
    }

    #[test]
    fn test_open_state_both_operations_are_no_ops() {
        let mut cb = CircuitBreaker::new(1, 2, 1000).expect("should create");
        cb.record_failure(); // Open

        let fc_before = cb.failure_count();
        let sc_before = cb.success_count();

        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), fc_before);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), fc_before);
        assert_eq!(cb.success_count(), sc_before);
    }

    #[test]
    fn test_check_and_transition_below_threshold_returns_false() {
        let mut cb = CircuitBreaker::new(1, 1, 10000).expect("should create");
        cb.record_failure(); // Open

        assert!(!cb.check_and_transition(9999));
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        assert!(!cb.check_and_transition(0));
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_full_lifecycle_with_halfopen_failure_then_recovery() {
        let mut cb = CircuitBreaker::new(2, 2, 1000).expect("should create");

        // Closed → Open
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Open → HalfOpen
        cb.check_and_transition(1001);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        // HalfOpen → Open (failure)
        cb.record_success(); // 1 success
        cb.record_failure(); // reopens
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Open → HalfOpen again
        cb.check_and_transition(2001);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        // HalfOpen → Closed (full recovery)
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        // Verify full reset
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
        assert!(cb.is_execution_allowed());
    }

    #[test]
    fn test_closed_state_success_interleaved_with_failures() {
        let mut cb = CircuitBreaker::new(5, 2, 30000).expect("should create");

        // Failure, failure, success (reset), failure, failure, failure, failure (4 total)
        cb.record_failure();
        assert_eq!(cb.failure_count(), 1);
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0); // reset
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 4);
        assert_eq!(cb.state(), CircuitBreakerState::Closed); // Still under threshold
    }
}

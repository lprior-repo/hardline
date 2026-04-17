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

    // --- Serde roundtrips ---

    #[test]
    fn test_circuit_breaker_state_serde_roundtrip() {
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
    fn test_circuit_breaker_state_serde_uses_snake_case() {
        let json = serde_json::to_string(&CircuitBreakerState::HalfOpen).expect("serialize");
        assert_eq!(json, "\"half_open\"");
    }

    #[test]
    fn test_circuit_breaker_serde_roundtrip() {
        let cb = CircuitBreaker::new(5, 10000).expect("should create");
        let json = serde_json::to_string(&cb).expect("serialize");
        let deserialized: CircuitBreaker = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cb.failure_threshold, deserialized.failure_threshold);
        assert_eq!(cb.state(), deserialized.state());
    }

    // --- Additional state transition tests ---

    #[test]
    fn test_circuit_breaker_half_open_transitions_to_closed_on_success() {
        let mut cb = CircuitBreaker::new(1, 1).expect("should create circuit breaker");
        // 1ms recovery timeout
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Wait for recovery then explicitly transition to HalfOpen
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_to_open_on_failure() {
        let mut cb = CircuitBreaker::new(1, 1).expect("should create circuit breaker");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_try_transition_to_half_open_before_timeout() {
        let mut cb = CircuitBreaker::new(1, 60000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        let result = cb.try_transition_to_half_open();
        assert!(!result);
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_try_transition_to_half_open_after_timeout() {
        let mut cb = CircuitBreaker::new(1, 1).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        std::thread::sleep(std::time::Duration::from_millis(5));
        let result = cb.try_transition_to_half_open();
        assert!(result);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_try_transition_when_not_open() {
        let mut cb = CircuitBreaker::new(3, 5000).expect("should create");
        let result = cb.try_transition_to_half_open();
        assert!(!result);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_invalid_recovery_timeout_zero() {
        let result = CircuitBreaker::new(3, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_circuit_breaker_failure_count_increments() {
        let mut cb = CircuitBreaker::new(5, 5000).expect("should create");
        for i in 1..5 {
            cb.record_failure();
            assert_eq!(cb.failure_count(), i);
        }
    }

    #[test]
    fn test_circuit_breaker_record_success_in_closed_resets_count() {
        let mut cb = CircuitBreaker::new(5, 5000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_last_failure_at_set_on_failure() {
        let mut cb = CircuitBreaker::new(3, 5000).expect("should create");
        assert!(cb.last_failure_at.is_none());
        cb.record_failure();
        assert!(cb.last_failure_at.is_some());
    }

    #[test]
    fn test_circuit_breaker_failure_threshold_boundary() {
        let mut cb = CircuitBreaker::new(2, 5000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_state_serde_all_variants() {
        use crate::policies::CircuitBreakerState;
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

    // --- Exhaustive lifecycle tests (ha-g1t4) ---

    #[test]
    fn test_can_execute_always_true_in_closed_state() {
        let mut cb = CircuitBreaker::new(5, 5000).expect("should create");

        // Always true regardless of failure count
        for _ in 0..4 {
            cb.record_failure();
            assert!(cb.can_execute(), "must allow execution in Closed state");
        }
    }

    #[test]
    fn test_failure_count_increments_correctly_without_premature_open() {
        let mut cb = CircuitBreaker::new(10, 5000).expect("should create");

        for i in 1..10 {
            cb.record_failure();
            assert_eq!(cb.failure_count(), i, "failure count at step {}", i);
            assert_eq!(cb.state(), CircuitBreakerState::Closed, "state at step {}", i);
        }
    }

    #[test]
    fn test_record_failure_updates_last_failure_at_each_time() {
        let mut cb = CircuitBreaker::new(10, 5000).expect("should create");

        let t1 = Utc::now();
        cb.record_failure();
        let ts1 = cb.last_failure_at.expect("should be set after failure");
        assert!(ts1 >= t1);

        std::thread::sleep(std::time::Duration::from_millis(2));

        let t2 = Utc::now();
        cb.record_failure();
        let ts2 = cb.last_failure_at.expect("should be set after failure");
        assert!(ts2 >= t2);
        assert!(ts2 > ts1, "timestamp should update on each failure");
    }

    #[test]
    fn test_repeated_lifecycle_cycles() {
        for cycle in 0..3 {
            let mut cb = CircuitBreaker::new(2, 1).expect("should create");

            // Closed → Open
            cb.record_failure();
            cb.record_failure();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::Open,
                "cycle {}: should be Open",
                cycle
            );

            // Open → HalfOpen
            std::thread::sleep(std::time::Duration::from_millis(2));
            assert!(
                cb.try_transition_to_half_open(),
                "cycle {}: should transition to HalfOpen",
                cycle
            );
            assert_eq!(
                cb.state(),
                CircuitBreakerState::HalfOpen,
                "cycle {}: should be HalfOpen",
                cycle
            );

            // HalfOpen → Closed
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitBreakerState::Closed,
                "cycle {}: should be Closed after success",
                cycle
            );
            assert_eq!(cb.failure_count(), 0, "cycle {}: failure_count reset", cycle);
        }
    }

    #[test]
    fn test_halfopen_failure_reopens_immediately() {
        let mut cb = CircuitBreaker::new(1, 1).expect("should create");
        cb.record_failure(); // Open
        std::thread::sleep(std::time::Duration::from_millis(2));
        cb.try_transition_to_half_open(); // HalfOpen

        cb.record_failure(); // Immediately reopens
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(cb.failure_count() >= 1);
    }

    #[test]
    fn test_halfopen_success_closes_and_resets_failure_count() {
        let mut cb = CircuitBreaker::new(3, 1).expect("should create");

        // Build up failures
        cb.record_failure();
        cb.record_failure();
        cb.record_failure(); // Open
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), 3);

        // Recover
        std::thread::sleep(std::time::Duration::from_millis(2));
        cb.try_transition_to_half_open();
        cb.record_success();

        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_success_in_closed_resets_accumulated_failures() {
        let mut cb = CircuitBreaker::new(5, 5000).expect("should create");

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 3);

        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_can_execute_in_halfopen_returns_true() {
        let mut cb = CircuitBreaker::new(1, 1).expect("should create");
        cb.record_failure(); // Open
        std::thread::sleep(std::time::Duration::from_millis(2));
        cb.try_transition_to_half_open();

        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
        assert!(cb.can_execute(), "HalfOpen must allow probe requests");
    }

    #[test]
    fn test_try_transition_to_half_open_only_works_from_open() {
        let mut cb = CircuitBreaker::new(3, 5000).expect("should create");
        // Closed — should not transition
        assert!(!cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_full_lifecycle_closed_open_halfopen_closed_via_can_execute() {
        let mut cb = CircuitBreaker::new(2, 1).expect("should create");

        // Phase 1: Closed
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());

        // Phase 2: Accumulate failures below threshold
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());

        // Phase 3: Trip to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute(), "Open must reject execution immediately");

        // Phase 4: Wait and check — can_execute returns true but state stays Open
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(cb.can_execute(), "can_execute returns true after recovery timeout");
        // Note: can_execute is &self, state is still Open

        // Phase 5: Explicitly transition to HalfOpen
        assert!(cb.try_transition_to_half_open());
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        // Phase 6: Record success → Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    // --- Proptests for circuit.rs (ha-g1t4) ---

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_failure_threshold_never_opens_before_threshold(
            threshold in 1u32..20u32,
            failures in 0u32..20u32,
        ) {
            let mut cb = CircuitBreaker::new(threshold, 1000).expect("should create");
            for _ in 0..failures {
                if cb.state() == CircuitBreakerState::Closed {
                    cb.record_failure();
                }
            }
            let should_be_open = failures >= threshold;
            prop_assert_eq!(
                cb.state() == CircuitBreakerState::Open,
                should_be_open
            );
        }

        #[test]
        fn prop_success_resets_failure_count_in_closed(
            num_failures in 0u32..10u32,
        ) {
            let mut cb = CircuitBreaker::new(100, 5000).expect("should create");
            for _ in 0..num_failures {
                cb.record_failure();
            }
            cb.record_success();
            prop_assert_eq!(cb.failure_count(), 0);
            prop_assert_eq!(cb.state(), CircuitBreakerState::Closed);
        }

        #[test]
        fn prop_halfopen_failure_always_reopens(
            threshold in 1u32..10u32,
        ) {
            let mut cb = CircuitBreaker::new(threshold, 1).expect("should create");
            for _ in 0..threshold {
                cb.record_failure();
            }
            prop_assert_eq!(cb.state(), CircuitBreakerState::Open);

            std::thread::sleep(std::time::Duration::from_millis(2));
            let transitioned = cb.try_transition_to_half_open();
            prop_assert!(transitioned);
            prop_assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

            cb.record_failure();
            prop_assert_eq!(cb.state(), CircuitBreakerState::Open);
        }
    }
}

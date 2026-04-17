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

    // --- Open state ignores all events ---

    #[test]
    fn test_open_state_ignores_success() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Open); // Still open
    }

    #[test]
    fn test_open_state_ignores_failure() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), 1);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        // Open state does not increment failure_count
        assert_eq!(cb.failure_count(), 1);
    }

    // --- check_and_transition edge cases ---

    #[test]
    fn test_check_and_transition_not_yet_elapsed() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        let transitioned = cb.check_and_transition(29999);
        assert!(!transitioned);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_check_and_transition_already_half_open() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Already half-open, check_and_transition should not change state
        let transitioned = cb.check_and_transition(50000);
        assert!(!transitioned);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_check_and_transition_already_closed() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        assert_eq!(cb.state(), CircuitState::Closed);
        let transitioned = cb.check_and_transition(100000);
        assert!(!transitioned);
    }

    #[test]
    fn test_check_and_transition_resets_success_count() {
        let mut cb = CircuitBreaker::new(1, 2, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert_eq!(cb.success_count(), 0);
    }

    // --- HalfOpen -> Closed after threshold ---

    #[test]
    fn test_halfopen_to_closed_with_success_threshold_of_1() {
        let mut cb = CircuitBreaker::new(1, 1, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
    }

    #[test]
    fn test_halfopen_to_closed_with_high_success_threshold() {
        let mut cb = CircuitBreaker::new(1, 5, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        for i in 0..4 {
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitState::HalfOpen,
                "Still half-open at success {}",
                i + 1
            );
        }
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    // --- HalfOpen -> Open on failure resets success_count ---

    #[test]
    fn test_halfopen_failure_resets_success_count() {
        let mut cb = CircuitBreaker::new(1, 3, 30000).expect("should create");
        cb.record_failure();
        cb.check_and_transition(30001);
        cb.record_success(); // success_count = 1
        cb.record_success(); // success_count = 2
        cb.record_failure(); // Back to Open, success_count reset
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.success_count(), 0);
    }

    // --- Closed state resets failure_count on success ---

    #[test]
    fn test_closed_success_resets_failure_count() {
        let mut cb = CircuitBreaker::new(5, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 3);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    // --- Getter accuracy ---

    #[test]
    fn test_getters_after_construction() {
        let cb = CircuitBreaker::new(7, 3, 5000).expect("should create");
        assert_eq!(cb.failure_threshold(), 7);
        assert_eq!(cb.success_threshold(), 3);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
        assert_eq!(cb.open_duration(), std::time::Duration::from_millis(5000));
    }

    // --- is_execution_allowed ---

    #[test]
    fn test_is_execution_allowed() {
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
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
    }

    // --- Error display ---

    #[test]
    fn test_circuit_breaker_error_display() {
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

    // --- Error trait ---

    #[test]
    fn test_circuit_breaker_error_implements_error() {
        use std::error::Error;
        let err = CircuitBreakerError::InvalidFailureThreshold;
        assert!(err.source().is_none());
    }

    // --- Single failure threshold ---

    #[test]
    fn test_single_failure_threshold_opens_immediately() {
        let mut cb = CircuitBreaker::new(1, 1, 30000).expect("should create");
        assert!(cb.is_execution_allowed());
        cb.record_failure();
        assert!(!cb.is_execution_allowed());
        assert_eq!(cb.state(), CircuitState::Open);
    }

    // --- High threshold doesn't open prematurely ---

    #[test]
    fn test_high_threshold_does_not_open_prematurely() {
        let mut cb = CircuitBreaker::new(100, 2, 30000).expect("should create");
        for _ in 0..99 {
            cb.record_failure();
            assert_eq!(cb.state(), CircuitState::Closed);
            assert!(cb.is_execution_allowed());
        }
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_execution_allowed());
    }

    // --- Full end-to-end lifecycle test ---

    #[test]
    fn test_full_lifecycle_closed_open_halfopen_closed() {
        // Thresholds: 3 failures to open, 2 successes to close from half-open, 30s open duration
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");

        // Phase 1: Closed — requests allowed
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_execution_allowed());

        // Phase 2: Accumulate failures BELOW threshold — stays Closed
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed, "2 failures < threshold 3");
        assert!(cb.is_execution_allowed());
        assert_eq!(cb.failure_count(), 2);

        // Phase 3: Hit failure threshold — transitions to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open, "3 failures >= threshold 3");
        assert!(!cb.is_execution_allowed(), "Open rejects requests");

        // Phase 3b: Open ignores successes and failures
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Open, "Open ignores success");
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open, "Open ignores failure");

        // Phase 4: Not enough time elapsed — stays Open
        assert!(
            !cb.check_and_transition(29999),
            "29999ms < 30000ms open duration"
        );
        assert_eq!(cb.state(), CircuitState::Open);

        // Phase 5: Enough time elapsed — transitions to HalfOpen (probe)
        assert!(cb.check_and_transition(30000));
        assert_eq!(cb.state(), CircuitState::HalfOpen, "elapsed >= open_duration");
        assert!(cb.is_execution_allowed(), "HalfOpen allows probe requests");
        assert_eq!(cb.success_count(), 0, "success_count reset on transition");

        // Phase 6: Partial successes — stays HalfOpen
        cb.record_success();
        assert_eq!(
            cb.state(),
            CircuitState::HalfOpen,
            "1 success < success_threshold 2"
        );
        assert_eq!(cb.success_count(), 1);

        // Phase 7: Hit success threshold — transitions to Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed, "recovered after 2 successes");
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
        assert_eq!(cb.state(), CircuitState::Open);

        // Transition to HalfOpen
        cb.check_and_transition(30001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Partial success then failure — reopens immediately
        cb.record_success();
        assert_eq!(cb.success_count(), 1);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open, "failure in HalfOpen reopens");
        assert_eq!(cb.success_count(), 0, "success_count reset on reopen");

        // Can transition to HalfOpen again
        cb.check_and_transition(60001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // This time, succeed fully
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
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
            prop_assert_eq!(cb.state() == CircuitState::Open, should_be_open);
        }

        #[test]
        fn prop_circuit_breaker_success_resets_failure_count_when_closed(
            num_failures in 0u32..10u32,
        ) {
            // Use a high threshold so we stay in Closed state
            let mut cb = CircuitBreaker::new(100, 1, 30000).expect("should create");
            for _ in 0..num_failures {
                cb.record_failure();
            }
            // After recording success, failure_count must be 0 (Closed state behavior)
            cb.record_success();
            let count_after_success = cb.failure_count();
            prop_assert_eq!(count_after_success, 0);
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
                    CircuitState::Closed => {
                        prop_assert!(cb.is_execution_allowed());
                        prop_assert!(cb.failure_count() < failure_threshold);
                    }
                    CircuitState::Open => {
                        prop_assert!(!cb.is_execution_allowed());
                    }
                    CircuitState::HalfOpen => {
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
            prop_assert_eq!(cb.state(), CircuitState::Closed);

            // Phase 2: Trip to Open
            for _ in 0..failure_threshold {
                cb.record_failure();
            }
            prop_assert_eq!(cb.state(), CircuitState::Open);

            // Phase 3: Transition to HalfOpen
            prop_assert!(cb.check_and_transition(open_duration_ms));
            prop_assert_eq!(cb.state(), CircuitState::HalfOpen);

            // Phase 4: Recover to Closed
            for _ in 0..success_threshold {
                cb.record_success();
            }
            prop_assert_eq!(cb.state(), CircuitState::Closed);
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
                prop_assert_eq!(cb.state(), CircuitState::Closed);
            }

            // One more failure trips to Open
            cb.record_failure();
            prop_assert_eq!(cb.state(), CircuitState::Open);
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
                CircuitState::Open => {
                    // May or may not transition depending on elapsed
                }
                CircuitState::Closed | CircuitState::HalfOpen => {
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
        assert_eq!(cb.state(), CircuitState::Open);

        // Exactly at the boundary — must transition
        let transitioned = cb.check_and_transition(30000);
        assert!(transitioned, "must transition when elapsed == open_duration");
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_failure_count_clamps_at_threshold() {
        let mut cb = CircuitBreaker::new(3, 2, 30000).expect("should create");
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), 3, "failure_count stays at threshold");

        // Additional failures in Open state don't increment
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 3, "failure_count unchanged in Open state");
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
                CircuitState::Open,
                "cycle {}: should be Open after 2 failures",
                cycle
            );

            // Open → HalfOpen
            assert!(cb.check_and_transition(1001));
            assert_eq!(
                cb.state(),
                CircuitState::HalfOpen,
                "cycle {}: should be HalfOpen after transition",
                cycle
            );

            // HalfOpen → Closed
            cb.record_success();
            cb.record_success();
            assert_eq!(
                cb.state(),
                CircuitState::Closed,
                "cycle {}: should be Closed after 2 successes",
                cycle
            );
            assert_eq!(cb.failure_count(), 0, "cycle {}: failure_count reset", cycle);
            assert_eq!(cb.success_count(), 0, "cycle {}: success_count reset", cycle);
        }
    }

    #[test]
    fn test_repeated_open_halfopen_cycles_without_recovery() {
        let mut cb = CircuitBreaker::new(1, 3, 1000).expect("should create");
        cb.record_failure(); // Open

        for cycle in 0..4 {
            // Open → HalfOpen
            assert!(cb.check_and_transition(1001 + (cycle as u64 * 2000)));
            assert_eq!(cb.state(), CircuitState::HalfOpen, "cycle {}", cycle);

            // Partial success (not enough to close)
            cb.record_success();
            assert_eq!(cb.state(), CircuitState::HalfOpen, "cycle {} after 1 success", cycle);

            // Failure reopens
            cb.record_failure();
            assert_eq!(cb.state(), CircuitState::Open, "cycle {} after failure", cycle);
            assert_eq!(cb.success_count(), 0);
        }

        // Finally recover
        assert!(cb.check_and_transition(20000));
        cb.record_success();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_success_in_closed_resets_accumulated_failures() {
        let mut cb = CircuitBreaker::new(5, 2, 30000).expect("should create");

        // Build up to 4 failures (one below threshold)
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.failure_count(), 4);
        assert_eq!(cb.state(), CircuitState::Closed);

        // Success resets
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);

        // Build up again
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.failure_count(), 4);
        assert_eq!(cb.state(), CircuitState::Closed);

        // One more failure trips
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_halfopen_to_closed_resets_both_counters() {
        let mut cb = CircuitBreaker::new(1, 2, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        cb.record_success(); // success_count = 1
        assert_eq!(cb.success_count(), 1);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success(); // trips to Closed
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0, "failure_count must be 0");
        assert_eq!(cb.success_count(), 0, "success_count must be 0");
    }

    #[test]
    fn test_large_threshold_many_failures_before_open() {
        let mut cb = CircuitBreaker::new(1000, 1, 30000).expect("should create");

        for i in 0..999 {
            cb.record_failure();
            assert_eq!(cb.state(), CircuitState::Closed, "failure {}", i);
        }
        cb.record_failure(); // 1000th
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_success_threshold_1_single_success_closes_from_halfopen() {
        let mut cb = CircuitBreaker::new(1, 1, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        cb.record_success(); // Immediately closes
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_halfopen_partial_success_then_failure_resets_progress() {
        let mut cb = CircuitBreaker::new(1, 5, 1000).expect("should create");
        cb.record_failure(); // Open
        cb.check_and_transition(1001); // HalfOpen

        // Accumulate 4 successes
        for i in 0..4 {
            cb.record_success();
            assert_eq!(cb.state(), CircuitState::HalfOpen, "after success {}", i + 1);
            assert_eq!(cb.success_count(), i as u32 + 1);
        }

        // Single failure blows all progress
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.success_count(), 0);
    }

    #[test]
    fn test_open_state_both_operations_are_no_ops() {
        let mut cb = CircuitBreaker::new(1, 2, 1000).expect("should create");
        cb.record_failure(); // Open

        let fc_before = cb.failure_count();
        let sc_before = cb.success_count();

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), fc_before);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), fc_before);
        assert_eq!(cb.success_count(), sc_before);
    }

    #[test]
    fn test_check_and_transition_below_threshold_returns_false() {
        let mut cb = CircuitBreaker::new(1, 1, 10000).expect("should create");
        cb.record_failure(); // Open

        assert!(!cb.check_and_transition(9999));
        assert_eq!(cb.state(), CircuitState::Open);

        assert!(!cb.check_and_transition(0));
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_full_lifecycle_with_halfopen_failure_then_recovery() {
        let mut cb = CircuitBreaker::new(2, 2, 1000).expect("should create");

        // Closed → Open
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Open → HalfOpen
        cb.check_and_transition(1001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // HalfOpen → Open (failure)
        cb.record_success(); // 1 success
        cb.record_failure(); // reopens
        assert_eq!(cb.state(), CircuitState::Open);

        // Open → HalfOpen again
        cb.check_and_transition(2001);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // HalfOpen → Closed (full recovery)
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);

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
        assert_eq!(cb.state(), CircuitState::Closed); // Still under threshold
    }
}

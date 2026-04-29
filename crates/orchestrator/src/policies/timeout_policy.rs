//! Timeout policy with type-level validation

use std::num::NonZeroU64;

/// Configuration for phase timeout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutPolicy {
    timeout_ms: Option<NonZeroU64>,
}

impl TimeoutPolicy {
    /// Create a TimeoutPolicy if the timeout value is valid (> 0ms)
    pub fn new(timeout_ms: u64) -> Result<Self, TimeoutPolicyError> {
        if timeout_ms == 0 {
            return Err(TimeoutPolicyError::ZeroTimeout);
        }
        NonZeroU64::new(timeout_ms)
            .map(|nz| Self {
                timeout_ms: Some(nz),
            })
            .ok_or(TimeoutPolicyError::ZeroTimeout)
    }

    /// Create a TimeoutPolicy with no timeout (infinite)
    #[must_use]
    pub const fn none() -> Self {
        Self { timeout_ms: None }
    }

    /// Returns the effective timeout in milliseconds
    #[must_use]
    pub fn get_timeout_ms(&self) -> Option<u64> {
        self.timeout_ms.map(|nz| nz.get())
    }

    /// Returns true if no timeout is configured
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.timeout_ms.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutPolicyError {
    ZeroTimeout,
}

impl std::fmt::Display for TimeoutPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroTimeout => {
                write!(f, "timeout must be greater than 0ms")
            }
        }
    }
}

impl std::error::Error for TimeoutPolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────────

    #[test]
    fn test_new_valid_timeout() {
        let policy = TimeoutPolicy::new(5000).expect("should create policy");
        assert_eq!(policy.get_timeout_ms(), Some(5000));
        assert!(!policy.is_none());
    }

    #[test]
    fn test_new_minimum_valid_timeout() {
        let policy = TimeoutPolicy::new(1).expect("1ms is the minimum valid timeout");
        assert_eq!(policy.get_timeout_ms(), Some(1));
    }

    #[test]
    fn test_new_zero_timeout_returns_error() {
        let result = TimeoutPolicy::new(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TimeoutPolicyError::ZeroTimeout);
    }

    #[test]
    fn test_new_max_u64_timeout() {
        let policy = TimeoutPolicy::new(u64::MAX).expect("u64::MAX is valid");
        assert_eq!(policy.get_timeout_ms(), Some(u64::MAX));
    }

    #[test]
    fn test_none_creates_infinite_timeout() {
        let policy = TimeoutPolicy::none();
        assert!(policy.is_none());
        assert_eq!(policy.get_timeout_ms(), None);
    }

    // ── Accessors ─────────────────────────────────────────────────────

    #[test]
    fn test_get_timeout_ms_returns_value_for_finite_policy() {
        let policy = TimeoutPolicy::new(12345).unwrap();
        assert_eq!(policy.get_timeout_ms(), Some(12345));
    }

    #[test]
    fn test_get_timeout_ms_returns_none_for_infinite_policy() {
        let policy = TimeoutPolicy::none();
        assert_eq!(policy.get_timeout_ms(), None);
    }

    #[test]
    fn test_is_none_false_for_finite_policy() {
        let policy = TimeoutPolicy::new(100).unwrap();
        assert!(!policy.is_none());
    }

    #[test]
    fn test_is_none_true_for_infinite_policy() {
        assert!(TimeoutPolicy::none().is_none());
    }

    // ── Equality & Copy semantics ─────────────────────────────────────

    #[test]
    fn test_equal_policies() {
        assert_eq!(
            TimeoutPolicy::new(5000).unwrap(),
            TimeoutPolicy::new(5000).unwrap()
        );
    }

    #[test]
    fn test_unequal_policies() {
        assert_ne!(
            TimeoutPolicy::new(5000).unwrap(),
            TimeoutPolicy::new(10000).unwrap()
        );
    }

    #[test]
    fn test_none_not_equal_to_finite() {
        assert_ne!(TimeoutPolicy::none(), TimeoutPolicy::new(5000).unwrap());
    }

    #[test]
    fn test_none_equal_to_none() {
        assert_eq!(TimeoutPolicy::none(), TimeoutPolicy::none());
    }

    #[test]
    fn test_copy_semantics() {
        let original = TimeoutPolicy::new(3000).unwrap();
        let copy = original;
        assert_eq!(original, copy);
    }

    // ── Error type ────────────────────────────────────────────────────

    #[test]
    fn test_error_display_message() {
        let msg = format!("{}", TimeoutPolicyError::ZeroTimeout);
        assert!(msg.contains("timeout must be greater than 0ms"));
    }

    #[test]
    fn test_error_implements_std_error() {
        use std::error::Error;
        assert!(TimeoutPolicyError::ZeroTimeout.source().is_none());
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(
            TimeoutPolicyError::ZeroTimeout,
            TimeoutPolicyError::ZeroTimeout
        );
    }

    #[test]
    fn test_error_clone() {
        let err = TimeoutPolicyError::ZeroTimeout;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ── Deadline enforcement ──────────────────────────────────────────
    //
    // TimeoutPolicy is a configuration value. Deadline enforcement is
    // exercised by combining TimeoutPolicy with Deadline/PhaseTimeout.
    // These tests verify the policy's timeout value drives correct
    // deadline behavior.

    #[test]
    fn test_deadline_from_policy_expires_after_timeout() {
        let policy = TimeoutPolicy::new(50).unwrap();
        let timeout_ms = policy.get_timeout_ms().expect("should have timeout");
        let deadline = super::super::Deadline::from_now(timeout_ms);

        // Immediately after creation, deadline should NOT be exceeded
        assert!(!deadline.is_exceeded());

        // Simulate waiting past the deadline by creating a deadline in the past
        let expired =
            super::super::Deadline::at(chrono::Utc::now() - chrono::Duration::milliseconds(100));
        assert!(expired.is_exceeded());
    }

    #[test]
    fn test_deadline_from_policy_not_expired_before_timeout() {
        let policy = TimeoutPolicy::new(5000).unwrap();
        let timeout_ms = policy.get_timeout_ms().expect("should have timeout");
        let deadline = super::super::Deadline::from_now(timeout_ms);

        assert!(!deadline.is_exceeded());
        assert!(deadline.remaining_ms() > 4000);
    }

    #[test]
    fn test_none_policy_means_no_deadline() {
        let policy = TimeoutPolicy::none();
        // None policy → no timeout → no deadline to enforce
        assert!(policy.get_timeout_ms().is_none());
        // Caller would skip deadline creation entirely
    }

    // ── Expiration via PhaseTimeout ───────────────────────────────────

    #[test]
    fn test_phase_timeout_from_policy_value_expires() {
        let policy = TimeoutPolicy::new(50).unwrap();
        let timeout_ms = policy.get_timeout_ms().expect("should have timeout");
        let phase = super::super::PhaseTimeout::new(timeout_ms).expect("should create");

        let started_past = chrono::Utc::now() - chrono::Duration::milliseconds(100);
        assert!(phase.is_expired(started_past));
    }

    #[test]
    fn test_phase_timeout_from_policy_value_not_expired() {
        let policy = TimeoutPolicy::new(5000).unwrap();
        let timeout_ms = policy.get_timeout_ms().expect("should have timeout");
        let phase = super::super::PhaseTimeout::new(timeout_ms).expect("should create");

        let started_now = chrono::Utc::now();
        assert!(!phase.is_expired(started_now));
    }

    #[test]
    fn test_phase_timeout_boundary_expiration() {
        let policy = TimeoutPolicy::new(10).unwrap();
        let timeout_ms = policy.get_timeout_ms().expect("should have timeout");
        let phase = super::super::PhaseTimeout::new(timeout_ms).expect("should create");

        // Exactly at boundary (started 10ms ago, timeout is 10ms) → expired (>=)
        let started_at_boundary =
            chrono::Utc::now() - chrono::Duration::milliseconds(timeout_ms as i64);
        assert!(phase.is_expired(started_at_boundary));
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn test_zero_timeout_rejected_consistently() {
        // Multiple attempts to confirm consistent rejection
        for _ in 0..10 {
            assert!(TimeoutPolicy::new(0).is_err());
        }
    }

    #[test]
    fn test_one_ms_timeout_is_valid() {
        let policy = TimeoutPolicy::new(1).unwrap();
        assert_eq!(policy.get_timeout_ms(), Some(1));
    }

    #[test]
    fn test_infinite_timeout_means_no_deadline() {
        let policy = TimeoutPolicy::none();
        assert!(policy.is_none());
        assert!(policy.get_timeout_ms().is_none());
        // With no timeout, operations never expire
    }

    // ── Debug representation ──────────────────────────────────────────

    #[test]
    fn test_debug_format_finite() {
        let policy = TimeoutPolicy::new(999).unwrap();
        let debug = format!("{policy:?}");
        assert!(debug.contains("TimeoutPolicy"));
    }

    #[test]
    fn test_debug_format_none() {
        let policy = TimeoutPolicy::none();
        let debug = format!("{policy:?}");
        assert!(debug.contains("TimeoutPolicy"));
    }

    // ── Proptests ─────────────────────────────────────────────────────

    use proptest::{prelude::*, prop_assert};

    proptest! {
        #[test]
        fn prop_new_rejects_zero(timeout_ms in 0u64..=0) {
            prop_assert!(TimeoutPolicy::new(timeout_ms).is_err());
        }

        #[test]
        fn prop_new_accepts_positive(timeout_ms in 1u64..u64::MAX) {
            let policy = TimeoutPolicy::new(timeout_ms);
            prop_assert!(policy.is_ok());
            prop_assert_eq!(policy.unwrap().get_timeout_ms(), Some(timeout_ms));
        }

        #[test]
        fn prop_none_always_is_none(_ in 0u64..10u64) {
            let policy = TimeoutPolicy::none();
            prop_assert!(policy.is_none());
            prop_assert!(policy.get_timeout_ms().is_none());
        }

        #[test]
        fn prop_equality_reflexive(timeout_ms in 1u64..u64::MAX) {
            let a = TimeoutPolicy::new(timeout_ms).unwrap();
            prop_assert_eq!(a, a);
        }

        #[test]
        fn prop_equality_symmetric(a_ms in 1u64..u64::MAX, b_ms in 1u64..u64::MAX) {
            let a = TimeoutPolicy::new(a_ms).unwrap();
            let b = TimeoutPolicy::new(b_ms).unwrap();
            if a_ms == b_ms {
                prop_assert_eq!(a, b);
            } else {
                prop_assert_ne!(a, b);
            }
        }

        #[test]
        fn prop_copy_preserves_value(timeout_ms in 1u64..10000u64) {
            let original = TimeoutPolicy::new(timeout_ms).unwrap();
            let copy = original;
            prop_assert_eq!(original, copy);
            prop_assert_eq!(original.get_timeout_ms(), copy.get_timeout_ms());
        }
    }
}

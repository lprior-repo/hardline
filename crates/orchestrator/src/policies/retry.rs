//! Policy: Retry configuration with exponential backoff

use serde::{Deserialize, Serialize};

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
    ) -> Result<Self, super::ConfigError> {
        if base_delay_ms == 0 {
            return Err(super::ConfigError::InvalidBaseDelay {
                delay_ms: base_delay_ms,
            });
        }
        if max_delay_ms < base_delay_ms {
            return Err(super::ConfigError::InvalidMaxDelay {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_retry_policy_total_attempts() {
        let policy = RetryPolicy::new(3, 100, 1000).expect("should create policy");
        assert_eq!(policy.total_attempts(), 4); // initial + 3 retries
    }

    #[test]
    fn test_retry_policy_zero_retries() {
        let policy = RetryPolicy::new(0, 100, 1000).expect("should create policy");
        assert_eq!(policy.total_attempts(), 1);
        assert_eq!(policy.max_retries(), 0);
    }

    #[test]
    fn test_retry_policy_equal_base_and_max_delay() {
        let policy = RetryPolicy::new(3, 100, 100).expect("should create policy");
        // All delays should be capped at 100
        assert_eq!(policy.calculate_delay(0), 100);
        assert_eq!(policy.calculate_delay(10), 100);
    }

    #[test]
    fn test_retry_policy_calculate_delay_zero_attempt() {
        let policy = RetryPolicy::new(5, 200, 5000).expect("should create policy");
        // 200 * 2^0 = 200
        assert_eq!(policy.calculate_delay(0), 200);
    }

    // --- Serde roundtrip ---

    #[test]
    fn test_retry_policy_serde_roundtrip() {
        let policy = RetryPolicy::new(5, 200, 5000).expect("should create");
        let json = serde_json::to_string(&policy).expect("serialize");
        let deserialized: RetryPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy.max_retries(), deserialized.max_retries());
    }

    #[test]
    fn test_retry_policy_calculate_delay_with_large_attempt() {
        let policy = RetryPolicy::new(100, 100, 10000).expect("should create");
        // Should not overflow, should be capped at max
        let delay = policy.calculate_delay(100);
        assert!(delay <= 10000);
    }

    #[test]
    fn test_retry_policy_base_delay_equals_max() {
        let policy = RetryPolicy::new(5, 500, 500).expect("should create");
        assert_eq!(policy.calculate_delay(0), 500);
        assert_eq!(policy.calculate_delay(1), 500); // 500 * 2 = 1000, capped at 500
    }

    #[test]
    fn test_retry_policy_new_equal_base_and_max() {
        let policy = RetryPolicy::new(3, 500, 500).expect("should create");
        // base == max is allowed
        assert_eq!(policy.max_retries(), 3);
    }

    #[test]
    fn test_retry_policy_display() {
        let err = crate::policies::ConfigError::InvalidBaseDelay { delay_ms: 0 };
        let msg = format!("{err}");
        assert!(msg.contains("Base delay"));
    }

    #[test]
    fn test_retry_policy_calculate_delay_saturation() {
        let policy = RetryPolicy::new(5, u64::MAX, u64::MAX).expect("should create");
        // saturating_mul prevents overflow
        let delay = policy.calculate_delay(1); // u64::MAX * 2 saturates
        assert_eq!(delay, u64::MAX);
    }

    // ── Exhaustive max retries tests ──────────────────────────────────────

    #[test]
    fn test_max_retries_zero_means_single_attempt() {
        let policy = RetryPolicy::new(0, 100, 1000).expect("ok");
        assert_eq!(policy.max_retries(), 0);
        assert_eq!(policy.total_attempts(), 1);
    }

    #[test]
    fn test_max_retries_one() {
        let policy = RetryPolicy::new(1, 100, 1000).expect("ok");
        assert_eq!(policy.max_retries(), 1);
        assert_eq!(policy.total_attempts(), 2);
    }

    #[test]
    fn test_max_retries_large_value() {
        let policy = RetryPolicy::new(10_000, 100, 1000).expect("ok");
        assert_eq!(policy.max_retries(), 10_000);
        assert_eq!(policy.total_attempts(), 10_001);
    }

    #[test]
    fn test_max_retries_max_u32() {
        // u32::MAX + 1 would overflow for total_attempts, but max_retries is still valid
        let policy = RetryPolicy::new(u32::MAX, 100, 1000).expect("ok");
        assert_eq!(policy.max_retries(), u32::MAX);
        // total_attempts wraps due to u32::MAX + 1 = 0 (wrapping_add)
        // This is a known edge case: the struct doesn't guard against it
    }

    #[test]
    fn test_max_retries_independent_of_delay_config() {
        let p1 = RetryPolicy::new(5, 10, 100).expect("ok");
        let p2 = RetryPolicy::new(5, 999, 9999).expect("ok");
        assert_eq!(p1.max_retries(), p2.max_retries());
        assert_eq!(p1.total_attempts(), p2.total_attempts());
    }

    // ── Exhaustive backoff calculation tests ──────────────────────────────

    #[test]
    fn test_backoff_formula_exact_values() {
        // Formula: min(base_delay_ms * 2^attempt, max_delay_ms)
        let policy = RetryPolicy::new(10, 100, 10_000).expect("ok");
        assert_eq!(policy.calculate_delay(0), 100);    // 100 * 2^0 = 100
        assert_eq!(policy.calculate_delay(1), 200);    // 100 * 2^1 = 200
        assert_eq!(policy.calculate_delay(2), 400);    // 100 * 2^2 = 400
        assert_eq!(policy.calculate_delay(3), 800);    // 100 * 2^3 = 800
        assert_eq!(policy.calculate_delay(4), 1600);   // 100 * 2^4 = 1600
        assert_eq!(policy.calculate_delay(5), 3200);   // 100 * 2^5 = 3200
        assert_eq!(policy.calculate_delay(6), 6400);   // 100 * 2^6 = 6400
    }

    #[test]
    fn test_backoff_cap_activates_at_exact_boundary() {
        // base=100, max=800: 100*2^3=800 (exact), 100*2^4=1600>capped
        let policy = RetryPolicy::new(10, 100, 800).expect("ok");
        assert_eq!(policy.calculate_delay(2), 400);
        assert_eq!(policy.calculate_delay(3), 800); // exact boundary
        assert_eq!(policy.calculate_delay(4), 800); // capped
        assert_eq!(policy.calculate_delay(100), 800); // still capped
    }

    #[test]
    fn test_backoff_monotonically_increasing() {
        let policy = RetryPolicy::new(50, 1, 1_000_000).expect("ok");
        let mut prev = 0u64;
        for attempt in 0..20 {
            let delay = policy.calculate_delay(attempt);
            assert!(delay >= prev, "Non-monotonic at attempt {attempt}: {delay} < {prev}");
            prev = delay;
        }
    }

    #[test]
    fn test_backoff_monotonically_non_decreasing_with_cap() {
        let policy = RetryPolicy::new(50, 50, 300).expect("ok");
        let mut prev = 0u64;
        for attempt in 0..30 {
            let delay = policy.calculate_delay(attempt);
            assert!(delay >= prev, "Non-monotonic at attempt {attempt}: {delay} < {prev}");
            assert!(delay <= 300, "Exceeded max at attempt {attempt}: {delay} > 300");
            prev = delay;
        }
    }

    #[test]
    fn test_backoff_attempt_zero_always_equals_base() {
        let cases = [(100, 1000), (1, 1), (500, 500), (999, 1000)];
        for (base, max) in cases {
            let policy = RetryPolicy::new(5, base, max).expect("ok");
            assert_eq!(policy.calculate_delay(0), base, "base={base}, max={max}");
        }
    }

    #[test]
    fn test_backoff_base_one_grows_as_powers_of_two() {
        let policy = RetryPolicy::new(30, 1, 1_000_000).expect("ok");
        assert_eq!(policy.calculate_delay(0), 1);      // 2^0
        assert_eq!(policy.calculate_delay(1), 2);      // 2^1
        assert_eq!(policy.calculate_delay(10), 1024);   // 2^10
        assert_eq!(policy.calculate_delay(19), 524_288); // 2^19
    }

    #[test]
    fn test_backoff_max_equals_base_means_constant_delay() {
        let policy = RetryPolicy::new(5, 200, 200).expect("ok");
        for attempt in 0..20 {
            assert_eq!(policy.calculate_delay(attempt), 200);
        }
    }

    #[test]
    fn test_backoff_saturating_no_panic_on_large_base() {
        let policy = RetryPolicy::new(5, u64::MAX / 2, u64::MAX).expect("ok");
        // Should not panic, saturates without overflow
        let delay = policy.calculate_delay(1);
        // u64::MAX / 2 * 2 = u64::MAX - 1 (saturating_mul gives exact result here)
        assert!(delay > 0);
        assert!(delay <= u64::MAX);
    }

    #[test]
    fn test_backoff_saturating_pow_on_large_attempt() {
        let policy = RetryPolicy::new(5, 100, u64::MAX).expect("ok");
        // 2^100 saturates, so 100 * saturated_value saturates
        let delay = policy.calculate_delay(100);
        assert!(delay > 0); // doesn't panic or return 0
    }

    #[test]
    fn test_backoff_delay_sequence_complete() {
        let policy = RetryPolicy::new(10, 50, 500).expect("ok");
        let delays: Vec<u64> = (0..10).map(|a| policy.calculate_delay(a)).collect();
        assert_eq!(delays[0], 50);
        assert_eq!(delays[1], 100);
        assert_eq!(delays[2], 200);
        assert_eq!(delays[3], 400);
        assert_eq!(delays[4], 500); // capped: 50*2^4=800 > 500
        assert!(delays[5..].iter().all(|&d| d == 500));
    }

    #[test]
    fn test_backoff_base_above_max_rejected() {
        let result = RetryPolicy::new(5, 1000, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_backoff_large_base_with_small_cap() {
        // base > max would be rejected, so this tests base == max
        let policy = RetryPolicy::new(5, 10_000, 10_000).expect("ok");
        assert_eq!(policy.calculate_delay(0), 10_000);
        assert_eq!(policy.calculate_delay(1), 10_000); // 10_000 * 2 = 20_000 capped
    }

    // ── Exhaustive total_attempts tests ───────────────────────────────────

    #[test]
    fn test_total_attempts_one_for_zero_retries() {
        let policy = RetryPolicy::new(0, 100, 1000).expect("ok");
        assert_eq!(policy.total_attempts(), 1);
    }

    #[test]
    fn test_total_attempts_two_for_one_retry() {
        let policy = RetryPolicy::new(1, 100, 1000).expect("ok");
        assert_eq!(policy.total_attempts(), 2);
    }

    #[test]
    fn test_total_attempts_equals_max_retries_plus_one() {
        for retries in [0, 1, 3, 5, 10, 50, 100] {
            let policy = RetryPolicy::new(retries, 100, 1000).expect("ok");
            assert_eq!(policy.total_attempts(), retries + 1);
        }
    }

    // ── Exhaustive validation tests ───────────────────────────────────────

    #[test]
    fn test_validation_rejects_zero_base_delay() {
        let err = RetryPolicy::new(3, 0, 1000).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Base delay"));
    }

    #[test]
    fn test_validation_rejects_max_less_than_base() {
        let err = RetryPolicy::new(3, 200, 100).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("50") || msg.contains("Max delay"));
    }

    #[test]
    fn test_validation_accepts_equal_base_and_max() {
        assert!(RetryPolicy::new(3, 500, 500).is_ok());
    }

    #[test]
    fn test_validation_accepts_one_ms_base_delay() {
        assert!(RetryPolicy::new(3, 1, 1).is_ok());
    }

    #[test]
    fn test_validation_error_display_messages() {
        let err1 = RetryPolicy::new(3, 0, 1000).unwrap_err();
        assert!(format!("{err1}").contains("Base delay"));

        let err2 = RetryPolicy::new(3, 200, 100).unwrap_err();
        let msg = format!("{err2}");
        assert!(msg.contains("Max delay") || msg.contains("base delay"));
    }

    // ── Trait derivation tests ────────────────────────────────────────────

    #[test]
    fn test_copy_semantics() {
        let p1 = RetryPolicy::new(3, 100, 1000).expect("ok");
        let p2 = p1; // Copy, not move
        assert_eq!(p1.max_retries(), p2.max_retries());
        assert_eq!(p1.calculate_delay(0), p2.calculate_delay(0));
    }

    #[test]
    fn test_clone_produces_equal_policy() {
        let p1 = RetryPolicy::new(3, 100, 1000).expect("ok");
        let p2 = p1.clone();
        assert_eq!(p1.max_retries(), p2.max_retries());
        assert_eq!(p1.calculate_delay(5), p2.calculate_delay(5));
    }

    #[test]
    fn test_debug_format() {
        let policy = RetryPolicy::new(3, 100, 1000).expect("ok");
        let debug = format!("{policy:?}");
        assert!(debug.contains("RetryPolicy"));
    }

    #[test]
    fn test_serde_roundtrip_preserves_all_fields() {
        let policy = RetryPolicy::new(7, 250, 5000).expect("ok");
        let json = serde_json::to_string(&policy).expect("serialize");
        let de: RetryPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy.max_retries(), de.max_retries());
        // Verify backoff behavior is preserved
        for attempt in 0..10 {
            assert_eq!(
                policy.calculate_delay(attempt),
                de.calculate_delay(attempt),
                "Mismatch at attempt {attempt}"
            );
        }
    }

    #[test]
    fn test_serde_roundtrip_edge_values() {
        let policy = RetryPolicy::new(0, 1, 1).expect("ok");
        let json = serde_json::to_string(&policy).expect("serialize");
        let de: RetryPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy.max_retries(), de.max_retries());
        assert_eq!(policy.calculate_delay(0), de.calculate_delay(0));
    }
}

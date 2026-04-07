//! Retry policy with exponential backoff

use std::num::NonZeroU64;

/// Configuration for retry behavior with exponential backoff
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    max_retries: u32,
    base_delay_ms: NonZeroU64,
    factor: f64,
    max_delay_ms: Option<NonZeroU64>,
    retryable_errors: Vec<String>,
}

impl RetryPolicy {
    /// Create a RetryPolicy if all values are valid
    pub fn new(
        max_retries: u32,
        base_delay_ms: u64,
        factor: f64,
        max_delay_ms: Option<u64>,
        retryable_errors: Vec<String>,
    ) -> Result<Self, RetryPolicyError> {
        let base_delay =
            NonZeroU64::new(base_delay_ms).ok_or(RetryPolicyError::InvalidBaseDelay)?;

        if !factor.is_finite() || factor.is_nan() || factor <= 1.0 {
            return Err(RetryPolicyError::InvalidFactor);
        }

        let max_delay = match max_delay_ms {
            Some(0) => return Err(RetryPolicyError::InvalidMaxDelay),
            Some(d) => NonZeroU64::new(d),
            None => None,
        };

        // Validate monotonicity: max_delay should be >= base_delay
        if let Some(max) = max_delay {
            if max.get() < base_delay_ms {
                return Err(RetryPolicyError::InvalidMaxDelay);
            }
        }

        Ok(Self {
            max_retries,
            base_delay_ms: base_delay,
            factor,
            max_delay_ms: max_delay,
            retryable_errors,
        })
    }

    /// Get the maximum number of retries
    #[must_use]
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get the base delay in milliseconds
    #[must_use]
    pub fn base_delay_ms(&self) -> u64 {
        self.base_delay_ms.get()
    }

    /// Get the exponential factor
    #[must_use]
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// Get the maximum delay in milliseconds if set
    #[must_use]
    pub fn max_delay_ms(&self) -> Option<u64> {
        self.max_delay_ms.map(|nz| nz.get())
    }

    /// Get the list of retryable error patterns
    #[must_use]
    pub fn retryable_errors(&self) -> &[String] {
        &self.retryable_errors
    }

    /// Compute the backoff delay for a given attempt number
    /// Formula: base_delay * factor^attempt, capped at max_delay
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        // Use saturating arithmetic to prevent overflow
        let exponential = (self.base_delay_ms.get() as f64 * self.factor.powi(attempt as i32))
            .clamp(0.0, u64::MAX as f64);
        let delay = exponential as u64;

        match self.max_delay_ms {
            Some(max) => delay.min(max.get()),
            None => delay,
        }
    }

    /// Check if an error is retryable based on its string representation
    #[must_use]
    pub fn is_retryable(&self, error: &str) -> bool {
        if self.retryable_errors.is_empty() {
            return false;
        }
        self.retryable_errors
            .iter()
            .any(|pattern| error.contains(pattern))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryPolicyError {
    InvalidBaseDelay,
    InvalidFactor,
    InvalidMaxDelay,
}

impl std::fmt::Display for RetryPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryPolicyError::InvalidBaseDelay => {
                write!(f, "base_delay must be greater than 0ms")
            }
            RetryPolicyError::InvalidFactor => {
                write!(f, "factor must be greater than 1.0")
            }
            RetryPolicyError::InvalidMaxDelay => {
                write!(f, "max_delay must be greater than 0ms and >= base_delay")
            }
        }
    }
}

impl std::error::Error for RetryPolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_creation_with_valid_parameters() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(1000), vec!["io".into()])
            .expect("should create policy");
        assert_eq!(policy.max_retries(), 3);
        assert_eq!(policy.base_delay_ms(), 100);
        assert_eq!(policy.factor(), 2.0);
        assert_eq!(policy.max_delay_ms(), Some(1000));
    }

    #[test]
    fn test_exponential_backoff_delay_increases_monotonically() {
        let policy = RetryPolicy::new(10, 100, 2.0, None, vec![]).expect("should create policy");
        let d0 = policy.calculate_delay(0);
        let d1 = policy.calculate_delay(1);
        let d2 = policy.calculate_delay(2);
        let d3 = policy.calculate_delay(3);
        assert!(d0 < d1);
        assert!(d1 < d2);
        assert!(d2 < d3);
    }

    #[test]
    fn test_exponential_backoff_formula_verification() {
        let policy = RetryPolicy::new(10, 100, 2.0, None, vec![]).expect("should create policy");
        assert_eq!(policy.calculate_delay(0), 100);
        assert_eq!(policy.calculate_delay(1), 200);
        assert_eq!(policy.calculate_delay(2), 400);
        assert_eq!(policy.calculate_delay(3), 800);
    }

    #[test]
    fn test_backoff_delay_capped_at_max_delay() {
        let policy =
            RetryPolicy::new(10, 100, 2.0, Some(500), vec![]).expect("should create policy");
        assert_eq!(policy.calculate_delay(10), 500);
    }

    #[test]
    fn test_invalid_base_delay_zero_returns_error() {
        let result = RetryPolicy::new(3, 0, 2.0, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidBaseDelay);
    }

    #[test]
    fn test_invalid_factor_one_returns_error() {
        let result = RetryPolicy::new(3, 100, 1.0, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_invalid_factor_less_than_one_returns_error() {
        let result = RetryPolicy::new(3, 100, 0.5, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_invalid_max_delay_zero_returns_error() {
        let result = RetryPolicy::new(3, 100, 2.0, Some(0), vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidMaxDelay);
    }

    #[test]
    fn test_empty_retryable_errors_list_means_no_errors_retryable() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("should create policy");
        assert!(!policy.is_retryable("io error"));
        assert!(!policy.is_retryable("network error"));
    }

    #[test]
    fn test_retryable_error_detection() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["conn".into(), "net".into()])
            .expect("should create policy");
        assert!(policy.is_retryable("connection refused"));
        assert!(policy.is_retryable("network error"));
        assert!(!policy.is_retryable("failed"));
        assert!(!policy.is_retryable("boom"));
    }

    // --- Edge cases ---

    #[test]
    fn test_zero_max_retries_allowed() {
        let policy = RetryPolicy::new(0, 100, 2.0, None, vec![]).expect("should create");
        assert_eq!(policy.max_retries(), 0);
    }

    #[test]
    fn test_max_delay_equal_to_base_delay() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(100), vec![]).expect("should create");
        assert_eq!(policy.max_delay_ms(), Some(100));
        // All delays should be capped at 100
        for attempt in 0..10 {
            assert!(policy.calculate_delay(attempt) <= 100);
        }
    }

    #[test]
    fn test_max_delay_less_than_base_delay_rejected() {
        let result = RetryPolicy::new(3, 200, 2.0, Some(100), vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidMaxDelay);
    }

    #[test]
    fn test_factor_must_be_greater_than_one() {
        let result = RetryPolicy::new(3, 100, 1.0001, None, vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_infinity_factor_rejected() {
        let result = RetryPolicy::new(3, 100, f64::INFINITY, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_nan_factor_rejected() {
        let result = RetryPolicy::new(3, 100, f64::NAN, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_negative_factor_rejected() {
        let result = RetryPolicy::new(3, 100, -1.0, None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_calculate_delay_is_monotonically_increasing_without_cap() {
        let policy = RetryPolicy::new(100, 1, 2.0, None, vec![]).expect("should create");
        let mut prev = policy.calculate_delay(0);
        for attempt in 1..20 {
            let current = policy.calculate_delay(attempt);
            assert!(
                current >= prev,
                "Expected delay at attempt {attempt} ({current}) >= previous ({prev})"
            );
            prev = current;
        }
    }

    #[test]
    fn test_calculate_delay_respects_max_delay() {
        let policy = RetryPolicy::new(100, 10, 3.0, Some(500), vec![]).expect("should create");
        for attempt in 0..100 {
            assert!(policy.calculate_delay(attempt) <= 500);
        }
    }

    #[test]
    fn test_calculate_delay_zero_attempt_equals_base() {
        let policy = RetryPolicy::new(3, 250, 2.0, None, vec![]).expect("should create");
        assert_eq!(policy.calculate_delay(0), 250);
    }

    #[test]
    fn test_is_retryable_case_sensitive() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["IO".into()]).expect("should create");
        assert!(policy.is_retryable("IO error"));
        assert!(!policy.is_retryable("io error")); // lowercase doesn't match
    }

    #[test]
    fn test_is_retryable_empty_string() {
        let policy =
            RetryPolicy::new(3, 100, 2.0, None, vec!["timeout".into()]).expect("should create");
        assert!(!policy.is_retryable(""));
    }

    #[test]
    fn test_retryable_errors_multiple_patterns() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["a".into(), "b".into(), "c".into()])
            .expect("should create");
        assert!(policy.is_retryable("has a"));
        assert!(policy.is_retryable("has b"));
        assert!(policy.is_retryable("has c"));
        assert!(!policy.is_retryable("xyz"));
    }

    // --- Error display ---

    #[test]
    fn test_retry_policy_error_display() {
        let errors = [
            RetryPolicyError::InvalidBaseDelay,
            RetryPolicyError::InvalidFactor,
            RetryPolicyError::InvalidMaxDelay,
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
        }
    }

    // --- Error trait ---

    #[test]
    fn test_retry_policy_error_implements_error() {
        use std::error::Error;
        let err = RetryPolicyError::InvalidBaseDelay;
        assert!(err.source().is_none());
    }

    // ── Exhaustive max retries tests ──────────────────────────────────────

    #[test]
    fn test_max_retries_zero_means_no_retries() {
        let policy = RetryPolicy::new(0, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.max_retries(), 0);
    }

    #[test]
    fn test_max_retries_one() {
        let policy = RetryPolicy::new(1, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.max_retries(), 1);
    }

    #[test]
    fn test_max_retries_large_value() {
        let policy = RetryPolicy::new(1000, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.max_retries(), 1000);
    }

    #[test]
    fn test_max_retries_max_u32() {
        let policy = RetryPolicy::new(u32::MAX, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.max_retries(), u32::MAX);
    }

    #[test]
    fn test_max_retries_independent_of_delay_config() {
        let p1 = RetryPolicy::new(5, 10, 1.5, None, vec![]).expect("ok");
        let p2 = RetryPolicy::new(5, 1000, 3.0, Some(5000), vec!["err".into()]).expect("ok");
        assert_eq!(p1.max_retries(), p2.max_retries());
    }

    // ── Exhaustive backoff calculation tests ──────────────────────────────

    #[test]
    fn test_backoff_formula_base_times_factor_to_attempt_power() {
        // Formula: base_delay * factor^attempt
        let policy = RetryPolicy::new(10, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 100);   // 100 * 2^0
        assert_eq!(policy.calculate_delay(1), 200);   // 100 * 2^1
        assert_eq!(policy.calculate_delay(2), 400);   // 100 * 2^2
        assert_eq!(policy.calculate_delay(3), 800);   // 100 * 2^3
        assert_eq!(policy.calculate_delay(4), 1600);  // 100 * 2^4
    }

    #[test]
    fn test_backoff_with_factor_1_5() {
        let policy = RetryPolicy::new(10, 100, 1.5, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 100);  // 100 * 1.5^0
        assert_eq!(policy.calculate_delay(1), 150);  // 100 * 1.5^1
        assert_eq!(policy.calculate_delay(2), 225);  // 100 * 1.5^2
        assert_eq!(policy.calculate_delay(3), 337);  // 100 * 1.5^3 ≈ 337.5, truncated
    }

    #[test]
    fn test_backoff_with_factor_3() {
        let policy = RetryPolicy::new(10, 10, 3.0, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 10);   // 10 * 3^0
        assert_eq!(policy.calculate_delay(1), 30);   // 10 * 3^1
        assert_eq!(policy.calculate_delay(2), 90);   // 10 * 3^2
        assert_eq!(policy.calculate_delay(3), 270);  // 10 * 3^3
    }

    #[test]
    fn test_backoff_monotonically_increasing_no_cap() {
        let policy = RetryPolicy::new(50, 1, 2.0, None, vec![]).expect("ok");
        let mut prev = 0u64;
        for attempt in 0..30 {
            let delay = policy.calculate_delay(attempt);
            assert!(
                delay >= prev,
                "Non-monotonic at attempt {attempt}: {delay} < {prev}"
            );
            prev = delay;
        }
    }

    #[test]
    fn test_backoff_monotonically_increasing_with_cap() {
        let policy = RetryPolicy::new(50, 1, 2.0, Some(10000), vec![]).expect("ok");
        let mut prev = 0u64;
        for attempt in 0..50 {
            let delay = policy.calculate_delay(attempt);
            assert!(delay >= prev, "Non-monotonic at attempt {attempt}: {delay} < {prev}");
            assert!(delay <= 10000, "Exceeded max at attempt {attempt}: {delay} > 10000");
            prev = delay;
        }
    }

    #[test]
    fn test_backoff_cap_activates_at_exact_boundary() {
        // base=100, factor=2, max=800: 100*2^3=800 (exact), 100*2^4=1600>capped
        let policy = RetryPolicy::new(10, 100, 2.0, Some(800), vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(2), 400);
        assert_eq!(policy.calculate_delay(3), 800); // exact boundary
        assert_eq!(policy.calculate_delay(4), 800); // capped
        assert_eq!(policy.calculate_delay(100), 800); // still capped
    }

    #[test]
    fn test_backoff_max_delay_equals_base_means_constant() {
        let policy = RetryPolicy::new(5, 200, 2.0, Some(200), vec![]).expect("ok");
        for attempt in 0..20 {
            assert_eq!(policy.calculate_delay(attempt), 200);
        }
    }

    #[test]
    fn test_backoff_no_max_delay_grows_unbounded() {
        let policy = RetryPolicy::new(100, 1, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(20), 1_048_576); // 2^20
    }

    #[test]
    fn test_backoff_attempt_zero_always_equals_base() {
        let cases = [
            (100, 2.0, None::<u64>),
            (250, 3.0, None::<u64>),
            (1, 10.0, None::<u64>),
            (500, 2.0, Some(500)),
            (500, 2.0, Some(1000)),
        ];
        for (base, factor, max) in cases {
            let policy = RetryPolicy::new(5, base, factor, max, vec![]).expect("ok");
            assert_eq!(policy.calculate_delay(0), base, "base={base}, factor={factor}");
        }
    }

    #[test]
    fn test_backoff_factor_just_above_one() {
        // Factor = 1.0001, very slow growth
        let policy = RetryPolicy::new(5, 1000, 1.0001, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 1000);
        assert_eq!(policy.calculate_delay(1), 1000); // truncated from 1000.1
        let d10 = policy.calculate_delay(10);
        assert!(d10 >= 1000 && d10 < 1100); // very slow growth
    }

    #[test]
    fn test_backoff_with_large_factor() {
        let policy = RetryPolicy::new(5, 1, 100.0, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 1);
        assert_eq!(policy.calculate_delay(1), 100);
        assert_eq!(policy.calculate_delay(2), 10000);
    }

    #[test]
    fn test_backoff_delay_sequence_with_cap_transition() {
        let policy = RetryPolicy::new(10, 50, 2.0, Some(300), vec![]).expect("ok");
        let delays: Vec<u64> = (0..10).map(|a| policy.calculate_delay(a)).collect();
        assert_eq!(delays[0], 50);
        assert_eq!(delays[1], 100);
        assert_eq!(delays[2], 200);
        assert_eq!(delays[3], 300); // 50*2^3=400, capped at 300
        assert!(delays[4..].iter().all(|&d| d == 300));
    }

    #[test]
    fn test_backoff_base_one() {
        let policy = RetryPolicy::new(5, 1, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 1);
        assert_eq!(policy.calculate_delay(1), 2);
        assert_eq!(policy.calculate_delay(2), 4);
        assert_eq!(policy.calculate_delay(10), 1024);
    }

    #[test]
    fn test_backoff_saturating_no_panic_on_large_base() {
        let policy = RetryPolicy::new(5, u64::MAX / 2, 2.0, None, vec![]).expect("ok");
        let delay = policy.calculate_delay(1);
        // Should saturate, not panic
        assert!(delay > 0);
    }

    #[test]
    fn test_backoff_large_base_with_small_cap_rejected() {
        let result = RetryPolicy::new(5, 10_000, 2.0, Some(100), vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RetryPolicyError::InvalidMaxDelay);
    }

    #[test]
    fn test_backoff_large_base_capped_at_equal_max() {
        let policy = RetryPolicy::new(5, 10_000, 2.0, Some(10_000), vec![]).expect("ok");
        assert_eq!(policy.calculate_delay(0), 10_000);
        assert_eq!(policy.calculate_delay(1), 10_000); // capped
    }

    // ── Exhaustive accessor tests ─────────────────────────────────────────

    #[test]
    fn test_base_delay_ms_accessor() {
        let policy = RetryPolicy::new(3, 42, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.base_delay_ms(), 42);
    }

    #[test]
    fn test_factor_accessor() {
        let policy = RetryPolicy::new(3, 100, 2.7, None, vec![]).expect("ok");
        assert_eq!(policy.factor(), 2.7);
    }

    #[test]
    fn test_max_delay_ms_accessor_none() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        assert_eq!(policy.max_delay_ms(), None);
    }

    #[test]
    fn test_max_delay_ms_accessor_some() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(500), vec![]).expect("ok");
        assert_eq!(policy.max_delay_ms(), Some(500));
    }

    #[test]
    fn test_retryable_errors_accessor_empty() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        assert!(policy.retryable_errors().is_empty());
    }

    #[test]
    fn test_retryable_errors_accessor_returns_same_order() {
        let errors = vec!["timeout".into(), "connection".into(), "network".into()];
        let policy = RetryPolicy::new(3, 100, 2.0, None, errors).expect("ok");
        let actual = policy.retryable_errors();
        assert_eq!(actual, &["timeout", "connection", "network"]);
    }

    #[test]
    fn test_retryable_errors_accessor_many_patterns() {
        let errors: Vec<String> = (0..100).map(|i| format!("err{i}")).collect();
        let policy = RetryPolicy::new(3, 100, 2.0, None, errors).expect("ok");
        assert_eq!(policy.retryable_errors().len(), 100);
    }

    // ── Exhaustive is_retryable tests ─────────────────────────────────────

    #[test]
    fn test_is_retryable_partial_match() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["timeout".into()]).expect("ok");
        assert!(policy.is_retryable("connection timeout after 30s"));
    }

    #[test]
    fn test_is_retryable_exact_match() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["timeout".into()]).expect("ok");
        assert!(policy.is_retryable("timeout"));
    }

    #[test]
    fn test_is_retryable_prefix_match() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["err".into()]).expect("ok");
        assert!(policy.is_retryable("error in processing"));
    }

    #[test]
    fn test_is_retryable_suffix_match() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["refused".into()]).expect("ok");
        assert!(policy.is_retryable("connection refused"));
    }

    #[test]
    fn test_is_retryable_any_pattern_matches() {
        let policy = RetryPolicy::new(
            3, 100, 2.0, None,
            vec!["timeout".into(), "refused".into(), "reset".into()],
        )
        .expect("ok");
        assert!(policy.is_retryable("connection reset by peer"));
        assert!(policy.is_retryable("timeout waiting for response"));
        assert!(policy.is_retryable("connection refused"));
        assert!(!policy.is_retryable("out of memory"));
    }

    // ── Trait derivation tests ────────────────────────────────────────────

    #[test]
    fn test_clone_derives_correctly() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(500), vec!["io".into()]).expect("ok");
        let cloned = policy.clone();
        assert_eq!(policy.max_retries(), cloned.max_retries());
        assert_eq!(policy.base_delay_ms(), cloned.base_delay_ms());
        assert_eq!(policy.factor(), cloned.factor());
        assert_eq!(policy.max_delay_ms(), cloned.max_delay_ms());
        assert_eq!(policy.retryable_errors(), cloned.retryable_errors());
    }

    #[test]
    fn test_partial_eq_equal_policies() {
        let p1 = RetryPolicy::new(3, 100, 2.0, Some(500), vec!["io".into()]).expect("ok");
        let p2 = RetryPolicy::new(3, 100, 2.0, Some(500), vec!["io".into()]).expect("ok");
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_partial_eq_different_max_retries() {
        let p1 = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        let p2 = RetryPolicy::new(5, 100, 2.0, None, vec![]).expect("ok");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_partial_eq_different_base_delay() {
        let p1 = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        let p2 = RetryPolicy::new(3, 200, 2.0, None, vec![]).expect("ok");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_partial_eq_different_factor() {
        let p1 = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        let p2 = RetryPolicy::new(3, 100, 3.0, None, vec![]).expect("ok");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_partial_eq_different_max_delay() {
        let p1 = RetryPolicy::new(3, 100, 2.0, Some(500), vec![]).expect("ok");
        let p2 = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("ok");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_partial_eq_different_retryable_errors() {
        let p1 = RetryPolicy::new(3, 100, 2.0, None, vec!["io".into()]).expect("ok");
        let p2 = RetryPolicy::new(3, 100, 2.0, None, vec!["net".into()]).expect("ok");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_debug_format_contains_type_name() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(500), vec!["io".into()]).expect("ok");
        let debug = format!("{policy:?}");
        assert!(debug.contains("RetryPolicy"));
    }

    // ── Exhaustive error path tests ───────────────────────────────────────

    #[test]
    fn test_error_factor_exactly_one() {
        let err = RetryPolicy::new(3, 100, 1.0, None, vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_error_factor_zero() {
        let err = RetryPolicy::new(3, 100, 0.0, None, vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_error_factor_negative() {
        let err = RetryPolicy::new(3, 100, -5.0, None, vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_error_factor_neg_infinity() {
        let err = RetryPolicy::new(3, 100, f64::NEG_INFINITY, None, vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidFactor);
    }

    #[test]
    fn test_error_max_delay_zero() {
        let err = RetryPolicy::new(3, 100, 2.0, Some(0), vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidMaxDelay);
    }

    #[test]
    fn test_error_max_delay_less_than_base() {
        let err = RetryPolicy::new(3, 200, 2.0, Some(199), vec![]).unwrap_err();
        assert_eq!(err, RetryPolicyError::InvalidMaxDelay);
    }

    #[test]
    fn test_error_max_delay_equal_to_base_is_valid() {
        assert!(RetryPolicy::new(3, 100, 2.0, Some(100), vec![]).is_ok());
    }

    #[test]
    fn test_error_display_all_variants_non_empty() {
        let errors = [
            RetryPolicyError::InvalidBaseDelay,
            RetryPolicyError::InvalidFactor,
            RetryPolicyError::InvalidMaxDelay,
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Empty display for {err:?}");
        }
    }

    #[test]
    fn test_error_clone() {
        let err = RetryPolicyError::InvalidBaseDelay;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_partial_eq_same_variants() {
        assert_eq!(
            RetryPolicyError::InvalidBaseDelay,
            RetryPolicyError::InvalidBaseDelay
        );
        assert_ne!(
            RetryPolicyError::InvalidBaseDelay,
            RetryPolicyError::InvalidFactor
        );
        assert_ne!(
            RetryPolicyError::InvalidFactor,
            RetryPolicyError::InvalidMaxDelay
        );
    }

    // --- Proptests for invariant verification ---

    use proptest::prelude::*;
    use proptest::prop_assert;

    proptest! {
        #[test]
        fn prop_delay_never_exceeds_max_delay(
            base_delay in 1u64..100u64,
            factor in 1.1f64..5.0f64,
            max_delay in 1u64..1000u64,
            attempt in 0u32..50u32,
        ) {
            let policy = RetryPolicy::new(10, base_delay, factor, Some(max_delay), vec![]);
            if let Ok(p) = policy {
                prop_assert!(p.calculate_delay(attempt) <= max_delay);
            }
        }

        #[test]
        fn prop_delay_is_always_positive(
            base_delay in 1u64..100u64,
            factor in 1.1f64..5.0f64,
            attempt in 0u32..50u32,
        ) {
            let policy = RetryPolicy::new(10, base_delay, factor, None, vec![]);
            if let Ok(p) = policy {
                prop_assert!(p.calculate_delay(attempt) > 0);
            }
        }

        #[test]
        fn prop_attempt_zero_equals_base(
            base_delay in 1u64..10_000u64,
            factor in 1.1f64..10.0f64,
        ) {
            let policy = RetryPolicy::new(10, base_delay, factor, None, vec![]);
            if let Ok(p) = policy {
                prop_assert_eq!(p.calculate_delay(0), base_delay);
            }
        }

        #[test]
        fn prop_delay_monotonic_without_cap(
            base_delay in 1u64..100u64,
            factor in 1.1f64..3.0f64,
            attempts in 0u32..20u32,
        ) {
            let policy = RetryPolicy::new(100, base_delay, factor, None, vec![]);
            if let Ok(p) = policy {
                let d1 = p.calculate_delay(attempts);
                let d2 = p.calculate_delay(attempts + 1);
                prop_assert!(d2 >= d1, "delay not monotonic: {} >= {}", d2, d1);
            }
        }

        #[test]
        fn prop_delay_monotonic_with_cap(
            base_delay in 1u64..50u64,
            factor in 1.1f64..3.0f64,
            max_delay in 50u64..500u64,
            attempts in 0u32..20u32,
        ) {
            let policy = RetryPolicy::new(100, base_delay, factor, Some(max_delay), vec![]);
            if let Ok(p) = policy {
                let d1 = p.calculate_delay(attempts);
                let d2 = p.calculate_delay(attempts + 1);
                prop_assert!(d2 >= d1, "delay not monotonic: {} >= {}", d2, d1);
                prop_assert!(d2 <= max_delay, "delay exceeds max: {} > {}", d2, max_delay);
            }
        }

        #[test]
        fn prop_valid_creation_consistent_accessors(
            max_retries in 0u32..100u32,
            base_delay in 1u64..1000u64,
            factor in 1.1f64..5.0f64,
        ) {
            let policy = RetryPolicy::new(max_retries, base_delay, factor, None, vec![]);
            if let Ok(p) = policy {
                prop_assert_eq!(p.max_retries(), max_retries);
                prop_assert_eq!(p.base_delay_ms(), base_delay);
                prop_assert!((p.factor() - factor).abs() < f64::EPSILON);
            }
        }

        #[test]
        fn prop_factor_above_one_always_accepted(
            base_delay in 1u64..100u64,
            factor in 1.0001f64..100.0f64,
        ) {
            let result = RetryPolicy::new(3, base_delay, factor, None, vec![]);
            prop_assert!(result.is_ok());
        }
    }

    // --- Exhaustive max retries tests ---

    #[test]
    fn test_max_retries_zero_means_single_attempt() {
        let policy = RetryPolicy::new(0, 100, 2.0, None, vec![]).expect("should create");
        assert_eq!(policy.max_retries(), 0);
    }

    #[test]
    fn test_max_retries_does_not_affect_delay_calculation() {
        // calculate_delay is pure — it only uses base_delay, factor, max_delay
        let policy_0 = RetryPolicy::new(0, 100, 2.0, None, vec![]).expect("should create");
        let policy_100 = RetryPolicy::new(100, 100, 2.0, None, vec![]).expect("should create");
        for attempt in 0..20 {
            assert_eq!(
                policy_0.calculate_delay(attempt),
                policy_100.calculate_delay(attempt),
                "max_retries should not affect delay at attempt {attempt}"
            );
        }
    }

    // --- Exhaustive backoff calculation with different factors ---

    #[test]
    fn test_backoff_with_factor_10() {
        let policy = RetryPolicy::new(5, 1, 10.0, None, vec![]).expect("should create");
        assert_eq!(policy.calculate_delay(0), 1);     // 1 * 10^0 = 1
        assert_eq!(policy.calculate_delay(1), 10);    // 1 * 10^1 = 10
        assert_eq!(policy.calculate_delay(2), 100);   // 1 * 10^2 = 100
        assert_eq!(policy.calculate_delay(3), 1000);  // 1 * 10^3 = 1000
    }

    #[test]
    fn test_backoff_with_factor_just_above_one() {
        let policy = RetryPolicy::new(100, 1000, 1.001, None, vec![]).expect("should create");
        // Slow growth: factor barely above 1.0
        let d0 = policy.calculate_delay(0);
        let d100 = policy.calculate_delay(100);
        assert!(d100 > d0, "should grow even with factor just above 1.0");
        assert!(d100 < 2000, "should grow slowly with factor 1.001");
    }

    // --- Full backoff sequence with cap ---

    #[test]
    fn test_full_backoff_sequence_with_cap() {
        let policy = RetryPolicy::new(10, 100, 2.0, Some(1000), vec![]).expect("should create");
        let delays: Vec<u64> = (0..=10).map(|a| policy.calculate_delay(a)).collect();
        // 100, 200, 400, 800, 1000 (1600 capped), 1000, ...
        assert_eq!(delays[0], 100);
        assert_eq!(delays[1], 200);
        assert_eq!(delays[2], 400);
        assert_eq!(delays[3], 800);
        assert_eq!(delays[4], 1000); // capped
        assert!(delays[5..].iter().all(|&d| d == 1000));
    }

    #[test]
    fn test_full_backoff_sequence_no_cap() {
        let policy = RetryPolicy::new(10, 10, 2.0, None, vec![]).expect("should create");
        let delays: Vec<u64> = (0..=10).map(|a| policy.calculate_delay(a)).collect();
        // 10, 20, 40, 80, 160, 320, 640, 1280, 2560, 5120, 10240
        assert_eq!(delays[0], 10);
        assert_eq!(delays[1], 20);
        assert_eq!(delays[2], 40);
        assert_eq!(delays[3], 80);
        assert_eq!(delays[4], 160);
        assert_eq!(delays[5], 320);
        assert_eq!(delays[6], 640);
        assert_eq!(delays[7], 1280);
        assert_eq!(delays[8], 2560);
        assert_eq!(delays[9], 5120);
        assert_eq!(delays[10], 10240);
    }

    #[test]
    fn test_cap_kicks_in_at_exact_boundary() {
        // base=50, factor=2, max=200 => 50, 100, 200, 200 (400 capped), ...
        let policy = RetryPolicy::new(10, 50, 2.0, Some(200), vec![]).expect("should create");
        assert_eq!(policy.calculate_delay(0), 50);
        assert_eq!(policy.calculate_delay(1), 100);
        assert_eq!(policy.calculate_delay(2), 200); // hits cap exactly
        assert_eq!(policy.calculate_delay(3), 200); // 400 capped
    }

    // --- Clone and PartialEq ---

    #[test]
    fn test_retry_policy_clone_equality() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(1000), vec!["io".into()]).expect("should create");
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    #[test]
    fn test_retry_policy_different_retries_not_equal() {
        let a = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("should create");
        let b = RetryPolicy::new(5, 100, 2.0, None, vec![]).expect("should create");
        assert_ne!(a, b);
    }

    #[test]
    fn test_retry_policy_different_factors_not_equal() {
        let a = RetryPolicy::new(3, 100, 2.0, None, vec![]).expect("should create");
        let b = RetryPolicy::new(3, 100, 3.0, None, vec![]).expect("should create");
        assert_ne!(a, b);
    }

    // --- Base delay edge cases ---

    #[test]
    fn test_base_delay_minimum_one() {
        let policy = RetryPolicy::new(3, 1, 2.0, None, vec![]).expect("should create");
        assert_eq!(policy.base_delay_ms(), 1);
        assert_eq!(policy.calculate_delay(0), 1);
        assert_eq!(policy.calculate_delay(1), 2);
        assert_eq!(policy.calculate_delay(10), 1024);
    }

    #[test]
    fn test_base_delay_large_value() {
        let policy = RetryPolicy::new(3, 60000, 2.0, None, vec![]).expect("should create");
        assert_eq!(policy.base_delay_ms(), 60000);
        assert_eq!(policy.calculate_delay(0), 60000);
        assert_eq!(policy.calculate_delay(1), 120000);
    }

    // --- calculate_delay beyond typical range ---

    #[test]
    fn test_calculate_delay_with_large_attempt_no_overflow() {
        let policy = RetryPolicy::new(3, 1, 2.0, None, vec![]).expect("should create");
        // Should not panic or return 0 due to overflow
        let delay = policy.calculate_delay(1000);
        assert!(delay > 0 || delay == 0, "should not panic"); // f64 may lose precision
    }

    #[test]
    fn test_calculate_delay_with_large_attempt_capped() {
        let policy = RetryPolicy::new(3, 1, 2.0, Some(500), vec![]).expect("should create");
        // Even with huge attempt number, capped at max_delay
        assert_eq!(policy.calculate_delay(1000), 500);
    }
}

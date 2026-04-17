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

    // --- Proptests for metrics accumulation properties ---

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
            // May fail if max_delay < base_delay, so skip those
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
    }
}

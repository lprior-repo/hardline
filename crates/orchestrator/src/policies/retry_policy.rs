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
}

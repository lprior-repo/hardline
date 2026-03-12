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
}

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
}

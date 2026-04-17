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
    pub fn none() -> Self {
        Self { timeout_ms: None }
    }

    /// Returns the effective timeout in milliseconds
    #[must_use]
    pub fn get_timeout_ms(&self) -> Option<u64> {
        self.timeout_ms.map(|nz| nz.get())
    }

    /// Returns true if no timeout is configured
    #[must_use]
    pub fn is_none(&self) -> bool {
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
            TimeoutPolicyError::ZeroTimeout => {
                write!(f, "timeout must be greater than 0ms")
            }
        }
    }
}

impl std::error::Error for TimeoutPolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_policy_creation_with_valid_value() {
        let policy = TimeoutPolicy::new(5000).expect("should create policy");
        assert_eq!(policy.get_timeout_ms(), Some(5000));
    }

    #[test]
    fn test_invalid_timeout_zero_returns_error() {
        let result = TimeoutPolicy::new(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TimeoutPolicyError::ZeroTimeout);
    }

    #[test]
    fn test_timeout_policy_none_means_no_timeout() {
        let policy = TimeoutPolicy::none();
        assert!(policy.is_none());
        assert_eq!(policy.get_timeout_ms(), None);
    }

    #[test]
    fn test_very_large_timeout_value_handled() {
        let policy = TimeoutPolicy::new(u64::MAX).expect("should create policy");
        assert_eq!(policy.get_timeout_ms(), Some(u64::MAX));
    }

    #[test]
    fn test_timeout_policy_equality() {
        let a = TimeoutPolicy::new(5000).expect("create");
        let b = TimeoutPolicy::new(5000).expect("create");
        assert_eq!(a, b);
    }

    #[test]
    fn test_timeout_policy_inequality() {
        let a = TimeoutPolicy::new(5000).expect("create");
        let b = TimeoutPolicy::new(10000).expect("create");
        assert_ne!(a, b);
    }

    #[test]
    fn test_timeout_policy_none_vs_value_inequality() {
        let none = TimeoutPolicy::none();
        let some = TimeoutPolicy::new(5000).expect("create");
        assert_ne!(none, some);
    }

    #[test]
    fn test_timeout_policy_error_display() {
        let err = TimeoutPolicyError::ZeroTimeout;
        let msg = format!("{err}");
        assert!(msg.contains("timeout must be greater than 0ms"));
    }

    #[test]
    fn test_timeout_policy_error_implements_error() {
        use std::error::Error;
        let err = TimeoutPolicyError::ZeroTimeout;
        assert!(err.source().is_none());
    }
}

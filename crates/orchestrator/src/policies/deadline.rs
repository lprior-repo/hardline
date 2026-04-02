//! Policy: Deadline configuration for global pipeline timeout

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Deadline configuration for global pipeline timeout
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Deadline {
    /// The absolute deadline timestamp
    deadline_at: DateTime<Utc>,
}

impl Deadline {
    /// Create a deadline from now plus the given duration
    pub fn from_now(duration_ms: u64) -> Self {
        Self {
            deadline_at: Utc::now() + chrono::Duration::milliseconds(duration_ms as i64),
        }
    }

    /// Create a deadline at a specific timestamp
    #[must_use]
    pub fn at(deadline_at: DateTime<Utc>) -> Self {
        Self { deadline_at }
    }

    /// Get the deadline timestamp
    #[must_use]
    pub fn deadline_at(&self) -> DateTime<Utc> {
        self.deadline_at
    }

    /// Check if the deadline has been exceeded
    pub fn is_exceeded(&self) -> bool {
        Utc::now() > self.deadline_at
    }

    /// Get remaining time in milliseconds (0 if exceeded)
    pub fn remaining_ms(&self) -> i64 {
        let remaining = self.deadline_at.signed_duration_since(Utc::now());
        remaining.num_milliseconds().max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadline_is_exceeded() {
        let deadline = Deadline::from_now(1000);
        // Should not be exceeded immediately
        assert!(!deadline.is_exceeded());
    }

    #[test]
    fn test_deadline_remaining_ms() {
        let deadline = Deadline::from_now(5000);
        let remaining = deadline.remaining_ms();
        // Should be approximately 5000 (within small margin)
        assert!(remaining >= 4900 && remaining <= 5100);
    }

    #[test]
    fn test_deadline_at_constructor() {
        let specific_time = Utc::now() + chrono::Duration::hours(1);
        let deadline = Deadline::at(specific_time);
        assert_eq!(deadline.deadline_at(), specific_time);
        assert!(!deadline.is_exceeded());
    }

    #[test]
    fn test_deadline_is_exceeded_past() {
        let past = Utc::now() - chrono::Duration::milliseconds(100);
        let deadline = Deadline::at(past);
        assert!(deadline.is_exceeded());
    }

    #[test]
    fn test_deadline_remaining_ms_clamped_at_zero() {
        let past = Utc::now() - chrono::Duration::milliseconds(100);
        let deadline = Deadline::at(past);
        assert_eq!(deadline.remaining_ms(), 0);
    }

    #[test]
    fn test_deadline_from_now_zero_duration() {
        let deadline = Deadline::from_now(0);
        // Deadline was set at now + 0ms, so by the time we check it should be exceeded or very close
        // Even if not exceeded, remaining_ms should be clamped to 0
        let remaining = deadline.remaining_ms();
        assert!(remaining <= 1);
    }
}

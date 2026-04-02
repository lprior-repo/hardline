//! Policy: Timeout configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Timeout configuration for a phase
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseTimeout {
    /// Maximum duration in milliseconds
    duration_ms: u64,
}

impl PhaseTimeout {
    /// Create a new timeout with the given duration in milliseconds
    pub fn new(duration_ms: u64) -> Result<Self, super::ConfigError> {
        if duration_ms == 0 {
            return Err(super::ConfigError::InvalidTimeout { duration_ms });
        }
        Ok(Self { duration_ms })
    }

    /// Get the timeout duration in milliseconds
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// Check if the timeout has elapsed since the given start time
    pub fn is_expired(&self, started_at: DateTime<Utc>) -> bool {
        let elapsed = Utc::now().signed_duration_since(started_at);
        elapsed.num_milliseconds() >= self.duration_ms as i64
    }

    /// Get elapsed time in milliseconds since the given start time
    pub fn elapsed_ms(&self, started_at: DateTime<Utc>) -> u64 {
        let elapsed = Utc::now().signed_duration_since(started_at);
        elapsed.num_milliseconds().max(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_timeout_new_valid() {
        let timeout = PhaseTimeout::new(1000).expect("should create timeout");
        assert_eq!(timeout.duration_ms(), 1000);
    }

    #[test]
    fn test_phase_timeout_new_zero() {
        let result = PhaseTimeout::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_timeout_is_expired() {
        let timeout = PhaseTimeout::new(50).expect("should create timeout");
        let started = Utc::now() - chrono::Duration::milliseconds(100);
        assert!(timeout.is_expired(started));
    }

    #[test]
    fn test_phase_timeout_not_expired_for_recent_start() {
        let timeout = PhaseTimeout::new(5000).expect("should create timeout");
        let started = Utc::now();
        assert!(!timeout.is_expired(started));
    }

    #[test]
    fn test_phase_timeout_elapsed_ms() {
        let timeout = PhaseTimeout::new(1000).expect("should create timeout");
        let started = Utc::now() - chrono::Duration::milliseconds(200);
        let elapsed = timeout.elapsed_ms(started);
        // Should be approximately 200ms
        assert!(elapsed >= 190 && elapsed <= 250);
    }

    #[test]
    fn test_phase_timeout_elapsed_ms_clamped_at_zero() {
        let timeout = PhaseTimeout::new(1000).expect("should create timeout");
        // Start in the future (edge case: clock skew)
        let started = Utc::now() + chrono::Duration::milliseconds(500);
        let elapsed = timeout.elapsed_ms(started);
        assert_eq!(elapsed, 0);
    }

    #[test]
    fn test_phase_timeout_is_expired_boundary() {
        let timeout = PhaseTimeout::new(10).expect("should create timeout");
        let started = Utc::now() - chrono::Duration::milliseconds(10);
        // Exactly at the boundary should be expired (>=)
        assert!(timeout.is_expired(started));
    }
}

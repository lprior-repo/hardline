//! Simple in-memory rate limiter per ADR-015.
//!
//! Uses a sliding window approach with bounded request tracking.
//! Zero panic, zero unwrap.

use std::time::Instant;

// ========================================================================
// RateLimiter
// ========================================================================

/// Simple sliding-window rate limiter.
///
/// Tracks request timestamps within a configurable window and enforces
/// a maximum number of requests per window.
pub struct RateLimiter {
    /// Maximum requests allowed within the window
    max_requests: u32,
    /// Window duration in seconds
    window_secs: u64,
    /// Timestamps of recent requests
    requests: Vec<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Panics
    ///
    /// Does not panic. `max_requests` of 0 will deny all requests.
    #[must_use]
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            requests: Vec::new(),
        }
    }

    /// Check whether a request is allowed under the rate limit.
    ///
    /// Returns `true` if the request is within limits (allowed),
    /// `false` if the rate limit has been exceeded (denied).
    ///
    /// Each call records the request timestamp if allowed.
    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);
        let cutoff = match now.checked_sub(window) {
            Some(t) => t,
            None => Instant::now(), // window overflow, allow request
        };

        // Prune expired entries
        self.requests.retain(|&ts| ts > cutoff);

        // Check if under limit
        let max_usize = match usize::try_from(self.max_requests) {
            Ok(n) => n,
            Err(_) => return false,
        };

        if self.requests.len() < max_usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    /// Return the number of requests recorded in the current window.
    #[must_use]
    pub fn current_count(&self) -> usize {
        self.requests.len()
    }

    /// Return the configured maximum requests per window.
    #[must_use]
    pub const fn max_requests(&self) -> u32 {
        self.max_requests
    }

    /// Return the configured window duration in seconds.
    #[must_use]
    pub const fn window_secs(&self) -> u64 {
        self.window_secs
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_within_limit() {
        let mut limiter = RateLimiter::new(5, 60);
        for _ in 0..5 {
            assert!(limiter.check(), "should allow within limit");
        }
        assert_eq!(limiter.current_count(), 5);
    }

    #[test]
    fn test_denies_over_limit() {
        let mut limiter = RateLimiter::new(3, 60);
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(!limiter.check(), "should deny over limit");
        assert!(!limiter.check(), "should continue denying");
        // Count stays at max because denied requests are not recorded
        assert_eq!(limiter.current_count(), 3);
    }

    #[test]
    fn test_zero_max_denies_all() {
        let mut limiter = RateLimiter::new(0, 60);
        assert!(!limiter.check(), "zero max should deny");
        assert_eq!(limiter.current_count(), 0);
    }

    #[test]
    fn test_accessors() {
        let limiter = RateLimiter::new(10, 120);
        assert_eq!(limiter.max_requests(), 10);
        assert_eq!(limiter.window_secs(), 120);
        assert_eq!(limiter.current_count(), 0);
    }
}

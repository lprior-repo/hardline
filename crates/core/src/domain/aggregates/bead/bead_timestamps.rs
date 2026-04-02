//! Bead timestamps.

use chrono::{DateTime, Utc};

/// Timestamps for bead reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeadTimestamps {
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

impl BeadTimestamps {
    /// Create new timestamps.
    #[must_use]
    pub const fn new(created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        Self {
            created_at,
            updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};

    use super::BeadTimestamps;

    #[test]
    fn new_with_equal_timestamps() {
        let now = Utc::now();
        let ts = BeadTimestamps::new(now, now);
        assert_eq!(ts.created_at, ts.updated_at);
    }

    #[test]
    fn new_with_future_updated_at() {
        let created = Utc::now();
        let updated = created + TimeDelta::seconds(5);
        let ts = BeadTimestamps::new(created, updated);
        assert!(ts.updated_at > ts.created_at);
    }

    #[test]
    fn new_is_copy() {
        let now = Utc::now();
        let ts = BeadTimestamps::new(now, now);
        let _ts2 = ts; // Should compile without move
        let _ts3 = ts; // Should compile without move (Copy)
    }

    #[test]
    fn equality() {
        let now = Utc::now();
        let a = BeadTimestamps::new(now, now);
        let b = BeadTimestamps::new(now, now);
        assert_eq!(a, b);
    }
}

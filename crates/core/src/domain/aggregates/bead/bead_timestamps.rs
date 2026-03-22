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

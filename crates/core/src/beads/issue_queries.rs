use chrono::{DateTime, Utc};

use crate::beads::issue_data::Issue;

impl Issue {
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        self.state.is_blocked() || !self.blocked_by.is_empty()
    }

    /// Check if the issue is active (open or in progress).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Check if the issue is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.state.is_closed()
    }

    /// Check if the issue has a parent.
    #[must_use]
    pub const fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    /// Get the closed timestamp if closed.
    #[must_use]
    pub const fn closed_at(&self) -> Option<DateTime<Utc>> {
        self.state.closed_at()
    }
}

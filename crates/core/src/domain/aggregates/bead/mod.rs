//! Bead aggregate root with business rules and invariants.
//!
//! The Bead aggregate represents an issue/task with:
//! - Unique identity (`BeadId`)
//! - Title and optional description
//! - State (Open, `InProgress`, Blocked, Deferred, Closed)
//! - Creation and modification timestamps
//!
//! # Invariants
//!
//! 1. Bead IDs must be unique
//! 2. Title cannot be empty
//! 3. Closed state MUST have a `closed_at` timestamp (enforced by type)
//! 4. Once closed, a bead remains closed (no reopening without explicit business rule)
//! 5. Timestamps must be monotonic (`updated_at` >= `created_at`)

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod bead_constructors;
pub mod bead_error;
pub mod bead_tests;
pub mod bead_timestamps;
pub mod bead_transitions;
pub mod bead_updates;
pub mod bead_validation;

// Re-export types for convenience
pub use bead_error::{BeadError, BeadState};
pub use bead_timestamps::BeadTimestamps;
use chrono::{DateTime, Utc};

use crate::{
    beads::{Description, Title},
    domain::identifiers::BeadId,
};

// ============================================================================
// BEAD AGGREGATE ROOT
// ============================================================================

/// Bead aggregate root.
///
/// Enforces all business rules and invariants for beads/issues.
/// All state transitions go through validated methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bead {
    /// Unique bead identifier
    pub id: BeadId,
    /// Bead title (validated)
    pub title: Title,
    /// Bead description (optional, validated)
    pub description: Option<Description>,
    /// Current state
    pub state: BeadState,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

impl Bead {
    // ========================================================================
    // QUERY METHODS
    // ========================================================================

    /// Check if bead is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, BeadState::Open)
    }

    /// Check if bead is in progress.
    #[must_use]
    pub const fn is_in_progress(&self) -> bool {
        matches!(self.state, BeadState::InProgress)
    }

    /// Check if bead is blocked.
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        matches!(self.state, BeadState::Blocked)
    }

    /// Check if bead is deferred.
    #[must_use]
    pub const fn is_deferred(&self) -> bool {
        matches!(self.state, BeadState::Deferred)
    }

    /// Check if bead is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.state.is_closed()
    }

    /// Check if bead is active (open or in progress).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Get the closed timestamp if bead is closed.
    #[must_use]
    pub const fn closed_at(&self) -> Option<DateTime<Utc>> {
        self.state.closed_at()
    }
}

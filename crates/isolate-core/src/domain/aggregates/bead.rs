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

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::{
    beads::{Description, DomainError, IssueState, Title},
    domain::identifiers::BeadId,
};

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

/// Errors that can occur during bead operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BeadError {
    /// Invalid title
    #[error("invalid title: {0}")]
    InvalidTitle(String),

    /// Invalid description
    #[error("invalid description: {0}")]
    InvalidDescription(String),

    /// Invalid state transition
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: BeadState, to: BeadState },

    /// Cannot modify closed bead
    #[error("cannot modify closed bead")]
    CannotModifyClosed,

    /// Timestamps are not monotonic
    #[error("timestamps must be monotonic: updated_at ({updated_at}) < created_at ({created_at})")]
    NonMonotonicTimestamps {
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    },

    /// Title is required
    #[error("title is required")]
    TitleRequired,

    /// Domain error from beads module
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
}

// ============================================================================
// BEAD STATE
// ============================================================================

/// Bead state matches the beads `IssueState`.
///
/// Closed state requires a timestamp inline.
pub type BeadState = IssueState;

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
    // CONSTRUCTORS
    // ========================================================================

    /// Create a new open bead.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::InvalidTitle` if title validation fails.
    /// Returns `BeadError::InvalidDescription` if description validation fails.
    pub fn new(
        id: BeadId,
        title: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Result<Self, BeadError> {
        let title = Title::new(title.into()).map_err(|e| BeadError::InvalidTitle(e.to_string()))?;
        let description = description
            .map(|d| {
                Description::new(d.into()).map_err(|e| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        let now = Utc::now();

        Ok(Self {
            id,
            title,
            description,
            state: BeadState::Open,
            created_at: now,
            updated_at: now,
        })
    }

    /// Reconstruct a bead from persisted data.
    ///
    /// # Errors
    ///
    /// Returns `BeadError` if validation fails.
    pub fn reconstruct(
        id: BeadId,
        title: impl Into<String>,
        description: Option<impl Into<String>>,
        state: BeadState,
        timestamps: BeadTimestamps,
    ) -> Result<Self, BeadError> {
        let title = Title::new(title.into()).map_err(|e| BeadError::InvalidTitle(e.to_string()))?;
        let description = description
            .map(|d| {
                Description::new(d.into()).map_err(|e| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        // Validate monotonic timestamps
        if timestamps.updated_at < timestamps.created_at {
            return Err(BeadError::NonMonotonicTimestamps {
                created_at: timestamps.created_at,
                updated_at: timestamps.updated_at,
            });
        }

        // Validate closed state has timestamp (enforced by type, but double-check)
        if matches!(state, BeadState::Closed { .. }) && state.closed_at().is_none() {
            return Err(BeadError::InvalidStateTransition {
                from: BeadState::Open,
                to: state,
            });
        }

        Ok(Self {
            id,
            title,
            description,
            state,
            created_at: timestamps.created_at,
            updated_at: timestamps.updated_at,
        })
    }

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

    // ========================================================================
    // STATE TRANSITION METHODS
    // ========================================================================

    /// Transition to Open state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    pub fn open(&self) -> Result<Self, BeadError> {
        self.transition_to(BeadState::Open)
    }

    /// Transition to `InProgress` state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    pub fn start(&self) -> Result<Self, BeadError> {
        self.transition_to(BeadState::InProgress)
    }

    /// Transition to Blocked state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    pub fn block(&self) -> Result<Self, BeadError> {
        self.transition_to(BeadState::Blocked)
    }

    /// Transition to Deferred state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    pub fn defer(&self) -> Result<Self, BeadError> {
        self.transition_to(BeadState::Deferred)
    }

    /// Transition to Closed state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if already closed.
    pub fn close(&self) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let now = Utc::now();
        let new_state = BeadState::Closed { closed_at: now };

        Ok(Self {
            state: new_state,
            updated_at: now,
            ..self.clone()
        })
    }

    /// Transition to a new state with validation.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError::InvalidStateTransition` if transition is invalid.
    fn transition_to(&self, new_state: BeadState) -> Result<Self, BeadError> {
        // Cannot modify closed beads
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        // Validate transition (using domain logic)
        let _ =
            self.state
                .transition_to(new_state)
                .map_err(|_| BeadError::InvalidStateTransition {
                    from: self.state,
                    to: new_state,
                })?;

        Ok(Self {
            state: new_state,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    // ========================================================================
    // UPDATE METHODS
    // ========================================================================

    /// Update the bead title.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError::InvalidTitle` if title validation fails.
    pub fn update_title(&self, new_title: impl Into<String>) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let title =
            Title::new(new_title.into()).map_err(|e| BeadError::InvalidTitle(e.to_string()))?;

        Ok(Self {
            title,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    /// Update the bead description.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError::InvalidDescription` if description validation fails.
    pub fn update_description(
        &self,
        new_description: Option<impl Into<String>>,
    ) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let description = new_description
            .map(|d| {
                Description::new(d.into()).map_err(|e| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        Ok(Self {
            description,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    /// Update both title and description.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    /// Returns `BeadError` if validation fails.
    pub fn update(
        &self,
        new_title: impl Into<String>,
        new_description: Option<impl Into<String>>,
    ) -> Result<Self, BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }

        let title =
            Title::new(new_title.into()).map_err(|e| BeadError::InvalidTitle(e.to_string()))?;
        let description = new_description
            .map(|d| {
                Description::new(d.into()).map_err(|e| BeadError::InvalidDescription(e.to_string()))
            })
            .transpose()?;

        Ok(Self {
            title,
            description,
            updated_at: Utc::now(),
            ..self.clone()
        })
    }

    // ========================================================================
    // VALIDATION METHODS
    // ========================================================================

    /// Validate that bead can be modified.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::CannotModifyClosed` if bead is closed.
    pub const fn validate_can_modify(&self) -> Result<(), BeadError> {
        if self.is_closed() {
            return Err(BeadError::CannotModifyClosed);
        }
        Ok(())
    }

    /// Validate that bead is in a consistent state.
    ///
    /// # Errors
    ///
    /// Returns `BeadError` if validation fails.
    pub fn validate(&self) -> Result<(), BeadError> {
        // Check timestamp monotonicity
        if self.updated_at < self.created_at {
            return Err(BeadError::NonMonotonicTimestamps {
                created_at: self.created_at,
                updated_at: self.updated_at,
            });
        }

        // Check closed state has timestamp
        if matches!(self.state, BeadState::Closed { .. }) && self.closed_at().is_none() {
            return Err(BeadError::InvalidStateTransition {
                from: BeadState::Open,
                to: self.state,
            });
        }

        Ok(())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::BeadId;

    fn create_test_bead() -> Bead {
        let id = BeadId::parse("bd-1").expect("valid id");
        Bead::new(id, "Test Bead", None::<String>).expect("bead created")
    }

    #[test]
    fn test_create_bead() {
        let bead = create_test_bead();

        assert!(bead.is_open());
        assert!(!bead.is_in_progress());
        assert!(!bead.is_closed());
        assert!(bead.is_active());
        assert_eq!(bead.title.as_str(), "Test Bead");
    }

    #[test]
    fn test_open_to_in_progress() {
        let bead = create_test_bead();

        let in_progress = bead.start().expect("transition valid");

        assert!(in_progress.is_in_progress());
        assert!(!in_progress.is_open());
        assert!(in_progress.is_active());
    }

    #[test]
    fn test_in_progress_to_blocked() {
        let bead = create_test_bead();
        let in_progress = bead.start().expect("transition valid");

        let blocked = in_progress.block().expect("transition valid");

        assert!(blocked.is_blocked());
        assert!(!blocked.is_active());
    }

    #[test]
    fn test_blocked_to_deferred() {
        let bead = create_test_bead();
        let blocked = bead.start().and_then(|b| b.block()).expect("state valid");

        let deferred = blocked.defer().expect("transition valid");

        assert!(deferred.is_deferred());
        assert!(!deferred.is_blocked());
    }

    #[test]
    fn test_close_bead() {
        let bead = create_test_bead();

        let closed = bead.close().expect("close valid");

        assert!(closed.is_closed());
        assert!(closed.closed_at().is_some());
        assert!(!closed.is_active());
    }

    #[test]
    fn test_cannot_modify_closed_bead() {
        let bead = create_test_bead();
        let closed = bead.close().expect("close valid");

        // Cannot transition from closed
        let result = closed.start();
        assert!(matches!(result, Err(BeadError::CannotModifyClosed)));

        // Cannot update title
        let result = closed.update_title("New Title");
        assert!(matches!(result, Err(BeadError::CannotModifyClosed)));

        // Cannot update description
        let result = closed.update_description(Some("New description"));
        assert!(matches!(result, Err(BeadError::CannotModifyClosed)));
    }

    #[test]
    fn test_update_title() {
        let bead = create_test_bead();

        let updated = bead.update_title("Updated Title").expect("update valid");

        assert_eq!(updated.title.as_str(), "Updated Title");
        assert!(updated.updated_at >= updated.created_at);
    }

    #[test]
    fn test_update_description() {
        let bead = create_test_bead();

        let updated = bead
            .update_description(Some("New description"))
            .expect("update valid");

        assert!(updated.description.is_some());
        assert_eq!(
            updated
                .description
                .as_ref()
                .map(crate::beads::Description::as_str),
            Some("New description")
        );
    }

    #[test]
    fn test_update_both() {
        let bead = create_test_bead();

        let updated = bead
            .update("New Title", Some("New description"))
            .expect("update valid");

        assert_eq!(updated.title.as_str(), "New Title");
        assert!(updated.description.is_some());
    }

    #[test]
    fn test_invalid_title() {
        let id = BeadId::parse("bd-1").expect("valid id");

        // Empty title
        let result = Bead::new(id.clone(), "", None::<String>);
        assert!(matches!(result, Err(BeadError::InvalidTitle(_))));

        // Whitespace-only title
        let result = Bead::new(id, "   ", None::<String>);
        assert!(matches!(result, Err(BeadError::InvalidTitle(_))));
    }

    #[test]
    fn test_non_monotonic_timestamps() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let created = Utc::now();
        let updated = created - chrono::Duration::seconds(1);

        let result = Bead::reconstruct(
            id,
            "Test",
            None::<String>,
            BeadState::Open,
            BeadTimestamps::new(created, updated),
        );

        assert!(matches!(
            result,
            Err(BeadError::NonMonotonicTimestamps { .. })
        ));
    }

    #[test]
    fn test_validate_can_modify() {
        let bead = create_test_bead();

        assert!(bead.validate_can_modify().is_ok());

        let closed = bead.close().expect("close valid");
        assert!(matches!(
            closed.validate_can_modify(),
            Err(BeadError::CannotModifyClosed)
        ));
    }

    #[test]
    fn test_reconstruct() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let now = Utc::now();

        let bead = Bead::reconstruct(
            id.clone(),
            "Test Bead",
            Some("Description"),
            BeadState::Open,
            BeadTimestamps::new(now, now),
        )
        .expect("reconstruct valid");

        assert_eq!(bead.id, id);
        assert_eq!(bead.title.as_str(), "Test Bead");
        assert!(bead.description.is_some());
        assert!(bead.is_open());
    }

    #[test]
    fn test_reconstruct_closed() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let now = Utc::now();

        let bead = Bead::reconstruct(
            id,
            "Test Bead",
            None::<String>,
            BeadState::Closed { closed_at: now },
            BeadTimestamps::new(now - chrono::Duration::seconds(10), now),
        )
        .expect("reconstruct valid");

        assert!(bead.is_closed());
        assert_eq!(bead.closed_at(), Some(now));
    }
}

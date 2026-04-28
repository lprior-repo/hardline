//! Bead constructors.

use chrono::Utc;

use super::{Bead, BeadError, BeadState, BeadTimestamps};
use crate::{
    beads::{Description, DomainError, Title},
    domain::identifiers::BeadId,
};

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
        let title = Title::new(title.into())
            .map_err(|e: DomainError| BeadError::InvalidTitle(e.to_string()))?;
        let description = description
            .map(|d| {
                Description::new(d.into())
                    .map_err(|e: DomainError| BeadError::InvalidDescription(e.to_string()))
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
    #[allow(clippy::too_many_lines)]
    pub fn reconstruct(
        id: BeadId,
        title: impl Into<String>,
        description: Option<impl Into<String>>,
        state: BeadState,
        timestamps: BeadTimestamps,
    ) -> Result<Self, BeadError> {
        let title = Title::new(title.into())
            .map_err(|e: DomainError| BeadError::InvalidTitle(e.to_string()))?;
        let description = description
            .map(|d| {
                Description::new(d.into())
                    .map_err(|e: DomainError| BeadError::InvalidDescription(e.to_string()))
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
}

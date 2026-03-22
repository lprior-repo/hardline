//! Bead validation methods.

use super::{Bead, BeadError, BeadState};

impl Bead {
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

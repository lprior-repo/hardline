//! Bead state transitions.

use chrono::Utc;

use super::{Bead, BeadError, BeadState};

impl Bead {
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
}

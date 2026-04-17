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

#[cfg(test)]
mod tests {
    use crate::domain::aggregates::bead::{Bead, BeadError};
    use crate::domain::identifiers::BeadId;

    fn create_test_bead() -> Bead {
        let id = BeadId::parse("bd-1").expect("valid id");
        Bead::new(id, "Test Bead", None::<String>).expect("bead created")
    }

    // -- open() --

    #[test]
    fn open_from_open_is_ok() {
        let bead = create_test_bead();
        let result = bead.open();
        assert!(result.is_ok());
        assert!(result.expect("ok").is_open());
    }

    #[test]
    fn open_from_in_progress_is_ok() {
        let bead = create_test_bead().start().expect("start ok");
        let result = bead.open();
        assert!(result.is_ok());
    }

    #[test]
    fn open_from_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        let result = bead.open();
        assert_eq!(result, Err(BeadError::CannotModifyClosed));
    }

    // -- start() --

    #[test]
    fn start_from_open_is_ok() {
        let bead = create_test_bead();
        let result = bead.start();
        assert!(result.is_ok());
        assert!(result.expect("ok").is_in_progress());
    }

    #[test]
    fn start_from_deferred_is_ok() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .and_then(|b| b.defer())
            .expect("defer ok");
        let result = bead.start();
        assert!(result.is_ok());
    }

    #[test]
    fn start_from_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(bead.start(), Err(BeadError::CannotModifyClosed));
    }

    // -- block() --

    #[test]
    fn block_from_in_progress_is_ok() {
        let bead = create_test_bead().start().expect("start ok");
        let result = bead.block();
        assert!(result.is_ok());
        assert!(result.expect("ok").is_blocked());
    }

    #[test]
    fn block_from_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(bead.block(), Err(BeadError::CannotModifyClosed));
    }

    // -- defer() --

    #[test]
    fn defer_from_blocked_is_ok() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .expect("block ok");
        let result = bead.defer();
        assert!(result.is_ok());
        assert!(result.expect("ok").is_deferred());
    }

    #[test]
    fn defer_from_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(bead.defer(), Err(BeadError::CannotModifyClosed));
    }

    // -- close() --

    #[test]
    fn close_sets_closed_state() {
        let bead = create_test_bead();
        let closed = bead.close().expect("close ok");
        assert!(closed.is_closed());
        assert!(closed.closed_at().is_some());
    }

    #[test]
    fn close_from_in_progress_is_ok() {
        let bead = create_test_bead().start().expect("start ok");
        let result = bead.close();
        assert!(result.is_ok());
        assert!(result.expect("ok").is_closed());
    }

    #[test]
    fn close_from_blocked_is_ok() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .expect("block ok");
        let result = bead.close();
        assert!(result.is_ok());
    }

    #[test]
    fn close_from_deferred_is_ok() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .and_then(|b| b.defer())
            .expect("defer ok");
        let result = bead.close();
        assert!(result.is_ok());
    }

    #[test]
    fn close_already_closed_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(bead.close(), Err(BeadError::CannotModifyClosed));
    }

    // -- Full lifecycle --

    #[test]
    fn full_lifecycle_open_progress_blocked_deferred_progress_close() {
        let bead = create_test_bead();
        let final_bead = bead
            .start()
            .and_then(|b| b.block())
            .and_then(|b| b.defer())
            .and_then(|b| b.start())
            .and_then(|b| b.close())
            .expect("full lifecycle ok");
        assert!(final_bead.is_closed());
    }

    #[test]
    fn close_updates_timestamp() {
        let bead = create_test_bead();
        let closed = bead.close().expect("close ok");
        assert!(closed.updated_at >= closed.created_at);
        assert_eq!(closed.closed_at(), Some(closed.updated_at));
    }
}

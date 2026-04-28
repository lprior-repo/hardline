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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::domain::{
        aggregates::bead::{Bead, BeadError, BeadState, BeadTimestamps},
        identifiers::BeadId,
    };

    fn create_test_bead() -> Bead {
        let id = BeadId::parse("bd-1").expect("valid id");
        Bead::new(id, "Test Bead", None::<String>).expect("bead created")
    }

    // -- validate_can_modify --

    #[test]
    fn validate_can_modify_open_bead() {
        let bead = create_test_bead();
        assert!(bead.validate_can_modify().is_ok());
    }

    #[test]
    fn validate_can_modify_in_progress_bead() {
        let bead = create_test_bead().start().expect("start ok");
        assert!(bead.validate_can_modify().is_ok());
    }

    #[test]
    fn validate_can_modify_blocked_bead() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .expect("block ok");
        assert!(bead.validate_can_modify().is_ok());
    }

    #[test]
    fn validate_can_modify_deferred_bead() {
        let bead = create_test_bead()
            .start()
            .and_then(|b| b.block())
            .and_then(|b| b.defer())
            .expect("defer ok");
        assert!(bead.validate_can_modify().is_ok());
    }

    #[test]
    fn validate_can_modify_closed_bead_rejects() {
        let bead = create_test_bead().close().expect("close ok");
        assert_eq!(
            bead.validate_can_modify(),
            Err(BeadError::CannotModifyClosed)
        );
    }

    // -- validate() --

    #[test]
    fn validate_open_bead_is_valid() {
        let bead = create_test_bead();
        assert!(bead.validate().is_ok());
    }

    #[test]
    fn validate_rejects_non_monotonic_timestamps() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let created = Utc::now();
        let updated = created - Duration::seconds(1);

        // Create a bead with non-monotonic timestamps directly.
        // reconstruct() itself rejects non-monotonic timestamps, so we
        // construct the struct manually to test the validate() method in isolation.
        let bead = Bead {
            id,
            title: crate::beads::Title::new("Test".to_string()).expect("valid title"),
            description: None,
            state: BeadState::Open,
            created_at: created,
            updated_at: updated,
        };

        let result = bead.validate();
        assert!(matches!(
            result,
            Err(BeadError::NonMonotonicTimestamps { .. })
        ));
    }

    #[test]
    fn validate_equal_timestamps_is_valid() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let now = Utc::now();

        let bead = Bead::reconstruct(
            id,
            "Test",
            None::<String>,
            BeadState::Open,
            BeadTimestamps::new(now, now),
        )
        .expect("reconstruct ok");

        assert!(bead.validate().is_ok());
    }

    #[test]
    fn validate_closed_bead_with_timestamp_is_valid() {
        let id = BeadId::parse("bd-1").expect("valid id");
        let now = Utc::now();

        let bead = Bead::reconstruct(
            id,
            "Test",
            None::<String>,
            BeadState::Closed { closed_at: now },
            BeadTimestamps::new(now - Duration::seconds(1), now),
        )
        .expect("reconstruct ok");

        assert!(bead.validate().is_ok());
    }
}

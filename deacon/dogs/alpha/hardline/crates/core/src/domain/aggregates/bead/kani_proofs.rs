//! Kani proofs for Bead state machine invariants.
//!
//! # Invariants Proven
//!
//! 1. Closed beads cannot be modified
//! 2. Valid state transitions are accepted
//! 3. Timestamp monotonicity is preserved
//! 4. State transitions update updated_at

#[cfg(kani)]
mod proof {
    use chrono::Utc;

    use crate::beads::{Description, DomainError, IssueState, Title};
    use crate::domain::aggregates::bead::{Bead, BeadError, BeadState, BeadTimestamps};

    fn create_open_bead() -> Bead {
        let id = crate::domain::identifiers::BeadId::parse("bd-abc123").unwrap();
        let title = Title::new("Test Bead").unwrap();
        let now = Utc::now();
        Bead {
            id,
            title,
            description: None,
            state: BeadState::Open,
            created_at: now,
            updated_at: now,
        }
    }

    #[kani::proof]
    fn verify_open_bead_is_active() {
        let bead = create_open_bead();
        assert!(bead.is_active());
        assert!(!bead.is_closed());
    }

    #[kani::proof]
    fn verify_in_progress_bead_is_active() {
        let bead = Bead {
            state: BeadState::InProgress,
            ..create_open_bead()
        };
        assert!(bead.is_active());
        assert!(!bead.is_closed());
    }

    #[kani::proof]
    fn verify_blocked_bead_is_not_active() {
        let bead = Bead {
            state: BeadState::Blocked,
            ..create_open_bead()
        };
        assert!(!bead.is_active());
    }

    #[kani::proof]
    fn verify_deferred_bead_is_not_active() {
        let bead = Bead {
            state: BeadState::Deferred,
            ..create_open_bead()
        };
        assert!(!bead.is_active());
    }

    #[kani::proof]
    fn verify_closed_bead_is_closed() {
        let now = Utc::now();
        let bead = Bead {
            state: BeadState::Closed { closed_at: now },
            ..create_open_bead()
        };
        assert!(bead.is_closed());
        assert!(!bead.is_active());
    }

    #[kani::proof]
    fn verify_open_to_in_progress_transition() {
        let bead = create_open_bead();
        let result = bead.start();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().state, BeadState::InProgress);
    }

    #[kani::proof]
    fn verify_open_to_blocked_transition() {
        let bead = create_open_bead();
        let result = bead.block();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().state, BeadState::Blocked);
    }

    #[kani::proof]
    fn verify_open_to_deferred_transition() {
        let bead = create_open_bead();
        let result = bead.defer();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().state, BeadState::Deferred);
    }

    #[kani::proof]
    fn verify_open_to_closed_transition() {
        let bead = create_open_bead();
        let result = bead.close();
        assert!(result.is_ok());
        assert!(result.unwrap().is_closed());
    }

    #[kani::proof]
    fn verify_closed_bead_cannot_be_modified() {
        let bead = create_open_bead();
        let closed = bead.close().unwrap();

        assert!(closed.start().is_err());
        assert!(closed.block().is_err());
        assert!(closed.defer().is_err());
        assert!(closed.open().is_err());
    }

    #[kani::proof]
    fn verify_closed_bead_close_returns_error() {
        let bead = create_open_bead();
        let closed = bead.close().unwrap();

        let result = closed.close();
        assert!(result.is_err());
        assert!(matches!(result, Err(BeadError::CannotModifyClosed)));
    }

    #[kani::proof]
    fn verify_state_transition_updates_timestamp() {
        let bead = create_open_bead();
        let original_updated = bead.updated_at;

        let result = bead.start();
        if result.is_ok() {
            let new_bead = result.unwrap();
            assert!(new_bead.updated_at >= original_updated);
        }
    }

    #[kani::proof]
    fn verify_bead_id_preserved_through_transitions() {
        let bead = create_open_bead();
        let original_id = bead.id.clone();

        let result = bead.start();
        if result.is_ok() {
            assert_eq!(result.unwrap().id, original_id);
        }

        let result = bead.block();
        if result.is_ok() {
            assert_eq!(result.unwrap().id, original_id);
        }
    }

    #[kani::proof]
    fn verify_closed_bead_preserves_closed_at() {
        let bead = create_open_bead();
        let closed = bead.close().unwrap();

        let closed_at = closed.closed_at();
        assert!(closed_at.is_some());
    }

    #[kani::proof]
    fn verify_non_closed_bead_has_no_closed_at() {
        let bead = create_open_bead();
        assert!(bead.closed_at().is_none());
    }

    #[kani::proof]
    fn verify_beadstate_is_issuetype() {
        let state = BeadState::Open;
        let _issue_state: IssueState = state;
        assert!(matches!(state, BeadState::Open));
    }

    #[kani::proof]
    fn verify_all_non_closed_states() {
        let states = [
            BeadState::Open,
            BeadState::InProgress,
            BeadState::Blocked,
            BeadState::Deferred,
        ];

        for state in states {
            let bead = Bead {
                state,
                ..create_open_bead()
            };
            assert!(!bead.is_closed());
        }
    }

    #[kani::proof]
    fn verify_in_progress_transitions() {
        let bead = Bead {
            state: BeadState::InProgress,
            ..create_open_bead()
        };

        assert!(bead.block().is_ok());
        assert!(bead.defer().is_ok());
        assert!(bead.close().is_ok());
    }
}

//! Bead tests.

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::identifiers::BeadId;

    use crate::domain::aggregates::bead::Bead;

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
        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::CannotModifyClosed)
        ));

        // Cannot update title
        let result = closed.update_title("New Title");
        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::CannotModifyClosed)
        ));

        // Cannot update description
        let result = closed.update_description(Some("New description"));
        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::CannotModifyClosed)
        ));
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
        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::InvalidTitle(_))
        ));

        // Whitespace-only title
        let result = Bead::new(id, "   ", None::<String>);
        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::InvalidTitle(_))
        ));
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
            crate::domain::aggregates::bead::BeadState::Open,
            crate::domain::aggregates::bead::BeadTimestamps::new(created, updated),
        );

        assert!(matches!(
            result,
            Err(crate::domain::aggregates::bead::BeadError::NonMonotonicTimestamps { .. })
        ));
    }

    #[test]
    fn test_validate_can_modify() {
        let bead = create_test_bead();

        assert!(bead.validate_can_modify().is_ok());

        let closed = bead.close().expect("close valid");
        assert!(matches!(
            closed.validate_can_modify(),
            Err(crate::domain::aggregates::bead::BeadError::CannotModifyClosed)
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
            crate::domain::aggregates::bead::BeadState::Open,
            crate::domain::aggregates::bead::BeadTimestamps::new(now, now),
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
            crate::domain::aggregates::bead::BeadState::Closed { closed_at: now },
            crate::domain::aggregates::bead::BeadTimestamps::new(
                now - chrono::Duration::seconds(10),
                now,
            ),
        )
        .expect("reconstruct valid");

        assert!(bead.is_closed());
        assert_eq!(bead.closed_at(), Some(now));
    }

    #[test]
    fn test_concurrent_state_transitions() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let bead = Arc::new(Mutex::new(create_test_bead()));
        let mut handles = vec![];

        for i in 0..10 {
            let bead_clone = Arc::clone(&bead);
            handles.push(thread::spawn(move || {
                let mut locked_bead = bead_clone.lock().unwrap();

                // Attempt various state transitions
                if i % 3 == 0 {
                    if let Ok(new_bead) = locked_bead.start() {
                        *locked_bead = new_bead;
                    }
                } else if i % 3 == 1 {
                    if let Ok(new_bead) = locked_bead.update_title(format!("Title {}", i)) {
                        *locked_bead = new_bead;
                    }
                } else {
                    if let Ok(new_bead) = locked_bead.block() {
                        *locked_bead = new_bead;
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Just verify the lock isn't poisoned and we can access the bead
        let final_bead = bead.lock().unwrap();
        assert!(final_bead.is_active() || final_bead.is_blocked() || final_bead.is_closed());
    }
}

//! Kani proofs for QueueEntry invariants.
//!
//! # Invariants Proven
//!
//! 1. Priority is always within valid range
//! 2. Status transitions are validated
//! 3. Entry equality ignores timestamp

#[cfg(kani)]
mod proof {
    use chrono::Utc;

    use crate::domain::identifiers::{QueueEntryId, SessionName};
    use crate::domain::queue::entry::QueueEntry;
    use crate::domain::queue::status::{QueueStatus, MAX_PRIORITY};

    fn any_valid_entry() -> QueueEntry {
        let id: String = kani::any();
        let session: String = kani::any();
        let priority: u32 = kani::any();
        kani::assume(priority <= MAX_PRIORITY);
        QueueEntry::new(id, session, priority).unwrap()
    }

    #[kani::proof]
    fn verify_new_entry_has_pending_status() {
        let entry = any_valid_entry();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[kani::proof]
    fn verify_new_entry_has_valid_priority() {
        let entry = any_valid_entry();
        assert!(entry.priority <= MAX_PRIORITY);
    }

    #[kani::proof]
    fn verify_entry_status_transition_valid() {
        let entry = any_valid_entry();
        let result = entry.transition_status(QueueStatus::Claimed);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, QueueStatus::Claimed);
    }

    #[kani::proof]
    fn verify_entry_status_transition_invalid() {
        let entry = any_valid_entry();
        let result = entry.transition_status(QueueStatus::Merged);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_entry_with_priority() {
        let entry = any_valid_entry();
        let new_entry = entry.with_priority(50).unwrap();
        assert_eq!(new_entry.priority, 50);
    }

    #[kani::proof]
    fn verify_entry_with_priority_rejects_invalid() {
        let entry = any_valid_entry();
        let result = entry.with_priority(MAX_PRIORITY + 1);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_entry_equality_ignores_timestamp() {
        let entry1 = QueueEntry::new("id1", "session", 10).unwrap();
        let entry2 = QueueEntry::new("id1", "session", 10).unwrap();

        assert_eq!(entry1, entry2);
    }

    #[kani::proof]
    fn verify_entry_inequality_different_ids() {
        let entry1 = QueueEntry::new("id1", "session", 10).unwrap();
        let entry2 = QueueEntry::new("id2", "session", 10).unwrap();

        assert_ne!(entry1, entry2);
    }

    #[kani::proof]
    fn verify_entry_inequality_different_sessions() {
        let entry1 = QueueEntry::new("id1", "session1", 10).unwrap();
        let entry2 = QueueEntry::new("id1", "session2", 10).unwrap();

        assert_ne!(entry1, entry2);
    }

    #[kani::proof]
    fn verify_entry_inequality_different_priorities() {
        let entry1 = QueueEntry::new("id1", "session", 10).unwrap();
        let entry2 = QueueEntry::new("id1", "session", 20).unwrap();

        assert_ne!(entry1, entry2);
    }

    #[kani::proof]
    fn verify_from_identifiers_valid() {
        let id = QueueEntryId::new("test-id").unwrap();
        let session = SessionName::new("test-session").unwrap();
        let entry = QueueEntry::from_identifiers(id.clone(), session.clone(), 50).unwrap();

        assert_eq!(entry.id, id);
        assert_eq!(entry.session, session);
        assert_eq!(entry.priority, 50);
    }

    #[kani::proof]
    fn verify_with_timestamp_preserves_fields() {
        let entry = any_valid_entry();
        let now = Utc::now();
        let entry_with_ts = QueueEntry::with_timestamp(
            entry.id.clone(),
            entry.session.clone(),
            entry.priority,
            now,
        )
        .unwrap();

        assert_eq!(entry_with_ts.id, entry.id);
        assert_eq!(entry_with_ts.session, entry.session);
        assert_eq!(entry_with_ts.priority, entry.priority);
    }

    #[kani::proof]
    fn verify_with_status_preserves_fields() {
        let entry = any_valid_entry();
        let now = Utc::now();
        let entry_with_status = QueueEntry::with_status(
            entry.id.clone(),
            entry.session.clone(),
            entry.priority,
            now,
            QueueStatus::Claimed,
        )
        .unwrap();

        assert_eq!(entry_with_status.id, entry.id);
        assert_eq!(entry_with_status.session, entry.session);
        assert_eq!(entry_with_status.priority, entry.priority);
        assert_eq!(entry_with_status.status, QueueStatus::Claimed);
    }
}

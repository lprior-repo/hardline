pub mod entity;
pub mod lifecycle;
pub mod priority;
pub mod metadata;

pub use entity::{
    QueueEntry, QueueStatus, Pending, Claimed, Rebasing, Testing, ReadyToMerge, Merging, Merged,
    FailedRetryable, FailedTerminal, Cancelled,
};
pub use priority::{QueueDsl, QueueEntryBuilder};
pub use metadata::{EntryMetadata, RetryMetadata, TerminalMetadata, TestMetadata};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Priority;

    #[test]
    fn queue_entry_when_created_then_has_pending_status() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_given_pending_when_claim_then_has_claimed_status() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let claimed = entry.claim().unwrap();
        assert_eq!(claimed.status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_given_merged_when_claim_then_fails() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let _merged = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_ready_to_merge())
            .and_then(|e| e.start_merging())
            .and_then(|e| e.mark_merged())
            .unwrap();
    }

    #[test]
    fn queue_entry_can_retry_returns_true_for_failed_retryable_under_limit() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let failed = entry
            .claim()
            .and_then(|e| e.start_rebase())
            .and_then(|e| e.start_testing())
            .and_then(|e| e.mark_failed_retryable("error".into()));
        assert!(failed.is_ok());
        assert!(failed.unwrap().can_retry());
    }

    #[test]
    fn queue_entry_rejects_empty_session_id() {
        let result = QueueEntry::<Pending>::enqueue("".to_string(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_rejects_whitespace_session_id() {
        let result = QueueEntry::<Pending>::enqueue("   ".to_string(), None, Priority::default());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_builder_works() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_high_priority()
            .enqueue()
            .unwrap();
        assert_eq!(entry.session_id, "test-session");
        assert_eq!(entry.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_dsl_works() {
        let mut builder = QueueEntryBuilder::new();
        builder
            .enqueue_session("dsl-session")
            .with_critical_priority()
            .execute()
            .unwrap();
    }

    #[test]
    fn queue_entry_full_lifecycle_happy_path() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Merged);
        assert!(entry.is_terminal());
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
    }

    #[test]
    fn queue_entry_builder_with_bead() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_bead("bead-42")
            .enqueue()
            .unwrap();
        assert_eq!(entry.bead_id(), Some("bead-42"));
    }

    #[test]
    fn queue_entry_builder_with_custom_priority() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_priority(Priority::low())
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_builder_default_is_normal_priority() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 200);
    }

    #[test]
    fn queue_entry_builder_missing_session_returns_error() {
        let result = QueueEntryBuilder::new().enqueue();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_builder_default_trait() {
        let builder = QueueEntryBuilder::default();
        let result = builder.enqueue();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_dsl_missing_session_returns_error() {
        let mut builder = QueueEntryBuilder::new();
        let result = builder.execute();
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_dsl_with_low_priority() {
        let mut builder = QueueEntryBuilder::new();
        let entry = builder
            .enqueue_session("session")
            .with_low_priority()
            .execute()
            .unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_failed_retryable_stores_error() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("tests failed".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 1);
        assert_eq!(entry.error_message(), Some("tests failed"));
    }

    #[test]
    fn queue_entry_failed_terminal_stores_error() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_terminal("fatal error".into())
            .unwrap();

        assert!(entry.is_terminal());
        assert_eq!(entry.error_message(), Some("fatal error"));
    }

    #[test]
    fn queue_entry_failed_retryable_can_retry_increments() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error 1".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error 2".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 2);
        assert!(entry.can_retry());
    }

    #[test]
    fn queue_entry_failed_retryable_max_retries_exhausted() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e1".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e2".into())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e3".into())
            .unwrap();

        assert_eq!(entry.retry_count(), 3);
        assert!(!entry.can_retry());
    }

    #[test]
    fn queue_entry_cancel_from_pending() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_cancel_from_claimed() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_rebasing() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_testing() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_ready_to_merge() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_merging() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_cancel_from_failed_retryable() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("err".into())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_accessors() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-99".into()),
            Priority::high(),
        )
        .unwrap();

        assert_eq!(entry.session_id(), "session-1");
        assert_eq!(entry.bead_id(), Some("bead-99"));
        assert!(!entry.id().as_str().is_empty());
        assert_eq!(entry.status(), QueueStatus::Pending);
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
        assert!(entry.enqueued_at().timestamp() != 0);
        assert!(entry.updated_at().timestamp() != 0);
    }

    #[test]
    fn queue_entry_trimmed_session_id() {
        let entry =
            QueueEntry::<Pending>::enqueue("  spaced  ".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.session_id(), "spaced");
    }

    #[test]
    fn queue_entry_id_generate_is_unique() {
        let a = QueueEntryId::generate();
        let b = QueueEntryId::generate();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn queue_entry_id_parse_valid() {
        let id = QueueEntryId::parse(String::from("my-id"));
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "my-id");
    }

    #[test]
    fn queue_entry_id_parse_empty_rejected() {
        let result = QueueEntryId::parse(String::from(""));
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_id_default_generates() {
        let id = QueueEntryId::default();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn queue_entry_id_startswith_queue_prefix() {
        let id = QueueEntryId::generate();
        assert!(id.as_str().starts_with("queue-"));
    }

    #[test]
    fn queue_entry_serde_roundtrip() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-1".into()),
            Priority::normal(),
        )
        .unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let back: QueueEntry<Pending> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id(), "session-1");
        assert_eq!(back.bead_id(), Some("bead-1"));
    }

    #[test]
    fn queue_entry_queue_status_default_is_pending() {
        assert_eq!(QueueStatus::default(), QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_queue_status_is_terminal_all_cases() {
        assert!(QueueStatus::Merged.is_terminal());
        assert!(QueueStatus::FailedTerminal.is_terminal());
        assert!(QueueStatus::Cancelled.is_terminal());
        assert!(!QueueStatus::Pending.is_terminal());
        assert!(!QueueStatus::Claimed.is_terminal());
        assert!(!QueueStatus::Rebasing.is_terminal());
        assert!(!QueueStatus::Testing.is_terminal());
        assert!(!QueueStatus::ReadyToMerge.is_terminal());
        assert!(!QueueStatus::Merging.is_terminal());
        assert!(!QueueStatus::FailedRetryable.is_terminal());
    }

    #[test]
    fn queue_entry_enqueue_with_critical_priority() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::critical()).unwrap();
        assert_eq!(entry.priority().value(), u8::MAX);
    }

    #[test]
    fn queue_entry_enqueue_with_low_priority() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::low()).unwrap();
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_full_failure_path_retry_then_cancel() {
        let entry = QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("error".into())
            .unwrap()
            .cancel()
            .unwrap();

        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn queue_entry_position_is_front_after_enqueue() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        assert_eq!(entry.position().value(), 0);
    }

    #[test]
    fn queue_entry_enqueued_at_recent() {
        let before = Utc::now();
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let after = Utc::now();
        assert!(entry.enqueued_at() >= before);
        assert!(entry.enqueued_at() <= after);
    }

    #[test]
    fn queue_entry_updated_at_recent() {
        let before = Utc::now();
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let after = Utc::now();
        assert!(entry.updated_at() >= before);
        assert!(entry.updated_at() <= after);
    }

    #[test]
    fn queue_entry_transition_updates_updated_at() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();
        let created_at = entry.enqueued_at();

        std::thread::sleep(std::time::Duration::from_millis(5));

        let claimed = entry.claim().unwrap();
        assert!(claimed.updated_at() >= created_at);
    }

    #[test]
    fn queue_entry_builder_with_all_fields() {
        let entry = QueueEntryBuilder::new()
            .with_session("test-session")
            .with_bead("bead-1")
            .with_priority(Priority::critical())
            .enqueue()
            .unwrap();

        assert_eq!(entry.session_id(), "test-session");
        assert_eq!(entry.bead_id(), Some("bead-1"));
        assert_eq!(entry.priority().value(), u8::MAX);
        assert_eq!(entry.status(), QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_builder_chained_methods() {
        let entry = QueueEntryBuilder::new()
            .with_session("s1")
            .with_low_priority()
            .with_bead("b1")
            .enqueue()
            .unwrap();

        assert_eq!(entry.session_id(), "s1");
        assert_eq!(entry.bead_id(), Some("b1"));
        assert_eq!(entry.priority().value(), 100);
    }

    #[test]
    fn queue_entry_dsl_with_critical_priority() {
        let mut builder = QueueEntryBuilder::new();
        let entry = builder
            .enqueue_session("critical-session")
            .with_critical_priority()
            .execute()
            .unwrap();
        assert_eq!(entry.priority().value(), u8::MAX);
    }

    #[test]
    fn queue_entry_retry_full_cycle_three_failures() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::default()).unwrap();

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e1".into())
            .unwrap();
        assert!(entry.can_retry());

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e2".into())
            .unwrap();
        assert!(entry.can_retry());

        let entry = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_retryable("e3".into())
            .unwrap();
        assert!(!entry.can_retry());
        assert_eq!(entry.retry_count(), 3);
    }

    #[test]
    fn queue_entry_serde_roundtrip_claimed() {
        let entry = QueueEntry::<Pending>::enqueue(
            "session-1".into(),
            Some("bead-1".into()),
            Priority::normal(),
        )
        .unwrap();
        let claimed = entry.claim().unwrap();
        let json = serde_json::to_string(&claimed).unwrap();
        let back: QueueEntry<Claimed> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status(), QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_serde_roundtrip_merged() {
        let entry =
            QueueEntry::<Pending>::enqueue("session-1".into(), None, Priority::normal()).unwrap();
        let merged = entry
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();
        let json = serde_json::to_string(&merged).unwrap();
        let back: QueueEntry<Merged> = serde_json::from_str(&json).unwrap();
        assert!(back.is_terminal());
    }

    #[test]
    fn queue_entry_all_state_markers_terminal() {
        let merged = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_ready_to_merge()
            .unwrap()
            .start_merging()
            .unwrap()
            .mark_merged()
            .unwrap();
        assert!(merged.is_terminal());

        let failed_term = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .claim()
            .unwrap()
            .start_rebase()
            .unwrap()
            .start_testing()
            .unwrap()
            .mark_failed_terminal("err".into())
            .unwrap();
        assert!(failed_term.is_terminal());

        let cancelled = QueueEntry::<Pending>::enqueue("s".into(), None, Priority::default())
            .unwrap()
            .cancel()
            .unwrap();
        assert!(cancelled.is_terminal());
    }

    #[test]
    fn queue_entry_state_marker_units() {
        assert_eq!(Pending::default(), Pending);
        assert_eq!(Claimed::default(), Claimed);
        assert_eq!(Rebasing::default(), Rebasing);
        assert_eq!(Testing::default(), Testing);
        assert_eq!(ReadyToMerge::default(), ReadyToMerge);
        assert_eq!(Merging::default(), Merging);
        assert_eq!(Merged::default(), Merged);
        assert_eq!(FailedRetryable::default(), FailedRetryable);
        assert_eq!(FailedTerminal::default(), FailedTerminal);
        assert_eq!(Cancelled::default(), Cancelled);
    }

    #[test]
    fn queue_entry_status_serde_serializes_all_variants() {
        let statuses = [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            assert!(json.starts_with('"'), "JSON should be a string: {json}");
        }
    }

    #[test]
    fn queue_entry_accessors_all_fields() {
        let entry = QueueEntry::<Pending>::enqueue(
            "my-session".into(),
            Some("bead-7".into()),
            Priority::high(),
        )
        .unwrap();

        assert!(!entry.id().as_str().is_empty());
        assert!(entry.id().as_str().starts_with("queue-"));
        assert_eq!(entry.session_id(), "my-session");
        assert_eq!(entry.bead_id(), Some("bead-7"));
        assert_eq!(entry.priority().value(), 230);
        assert_eq!(entry.position().value(), 0);
        assert_eq!(entry.status(), QueueStatus::Pending);
        assert_eq!(entry.retry_count(), 0);
        assert!(entry.error_message().is_none());
    }

    #[test]
    fn queue_entry_builder_default_priority_is_normal() {
        let entry = QueueEntryBuilder::new()
            .with_session("test")
            .enqueue()
            .unwrap();
        assert_eq!(entry.priority().value(), 200);
    }
}

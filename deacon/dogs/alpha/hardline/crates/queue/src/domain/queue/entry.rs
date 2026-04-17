//! Queue entry - Value object representing a session in the merge queue

use chrono::{DateTime, Utc};

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::validation::ValidationResult;

use super::status::QueueStatus;
use super::validation::validate_range;

/// A queue entry representing a session waiting to be merged.
///
/// This is a value object - immutable and validated on construction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueEntry {
    /// Unique identifier
    pub id: QueueEntryId,
    /// Session name
    pub session: SessionName,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When enqueued
    pub enqueued_at: DateTime<Utc>,
    /// Current status
    pub status: QueueStatus,
}

impl QueueEntry {
    /// Create a new queue entry with validation.
    ///
    /// # Errors
    /// Returns `ValidationError` if:
    /// - The priority exceeds `MAX_PRIORITY`
    /// - The ID is invalid (via `QueueEntryId::new`)
    /// - The session name is invalid (via `SessionName::new`)
    pub fn new(
        id: impl Into<String>,
        session: impl Into<String>,
        priority: u32,
    ) -> ValidationResult<Self> {
        let id = QueueEntryId::new(id)?;
        let session = SessionName::new(session)?;

        validate_range(priority, 0, super::status::MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        })
    }

    /// Create a new queue entry from validated identifiers.
    ///
    /// # Errors
    /// Returns `ValidationError` if priority is out of range.
    pub fn from_identifiers(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, super::status::MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        })
    }

    /// Create a new queue entry with explicit timestamp (for testing/rehydration).
    ///
    /// # Errors
    /// Returns `ValidationError` if validation fails.
    pub fn with_timestamp(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
        enqueued_at: DateTime<Utc>,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, super::status::MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at,
            status: QueueStatus::Pending,
        })
    }

    /// Create a queue entry with a specific status (for rehydration).
    ///
    /// # Errors
    /// Returns `ValidationError` if priority is out of range.
    pub fn with_status(
        id: QueueEntryId,
        session: SessionName,
        priority: u32,
        enqueued_at: DateTime<Utc>,
        status: QueueStatus,
    ) -> ValidationResult<Self> {
        validate_range(priority, 0, super::status::MAX_PRIORITY, "priority")?;

        Ok(Self {
            id,
            session,
            priority,
            enqueued_at,
            status,
        })
    }

    /// Transition to a new status, returning a new entry.
    ///
    /// # Errors
    /// Returns `ValidationError` if the transition is invalid.
    pub fn transition_status(self, new_status: QueueStatus) -> ValidationResult<Self> {
        self.status
            .transition_to(new_status)
            .map(|status| QueueEntry { status, ..self })
    }

    /// Update the priority, returning a new entry.
    ///
    /// # Errors
    /// Returns `ValidationError` if the priority is out of range.
    pub fn with_priority(self, priority: u32) -> ValidationResult<Self> {
        validate_range(priority, 0, super::status::MAX_PRIORITY, "priority")?;
        Ok(QueueEntry { priority, ..self })
    }
}

/// Partial equality for QueueEntry (ignores timestamp)
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.session == other.session
            && self.priority == other.priority
            && self.status == other.status
    }
}

impl Eq for QueueEntry {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::{QueueEntryId, SessionName};
    use crate::domain::queue::status::MAX_PRIORITY;
    use crate::domain::validation::ValidationError;

    #[test]
    fn queue_entry_new_valid() {
        let entry = QueueEntry::new("test-1", "session-1", 50);
        assert!(entry.is_ok());
        let e = entry.unwrap();
        assert_eq!(e.id.as_str(), "test-1");
        assert_eq!(e.session.as_str(), "session-1");
        assert_eq!(e.priority, 50);
        assert_eq!(e.status, QueueStatus::Pending);
    }

    #[test]
    fn queue_entry_new_rejects_empty_id() {
        let result = QueueEntry::new("", "session-1", 50);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_new_rejects_empty_session() {
        let result = QueueEntry::new("id-1", "", 50);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_new_rejects_session_with_metacharacters() {
        let result = QueueEntry::new("id-1", "ses$ion", 50);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_new_rejects_priority_exceeding_max() {
        let result = QueueEntry::new("id-1", "session-1", MAX_PRIORITY + 1);
        assert!(matches!(
            result,
            Err(ValidationError::ExceedsMaximum { .. })
        ));
    }

    #[test]
    fn queue_entry_new_accepts_max_priority() {
        let result = QueueEntry::new("id-1", "session-1", MAX_PRIORITY);
        assert!(result.is_ok());
    }

    #[test]
    fn queue_entry_new_accepts_zero_priority() {
        let result = QueueEntry::new("id-1", "session-1", 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().priority, 0);
    }

    #[test]
    fn queue_entry_from_identifiers() {
        let id = QueueEntryId::new("test-id").unwrap();
        let session = SessionName::new("test-session").unwrap();
        let entry = QueueEntry::from_identifiers(id.clone(), session.clone(), 30);
        assert!(entry.is_ok());
        let e = entry.unwrap();
        assert_eq!(e.id.as_str(), "test-id");
        assert_eq!(e.session.as_str(), "test-session");
    }

    #[test]
    fn queue_entry_with_timestamp() {
        let ts = Utc::now();
        let id = QueueEntryId::new("id").unwrap();
        let session = SessionName::new("session").unwrap();
        let entry = QueueEntry::with_timestamp(id, session, 10, ts).unwrap();
        assert_eq!(entry.enqueued_at, ts);
    }

    #[test]
    fn queue_entry_with_status() {
        let id = QueueEntryId::new("id").unwrap();
        let session = SessionName::new("session").unwrap();
        let entry =
            QueueEntry::with_status(id, session, 10, Utc::now(), QueueStatus::Merged).unwrap();
        assert_eq!(entry.status, QueueStatus::Merged);
    }

    #[test]
    fn queue_entry_transition_status_valid() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let claimed = entry.transition_status(QueueStatus::Claimed);
        assert!(claimed.is_ok());
        assert_eq!(claimed.unwrap().status, QueueStatus::Claimed);
    }

    #[test]
    fn queue_entry_transition_status_invalid() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let result = entry.transition_status(QueueStatus::Merged);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_with_priority_valid() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let updated = entry.with_priority(50).unwrap();
        assert_eq!(updated.priority, 50);
    }

    #[test]
    fn queue_entry_with_priority_invalid() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let result = entry.with_priority(MAX_PRIORITY + 1);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_equality_ignores_timestamp() {
        let id1 = QueueEntryId::new("id-1").unwrap();
        let session1 = SessionName::new("session-1").unwrap();
        let ts1 = Utc::now();
        let entry_a = QueueEntry::with_timestamp(id1.clone(), session1.clone(), 10, ts1).unwrap();
        let ts2 = ts1 + chrono::Duration::seconds(60);
        let entry_b = QueueEntry::with_timestamp(id1, session1, 10, ts2).unwrap();
        assert_eq!(
            entry_a, entry_b,
            "Entries with same fields but different timestamps should be equal"
        );
    }

    #[test]
    fn queue_entry_equality_differs_on_id() {
        let a = QueueEntry::new("id-a", "session-1", 10).unwrap();
        let b = QueueEntry::new("id-b", "session-1", 10).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn queue_entry_equality_differs_on_priority() {
        let a = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let b = QueueEntry::new("id-1", "session-1", 20).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn queue_entry_serde_roundtrip() {
        let entry = QueueEntry::new("serde-id", "serde-session", 42).unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let back: QueueEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id.as_str(), "serde-id");
        assert_eq!(back.session.as_str(), "serde-session");
        assert_eq!(back.priority, 42);
    }

    #[test]
    fn queue_entry_trimmed_session() {
        let entry = QueueEntry::new("id-1", "  spaced-session  ", 10).unwrap();
        assert_eq!(entry.session.as_str(), "spaced-session");
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn queue_entry_new_with_max_priority() {
        let entry = QueueEntry::new("id-max", "session", MAX_PRIORITY);
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().priority, MAX_PRIORITY);
    }

    #[test]
    fn queue_entry_with_priority_max() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let updated = entry.with_priority(MAX_PRIORITY).unwrap();
        assert_eq!(updated.priority, MAX_PRIORITY);
    }

    #[test]
    fn queue_entry_with_priority_zero() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let updated = entry.with_priority(0).unwrap();
        assert_eq!(updated.priority, 0);
    }

    #[test]
    fn queue_entry_transition_to_cancelled() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let cancelled = entry.transition_status(QueueStatus::Cancelled);
        assert!(cancelled.is_ok());
        assert_eq!(cancelled.unwrap().status, QueueStatus::Cancelled);
    }

    #[test]
    fn queue_entry_transition_chain_to_failed_retryable() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let result = entry
            .transition_status(QueueStatus::Claimed)
            .and_then(|e| e.transition_status(QueueStatus::Rebasing))
            .and_then(|e| e.transition_status(QueueStatus::Testing))
            .and_then(|e| e.transition_status(QueueStatus::FailedRetryable));
        assert!(result.is_ok());
    }

    #[test]
    fn queue_entry_transition_chain_to_failed_terminal() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let result = entry
            .transition_status(QueueStatus::Claimed)
            .and_then(|e| e.transition_status(QueueStatus::Rebasing))
            .and_then(|e| e.transition_status(QueueStatus::Testing))
            .and_then(|e| e.transition_status(QueueStatus::FailedTerminal));
        assert!(result.is_ok());
    }

    #[test]
    fn queue_entry_new_rejects_whitespace_id() {
        let result = QueueEntry::new("   ", "session-1", 10);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_new_rejects_whitespace_session() {
        let result = QueueEntry::new("id-1", "   ", 10);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_from_identifiers_rejects_invalid_priority() {
        let id = QueueEntryId::new("test-id").unwrap();
        let session = SessionName::new("test-session").unwrap();
        let result = QueueEntry::from_identifiers(id, session, MAX_PRIORITY + 1);
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_with_timestamp_rejects_invalid_priority() {
        let id = QueueEntryId::new("id").unwrap();
        let session = SessionName::new("session").unwrap();
        let result = QueueEntry::with_timestamp(id, session, MAX_PRIORITY + 1, Utc::now());
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_with_status_rejects_invalid_priority() {
        let id = QueueEntryId::new("id").unwrap();
        let session = SessionName::new("session").unwrap();
        let result = QueueEntry::with_status(
            id,
            session,
            MAX_PRIORITY + 1,
            Utc::now(),
            QueueStatus::Merged,
        );
        assert!(result.is_err());
    }

    #[test]
    fn queue_entry_equality_differs_on_session() {
        let a = QueueEntry::new("id-1", "session-a", 10).unwrap();
        let b = QueueEntry::new("id-1", "session-b", 10).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn queue_entry_equality_differs_on_status() {
        let a = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let b = a.clone().transition_status(QueueStatus::Claimed).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn queue_entry_serde_roundtrip_with_status() {
        let entry = QueueEntry::new("serde-id", "serde-session", 42).unwrap();
        let claimed = entry.transition_status(QueueStatus::Claimed).unwrap();
        let json = serde_json::to_string(&claimed).unwrap();
        let back: QueueEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, QueueStatus::Claimed);
        assert_eq!(back.priority, 42);
    }

    #[test]
    fn queue_entry_with_status_all_statuses() {
        let id = QueueEntryId::new("id").unwrap();
        let session = SessionName::new("session").unwrap();
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
            let entry =
                QueueEntry::with_status(id.clone(), session.clone(), 10, Utc::now(), *status);
            assert!(
                entry.is_ok(),
                "Should create entry with status {:?}",
                status
            );
        }
    }

    #[test]
    fn queue_entry_clone_preserves_all_fields() {
        let entry = QueueEntry::new("id-1", "session-1", 42).unwrap();
        let cloned = entry.clone();
        assert_eq!(entry, cloned);
    }

    #[test]
    fn queue_entry_debug_not_empty() {
        let entry = QueueEntry::new("id-1", "session-1", 10).unwrap();
        let debug = format!("{entry:?}");
        assert!(!debug.is_empty());
    }
}

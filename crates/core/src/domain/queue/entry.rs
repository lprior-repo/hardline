//! Queue entry - Value object representing a session in the merge queue
//!
//! This is a value object - immutable and validated on construction.

use chrono::{DateTime, Utc};

use crate::domain::identifiers::{QueueEntryId, SessionName};
use crate::domain::queue::status::{QueueStatus, MAX_PRIORITY};
use crate::domain::validation::ValidationResult;

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
        let session = SessionName::parse(session)?;

        super::validation::validate_range(priority, 0, MAX_PRIORITY, "priority")?;

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
        super::validation::validate_range(priority, 0, MAX_PRIORITY, "priority")?;

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
        super::validation::validate_range(priority, 0, MAX_PRIORITY, "priority")?;

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
        super::validation::validate_range(priority, 0, MAX_PRIORITY, "priority")?;

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
        super::validation::validate_range(priority, 0, MAX_PRIORITY, "priority")?;
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

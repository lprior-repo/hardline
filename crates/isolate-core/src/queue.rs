//! Queue management - Merge queue types from Stak
//!
//! This module provides queue management for the merge queue functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Queue entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    Pending,
    Claimed,
    Rebasing,
    Testing,
    ReadyToMerge,
    Merging,
    Merged,
    FailedRetryable,
    FailedTerminal,
    Cancelled,
}

impl QueueStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::FailedRetryable | Self::FailedTerminal)
    }

    pub fn transition_to(&self, new: Self) -> Result<Self, Error> {
        match (self, new) {
            // Valid transitions
            (Self::Pending, Self::Claimed) => Ok(new),
            (Self::Pending, Self::Cancelled) => Ok(new),
            (Self::Claimed, Self::Rebasing) => Ok(new),
            (Self::Rebasing, Self::Testing) => Ok(new),
            (Self::Testing, Self::ReadyToMerge) => Ok(new),
            (Self::ReadyToMerge, Self::Merging) => Ok(new),
            (Self::Merging, Self::Merged) => Ok(new),
            (Self::Testing, Self::FailedRetryable) => Ok(new),
            (Self::Testing, Self::FailedTerminal) => Ok(new),
            (Self::Claimed, Self::FailedRetryable) => Ok(new),
            (Self::Pending, Self::FailedRetryable) => Ok(new),
            // Invalid transitions
            _ => Err(Error::InvalidState(format!(
                "Invalid transition from {:?} to {:?}",
                self, new
            ))),
        }
    }
}

/// Queue entry identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.is_empty() {
            Err(Error::InvalidId("QueueEntryId cannot be empty".into()))
        } else {
            Ok(Self(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for QueueEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if name.is_empty() {
            Err(Error::InvalidInput("Session name cannot be empty".into()))
        } else {
            Ok(Self(name))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Maximum priority value
pub const MAX_PRIORITY: u32 = 100;

/// Queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub session: SessionName,
    pub priority: u32,
    pub enqueued_at: DateTime<Utc>,
    pub status: QueueStatus,
}

impl QueueEntry {
    pub fn new(id: QueueEntryId, session: SessionName, priority: u32) -> Result<Self, Error> {
        if priority > MAX_PRIORITY {
            return Err(Error::InvalidInput(format!(
                "Priority {} exceeds max {}",
                priority, MAX_PRIORITY
            )));
        }
        Ok(Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        })
    }
}

/// Queue
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Queue {
    entries: Vec<QueueEntry>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn enqueue(&mut self, entry: QueueEntry) {
        let priority = entry.priority;
        let pos = self.entries.iter().position(|e| e.priority > priority);
        match pos {
            Some(idx) => self.entries.insert(idx, entry),
            None => self.entries.push(entry),
        }
    }

    pub fn dequeue(&mut self, id: &QueueEntryId) -> Option<QueueEntry> {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.id.as_str() == id.as_str())
        {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    pub fn find(&self, id: &QueueEntryId) -> Option<&QueueEntry> {
        self.entries.iter().find(|e| e.id.as_str() == id.as_str())
    }

    pub fn find_by_session(&self, session: &SessionName) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|e| e.session.as_str() == session.as_str())
    }

    pub fn next_pending(&self) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|e| e.status == QueueStatus::Pending)
    }

    pub fn entries(&self) -> &[QueueEntry] {
        &self.entries
    }

    pub fn insert(&mut self, position: usize, entry: QueueEntry) -> Result<(), Error> {
        if position > self.entries.len() {
            return Err(Error::InvalidInput(format!(
                "Position {} out of bounds",
                position
            )));
        }
        self.entries.insert(position, entry);
        Ok(())
    }
}

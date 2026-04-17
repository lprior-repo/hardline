//! Event metadata structures
//!
//! Provides metadata for events stored in the event store.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DomainEvent;

/// Metadata for an event in the event store
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event in the store
    pub event_number: i64,
    /// Stream identifier (e.g., "session-123")
    pub stream_id: String,
    /// Stream version (incrementing counter)
    pub stream_version: i64,
    /// When the event was stored
    pub stored_at: DateTime<Utc>,
}

/// A stored event with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredEvent {
    /// The domain event
    pub event: DomainEvent,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl StoredEvent {
    /// Create a new stored event
    #[must_use]
    pub const fn new(event: DomainEvent, metadata: EventMetadata) -> Self {
        Self { event, metadata }
    }

    /// Get the event number
    #[must_use]
    pub const fn event_number(&self) -> i64 {
        self.metadata.event_number
    }

    /// Get the stream identifier
    #[must_use]
    pub fn stream_id(&self) -> &str {
        &self.metadata.stream_id
    }

    /// Get the stream version
    #[must_use]
    pub const fn stream_version(&self) -> i64 {
        self.metadata.stream_version
    }
}

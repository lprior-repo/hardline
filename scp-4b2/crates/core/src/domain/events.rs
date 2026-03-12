//! Domain events module
//!
//! This module implements Domain-Driven Design event sourcing patterns.
//! Domain events represent important business events that have occurred in the system.
//!
//! # Design Principles
//!
//! - **Immutable**: Events cannot be modified after creation
//! - **Serializable**: All events can be serialized for persistence and transmission
//! - **Typed**: Each event carries specific, validated domain data
//! - **Timestamped**: All events include when they occurred
//! - **Pure**: Event creation is deterministic and side-effect free
//!
//! # Usage
//!
//! ```rust
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use chrono::Utc;
//! use isolate_core::domain::{events::DomainEvent, identifiers::SessionName};
//!
//! let event = DomainEvent::session_created(
//!     "session-123".to_string(),
//!     SessionName::parse("my-session")?,
//!     Utc::now(),
//! );
//! # Ok(())
//! # }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::identifiers::{BeadId, SessionName, WorkspaceName};

// ============================================================================
// Domain Event Enum
// ============================================================================

/// A domain event representing something important that happened.
///
/// Events are the single source of truth for state changes in the system.
/// They enable:
/// - Event sourcing (rebuilding state from event history)
/// - Audit logging (complete history of all changes)
/// - Projections (deriving read models from event stream)
/// - Integration (publishing events to external systems)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum DomainEvent {
    /// A new session was created
    SessionCreated(Box<SessionCreatedEvent>),

    /// A session was completed successfully
    SessionCompleted(Box<SessionCompletedEvent>),

    /// A session failed
    SessionFailed(Box<SessionFailedEvent>),

    /// A workspace was created
    WorkspaceCreated(Box<WorkspaceCreatedEvent>),

    /// A workspace was removed
    WorkspaceRemoved(Box<WorkspaceRemovedEvent>),

    /// A bead (task/issue) was created
    BeadCreated(Box<BeadCreatedEvent>),

    /// A bead was closed
    BeadClosed(Box<BeadClosedEvent>),
}

impl DomainEvent {
    /// Get the timestamp for when this event occurred
    #[must_use]
    pub const fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::SessionCreated(e) => &e.timestamp,
            Self::SessionCompleted(e) => &e.timestamp,
            Self::SessionFailed(e) => &e.timestamp,
            Self::WorkspaceCreated(e) => &e.timestamp,
            Self::WorkspaceRemoved(e) => &e.timestamp,
            Self::BeadCreated(e) => &e.timestamp,
            Self::BeadClosed(e) => &e.timestamp,
        }
    }

    /// Get the event type as a string
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::SessionCreated(_) => "session_created",
            Self::SessionCompleted(_) => "session_completed",
            Self::SessionFailed(_) => "session_failed",
            Self::WorkspaceCreated(_) => "workspace_created",
            Self::WorkspaceRemoved(_) => "workspace_removed",
            Self::BeadCreated(_) => "bead_created",
            Self::BeadClosed(_) => "bead_closed",
        }
    }

    /// Create a session created event
    #[must_use]
    pub fn session_created(
        session_id: String,
        session_name: SessionName,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionCreated(Box::new(SessionCreatedEvent {
            session_id,
            session_name,
            timestamp,
        }))
    }

    /// Create a session completed event
    #[must_use]
    pub fn session_completed(
        session_id: String,
        session_name: SessionName,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionCompleted(Box::new(SessionCompletedEvent {
            session_id,
            session_name,
            timestamp,
        }))
    }

    /// Create a session failed event
    #[must_use]
    pub fn session_failed(
        session_id: String,
        session_name: SessionName,
        reason: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionFailed(Box::new(SessionFailedEvent {
            session_id,
            session_name,
            reason,
            timestamp,
        }))
    }

    /// Create a workspace created event
    #[must_use]
    pub fn workspace_created(
        workspace_name: WorkspaceName,
        path: PathBuf,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::WorkspaceCreated(Box::new(WorkspaceCreatedEvent {
            workspace_name,
            path,
            timestamp,
        }))
    }

    /// Create a workspace removed event
    #[must_use]
    pub fn workspace_removed(
        workspace_name: WorkspaceName,
        path: PathBuf,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::WorkspaceRemoved(Box::new(WorkspaceRemovedEvent {
            workspace_name,
            path,
            timestamp,
        }))
    }

    /// Create a bead created event
    #[must_use]
    pub fn bead_created(
        bead_id: BeadId,
        title: String,
        description: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::BeadCreated(Box::new(BeadCreatedEvent {
            bead_id,
            title,
            description,
            timestamp,
        }))
    }

    /// Create a bead closed event
    #[must_use]
    pub fn bead_closed(
        bead_id: BeadId,
        closed_at: DateTime<Utc>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::BeadClosed(Box::new(BeadClosedEvent {
            bead_id,
            closed_at,
            timestamp,
        }))
    }
}

// ============================================================================
// Event Types
// ============================================================================

/// Event emitted when a new session is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCreatedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// When the session was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a session is completed successfully
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCompletedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// When the session was completed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a session fails
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFailedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// Human-readable reason for the failure
    pub reason: String,
    /// When the session failed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a workspace is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCreatedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path to the workspace on disk
    pub path: PathBuf,
    /// When the workspace was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a workspace is removed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRemovedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path where the workspace was located
    pub path: PathBuf,
    /// When the workspace was removed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a bead (task/issue) is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadCreatedEvent {
    /// Unique identifier for the bead
    pub bead_id: BeadId,
    /// Title of the bead
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// When the bead was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a bead is closed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadClosedEvent {
    /// Unique identifier for the bead
    pub bead_id: BeadId,
    /// When the bead was closed
    pub closed_at: DateTime<Utc>,
    /// When this event was emitted
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Event Metadata
// ============================================================================

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

// ============================================================================
// Event Serialization
// ============================================================================

/// Serialize an event to JSON
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event(event: &DomainEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(event)
}

/// Deserialize an event from JSON
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event(json: &str) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize an event to JSON bytes
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event_bytes(event: &DomainEvent) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(event)
}

/// Deserialize an event from JSON bytes
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event_bytes(bytes: &[u8]) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_slice(bytes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_created");
        assert_eq!(event.timestamp(), &timestamp);

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_session_completed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_completed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_completed");
    }

    #[test]
    fn test_session_failed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_failed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            "Out of memory".to_string(),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_failed");

        // Verify the event contains the failure reason
        if let DomainEvent::SessionFailed(e) = &event {
            assert_eq!(e.reason, "Out of memory");
        } else {
            panic!("Expected SessionFailed event");
        }
    }

    #[test]
    fn test_workspace_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_created(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_created");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_workspace_removed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_removed(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_removed");
    }

    #[test]
    fn test_bead_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::bead_created(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            "Fix the bug".to_string(),
            Some("Critical issue".to_string()),
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_created");

        // Verify the event contains the bead data
        if let DomainEvent::BeadCreated(e) = &event {
            assert_eq!(e.title, "Fix the bug");
            assert_eq!(e.description, Some("Critical issue".to_string()));
        } else {
            panic!("Expected BeadCreated event");
        }
    }

    #[test]
    fn test_bead_closed_event() {
        let timestamp = Utc::now();
        let closed_at = timestamp;

        let event = DomainEvent::bead_closed(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            closed_at,
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_closed");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let events = vec![
            DomainEvent::session_created(
                "session-123".to_string(),
                SessionName::parse("my-session").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("my-workspace").expect("valid"),
                PathBuf::from("/tmp/workspace"),
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc123").expect("valid"),
                "Test bead".to_string(),
                None,
                Utc::now(),
            ),
        ];

        for event in events {
            // Test JSON serialization
            let json = serialize_event(&event).expect("serialization failed");
            let deserialized = deserialize_event(&json).expect("deserialization failed");
            assert_eq!(event, deserialized);

            // Test bytes serialization
            let bytes = serialize_event_bytes(&event).expect("serialization failed");
            let deserialized_bytes =
                deserialize_event_bytes(&bytes).expect("deserialization failed");
            assert_eq!(event, deserialized_bytes);
        }
    }

    #[test]
    fn test_all_event_types_have_unique_types() {
        let events = [
            DomainEvent::session_created(
                "s1".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_completed(
                "s2".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_failed(
                "s3".to_string(),
                SessionName::parse("s").expect("valid"),
                "error".to_string(),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::workspace_removed(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc").expect("valid"),
                "t".to_string(),
                None,
                Utc::now(),
            ),
            DomainEvent::bead_closed(
                BeadId::parse("bd-abc").expect("valid"),
                Utc::now(),
                Utc::now(),
            ),
        ];

        let event_types: Vec<&str> = events.iter().map(super::DomainEvent::event_type).collect();

        // Check that all event types are unique
        let unique_types: std::collections::HashSet<_> = event_types.iter().collect();
        assert_eq!(
            unique_types.len(),
            event_types.len(),
            "Event types should be unique"
        );
    }
}

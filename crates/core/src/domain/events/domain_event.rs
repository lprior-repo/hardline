//! Domain event enum and implementations
//!
//! This module defines the main [`DomainEvent`] enum which represents
//! all domain events in the system.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    bead_events::{BeadClosedEvent, BeadCreatedEvent},
    session_events::{SessionCompletedEvent, SessionCreatedEvent, SessionFailedEvent},
    workspace_events::{WorkspaceCreatedEvent, WorkspaceRemovedEvent},
};
use crate::domain::identifiers::{BeadId, SessionName, WorkspaceName};

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

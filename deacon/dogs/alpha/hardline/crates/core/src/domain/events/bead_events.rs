//! Bead-related domain events
//!
//! Events emitted during bead (task/issue) lifecycle:
//! - [`BeadCreatedEvent`] - A bead was created
//! - [`BeadClosedEvent`] - A bead was closed

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::identifiers::BeadId;

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

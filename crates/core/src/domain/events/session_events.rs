//! Session-related domain events
//!
//! Events emitted during session lifecycle:
//! - [`SessionCreatedEvent`] - A new session was created
//! - [`SessionCompletedEvent`] - A session was completed successfully
//! - [`SessionFailedEvent`] - A session failed

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::identifiers::SessionName;

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

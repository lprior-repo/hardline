//! Session creation types - Data layer
//!
//! Defines the input/output types for session creation.
//!
//! # Data Architecture
//!
//! These types represent the DATA tier - inert, serializable, comparable.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

use crate::{
    domain::{
        identifiers::{AbsolutePath, SessionId, SessionName},
        session::BranchState,
    },
    output::ValidatedMetadata,
    types::SessionStatus,
    WorkspaceState,
};

/// Input for session creation
///
/// This is the input data structure for creating a session.
/// All fields are pre-validated by their respective newtype constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCreateInput {
    /// Unique session identifier (pre-validated by `SessionId::parse`)
    pub id: SessionId,
    /// Human-readable session name (pre-validated by `SessionName::parse`)
    pub name: SessionName,
    /// Branch state for the session
    pub branch: BranchState,
    /// Absolute path to workspace directory (pre-validated by `AbsolutePath::parse`)
    pub workspace_path: AbsolutePath,
}

/// Output from successful session creation
///
/// Contains the created session and metadata about the creation.
#[derive(Debug, Clone)]
pub struct SessionCreateOutput {
    /// The created session entity
    pub session: crate::types::Session,
    /// When the session was created
    pub created_at: DateTime<Utc>,
}

/// Configuration for session creation limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLimits {
    /// Maximum number of sessions allowed (default 100)
    pub max_sessions: usize,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self { max_sessions: 100 }
    }
}

impl SessionLimits {
    /// Create new limits with a custom max
    #[must_use]
    pub const fn new(max_sessions: usize) -> Self {
        Self { max_sessions }
    }
}

//! State enums - replaces boolean flags and optional fields
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Make illegal states unrepresentable** - Enums replace `bool` and `Option`
//! - **Explicit state representation** - Each state variant has exactly valid fields

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use super::ConflictDetail;
use crate::domain::SessionName;

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS THAT REPLACE BOOLEAN FLAGS - Make illegal states unrepresentable
// ═══════════════════════════════════════════════════════════════════════════

/// Recovery capability - replaces `recoverable: bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCapability {
    /// The issue can be recovered with a recommended action
    Recoverable { recommended_action: String },
    /// The issue cannot be recovered (requires manual intervention)
    NotRecoverable { reason: String },
}

/// Execution mode - replaces `automatic: bool`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Step executes automatically
    Automatic,
    /// Step requires manual execution
    Manual,
}

/// Merge status - replaces `merge_safe: bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeAnalysis {
    /// Whether the merge is safe
    pub safe: bool,
    /// Conflicts if not safe (empty if safe)
    pub conflicts: Vec<ConflictDetail>,
}

/// Outcome - replaces `success: bool`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
}

impl Outcome {
    /// Convert from boolean (for backward compatibility during migration)
    #[must_use]
    pub const fn from_bool(success: bool) -> Self {
        if success {
            Self::Success
        } else {
            Self::Failure
        }
    }

    /// Convert to boolean (for backward compatibility during migration)
    #[must_use]
    pub const fn to_bool(self) -> bool {
        match self {
            Self::Success => true,
            Self::Failure => false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS THAT REPLACE OPTION FIELDS - Explicit state representation
// ═══════════════════════════════════════════════════════════════════════════

/// Issue scope - replaces `session: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueScope {
    /// Issue is not associated with a session
    Standalone,
    /// Issue is associated with a specific session
    InSession { session: SessionName },
}

impl IssueScope {
    /// Get the session name if this is an `InSession` scope
    #[must_use]
    pub const fn session(&self) -> Option<&SessionName> {
        match self {
            Self::Standalone => None,
            Self::InSession { session } => Some(session),
        }
    }

    /// Create a standalone scope
    #[must_use]
    pub const fn standalone() -> Self {
        Self::Standalone
    }

    /// Create an `InSession` scope
    #[must_use]
    pub const fn in_session(session: SessionName) -> Self {
        Self::InSession { session }
    }

    /// Check if this is a standalone scope (for serde skip condition)
    #[must_use]
    pub const fn is_standalone(&self) -> bool {
        matches!(self, Self::Standalone)
    }
}

/// Action result - replaces `result: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult {
    /// Action is still pending
    Pending,
    /// Action completed with a result
    Completed { result: String },
}

impl ActionResult {
    /// Get the result if completed
    #[must_use]
    pub const fn result(&self) -> Option<&str> {
        match self {
            Self::Pending => None,
            Self::Completed { result } => Some(result.as_str()),
        }
    }

    /// Create a pending result
    #[must_use]
    pub const fn pending() -> Self {
        Self::Pending
    }

    /// Create a completed result
    #[must_use]
    pub fn completed(result: impl Into<String>) -> Self {
        Self::Completed {
            result: result.into(),
        }
    }

    /// Check if result is pending (for serde skip condition)
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// Recovery execution - replaces `command: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryExecution {
    /// Automatic execution with a command
    Automatic { command: super::Command },
    /// Manual execution required
    Manual,
}

impl RecoveryExecution {
    /// Get the command if this is automatic execution
    #[must_use]
    pub const fn command(&self) -> Option<&super::Command> {
        match self {
            Self::Automatic { command } => Some(command),
            Self::Manual => None,
        }
    }

    /// Create an automatic execution
    #[must_use]
    pub fn automatic(command: impl Into<String>) -> Self {
        Self::Automatic {
            command: super::Command::new(command),
        }
    }

    /// Create a manual execution
    #[must_use]
    pub const fn manual() -> Self {
        Self::Manual
    }

    /// Check if this is automatic execution
    #[must_use]
    pub const fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic { .. })
    }
}

/// Bead attachment - replaces `bead: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadAttachment {
    /// No bead attached
    None,
    /// Bead attached by ID
    Attached { bead_id: crate::domain::BeadId },
}

impl BeadAttachment {
    /// Get the bead ID if attached
    #[must_use]
    pub const fn bead_id(&self) -> Option<&crate::domain::BeadId> {
        match self {
            Self::None => None,
            Self::Attached { bead_id } => Some(bead_id),
        }
    }

    /// Create no attachment
    #[must_use]
    pub const fn none() -> Self {
        Self::None
    }

    /// Create an attachment
    #[must_use]
    pub const fn attached(bead_id: crate::domain::BeadId) -> Self {
        Self::Attached { bead_id }
    }
}

/// Agent assignment - replaces `agent: Option<String>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentAssignment {
    /// No agent assigned
    Unassigned,
    /// Agent assigned by ID
    Assigned { agent_id: String },
}

impl AgentAssignment {
    /// Get the agent ID if assigned
    #[must_use]
    pub const fn agent_id(&self) -> Option<&str> {
        match self {
            Self::Unassigned => None,
            Self::Assigned { agent_id } => Some(agent_id.as_str()),
        }
    }

    /// Create unassigned state
    #[must_use]
    pub const fn unassigned() -> Self {
        Self::Unassigned
    }

    /// Create an assigned state
    #[must_use]
    pub fn assigned(agent_id: impl Into<String>) -> Self {
        Self::Assigned {
            agent_id: agent_id.into(),
        }
    }
}

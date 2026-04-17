//! Agent Registry Domain Module
//!
//! Provides types for tracking active agents, detecting dead agents via heartbeat
//! timeout, capability registry, and agent state queries.
//!
//! # Architecture
//!
//! - [`Agent`] - Core entity representing a registered agent
//! - [`AgentStatus`] - Agent lifecycle state (Active, Idle, Disconnected, Registering)
//! - [`Capability`] - Skills/abilities an agent advertises
//! - [`Heartbeat`] - Periodic health check message
//! - [`AgentEvent`] - Observable events emitted by the registry
//! - [`AgentRepository`] - Persistence abstraction trait
//!
//! # Heartbeat Timeout
//!
//! Agents must send heartbeats every 30 seconds. After 90 seconds without a heartbeat,
//! an agent is considered disconnected.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::identifiers::{AgentId, BeadId};

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Workspace identifier for agent registry.
///
/// Note: This is a local definition. The broader system may use different
/// workspace ID types from other crates (workspace, snapshot, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// ERRORS
// ============================================================================

/// Domain errors for agent registry operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AgentRegistryError {
    /// Agent not found in registry
    #[error("agent not found: {0}")]
    AgentNotFound(AgentId),

    /// Agent already registered
    #[error("agent already registered: {0}")]
    AgentAlreadyExists(AgentId),

    /// Invalid agent state transition
    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        /// Current state
        from: status::AgentStatus,
        /// Attempted target state
        to: status::AgentStatus,
    },

    /// Heartbeat timeout
    #[error("agent {0} heartbeat timeout")]
    HeartbeatTimeout(AgentId),

    /// Invalid capability
    #[error("invalid capability: {0}")]
    InvalidCapability(String),
}

/// Result type for agent registry operations
pub type AgentRegistryResult<T> = Result<T, AgentRegistryError>;

// ============================================================================
// RE-EXPORTS FOR BACKWARD COMPATIBILITY
// ============================================================================

// Re-export common types at the module root for backward compatibility
pub use entities::{Agent, AgentMetadata, Heartbeat, HeartbeatConfig};
pub use events::AgentEvent;
pub use status::{AgentStatus, Capability, CapabilityName};
pub use types::SemanticVersion;

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod entities;
pub mod events;
pub mod operations;
pub mod repository;
pub mod status;
pub mod types;

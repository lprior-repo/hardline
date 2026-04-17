//! Agent entity and related types

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use super::{AgentRegistryError, AgentStatus, BeadId, Capability, WorkspaceId};

// ============================================================================
// AGENT METADATA
// ============================================================================

/// Additional metadata about an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub workspace_id: Option<WorkspaceId>,
    pub current_bead: Option<BeadId>,
    pub started_at: Option<DateTime<Utc>>,
    pub pid: Option<u32>,
    pub version: String,
}

impl Default for AgentMetadata {
    fn default() -> Self {
        Self {
            workspace_id: None,
            current_bead: None,
            started_at: None,
            pid: None,
            version: String::new(),
        }
    }
}

// ============================================================================
// AGENT
// ============================================================================

/// Agent entity representing a registered agent in the system.
///
/// Agents are the primary actors in the hardline system - they execute
/// beads, manage workspaces, and coordinate with each other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: super::AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub status: AgentStatus,
    pub last_heartbeat_at: DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

impl Agent {
    /// Create a new agent in Registering state
    #[must_use]
    pub fn new(id: super::AgentId, name: String, capabilities: Vec<Capability>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            capabilities,
            status: AgentStatus::Registering,
            last_heartbeat_at: now,
            registered_at: now,
            metadata: AgentMetadata::default(),
        }
    }

    /// Check if agent heartbeat is stale (older than cutoff)
    #[must_use]
    pub fn is_stale(&self, cutoff: DateTime<Utc>) -> bool {
        self.last_heartbeat_at < cutoff
    }

    /// Check if agent is available for work
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.status.is_available()
    }

    /// Check if agent can transition to target status
    #[must_use]
    pub fn can_transition_to(&self, target: AgentStatus) -> bool {
        use AgentStatus::{Active, Disconnected, Idle, Registering};

        match (&self.status, target) {
            // From Registering, can go to Active or Idle
            (Registering, Active | Idle) => true,
            // From Active/Idle, can go to Disconnected
            (Active | Idle, Disconnected) => true,
            // From Disconnected, must re-register (go to Registering)
            (Disconnected, Registering) => true,
            // Same status is allowed (no-op)
            (s, t) if *s == t => true,
            // Active <-> Idle transitions are allowed
            (Active, Idle) | (Idle, Active) => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Transition to a new status, returning error if invalid
    pub fn transition_to(&mut self, target: AgentStatus) -> AgentRegistryError {
        if self.can_transition_to(target) {
            self.status = target;
            Ok(())
        } else {
            Err(AgentRegistryError::InvalidStateTransition {
                from: self.status,
                to: target,
            })
        }
    }

    /// Update heartbeat information
    pub fn update_heartbeat(
        &mut self,
        timestamp: DateTime<Utc>,
        status: AgentStatus,
        workspace_id: Option<WorkspaceId>,
        bead_id: Option<BeadId>,
    ) -> AgentRegistryError {
        let previous_status = self.status;
        self.last_heartbeat_at = timestamp;
        self.status = status;
        self.metadata.workspace_id = workspace_id;
        self.metadata.current_bead = bead_id;

        if !self.can_transition_to(status) && previous_status != status {
            Err(AgentRegistryError::InvalidStateTransition {
                from: previous_status,
                to: status,
            })
        } else {
            Ok(())
        }
    }
}

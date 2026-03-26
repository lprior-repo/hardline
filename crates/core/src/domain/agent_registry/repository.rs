//! Agent Repository trait for persistence abstraction.
//!
//! Provides a trait interface for agent registry persistence,
//! enabling dependency injection and testability.

use crate::domain::agent_registry::{
    Agent, AgentEvent, AgentRegistryError, AgentRegistryResult, AgentStatus, HeartbeatConfig,
    WorkspaceId,
};
use crate::domain::identifiers::AgentId;
use chrono::{DateTime, Utc};

/// Repository trait for agent registry persistence.
///
/// Abstracts all agent storage operations, enabling:
/// - Dependency injection for business logic
/// - Mock implementations for testing
/// - Swappable backends (SQLite, PostgreSQL, etc.)
pub trait AgentRegistryRepository: Send + Sync {
    /// Save an agent to the repository.
    ///
    /// # Errors
    ///
    /// Returns `AgentAlreadyExists` if agent ID already exists.
    fn save(&self, agent: &Agent) -> AgentRegistryResult<()>;

    /// Find an agent by ID.
    ///
    /// # Errors
    ///
    /// Returns `AgentNotFound` if agent doesn't exist.
    fn find_by_id(&self, id: &AgentId) -> AgentRegistryResult<Option<Agent>>;

    /// Find an agent by name.
    ///
    /// # Errors
    ///
    /// Returns error on storage failure.
    fn find_by_name(&self, name: &str) -> AgentRegistryResult<Option<Agent>>;

    /// List all agents.
    ///
    /// # Errors
    ///
    /// Returns error on storage failure.
    fn list_all(&self) -> AgentRegistryResult<Vec<Agent>>;

    /// List agents by status.
    ///
    /// # Errors
    ///
    /// Returns error on storage failure.
    fn list_by_status(&self, status: AgentStatus) -> AgentRegistryResult<Vec<Agent>>;

    /// List agents in a workspace.
    ///
    /// # Errors
    ///
    /// Returns error on storage failure.
    fn list_by_workspace(&self, workspace_id: &WorkspaceId) -> AgentRegistryResult<Vec<Agent>>;

    /// Find agents with stale heartbeats (older than cutoff).
    ///
    /// # Errors
    ///
    /// Returns error on storage failure.
    fn find_stale_agents(&self, cutoff: DateTime<Utc>) -> AgentRegistryResult<Vec<Agent>>;

    /// Delete an agent from the repository.
    ///
    /// # Errors
    ///
    /// Returns `AgentNotFound` if agent doesn't exist.
    fn delete(&self, id: &AgentId) -> AgentRegistryResult<()>;

    /// Update agent status after heartbeat.
    ///
    /// # Errors
    ///
    /// Returns `AgentNotFound` if agent doesn't exist.
    fn update_heartbeat(
        &self,
        heartbeat: &crate::domain::agent_registry::Heartbeat,
    ) -> AgentRegistryResult<AgentEvent>;

    /// Process heartbeat and return resulting event.
    fn process_heartbeat(
        &self,
        agent: &mut Agent,
        heartbeat: crate::domain::agent_registry::Heartbeat,
    ) -> AgentRegistryResult<AgentEvent> {
        let previous_status = agent.status;
        agent.last_heartbeat_at = heartbeat.timestamp;
        agent.status = heartbeat.status;
        agent.metadata.workspace_id = heartbeat.workspace_id;
        agent.metadata.current_bead = heartbeat.bead_id;

        self.save(agent)?;

        let event = match (previous_status, agent.status) {
            (_, AgentStatus::Active) if previous_status != AgentStatus::Active => {
                AgentEvent::BecameActive {
                    agent_id: agent.id.clone(),
                }
            }
            (_, AgentStatus::Idle) if previous_status != AgentStatus::Idle => {
                AgentEvent::BecameIdle {
                    agent_id: agent.id.clone(),
                }
            }
            (AgentStatus::Active | AgentStatus::Idle, AgentStatus::Disconnected) => {
                AgentEvent::Disconnected {
                    agent_id: agent.id.clone(),
                }
            }
            _ => AgentEvent::HeartbeatReceived {
                agent_id: agent.id.clone(),
            },
        };

        Ok(event)
    }

    /// Cleanup disconnected agents based on heartbeat timeout.
    fn cleanup_disconnected_agents(
        &self,
        config: &HeartbeatConfig,
    ) -> AgentRegistryResult<Vec<AgentEvent>> {
        let cutoff = config.stale_cutoff();
        let disconnected = self.find_stale_agents(cutoff)?;

        let mut events = Vec::new();

        for mut agent in disconnected {
            let previous_status = agent.status;
            agent.status = AgentStatus::Disconnected;
            self.save(&agent)?;

            if previous_status != AgentStatus::Disconnected {
                events.push(AgentEvent::TimedOut {
                    agent_id: agent.id.clone(),
                });
            }
        }

        Ok(events)
    }
}

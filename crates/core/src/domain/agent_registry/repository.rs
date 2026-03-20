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

/// In-memory implementation of AgentRepository for testing.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAgentRepository {
    agents: std::sync::Arc<std::sync::RwLock<Vec<Agent>>>,
}

impl InMemoryAgentRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }
}

impl AgentRegistryRepository for InMemoryAgentRepository {
    fn save(&self, agent: &Agent) -> AgentRegistryResult<()> {
        let mut agents = self.agents.write().expect("lock poisoned");
        if let Some(idx) = agents.iter().position(|a| &a.id == &agent.id) {
            agents[idx] = agent.clone();
        } else {
            agents.push(agent.clone());
        }
        Ok(())
    }

    fn find_by_id(&self, id: &AgentId) -> AgentRegistryResult<Option<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents.iter().find(|a| &a.id == id).cloned())
    }

    fn find_by_name(&self, name: &str) -> AgentRegistryResult<Option<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents.iter().find(|a| a.name == name).cloned())
    }

    fn list_all(&self) -> AgentRegistryResult<Vec<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents.clone())
    }

    fn list_by_status(&self, status: AgentStatus) -> AgentRegistryResult<Vec<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents
            .iter()
            .filter(|a| a.status == status)
            .cloned()
            .collect())
    }

    fn list_by_workspace(&self, workspace_id: &WorkspaceId) -> AgentRegistryResult<Vec<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents
            .iter()
            .filter(|a| a.metadata.workspace_id.as_ref() == Some(workspace_id))
            .cloned()
            .collect())
    }

    fn find_stale_agents(&self, cutoff: DateTime<Utc>) -> AgentRegistryResult<Vec<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents
            .iter()
            .filter(|a| a.is_stale(cutoff))
            .cloned()
            .collect())
    }

    fn delete(&self, id: &AgentId) -> AgentRegistryResult<()> {
        let mut agents = self.agents.write().expect("lock poisoned");
        if let Some(idx) = agents.iter().position(|a| &a.id == id) {
            agents.remove(idx);
            Ok(())
        } else {
            Err(AgentRegistryError::AgentNotFound(id.clone()))
        }
    }

    fn update_heartbeat(
        &self,
        heartbeat: &crate::domain::agent_registry::Heartbeat,
    ) -> AgentRegistryResult<AgentEvent> {
        let mut agents = self.agents.write().expect("lock poisoned");
        if let Some(agent) = agents.iter_mut().find(|a| &a.id == &heartbeat.agent_id) {
            let previous_status = agent.status;
            agent.last_heartbeat_at = heartbeat.timestamp;
            agent.status = heartbeat.status;
            agent.metadata.workspace_id = heartbeat.workspace_id.clone();
            agent.metadata.current_bead = heartbeat.bead_id.clone();

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
        } else {
            Err(AgentRegistryError::AgentNotFound(
                heartbeat.agent_id.clone(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_repository_save_and_find() {
        let repo = InMemoryAgentRepository::new();
        let agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );

        repo.save(&agent).expect("save succeeds");
        let found = repo.find_by_id(&agent.id).expect("find succeeds");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Agent");
    }

    #[test]
    fn test_in_memory_repository_list_by_status() {
        let repo = InMemoryAgentRepository::new();
        let agent1 = Agent::new(
            AgentId::parse("agent-1").unwrap(),
            "Agent 1".to_string(),
            vec![],
        );
        let mut agent2 = Agent::new(
            AgentId::parse("agent-2").unwrap(),
            "Agent 2".to_string(),
            vec![],
        );
        agent2.status = AgentStatus::Active;

        repo.save(&agent1).expect("save succeeds");
        repo.save(&agent2).expect("save succeeds");

        let idle = repo
            .list_by_status(AgentStatus::Idle)
            .expect("list succeeds");
        assert_eq!(idle.len(), 1);
        assert_eq!(idle[0].name, "Agent 1");

        let active = repo
            .list_by_status(AgentStatus::Active)
            .expect("list succeeds");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "Agent 2");
    }

    #[test]
    fn test_in_memory_repository_delete() {
        let repo = InMemoryAgentRepository::new();
        let agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );

        repo.save(&agent).expect("save succeeds");
        repo.delete(&agent.id).expect("delete succeeds");

        let found = repo.find_by_id(&agent.id).expect("find succeeds");
        assert!(found.is_none());
    }

    #[test]
    fn test_heartbeat_config_stale_cutoff() {
        let config = HeartbeatConfig::default();
        let cutoff = config.stale_cutoff();
        assert!(cutoff < Utc::now());
    }
}

//! In-memory agent repository implementation for testing.

use crate::domain::agent_registry::{
    repository::AgentRegistryRepository, Agent, AgentEvent, AgentRegistryError,
    AgentRegistryResult, Heartbeat, HeartbeatConfig,
};
use crate::domain::identifiers::AgentId;
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};

/// In-memory implementation of AgentRepository for testing.
#[derive(Debug, Clone)]
pub struct InMemoryAgentRepository {
    agents: Arc<RwLock<Vec<Agent>>>,
}

impl InMemoryAgentRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryAgentRepository {
    fn default() -> Self {
        Self::new()
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

    fn list_by_status(
        &self,
        status: crate::domain::agent_registry::AgentStatus,
    ) -> AgentRegistryResult<Vec<Agent>> {
        let agents = self.agents.read().expect("lock poisoned");
        Ok(agents
            .iter()
            .filter(|a| a.status == status)
            .cloned()
            .collect())
    }

    fn list_by_workspace(
        &self,
        workspace_id: &crate::domain::agent_registry::WorkspaceId,
    ) -> AgentRegistryResult<Vec<Agent>> {
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

    fn update_heartbeat(&self, heartbeat: &Heartbeat) -> AgentRegistryResult<AgentEvent> {
        let mut agents = self.agents.write().expect("lock poisoned");
        if let Some(agent) = agents.iter_mut().find(|a| &a.id == &heartbeat.agent_id) {
            let previous_status = agent.status;
            agent.last_heartbeat_at = heartbeat.timestamp;
            agent.status = heartbeat.status;
            agent.metadata.workspace_id = heartbeat.workspace_id.clone();
            agent.metadata.current_bead = heartbeat.bead_id.clone();

            let event = match (previous_status, agent.status) {
                (_, crate::domain::agent_registry::AgentStatus::Active)
                    if previous_status != crate::domain::agent_registry::AgentStatus::Active =>
                {
                    AgentEvent::BecameActive {
                        agent_id: agent.id.clone(),
                    }
                }
                (_, crate::domain::agent_registry::AgentStatus::Idle)
                    if previous_status != crate::domain::agent_registry::AgentStatus::Idle =>
                {
                    AgentEvent::BecameIdle {
                        agent_id: agent.id.clone(),
                    }
                }
                (
                    crate::domain::agent_registry::AgentStatus::Active
                    | crate::domain::agent_registry::AgentStatus::Idle,
                    crate::domain::agent_registry::AgentStatus::Disconnected,
                ) => AgentEvent::Disconnected {
                    agent_id: agent.id.clone(),
                },
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
            AgentId::parse("test-agent").expect("valid agent id"),
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
            AgentId::parse("agent-1").expect("valid agent id"),
            "Agent 1".to_string(),
            vec![],
        );
        let mut agent2 = Agent::new(
            AgentId::parse("agent-2").expect("valid agent id"),
            "Agent 2".to_string(),
            vec![],
        );
        agent2.status = crate::domain::agent_registry::AgentStatus::Active;

        repo.save(&agent1).expect("save succeeds");
        repo.save(&agent2).expect("save succeeds");

        let idle = repo
            .list_by_status(crate::domain::agent_registry::AgentStatus::Idle)
            .expect("list succeeds");
        assert_eq!(idle.len(), 1);
        assert_eq!(idle[0].name, "Agent 1");

        let active = repo
            .list_by_status(crate::domain::agent_registry::AgentStatus::Active)
            .expect("list succeeds");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "Agent 2");
    }

    #[test]
    fn test_in_memory_repository_delete() {
        let repo = InMemoryAgentRepository::new();
        let agent = Agent::new(
            AgentId::parse("test-agent").expect("valid agent id"),
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

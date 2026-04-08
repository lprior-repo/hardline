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
    use crate::domain::agent_registry::{AgentStatus, Heartbeat};

    // Helper to create a basic agent
    fn make_agent(id: &str) -> Agent {
        Agent::new(
            AgentId::parse(id).expect("valid agent id"),
            format!("Agent-{id}"),
            vec![],
        )
    }

    fn make_agent_with_name(id: &str, name: &str) -> Agent {
        Agent::new(AgentId::parse(id).expect("valid agent id"), name.to_string(), vec![])
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // save + find_by_id
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_save_and_find_by_id() {
        let repo = InMemoryAgentRepository::new();
        let agent = make_agent("test-agent");

        repo.save(&agent).expect("save succeeds");
        let found = repo.find_by_id(&agent.id).expect("find succeeds");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Agent-test-agent");
    }

    #[test]
    fn test_find_by_id_nonexistent() {
        let repo = InMemoryAgentRepository::new();
        let id = AgentId::parse("ghost").expect("valid");
        let found = repo.find_by_id(&id).expect("find succeeds");
        assert!(found.is_none());
    }

    #[test]
    fn test_save_upsert_existing_agent() {
        let repo = InMemoryAgentRepository::new();
        let agent = make_agent("upsert-agent");
        repo.save(&agent).expect("first save");

        // Modify and save again — should update in place
        let mut updated = agent.clone();
        updated.status = AgentStatus::Active;
        repo.save(&updated).expect("second save (update)");

        let found = repo.find_by_id(&agent.id).expect("find").expect("present");
        assert_eq!(found.status, AgentStatus::Active);

        // Should still be only one agent
        assert_eq!(repo.list_all().expect("list").len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // find_by_name
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_find_by_name() {
        let repo = InMemoryAgentRepository::new();
        let agent = make_agent_with_name("abc", "special-name");
        repo.save(&agent).expect("save");

        let found = repo.find_by_name("special-name").expect("find by name");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, agent.id);
    }

    #[test]
    fn test_find_by_name_nonexistent() {
        let repo = InMemoryAgentRepository::new();
        let found = repo.find_by_name("nobody").expect("find");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_by_name_returns_first_match() {
        let repo = InMemoryAgentRepository::new();
        // Two agents with same name (different IDs)
        let a1 = make_agent_with_name("id-1", "dup-name");
        let a2 = make_agent_with_name("id-2", "dup-name");
        repo.save(&a1).expect("save 1");
        repo.save(&a2).expect("save 2");

        let found = repo.find_by_name("dup-name").expect("find");
        assert!(found.is_some());
        // Should return whichever was found first (either is valid)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // list_all
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_list_all_empty() {
        let repo = InMemoryAgentRepository::new();
        assert!(repo.list_all().expect("list").is_empty());
    }

    #[test]
    fn test_list_all_multiple() {
        let repo = InMemoryAgentRepository::new();
        let a1 = make_agent("a");
        let a2 = make_agent("b");
        let a3 = make_agent("c");
        repo.save(&a1).expect("save");
        repo.save(&a2).expect("save");
        repo.save(&a3).expect("save");

        let all = repo.list_all().expect("list");
        assert_eq!(all.len(), 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // list_by_status
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_list_by_status_filters() {
        let repo = InMemoryAgentRepository::new();

        let mut registering = make_agent("reg");
        registering.status = AgentStatus::Registering;

        let mut active = make_agent("active");
        active.status = AgentStatus::Active;

        let mut idle = make_agent("idle");
        idle.status = AgentStatus::Idle;

        let mut disconnected = make_agent("disco");
        disconnected.status = AgentStatus::Disconnected;

        repo.save(&registering).expect("save");
        repo.save(&active).expect("save");
        repo.save(&idle).expect("save");
        repo.save(&disconnected).expect("save");

        assert_eq!(repo.list_by_status(AgentStatus::Registering).expect("l").len(), 1);
        assert_eq!(repo.list_by_status(AgentStatus::Active).expect("l").len(), 1);
        assert_eq!(repo.list_by_status(AgentStatus::Idle).expect("l").len(), 1);
        assert_eq!(repo.list_by_status(AgentStatus::Disconnected).expect("l").len(), 1);
    }

    #[test]
    fn test_list_by_status_empty_result() {
        let repo = InMemoryAgentRepository::new();
        let agents = repo.list_by_status(AgentStatus::Active).expect("list");
        assert!(agents.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // delete
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_delete_removes_agent() {
        let repo = InMemoryAgentRepository::new();
        let agent = make_agent("del-me");
        repo.save(&agent).expect("save");

        repo.delete(&agent.id).expect("delete");
        assert!(repo.find_by_id(&agent.id).expect("find").is_none());
    }

    #[test]
    fn test_delete_nonexistent_returns_error() {
        let repo = InMemoryAgentRepository::new();
        let id = AgentId::parse("nope").expect("valid");
        let result = repo.delete(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_does_not_affect_others() {
        let repo = InMemoryAgentRepository::new();
        let a1 = make_agent("keep");
        let a2 = make_agent("remove");
        repo.save(&a1).expect("save");
        repo.save(&a2).expect("save");

        repo.delete(&a2.id).expect("delete");

        assert_eq!(repo.list_all().expect("list").len(), 1);
        assert!(repo.find_by_id(&a1.id).expect("find").is_some());
        assert!(repo.find_by_id(&a2.id).expect("find").is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // update_heartbeat
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_update_heartbeat_existing_agent() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("hb-agent");
        agent.status = AgentStatus::Idle;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(
            agent.id.clone(),
            AgentStatus::Active,
            None,
            None,
        );

        let event = repo.update_heartbeat(&heartbeat).expect("heartbeat");
        // Status changed Idle -> Active, so should emit BecameActive
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::BecameActive { .. }));

        let updated = repo.find_by_id(&agent.id).expect("find").expect("present");
        assert_eq!(updated.status, AgentStatus::Active);
    }

    #[test]
    fn test_update_heartbeat_same_status_emits_heartbeat_received() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("same-status");
        agent.status = AgentStatus::Active;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(
            agent.id.clone(),
            AgentStatus::Active,
            None,
            None,
        );

        let event = repo.update_heartbeat(&heartbeat).expect("heartbeat");
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::HeartbeatReceived { .. }));
    }

    #[test]
    fn test_update_heartbeat_active_to_disconnected() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("dc-agent");
        agent.status = AgentStatus::Active;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(
            agent.id.clone(),
            AgentStatus::Disconnected,
            None,
            None,
        );

        let event = repo.update_heartbeat(&heartbeat).expect("heartbeat");
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::Disconnected { .. }));
    }

    #[test]
    fn test_update_heartbeat_idle_to_active() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("idle-to-active");
        agent.status = AgentStatus::Idle;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(agent.id.clone(), AgentStatus::Active, None, None);
        let event = repo.update_heartbeat(&heartbeat).expect("heartbeat");
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::BecameActive { .. }));
    }

    #[test]
    fn test_update_heartbeat_active_to_idle() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("active-to-idle");
        agent.status = AgentStatus::Active;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(agent.id.clone(), AgentStatus::Idle, None, None);
        let event = repo.update_heartbeat(&heartbeat).expect("heartbeat");
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::BecameIdle { .. }));
    }

    #[test]
    fn test_update_heartbeat_nonexistent_returns_error() {
        let repo = InMemoryAgentRepository::new();
        let id = AgentId::parse("ghost").expect("valid");
        let heartbeat = Heartbeat::new(id.clone(), AgentStatus::Active, None, None);
        let result = repo.update_heartbeat(&heartbeat);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // find_stale_agents
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_find_stale_agents() {
        let repo = InMemoryAgentRepository::new();

        // Agent with old heartbeat
        let mut stale = make_agent("stale");
        stale.last_heartbeat_at = Utc::now() - chrono::TimeDelta::try_seconds(120).expect("valid");
        repo.save(&stale).expect("save");

        // Agent with recent heartbeat
        let fresh = make_agent("fresh");
        repo.save(&fresh).expect("save");

        let cutoff = Utc::now() - chrono::TimeDelta::try_seconds(60).expect("valid");
        let stale_agents = repo.find_stale_agents(cutoff).expect("find stale");

        assert_eq!(stale_agents.len(), 1);
        assert_eq!(stale_agents[0].id, stale.id);
    }

    #[test]
    fn test_find_stale_agents_none_stale() {
        let repo = InMemoryAgentRepository::new();
        let agent = make_agent("fresh");
        repo.save(&agent).expect("save");

        let cutoff = Utc::now() - chrono::TimeDelta::try_hours(1).expect("valid");
        let stale = repo.find_stale_agents(cutoff).expect("find stale");
        assert!(stale.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // list_by_workspace
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_list_by_workspace() {
        let repo = InMemoryAgentRepository::new();
        let ws = crate::domain::agent_registry::WorkspaceId::new("ws-1");

        let mut a1 = make_agent("ws-agent-1");
        a1.metadata.workspace_id = Some(ws.clone());
        repo.save(&a1).expect("save");

        let a2 = make_agent("ws-agent-2");
        // no workspace
        repo.save(&a2).expect("save");

        let in_ws = repo.list_by_workspace(&ws).expect("list");
        assert_eq!(in_ws.len(), 1);
        assert_eq!(in_ws[0].id, a1.id);
    }

    #[test]
    fn test_list_by_workspace_empty() {
        let repo = InMemoryAgentRepository::new();
        let ws = crate::domain::agent_registry::WorkspaceId::new("empty-ws");
        let result = repo.list_by_workspace(&ws).expect("list");
        assert!(result.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Trait default methods: process_heartbeat, cleanup_disconnected_agents
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_process_heartbeat_emits_event() {
        let repo = InMemoryAgentRepository::new();
        let mut agent = make_agent("proc-hb");
        agent.status = AgentStatus::Idle;
        repo.save(&agent).expect("save");

        let heartbeat = Heartbeat::new(agent.id.clone(), AgentStatus::Active, None, None);
        let event = repo.process_heartbeat(&mut agent, heartbeat).expect("process");
        assert!(matches!(event, crate::domain::agent_registry::AgentEvent::BecameActive { .. }));
        assert_eq!(agent.status, AgentStatus::Active);
    }

    #[test]
    fn test_cleanup_disconnected_agents() {
        let repo = InMemoryAgentRepository::new();

        // Stale + active agent
        let mut stale = make_agent("stale-to-dc");
        stale.status = AgentStatus::Active;
        stale.last_heartbeat_at = Utc::now() - chrono::TimeDelta::try_seconds(120).expect("valid");
        repo.save(&stale).expect("save");

        let mut fresh = make_agent("fresh-stay");
        fresh.status = AgentStatus::Idle;
        repo.save(&fresh).expect("save");

        let config = HeartbeatConfig::new(30, 60, 0);
        let events = repo.cleanup_disconnected_agents(&config).expect("cleanup");

        // One agent should be timed out
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            crate::domain::agent_registry::AgentEvent::TimedOut { .. }
        ));

        // Stale agent should now be Disconnected
        let updated = repo.find_by_id(&stale.id).expect("find").expect("present");
        assert_eq!(updated.status, AgentStatus::Disconnected);

        // Fresh agent should be unchanged
        let fresh_check = repo.find_by_id(&fresh.id).expect("find").expect("present");
        assert_eq!(fresh_check.status, AgentStatus::Idle);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Default / new
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_default_is_empty() {
        let repo = InMemoryAgentRepository::default();
        assert!(repo.list_all().expect("list").is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HeartbeatConfig
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_heartbeat_config_stale_cutoff() {
        let config = HeartbeatConfig::default();
        let cutoff = config.stale_cutoff();
        assert!(cutoff < Utc::now());
    }

    #[test]
    fn test_heartbeat_config_custom() {
        let config = HeartbeatConfig::new(10, 30, 3);
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Concurrent access — InMemoryAgentRepository
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_concurrent_save_different_agents() {
        use std::sync::Arc;
        use std::thread;

        let repo = Arc::new(InMemoryAgentRepository::new());
        let mut handles = Vec::new();

        for i in 0..50 {
            let r = Arc::clone(&repo);
            handles.push(thread::spawn(move || {
                let agent = make_agent(&format!("conc-{i}"));
                r.save(&agent)
            }));
        }

        for h in handles {
            assert!(h.join().expect("thread").is_ok());
        }

        assert_eq!(repo.list_all().expect("list").len(), 50);
    }

    #[test]
    fn test_concurrent_read_write_no_panic() {
        use std::sync::Arc;
        use std::thread;

        let repo = Arc::new(InMemoryAgentRepository::new());

        // Pre-populate
        for i in 0..10 {
            repo.save(&make_agent(&format!("pre-{i}"))).expect("save");
        }

        let mut handles = Vec::new();

        // Writers: save more agents
        for i in 0..10 {
            let r = Arc::clone(&repo);
            handles.push(thread::spawn(move || {
                r.save(&make_agent(&format!("writer-{i}")))
            }));
        }

        // Readers: list_all repeatedly
        for _ in 0..10 {
            let r = Arc::clone(&repo);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let list = r.list_all().expect("list");
                    assert!(list.len() >= 10);
                }
            }));
        }

        // Deleters
        for i in 0..5 {
            let r = Arc::clone(&repo);
            handles.push(thread::spawn(move || {
                let id = AgentId::parse(&format!("pre-{i}")).expect("valid");
                let _ = r.delete(&id);
            }));
        }

        for h in handles {
            h.join().expect("thread").ok();
        }
    }

    #[test]
    fn test_concurrent_heartbeat_updates() {
        use std::sync::Arc;
        use std::thread;

        let repo = Arc::new(InMemoryAgentRepository::new());

        // Register agents
        for i in 0..20 {
            let mut agent = make_agent(&format!("hb-{i}"));
            agent.status = AgentStatus::Active;
            repo.save(&agent).expect("save");
        }

        let mut handles = Vec::new();

        for i in 0..20 {
            let r = Arc::clone(&repo);
            handles.push(thread::spawn(move || {
                let id = AgentId::parse(&format!("hb-{i}")).expect("valid");
                for _ in 0..50 {
                    let hb = Heartbeat::new(id.clone(), AgentStatus::Active, None, None);
                    r.update_heartbeat(&hb).expect("heartbeat");
                }
            }));
        }

        for h in handles {
            h.join().expect("thread");
        }

        // All should still be there
        assert_eq!(repo.list_all().expect("list").len(), 20);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Proptests — InMemoryAgentRepository
    // ═══════════════════════════════════════════════════════════════════════════

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        prop_compose! {
            fn arb_agent_id()(s in "[a-zA-Z0-9_-]{1,32}") -> AgentId {
                AgentId::parse(s).expect("valid agent id from prop")
            }
        }

        prop_compose! {
            fn arb_agent()(id in arb_agent_id()) -> Agent {
                make_agent(id.as_str())
            }
        }

        proptest! {
            #[test]
            fn prop_save_then_find_by_id(agent in arb_agent()) {
                let repo = InMemoryAgentRepository::new();
                repo.save(&agent)?;
                let found = repo.find_by_id(&agent.id)?;
                prop_assert!(found.is_some());
                prop_assert_eq!(found.unwrap().id, agent.id);
            }

            #[test]
            fn prop_save_then_find_by_name(agent in arb_agent()) {
                let repo = InMemoryAgentRepository::new();
                let name = agent.name.clone();
                repo.save(&agent)?;
                let found = repo.find_by_name(&name)?;
                prop_assert!(found.is_some());
            }

            #[test]
            fn prop_delete_then_not_found(agent in arb_agent()) {
                let repo = InMemoryAgentRepository::new();
                repo.save(&agent)?;
                repo.delete(&agent.id)?;
                let found = repo.find_by_id(&agent.id)?;
                prop_assert!(found.is_none());
            }

            #[test]
            fn prop_list_all_count_matches(ids in prop::collection::vec(arb_agent_id(), 0..20)) {
                let repo = InMemoryAgentRepository::new();
                let unique: Vec<AgentId> = {
                    let mut seen = std::collections::HashSet::new();
                    ids.into_iter().filter(|id| seen.insert(id.clone())).collect()
                };

                for id in &unique {
                    repo.save(&make_agent(id.as_str()))?;
                }

                let all = repo.list_all()?;
                prop_assert_eq!(all.len(), unique.len());
            }

            #[test]
            fn prop_find_by_id_nonexistent(id in arb_agent_id()) {
                let repo = InMemoryAgentRepository::new();
                let found = repo.find_by_id(&id)?;
                prop_assert!(found.is_none());
            }

            #[test]
            fn prop_delete_nonexistent_errors(id in arb_agent_id()) {
                let repo = InMemoryAgentRepository::new();
                let result = repo.delete(&id);
                prop_assert!(result.is_err());
            }

            #[test]
            fn prop_upsert_preserves_latest(agent in arb_agent()) {
                let repo = InMemoryAgentRepository::new();
                repo.save(&agent)?;

                let mut updated = agent.clone();
                updated.status = AgentStatus::Active;
                repo.save(&updated)?;

                let found = repo.find_by_id(&agent.id)?.expect("present");
                prop_assert_eq!(found.status, AgentStatus::Active);
                prop_assert_eq!(repo.list_all()?.len(), 1);
            }

            #[test]
            fn prop_heartbeat_updates_status(agent in arb_agent()) {
                let repo = InMemoryAgentRepository::new();
                let mut a = agent;
                a.status = AgentStatus::Idle;
                repo.save(&a)?;

                let hb = Heartbeat::new(a.id.clone(), AgentStatus::Active, None, None);
                repo.update_heartbeat(&hb)?;

                let found = repo.find_by_id(&a.id)?.expect("present");
                prop_assert_eq!(found.status, AgentStatus::Active);
            }
        }
    }
}

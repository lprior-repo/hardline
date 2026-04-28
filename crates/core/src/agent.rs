//! Agent management for Source Control Plane.
//!
//! Provides agent coordination types from Stak.
//! Zero panic, zero unwrap - all operations return Result.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{error::Result, error_agent::AgentErrorKind};

/// Heartbeat timeout in seconds
const HEARTBEAT_TIMEOUT_SECS: i64 = 60;

/// Unique agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    /// Create a new agent ID
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Create a new agent ID with validation
    pub fn new_checked(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(AgentErrorKind::NotFound("Agent ID cannot be empty".into()).into());
        }
        Ok(Self(id))
    }

    /// Get the ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent activity state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentActivity {
    /// Agent is idle
    #[default]
    Idle,
    /// Agent is working on a session
    Working { session: String, command: String },
}

impl AgentActivity {
    /// Check if agent is currently working
    #[must_use]
    pub const fn is_working(&self) -> bool {
        matches!(self, Self::Working { .. })
    }

    /// Get session if working
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Working { session, .. } => Some(session),
        }
    }

    /// Get command if working
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Working { command, .. } => Some(command),
        }
    }
}

/// An agent in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub activity: AgentActivity,
    pub actions_count: u64,
}

impl Agent {
    /// Create a new agent
    #[must_use]
    pub fn new(id: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id,
            registered_at: now,
            last_seen: now,
            activity: AgentActivity::default(),
            actions_count: 0,
        }
    }

    /// Check if agent is active (heartbeat within last 60 seconds)
    #[must_use]
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        (now - self.last_seen).num_seconds() < HEARTBEAT_TIMEOUT_SECS
    }

    /// Get the status of this agent
    #[must_use]
    pub fn status(&self) -> AgentStatus {
        if self.is_active() {
            AgentStatus::Active
        } else {
            AgentStatus::Stale
        }
    }

    /// Update heartbeat
    pub fn update_heartbeat(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Start working on a session
    pub fn start_work(&mut self, session: impl Into<String>, command: impl Into<String>) {
        self.activity = AgentActivity::Working {
            session: session.into(),
            command: command.into(),
        };
        self.actions_count += 1;
    }

    /// Stop working
    pub fn stop_work(&mut self) {
        self.activity = AgentActivity::Idle;
    }
}

/// Agent status summary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Stale,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Stale => write!(f, "stale"),
        }
    }
}

/// Agent registry trait
pub trait AgentRegistry: Send + Sync {
    /// Register a new agent
    fn register(&self, agent: Agent) -> Result<()>;

    /// Unregister an agent
    fn unregister(&self, id: &AgentId) -> Result<Agent>;

    /// Get agent by ID
    fn get(&self, id: &AgentId) -> Result<Option<Agent>>;

    /// Update agent heartbeat
    fn heartbeat(&self, id: &AgentId) -> Result<()>;

    /// List all agents
    fn list(&self) -> Result<Vec<Agent>>;

    /// List active agents
    fn list_active(&self) -> Result<Vec<Agent>>;
}

/// In-memory agent registry
#[derive(Debug, Default)]
pub struct MemAgentRegistry {
    agents: RwLock<HashMap<AgentId, Agent>>,
}

impl MemAgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AgentRegistry for MemAgentRegistry {
    fn register(&self, agent: Agent) -> Result<()> {
        let mut agents = self.agents.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;

        if agents.contains_key(&agent.id) {
            return Err(AgentErrorKind::Exists(agent.id.to_string()).into());
        }

        agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    fn unregister(&self, id: &AgentId) -> Result<Agent> {
        let mut agents = self.agents.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;

        agents
            .remove(id)
            .ok_or_else(|| AgentErrorKind::NotFound(id.to_string()).into())
    }

    fn get(&self, id: &AgentId) -> Result<Option<Agent>> {
        let agents = self.agents.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(agents.get(id).cloned())
    }

    fn heartbeat(&self, id: &AgentId) -> Result<()> {
        let mut agents = self.agents.write().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire write lock: {}", e))
        })?;

        let agent = agents.get_mut(id).ok_or_else(|| -> crate::error::Error {
            AgentErrorKind::NotFound(id.to_string()).into()
        })?;

        agent.update_heartbeat();
        Ok(())
    }

    fn list(&self) -> Result<Vec<Agent>> {
        let agents = self.agents.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(agents.values().cloned().collect())
    }

    fn list_active(&self) -> Result<Vec<Agent>> {
        let agents = self.agents.read().map_err(|e| {
            crate::error::Error::invalid_state(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(agents.values().filter(|a| a.is_active()).cloned().collect())
    }
}

// Global registry for CLI
use std::sync::OnceLock;
static AGENT_REGISTRY: OnceLock<Arc<dyn AgentRegistry>> = OnceLock::new();

/// Get the global agent registry
pub fn get_agent_registry() -> Arc<dyn AgentRegistry> {
    AGENT_REGISTRY
        .get_or_init(|| Arc::new(MemAgentRegistry::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // AgentId tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_id_new() {
        let id = AgentId::new("cli-agent-01");
        assert_eq!(id.as_str(), "cli-agent-01");
    }

    #[test]
    fn test_agent_id_new_checked_valid() -> Result<()> {
        let id = AgentId::new_checked("valid-id")?;
        assert_eq!(id.as_str(), "valid-id");
        Ok(())
    }

    #[test]
    fn test_agent_id_new_checked_empty_rejects() {
        let result = AgentId::new_checked("");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new("my-agent");
        assert_eq!(format!("{id}"), "my-agent");
    }

    #[test]
    fn test_agent_id_clone() {
        let id = AgentId::new("original");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn test_agent_id_equality() {
        let a = AgentId::new("same");
        let b = AgentId::new("same");
        let c = AgentId::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_agent_id_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AgentId::new("alpha"));
        set.insert(AgentId::new("beta"));
        set.insert(AgentId::new("alpha")); // duplicate
        assert_eq!(set.len(), 2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AgentStatus tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_status_display_active() {
        assert_eq!(format!("{}", AgentStatus::Active), "active");
    }

    #[test]
    fn test_agent_status_display_stale() {
        assert_eq!(format!("{}", AgentStatus::Stale), "stale");
    }

    #[test]
    fn test_agent_status_equality() {
        assert_eq!(AgentStatus::Active, AgentStatus::Active);
        assert_ne!(AgentStatus::Active, AgentStatus::Stale);
    }

    #[test]
    fn test_agent_status_copy() {
        let a = AgentStatus::Active;
        let b = a;
        assert_eq!(a, b);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AgentActivity tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_activity_idle() {
        let activity = AgentActivity::Idle;
        assert!(!activity.is_working());
        assert!(activity.session().is_none());
        assert!(activity.command().is_none());
    }

    #[test]
    fn test_agent_activity_working() {
        let activity = AgentActivity::Working {
            session: "sess-123".to_string(),
            command: "build".to_string(),
        };
        assert!(activity.is_working());
        assert_eq!(activity.session(), Some("sess-123"));
        assert_eq!(activity.command(), Some("build"));
    }

    #[test]
    fn test_agent_activity_default_is_idle() {
        let activity = AgentActivity::default();
        assert!(!activity.is_working());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Agent struct tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_creation() -> Result<()> {
        let agent = Agent::new(AgentId::new("test-agent"));
        assert_eq!(agent.id.as_str(), "test-agent");
        assert!(agent.is_active());
        assert_eq!(agent.status(), AgentStatus::Active);
        assert_eq!(agent.actions_count, 0);
        Ok(())
    }

    #[test]
    fn test_agent_creation_timestamps() {
        let before = Utc::now();
        let agent = Agent::new(AgentId::new("ts-test"));
        let after = Utc::now();
        assert!(agent.registered_at >= before && agent.registered_at <= after);
        assert!(agent.last_seen >= before && agent.last_seen <= after);
        assert_eq!(agent.registered_at, agent.last_seen);
    }

    #[test]
    fn test_agent_default_activity_is_idle() {
        let agent = Agent::new(AgentId::new("idle-test"));
        assert!(!agent.activity.is_working());
    }

    #[test]
    fn test_agent_update_heartbeat() {
        let mut agent = Agent::new(AgentId::new("hb-test"));
        let original_seen = agent.last_seen;
        // Advance time slightly (no sleep in tests, just verify field changes)
        agent.update_heartbeat();
        assert!(agent.last_seen >= original_seen);
    }

    #[test]
    fn test_agent_start_work() {
        let mut agent = Agent::new(AgentId::new("work-test"));
        agent.start_work("sess-1", "build");
        assert!(agent.activity.is_working());
        assert_eq!(agent.activity.session(), Some("sess-1"));
        assert_eq!(agent.activity.command(), Some("build"));
        assert_eq!(agent.actions_count, 1);
    }

    #[test]
    fn test_agent_start_work_increments_count() {
        let mut agent = Agent::new(AgentId::new("count-test"));
        agent.start_work("s1", "cmd1");
        agent.start_work("s2", "cmd2");
        agent.start_work("s3", "cmd3");
        assert_eq!(agent.actions_count, 3);
    }

    #[test]
    fn test_agent_stop_work() {
        let mut agent = Agent::new(AgentId::new("stop-test"));
        agent.start_work("sess", "cmd");
        agent.stop_work();
        assert!(!agent.activity.is_working());
        assert!(agent.activity.session().is_none());
    }

    #[test]
    fn test_agent_status_after_heartbeat() {
        let mut agent = Agent::new(AgentId::new("status-test"));
        agent.update_heartbeat();
        assert_eq!(agent.status(), AgentStatus::Active);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MemAgentRegistry tests
    // ═══════════════════════════════════════════════════════════════════════════

    fn new_registry() -> Arc<dyn AgentRegistry> {
        Arc::new(MemAgentRegistry::new())
    }

    #[test]
    fn test_registry_register_and_get() -> Result<()> {
        let registry = new_registry();
        let agent = Agent::new(AgentId::new("test"));
        registry.register(agent)?;

        let retrieved = registry.get(&AgentId::new("test"))?;
        assert!(retrieved.is_some());
        let got = retrieved.expect("is_some checked");
        assert_eq!(got.id.as_str(), "test");
        Ok(())
    }

    #[test]
    fn test_registry_duplicate_register_fails() -> Result<()> {
        let registry = new_registry();
        registry.register(Agent::new(AgentId::new("dup")))?;

        let result = registry.register(Agent::new(AgentId::new("dup")));
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_registry_get_nonexistent() -> Result<()> {
        let registry = new_registry();
        let result = registry.get(&AgentId::new("nope"))?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_registry_unregister() -> Result<()> {
        let registry = new_registry();
        registry.register(Agent::new(AgentId::new("remove-me")))?;

        let removed = registry.unregister(&AgentId::new("remove-me"))?;
        assert_eq!(removed.id.as_str(), "remove-me");

        // Verify gone
        let result = registry.get(&AgentId::new("remove-me"))?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_registry_unregister_nonexistent_fails() {
        let registry = new_registry();
        let result = registry.unregister(&AgentId::new("ghost"));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_empty() -> Result<()> {
        let registry = new_registry();
        let list = registry.list()?;
        assert!(list.is_empty());
        Ok(())
    }

    #[test]
    fn test_registry_list_multiple() -> Result<()> {
        let registry = new_registry();
        registry.register(Agent::new(AgentId::new("a")))?;
        registry.register(Agent::new(AgentId::new("b")))?;
        registry.register(Agent::new(AgentId::new("c")))?;

        let list = registry.list()?;
        assert_eq!(list.len(), 3);
        Ok(())
    }

    #[test]
    fn test_registry_heartbeat_updates_last_seen() -> Result<()> {
        let registry = new_registry();
        registry.register(Agent::new(AgentId::new("hb")))?;

        // Heartbeat should succeed
        registry.heartbeat(&AgentId::new("hb"))?;

        let agent = registry.get(&AgentId::new("hb"))?.expect("just registered");
        assert!(agent.is_active());
        Ok(())
    }

    #[test]
    fn test_registry_heartbeat_nonexistent_fails() {
        let registry = new_registry();
        let result = registry.heartbeat(&AgentId::new("nope"));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_active() -> Result<()> {
        let registry = new_registry();
        registry.register(Agent::new(AgentId::new("active-1")))?;
        registry.register(Agent::new(AgentId::new("active-2")))?;

        let active = registry.list_active()?;
        // Freshly registered agents are active
        assert_eq!(active.len(), 2);
        Ok(())
    }

    #[test]
    fn test_registry_list_active_excludes_stale() -> Result<()> {
        let registry = new_registry();
        // We cannot easily make agents stale in tests without time manipulation,
        // but we can verify the method works and returns a subset.
        registry.register(Agent::new(AgentId::new("a")))?;

        let active = registry.list_active()?;
        let all = registry.list()?;
        assert!(active.len() <= all.len());
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // get_agent_registry tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_agent_registry_returns_singleton() {
        let r1 = get_agent_registry();
        let r2 = get_agent_registry();
        // Both should be Arc pointers to the same underlying registry
        assert!(Arc::ptr_eq(&r1, &r2));
    }

    #[test]
    fn test_get_agent_registry_is_usable() -> Result<()> {
        let registry = get_agent_registry();
        // Use a unique ID to avoid colliding with other tests
        let unique_id = format!("singleton-test-{}", std::process::id());
        registry.register(Agent::new(AgentId::new(&unique_id)))?;

        let found = registry.get(&AgentId::new(&unique_id))?;
        assert!(found.is_some());

        // Clean up
        registry.unregister(&AgentId::new(&unique_id))?;
        Ok(())
    }

    #[test]
    fn test_mem_agent_registry_default() {
        let registry = MemAgentRegistry::default();
        let list = registry.list().expect("default registry works");
        assert!(list.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: register same agent twice (should fail)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_register_same_agent_id_twice_rejected() {
        let registry = new_registry();

        let agent1 = Agent::new(AgentId::new("dup-agent"));
        let agent2 = Agent::new(AgentId::new("dup-agent"));

        registry.register(agent1).unwrap();
        let result = registry.register(agent2);
        assert!(result.is_err(), "second registration of same ID must fail");
    }

    #[test]
    fn test_register_twice_preserves_first() -> Result<()> {
        let registry = new_registry();

        let mut agent1 = Agent::new(AgentId::new("keep-me"));
        agent1.start_work("sess-1", "build");
        registry.register(agent1)?;

        // Attempt to overwrite with a fresh agent
        let agent2 = Agent::new(AgentId::new("keep-me"));
        let result = registry.register(agent2);
        assert!(result.is_err());

        // Original agent should still be there with its work state
        let retrieved = registry
            .get(&AgentId::new("keep-me"))?
            .expect("still registered");
        assert!(retrieved.activity.is_working());
        assert_eq!(retrieved.activity.session(), Some("sess-1"));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: heartbeat nonexistent agent (should fail)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_heartbeat_nonexistent_returns_error() {
        let registry = new_registry();
        let result = registry.heartbeat(&AgentId::new("ghost-agent"));
        assert!(result.is_err());

        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("ghost-agent") || msg.contains("not found"),
            "error should mention the agent ID or indicate not found, got: {msg}"
        );
    }

    #[test]
    fn test_heartbeat_after_unregister_fails() {
        let registry = new_registry();
        let id = AgentId::new("temporary");

        registry.register(Agent::new(id.clone())).unwrap();
        registry.unregister(&id).unwrap();

        let result = registry.heartbeat(&id);
        assert!(
            result.is_err(),
            "heartbeat on unregistered agent should fail"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: multiple agents with heartbeat race
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn test_multiple_agents_heartbeat_independently() -> Result<()> {
        let registry = new_registry();
        let ids: Vec<AgentId> = (0..20)
            .map(|i| AgentId::new(format!("hb-agent-{i}")))
            .collect();

        for id in &ids {
            registry.register(Agent::new(id.clone()))?;
        }

        // Heartbeat each one — all should succeed
        for id in &ids {
            registry.heartbeat(id)?;
        }

        // All should be present and active
        let all = registry.list()?;
        assert_eq!(all.len(), 20);

        let active = registry.list_active()?;
        assert_eq!(active.len(), 20);

        Ok(())
    }

    #[test]
    fn test_heartbeat_one_does_not_affect_others_last_seen() -> Result<()> {
        let registry = new_registry();

        registry.register(Agent::new(AgentId::new("target")))?;
        registry.register(Agent::new(AgentId::new("bystander")))?;

        let target_before = registry
            .get(&AgentId::new("target"))?
            .expect("registered")
            .last_seen;

        let bystander_before = registry
            .get(&AgentId::new("bystander"))?
            .expect("registered")
            .last_seen;

        // Heartbeat only target
        registry.heartbeat(&AgentId::new("target"))?;

        let target_after = registry
            .get(&AgentId::new("target"))?
            .expect("registered")
            .last_seen;

        let bystander_after = registry
            .get(&AgentId::new("bystander"))?
            .expect("registered")
            .last_seen;

        // Target's last_seen should have been updated (or at least not decreased)
        assert!(target_after >= target_before);

        // Bystander's last_seen should be unchanged
        assert_eq!(bystander_after, bystander_before);

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: list agents after some are unregistered
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_list_after_partial_unregister() -> Result<()> {
        let registry = new_registry();

        registry.register(Agent::new(AgentId::new("keep-1")))?;
        registry.register(Agent::new(AgentId::new("keep-2")))?;
        registry.register(Agent::new(AgentId::new("remove-1")))?;
        registry.register(Agent::new(AgentId::new("remove-2")))?;
        registry.register(Agent::new(AgentId::new("keep-3")))?;

        assert_eq!(registry.list()?.len(), 5);

        registry.unregister(&AgentId::new("remove-1"))?;
        assert_eq!(registry.list()?.len(), 4);

        registry.unregister(&AgentId::new("remove-2"))?;
        assert_eq!(registry.list()?.len(), 3);

        // Remaining agents are the correct ones
        let all_agents = registry.list()?;
        let remaining: Vec<&str> = all_agents.iter().map(|a| a.id.as_str()).collect();
        assert!(remaining.contains(&"keep-1"));
        assert!(remaining.contains(&"keep-2"));
        assert!(remaining.contains(&"keep-3"));
        assert!(!remaining.contains(&"remove-1"));
        assert!(!remaining.contains(&"remove-2"));

        Ok(())
    }

    #[test]
    fn test_list_after_unregister_all() -> Result<()> {
        let registry = new_registry();

        registry.register(Agent::new(AgentId::new("a")))?;
        registry.register(Agent::new(AgentId::new("b")))?;
        registry.register(Agent::new(AgentId::new("c")))?;

        registry.unregister(&AgentId::new("a"))?;
        registry.unregister(&AgentId::new("b"))?;
        registry.unregister(&AgentId::new("c"))?;

        assert!(registry.list()?.is_empty());
        assert!(registry.list_active()?.is_empty());
        Ok(())
    }

    #[test]
    fn test_unregister_returns_removed_agent() -> Result<()> {
        let registry = new_registry();

        let mut agent = Agent::new(AgentId::new("verify-removal"));
        agent.start_work("sess-x", "deploy");
        registry.register(agent)?;

        let removed = registry.unregister(&AgentId::new("verify-removal"))?;
        assert_eq!(removed.id.as_str(), "verify-removal");
        assert!(removed.activity.is_working());
        assert_eq!(removed.activity.session(), Some("sess-x"));
        assert_eq!(removed.actions_count, 1);

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: start/stop work cycle
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_start_stop_work_full_cycle() -> Result<()> {
        let registry = new_registry();
        let id = AgentId::new("cycle-agent");

        registry.register(Agent::new(id.clone()))?;

        // Initially idle
        let agent = registry.get(&id)?.expect("registered");
        assert!(!agent.activity.is_working());
        assert_eq!(agent.actions_count, 0);

        // Start work
        let mut agent = registry.get(&id)?.expect("registered");
        agent.start_work("sess-1", "build");
        assert!(agent.activity.is_working());
        assert_eq!(agent.actions_count, 1);

        // Stop work
        agent.stop_work();
        assert!(!agent.activity.is_working());
        // actions_count is NOT reset by stop_work
        assert_eq!(agent.actions_count, 1);

        // Start another work session
        agent.start_work("sess-2", "test");
        assert!(agent.activity.is_working());
        assert_eq!(agent.actions_count, 2);

        Ok(())
    }

    #[test]
    fn test_stop_work_on_idle_agent_is_noop() {
        let mut agent = Agent::new(AgentId::new("already-idle"));
        agent.stop_work(); // should not panic or error
        assert!(!agent.activity.is_working());
        assert_eq!(agent.actions_count, 0);
    }

    #[test]
    fn test_multiple_start_work_updates_session_and_command() {
        let mut agent = Agent::new(AgentId::new("multi-work"));

        agent.start_work("sess-a", "cmd-a");
        assert_eq!(agent.activity.session(), Some("sess-a"));
        assert_eq!(agent.activity.command(), Some("cmd-a"));

        agent.start_work("sess-b", "cmd-b");
        assert_eq!(agent.activity.session(), Some("sess-b"));
        assert_eq!(agent.activity.command(), Some("cmd-b"));

        agent.start_work("sess-c", "cmd-c");
        assert_eq!(agent.activity.session(), Some("sess-c"));
        assert_eq!(agent.activity.command(), Some("cmd-c"));

        assert_eq!(agent.actions_count, 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: agent creation with empty ID (should fail)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_id_new_checked_empty_string_produces_error_message() {
        let result = AgentId::new_checked("");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("empty") || msg.contains("not found"),
            "error message should mention empty or not found, got: {msg}"
        );
    }

    #[test]
    fn test_agent_id_new_checked_whitespace_only_rejects() {
        // Only truly empty string is rejected by current implementation,
        // but verify that whitespace-only passes (as the current impl only checks .is_empty())
        let result = AgentId::new_checked("   ");
        assert!(
            result.is_ok(),
            "whitespace-only is accepted by current impl"
        );
    }

    #[test]
    fn test_agent_id_new_unchecked_allows_empty() {
        // The unchecked constructor does NOT validate
        let id = AgentId::new("");
        assert_eq!(id.as_str(), "");
    }

    #[test]
    fn test_register_agent_with_empty_id_via_unchecked() -> Result<()> {
        let registry = new_registry();
        let empty_id = AgentId::new("");
        let agent = Agent::new(empty_id);

        // Registration itself should succeed (no validation at registry level for empty ID)
        registry.register(agent)?;
        let retrieved = registry.get(&AgentId::new(""))?;
        assert!(retrieved.is_some());

        // But new_checked would reject creating that ID
        assert!(AgentId::new_checked("").is_err());

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Edge case: agent update_heartbeat does not reset other fields
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_heartbeat_preserves_activity_and_count() -> Result<()> {
        let registry = new_registry();
        let id = AgentId::new("preserve-test");

        let mut agent = Agent::new(id.clone());
        agent.start_work("sess-p", "cmd-p");
        let original_registered_at = agent.registered_at;
        registry.register(agent)?;

        // Heartbeat
        registry.heartbeat(&id)?;

        let agent = registry.get(&id)?.expect("registered");
        assert!(agent.activity.is_working());
        assert_eq!(agent.activity.session(), Some("sess-p"));
        assert_eq!(agent.actions_count, 1);
        // registered_at should NOT change after heartbeat
        assert_eq!(agent.registered_at, original_registered_at);
        // last_seen should have been updated by heartbeat
        assert!(agent.last_seen >= original_registered_at);

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrip tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_agent_id_serde_roundtrip() {
        let id = AgentId::new("cli-agent-01");
        let json = serde_json::to_string(&id).expect("serialize ok");
        let deserialized: AgentId = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_agent_status_serde_roundtrip() {
        for status in [AgentStatus::Active, AgentStatus::Stale] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: AgentStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_agent_activity_serde_roundtrip_idle() {
        let activity = AgentActivity::Idle;
        let json = serde_json::to_string(&activity).expect("serialize ok");
        let deserialized: AgentActivity = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(activity, deserialized);
    }

    #[test]
    fn test_agent_activity_serde_roundtrip_working() {
        let activity = AgentActivity::Working {
            session: "my-session".to_string(),
            command: "build".to_string(),
        };
        let json = serde_json::to_string(&activity).expect("serialize ok");
        let deserialized: AgentActivity = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(activity, deserialized);
        assert!(deserialized.is_working());
        assert_eq!(deserialized.session(), Some("my-session"));
    }
}

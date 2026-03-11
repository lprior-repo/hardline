//! Agent management for Source Control Plane.
//!
//! Provides agent coordination types from Stak.
//! Zero panic, zero unwrap - all operations return Result.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
            return Err(Error::AgentNotFound("Agent ID cannot be empty".into()));
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
        let mut agents = self
            .agents
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        if agents.contains_key(&agent.id) {
            return Err(Error::AgentExists(agent.id.to_string()));
        }

        agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    fn unregister(&self, id: &AgentId) -> Result<Agent> {
        let mut agents = self
            .agents
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        agents
            .remove(id)
            .ok_or_else(|| Error::AgentNotFound(id.to_string()))
    }

    fn get(&self, id: &AgentId) -> Result<Option<Agent>> {
        let agents = self
            .agents
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(agents.get(id).cloned())
    }

    fn heartbeat(&self, id: &AgentId) -> Result<()> {
        let mut agents = self
            .agents
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let agent = agents
            .get_mut(id)
            .ok_or_else(|| Error::AgentNotFound(id.to_string()))?;

        agent.update_heartbeat();
        Ok(())
    }

    fn list(&self) -> Result<Vec<Agent>> {
        let agents = self
            .agents
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(agents.values().cloned().collect())
    }

    fn list_active(&self) -> Result<Vec<Agent>> {
        let agents = self
            .agents
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;
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

    #[test]
    fn test_agent_creation() -> Result<()> {
        let agent = Agent::new(AgentId::new("test-agent"));
        assert_eq!(agent.id.as_str(), "test-agent");
        assert!(agent.is_active());
        assert_eq!(agent.status(), AgentStatus::Active);
        Ok(())
    }

    #[test]
    fn test_agent_registry() -> Result<()> {
        let registry = Arc::new(MemAgentRegistry::new()) as Arc<dyn AgentRegistry>;

        let agent = Agent::new(AgentId::new("test"));
        registry.register(agent)?;

        let retrieved = registry.get(&AgentId::new("test"))?;
        assert!(retrieved.is_some());

        Ok(())
    }
}

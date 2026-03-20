//! Agent Registry and Heartbeat System
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
use thiserror::Error;

use crate::domain::identifiers::{AgentId, BeadId};

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

pub mod repository;

// ============================================================================
// ERRORS
// ============================================================================

/// Domain errors for agent registry operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
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
        from: AgentStatus,
        /// Attempted target state
        to: AgentStatus,
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
// SEMANTIC VERSION
// ============================================================================

/// Semantic version for capability versioning.
///
/// Follows semver.org format: major.minor.patch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemanticVersion {
    /// Create a new semantic version
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse from a string in format "major.minor.patch"
    pub fn parse(s: &str) -> Result<Self, AgentRegistryError> {
        let parts: Vec<u64> = s
            .split('.')
            .map(|p| {
                p.parse()
                    .map_err(|_| AgentRegistryError::InvalidCapability(s.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        match parts.as_slice() {
            [major, minor, patch] => Ok(Self::new(*major, *minor, *patch)),
            _ => Err(AgentRegistryError::InvalidCapability(s.to_string())),
        }
    }

    /// Convert to string format "major.minor.patch"
    #[must_use]
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ============================================================================
// AGENT STATUS
// ============================================================================

/// Agent lifecycle status.
///
/// Represents the current state of an agent in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is actively processing work, heartbeat is recent
    Active,
    /// Agent is idle/waiting for work, heartbeat is recent
    Idle,
    /// Agent heartbeat has expired (no heartbeat for 90+ seconds)
    Disconnected,
    /// Agent is in initial handshake/registration phase
    Registering,
}

impl AgentStatus {
    /// Check if agent is considered "available" (can receive work)
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Active | Self::Idle)
    }

    /// Check if agent is disconnected
    #[must_use]
    pub const fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    /// Check if agent is in registration phase
    #[must_use]
    pub const fn is_registering(&self) -> bool {
        matches!(self, Self::Registering)
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Idle => write!(f, "idle"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Registering => write!(f, "registering"),
        }
    }
}

// ============================================================================
// CAPABILITY
// ============================================================================

/// Well-known capability names for common agent abilities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityName {
    WorkspaceManagement,
    BeadClaim,
    QueueProcess,
    VcsOperation,
    HardlineExec,
    Custom(String),
}

impl CapabilityName {
    /// Get the capability name as a string identifier
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::WorkspaceManagement => "workspace:manage",
            Self::BeadClaim => "bead:claim",
            Self::QueueProcess => "queue:process",
            Self::VcsOperation => "vcs:operate",
            Self::HardlineExec => "hardline:exec",
            Self::Custom(name) => name,
        }
    }

    /// Parse from a string
    pub fn parse(s: &str) -> Self {
        match s {
            "workspace:manage" => Self::WorkspaceManagement,
            "bead:claim" => Self::BeadClaim,
            "queue:process" => Self::QueueProcess,
            "vcs:operate" => Self::VcsOperation,
            "hardline:exec" => Self::HardlineExec,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A capability that an agent advertises.
///
/// Capabilities define what an agent can do. Agents can have multiple
/// capabilities at different versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub name: CapabilityName,
    pub version: SemanticVersion,
    pub attributes: HashMap<String, String>,
}

impl Capability {
    /// Create a new capability with no attributes
    #[must_use]
    pub fn new(name: CapabilityName, version: SemanticVersion) -> Self {
        Self {
            name,
            version,
            attributes: HashMap::new(),
        }
    }

    /// Create with attributes
    #[must_use]
    pub fn with_attributes(
        mut self,
        attributes: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        self.attributes.extend(attributes);
        self
    }

    /// Check if this capability matches a query
    #[must_use]
    pub fn matches_name(&self, name: &str) -> bool {
        self.name.as_str() == name
    }
}

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
    pub id: AgentId,
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
    pub fn new(id: AgentId, name: String, capabilities: Vec<Capability>) -> Self {
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
    pub fn transition_to(&mut self, target: AgentStatus) -> AgentRegistryResult<()> {
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
    ) -> AgentRegistryResult<()> {
        let previous_status = self.status;
        self.last_heartbeat_at = timestamp;
        self.status = status;
        self.metadata.workspace_id = workspace_id;
        self.metadata.current_bead = bead_id;

        if !self.can_transition_to(status) && previous_status != status {
            return Err(AgentRegistryError::InvalidStateTransition {
                from: previous_status,
                to: status,
            });
        }
        Ok(())
    }
}

// ============================================================================
// HEARTBEAT
// ============================================================================

/// Configuration for heartbeat system.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            timeout_secs: 90,
            max_retries: 0,
        }
    }
}

impl HeartbeatConfig {
    /// Calculate the cutoff time for stale detection
    #[must_use]
    pub fn stale_cutoff(&self) -> DateTime<Utc> {
        Utc::now() - TimeDelta::try_seconds(self.timeout_secs as i64).unwrap_or(TimeDelta::zero())
    }

    /// Create a non-default configuration
    #[must_use]
    pub fn new(interval_secs: u64, timeout_secs: u64, max_retries: u32) -> Self {
        Self {
            interval_secs,
            timeout_secs,
            max_retries,
        }
    }
}

/// Heartbeat message from an agent.
///
/// Agents send heartbeat messages periodically to indicate they're alive.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub status: AgentStatus,
    pub workspace_id: Option<WorkspaceId>,
    pub bead_id: Option<BeadId>,
    pub load_average: Option<f64>,
}

impl Heartbeat {
    /// Create a new heartbeat
    #[must_use]
    pub fn new(
        agent_id: AgentId,
        status: AgentStatus,
        workspace_id: Option<WorkspaceId>,
        bead_id: Option<BeadId>,
    ) -> Self {
        Self {
            agent_id,
            timestamp: Utc::now(),
            status,
            workspace_id,
            bead_id,
            load_average: None,
        }
    }

    /// Create with load average
    #[must_use]
    pub fn with_load_average(mut self, load_average: f64) -> Self {
        self.load_average = Some(load_average);
        self
    }
}

// ============================================================================
// AGENT EVENTS
// ============================================================================

/// Events emitted by the agent registry for observability.
///
/// These events enable monitoring, logging, and auditing of agent lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum AgentEvent {
    Registered {
        agent_id: AgentId,
        name: String,
    },
    BecameActive {
        agent_id: AgentId,
    },
    BecameIdle {
        agent_id: AgentId,
    },
    Disconnected {
        agent_id: AgentId,
    },
    TimedOut {
        agent_id: AgentId,
    },
    HeartbeatReceived {
        agent_id: AgentId,
    },
    CapabilitiesUpdated {
        agent_id: AgentId,
        capabilities: Vec<Capability>,
    },
}

impl AgentEvent {
    /// Get the agent ID for this event
    #[must_use]
    pub fn agent_id(&self) -> &AgentId {
        match self {
            Self::Registered { agent_id, .. } => agent_id,
            Self::BecameActive { agent_id } => agent_id,
            Self::BecameIdle { agent_id } => agent_id,
            Self::Disconnected { agent_id } => agent_id,
            Self::TimedOut { agent_id } => agent_id,
            Self::HeartbeatReceived { agent_id } => agent_id,
            Self::CapabilitiesUpdated { agent_id, .. } => agent_id,
        }
    }
}

// ============================================================================
// AGENT REGISTRY OPERATIONS
// ============================================================================

/// Process a heartbeat from an agent and return the resulting event.
pub fn process_heartbeat(
    agent: &mut Agent,
    heartbeat: Heartbeat,
) -> AgentRegistryResult<AgentEvent> {
    let previous_status = agent.status;

    agent.last_heartbeat_at = heartbeat.timestamp;
    agent.status = heartbeat.status;
    agent.metadata.workspace_id = heartbeat.workspace_id;
    agent.metadata.current_bead = heartbeat.bead_id;

    let event = match (previous_status, agent.status) {
        (_, AgentStatus::Active) if previous_status != AgentStatus::Active => {
            AgentEvent::BecameActive {
                agent_id: agent.id.clone(),
            }
        }
        (_, AgentStatus::Idle) if previous_status != AgentStatus::Idle => AgentEvent::BecameIdle {
            agent_id: agent.id.clone(),
        },
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

/// Check for timed-out agents and mark them as disconnected.
pub fn cleanup_disconnected_agents(
    agents: &mut [Agent],
    config: &HeartbeatConfig,
) -> Vec<AgentEvent> {
    let cutoff = config.stale_cutoff();
    let mut events = Vec::new();

    for agent in agents.iter_mut() {
        if agent.status.is_available() && agent.is_stale(cutoff) {
            let previous_status = agent.status;
            agent.status = AgentStatus::Disconnected;

            if previous_status != AgentStatus::Disconnected {
                events.push(AgentEvent::TimedOut {
                    agent_id: agent.id.clone(),
                });
            }
        }
    }

    events
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_version_parse() {
        let v = SemanticVersion::parse("1.2.3").expect("valid version");
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_semantic_version_display() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_agent_status_is_available() {
        assert!(AgentStatus::Active.is_available());
        assert!(AgentStatus::Idle.is_available());
        assert!(!AgentStatus::Disconnected.is_available());
        assert!(!AgentStatus::Registering.is_available());
    }

    #[test]
    fn test_agent_can_transition() {
        let agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );

        // Registering can go to Active or Idle
        assert!(agent.can_transition_to(AgentStatus::Active));
        assert!(agent.can_transition_to(AgentStatus::Idle));

        // Active can go to Idle and Disconnected
        let mut a = agent.clone();
        a.status = AgentStatus::Active;
        assert!(a.can_transition_to(AgentStatus::Idle));
        assert!(a.can_transition_to(AgentStatus::Disconnected));

        // Disconnected must go to Registering
        let mut a = agent.clone();
        a.status = AgentStatus::Disconnected;
        assert!(a.can_transition_to(AgentStatus::Registering));
        assert!(!a.can_transition_to(AgentStatus::Active));
    }

    #[test]
    fn test_agent_transition_to() {
        let mut agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );

        agent
            .transition_to(AgentStatus::Active)
            .expect("valid transition");
        assert_eq!(agent.status, AgentStatus::Active);

        agent
            .transition_to(AgentStatus::Idle)
            .expect("valid transition");
        assert_eq!(agent.status, AgentStatus::Idle);
    }

    #[test]
    fn test_agent_is_stale() {
        let mut agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );

        let config = HeartbeatConfig::default();
        assert!(!agent.is_stale(config.stale_cutoff()));

        // Manually set heartbeat to 100 seconds ago
        agent.last_heartbeat_at = Utc::now() - Duration::from_secs(100);
        assert!(agent.is_stale(config.stale_cutoff()));
    }

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval_secs, 30);
        assert_eq!(config.timeout_secs, 90);
    }

    #[test]
    fn test_capability_name_parsing() {
        assert_eq!(
            CapabilityName::parse("workspace:manage"),
            CapabilityName::WorkspaceManagement
        );
        assert_eq!(
            CapabilityName::parse("bead:claim"),
            CapabilityName::BeadClaim
        );
        assert_eq!(
            CapabilityName::parse("custom:feature"),
            CapabilityName::Custom("custom:feature".to_string())
        );
    }

    #[test]
    fn test_agent_event_agent_id() {
        let event = AgentEvent::Registered {
            agent_id: AgentId::parse("test").unwrap(),
            name: "Test".to_string(),
        };
        assert_eq!(event.agent_id().as_str(), "test");
    }

    #[test]
    fn test_process_heartbeat() {
        let mut agent = Agent::new(
            AgentId::parse("test-agent").unwrap(),
            "Test Agent".to_string(),
            vec![],
        );
        agent.status = AgentStatus::Registering;

        let heartbeat = Heartbeat::new(
            AgentId::parse("test-agent").unwrap(),
            AgentStatus::Active,
            None,
            None,
        );

        let event = process_heartbeat(&mut agent, heartbeat).expect("success");
        assert!(matches!(event, AgentEvent::BecameActive { .. }));
        assert_eq!(agent.status, AgentStatus::Active);
    }
}

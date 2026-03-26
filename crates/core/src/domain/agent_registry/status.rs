//! Agent status and capability definitions

use serde::{Deserialize, Serialize};

use super::{AgentId, AgentRegistryError, BeadId, SemanticVersion, WorkspaceId};

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
    pub attributes: std::collections::HashMap<String, String>,
}

impl Capability {
    /// Create a new capability with no attributes
    #[must_use]
    pub fn new(name: CapabilityName, version: SemanticVersion) -> Self {
        Self {
            name,
            version,
            attributes: std::collections::HashMap::new(),
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

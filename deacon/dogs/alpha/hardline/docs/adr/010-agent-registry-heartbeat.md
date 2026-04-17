# ADR-010: Agent Registry & Heartbeat System

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline supports 600+ concurrent AI agents operating in isolated workspaces. The system needs to:

1. **Track active agents** - Know who's online, what's running
2. **Detect dead agents** - Heartbeat timeout to detect crashes/disconnects
3. **Capability registry** - Agents advertise what they can do
4. **State queries** - Query agent state, assigned workspaces
5. **Graceful shutdown** - Clean up when agents disconnect

This ADR defines the agent registry and heartbeat mechanism.

---

## Decision

### Agent Entity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: AgentName,
    pub capabilities: Vec<Capability>,
    pub status: AgentStatus,
    pub last_heartbeat_at: DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,      // Processing, heartbeat recent
    Idle,        // Waiting for work, heartbeat recent
    Disconnected, // Heartbeat expired
    Registering, // Initial handshake
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub workspace_id: Option<WorkspaceId>,
    pub current_bead: Option<BeadId>,
    pub started_at: Option<DateTime<Utc>>,
    pub pid: Option<u32>,
    pub version: String,
}
```

### Capability Registry

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub name: CapabilityName,
    pub version: SemanticVersion,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CapabilityName {
    WorkspaceManagement,
    BeadClaim,
    QueueProcess,
    VcsOperation,
    HardlineExec,
    Custom(String),
}

impl CapabilityName {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityName::WorkspaceManagement => "workspace:manage",
            CapabilityName::BeadClaim => "bead:claim",
            CapabilityName::QueueProcess => "queue:process",
            CapabilityName::VcsOperation => "vcs:operate",
            CapabilityName::HardlineExec => "hardline:exec",
            CapabilityName::Custom(name) => name,
        }
    }
}
```

### Heartbeat System

```rust
pub struct HeartbeatConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,      // Send heartbeat every 30 seconds
            timeout_secs: 90,       // Considered disconnected after 90 seconds (3 missed)
            max_retries: 0,         // No retries - immediate disconnect on timeout
        }
    }
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub status: AgentStatus,
    pub workspace_id: Option<WorkspaceId>,
    pub bead_id: Option<BeadId>,
    pub load_average: Option<f64>,
}

/// Register agent
pub fn register_agent(
    name: AgentName,
    capabilities: Vec<Capability>,
) -> Result<Agent, AgentError> {
    let agent = Agent {
        id: AgentId::new(),
        name,
        capabilities,
        status: AgentStatus::Registering,
        last_heartbeat_at: Utc::now(),
        registered_at: Utc::now(),
        metadata: AgentMetadata::default(),
    };
    
    Ok(agent)
}

/// Process heartbeat from agent
pub fn process_heartbeat(
    registry: &AgentRegistry,
    heartbeat: Heartbeat,
) -> Result<AgentEvent, AgentError> {
    let mut agent = registry.find_by_id(&heartbeat.agent_id)?
        .ok_or(AgentError::AgentNotFound(heartbeat.agent_id))?;
    
    let previous_status = agent.status;
    
    // Update heartbeat timestamp
    agent.last_heartbeat_at = heartbeat.timestamp;
    agent.status = heartbeat.status;
    agent.metadata.workspace_id = heartbeat.workspace_id;
    agent.metadata.current_bead = heartbeat.bead_id;
    
    // Save updated agent
    registry.save(&agent)?;
    
    // Return event for observability
    let event = match (previous_status, agent.status) {
        (_, AgentStatus::Active) if previous_status != AgentStatus::Active => {
            AgentEvent::BecameActive(agent.id)
        }
        (_, AgentStatus::Idle) if previous_status != AgentStatus::Idle => {
            AgentEvent::BecameIdle(agent.id)
        }
        (AgentStatus::Active | AgentStatus::Idle, AgentStatus::Disconnected) => {
            AgentEvent::Disconnected(agent.id)
        }
        _ => AgentEvent::HeartbeatReceived(agent.id),
    };
    
    Ok(event)
}

/// Check for timed-out agents
pub fn cleanup_disconnected_agents(
    registry: &AgentRegistry,
    config: &HeartbeatConfig,
) -> Result<Vec<AgentEvent>, AgentError> {
    let cutoff = Utc::now() - Duration::from_secs(config.timeout_secs);
    let disconnected = registry.find_stale_agents(cutoff)?;
    
    let mut events = Vec::new();
    
    for mut agent in disconnected {
        let previous_status = agent.status;
        agent.status = AgentStatus::Disconnected;
        registry.save(&agent)?;
        
        events.push(AgentEvent::TimedOut(agent.id));
    }
    
    Ok(events)
}
```

### Agent Repository Trait

```rust
pub trait AgentRepository: Send + Sync {
    fn save(&self, agent: &Agent) -> Result<(), AgentRepoError>;
    fn find_by_id(&self, id: &AgentId) -> Result<Option<Agent>, AgentRepoError>;
    fn find_by_name(&self, name: &AgentName) -> Result<Option<Agent>, AgentRepoError>;
    fn list_all(&self) -> Result<Vec<Agent>, AgentRepoError>;
    fn list_by_status(&self, status: AgentStatus) -> Result<Vec<Agent>, AgentRepoError>;
    fn list_by_workspace(&self, workspace_id: &WorkspaceId) -> Result<Vec<Agent>, AgentRepoError>;
    fn find_stale_agents(&self, cutoff: DateTime<Utc>) -> Result<Vec<Agent>, AgentRepoError>;
    fn delete(&self, id: &AgentId) -> Result<(), AgentRepoError>;
}
```

### Agent Events

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum AgentEvent {
    Registered { agent_id: AgentId, name: String },
    BecameActive { agent_id: AgentId },
    BecameIdle { agent_id: AgentId },
    Disconnected { agent_id: AgentId },
    TimedOut { agent_id: AgentId },
    HeartbeatReceived { agent_id: AgentId },
    CapabilitiesUpdated { agent_id: AgentId, capabilities: Vec<Capability> },
}
```

---

## Variants

### Variant A: Central Registry with DB (CHOSEN)

```rust
struct AgentRegistry {
    db: SqlitePool,
}
```

**Chosen because:**
- Durable - survives restarts
- Queryable - SQL for filtering
- Scalable to 600+ agents

### Variant B: Distributed Registry (Consul/Etcd)

**Rejected because:**
- Adds operational complexity
- Hardline doesn't need distributed consensus
- Single SQLite is simpler

### Variant C: In-Memory Only

**Rejected because:**
- Lost on restart
- Can't query across restarts
- Bad for debugging

---

## Invariants

### Agent Identity Invariants

```rust
/// INVARIANT: Agent ID is globally unique
assert!(agents.iter().all_unique_by(|a| &a.id));

/// INVARIANT: Agent name is unique
assert!(agents.iter().all_unique_by(|a| &a.name));

/// INVARIANT: No two agents can have same ID and name
assert!(agents.iter().all_combinations().all_unique());
```

### Heartbeat Invariants

```rust
/// INVARIANT: Active/Idle agents have recent heartbeat
fn assert_heartbeat_recent(agent: &Agent) {
    let age = Utc::now() - agent.last_heartbeat_at;
    match agent.status {
        AgentStatus::Active | AgentStatus::Idle => {
            assert!(age < Duration::from_secs(HEARTBEAT_TIMEOUT_SECONDS));
        }
        AgentStatus::Disconnected => {
            assert!(age >= Duration::from_secs(HEARTBEAT_TIMEOUT_SECONDS));
        }
        AgentStatus::Registering => {
            // No requirement
        }
    }
}

/// INVARIANT: Disconnected agent cannot transition to Active without re-registration
fn assert_disconnected_requires_reregistration(agent: &Agent) {
    if agent.status == AgentStatus::Disconnected {
        // Must go through Registering first
        assert_eq!(agent.last_heartbeat_at, agent.registered_at);
    }
}
```

### Capability Invariants

```rust
/// INVARIANT: Capabilities is non-empty for active agents
fn assert_capabilities_present(agent: &Agent) {
    if agent.status == AgentStatus::Active || agent.status == AgentStatus::Idle {
        assert!(!agent.capabilities.is_empty(), "Agent {} has no capabilities", agent.id);
    }
}

/// INVARIANT: Capability versions are valid semantic versions
fn assert_valid_semver(capability: &Capability) {
    assert!(capability.version.major >= 0);
    assert!(capability.version.minor >= 0);
    assert!(capability.version.patch >= 0);
}
```

### Workspace Assignment Invariants

```rust
/// INVARIANT: At most one agent can claim a workspace
fn assert_single_workspace_claim(workspace: &Workspace, agents: &[Agent]) {
    let claiming_agents: Vec<_> = agents
        .iter()
        .filter(|a| a.metadata.workspace_id == Some(workspace.id))
        .collect();
    
    assert!(claiming_agents.len() <= 1);
}

/// INVARIANT: Agent with workspace is Active
fn assert_workspace_agent_is_active(agent: &Agent) {
    if agent.metadata.workspace_id.is_some() {
        assert!(matches!(agent.status, AgentStatus::Active | AgentStatus::Idle));
    }
}
```

---

## Consequences

### Positive

1. **Dead agent detection** - Automatic cleanup after heartbeat timeout
2. **Queryable** - Find agents by status, workspace, capabilities
3. **Observable** - AgentEvent stream for monitoring
4. **Capability routing** - Route work to capable agents

### Negative

1. **Heartbeat overhead** - 30-second intervals, 600 agents = 20 msgs/sec
2. **Clock sync** - Assumes reasonable clock sync between agents

### CLI Commands

```bash
hardline agent list                          # List all agents
hardline agent list --status active          # Filter by status
hardline agent list --workspace <id>         # Agents in workspace
hardline agent status <id>                   # Agent details
hardline agent capabilities <id>             # Agent capabilities
hardline agent deregister <id>               # Force remove agent
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/domain/agent.rs` | Agent entity, AgentStatus |
| `crates/core/src/domain/capability.rs` | Capability types |
| `crates/core/src/infrastructure/agent_registry.rs` | SQLite repository |
| `crates/cli/src/commands/agent.rs` | CLI commands |

---

## Related ADRs

- ADR-006: Database Schema (agents table)
- ADR-001: CLI Architecture (agent commands)
- ADR-002: Durable Workflow Execution (agent crash recovery)

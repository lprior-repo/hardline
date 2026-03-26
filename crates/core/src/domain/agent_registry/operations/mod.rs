//! Operations for agent registry

use chrono::{DateTime, Utc};

use super::{Agent, AgentEvent, AgentRegistryError, AgentStatus, HeartbeatConfig};

/// Process a heartbeat from an agent and return the resulting event.
pub fn process_heartbeat(agent: &mut Agent, heartbeat: super::Heartbeat) -> AgentRegistryError {
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

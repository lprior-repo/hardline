//! Agent commands

use scp_core::{
    agent::{get_agent_registry, Agent, AgentActivity, AgentId},
    Result,
};

/// Create an agent
pub fn create(name: &str) -> Result<()> {
    let registry = get_agent_registry();

    let agent_id = AgentId::new_checked(name)?;
    let agent = Agent::new(agent_id);

    registry.register(agent)?;

    println!("✓ Agent '{}' created", name);
    Ok(())
}

/// Activity display info extracted from an agent
#[derive(Debug, Clone)]
pub struct AgentDisplayInfo {
    pub id: String,
    pub status: String,
    pub activity_label: String,
}

/// Pure calculation: Extract display info from an agent (no I/O)
fn compute_agent_display_info(agent: &Agent) -> AgentDisplayInfo {
    let status = agent.status().to_string();
    let activity_label = match &agent.activity {
        AgentActivity::Idle => "idle".to_string(),
        AgentActivity::Working { session, command } => {
            format!("working on '{}': {}", session, command)
        }
    };
    AgentDisplayInfo {
        id: agent.id.to_string(),
        status,
        activity_label,
    }
}

/// Pure calculation: Format a single agent line for display
fn format_agent_line(info: &AgentDisplayInfo) -> String {
    format!("  - {} [{}] {}", info.id, info.status, info.activity_label)
}

/// Pure calculation: Format the agents list header
fn format_agents_header(count: usize) -> String {
    format!("Agents ({} total):", count)
}

/// List agents
pub fn list() -> Result<()> {
    let registry = get_agent_registry();

    let agents = registry.list()?;

    if agents.is_empty() {
        println!("No agents registered");
    } else {
        let display_infos: Vec<AgentDisplayInfo> =
            agents.iter().map(compute_agent_display_info).collect();

        let working_agents: Vec<&AgentDisplayInfo> = display_infos
            .iter()
            .filter(|i| i.activity_label.starts_with("working on"))
            .collect();
        let idle_agents: Vec<&AgentDisplayInfo> = display_infos
            .iter()
            .filter(|i| !i.activity_label.starts_with("working on"))
            .collect();

        println!("{}", format_agents_header(agents.len()));

        for info in &working_agents {
            println!("  - {} [{}] {}", info.id, info.status, info.activity_label);
        }
        for info in &idle_agents {
            println!("{}", format_agent_line(info));
        }
    }

    Ok(())
}

/// Kill an agent
pub fn kill(id: &str) -> Result<()> {
    let registry = get_agent_registry();

    let agent_id = AgentId::new_checked(id)?;

    match registry.unregister(&agent_id) {
        Ok(_) => {
            println!("✓ Agent '{}' killed", id);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to kill agent: {}", e);
            Err(e)
        }
    }
}

/// Pure calculation: Build the detail status lines for an agent
fn build_agent_status_lines(agent_id: &str, agent: &Agent) -> Vec<String> {
    let activity_line = match &agent.activity {
        AgentActivity::Idle => "  Activity: idle".to_string(),
        AgentActivity::Working { session, command } => {
            format!("  Activity: working on '{}' - {}", session, command)
        }
    };

    vec![
        format!("Agent '{}':", agent_id),
        format!("  Status: {}", agent.status()),
        format!(
            "  Registered: {}",
            agent.registered_at.format("%Y-%m-%d %H:%M:%S")
        ),
        format!(
            "  Last seen: {}",
            agent.last_seen.format("%Y-%m-%d %H:%M:%S")
        ),
        format!("  Actions: {}", agent.actions_count),
        activity_line,
    ]
}

/// Pure calculation: Build the summary status lines for all agents
fn build_agents_summary_lines(agents: &[Agent], active: &[Agent]) -> Vec<String> {
    let active_agent_lines: Vec<String> =
        active.iter().map(|a| format!("    - {}", a.id)).collect();

    if active.is_empty() {
        vec![
            "Agent Status:".to_string(),
            format!("  Total: {}", agents.len()),
            format!("  Active: {}", active.len()),
        ]
    } else {
        vec![
            "Agent Status:".to_string(),
            format!("  Total: {}", agents.len()),
            format!("  Active: {}", active.len()),
            "  Active agents:".to_string(),
        ]
        .into_iter()
        .chain(active_agent_lines)
        .collect()
    }
}

/// Pure calculation: Build not-found error message
fn status_not_found_message(agent_id: &str) -> String {
    format!("Agent '{}' not found", agent_id)
}

/// Print detailed status for a single agent
fn print_agent_status(agent_id: &str, agent: &Agent) {
    build_agent_status_lines(agent_id, agent)
        .into_iter()
        .for_each(|line| println!("{}", line));
}

/// Print summary status for all agents
fn print_agents_summary(agents: &[Agent], active: &[Agent]) {
    build_agents_summary_lines(agents, active)
        .into_iter()
        .for_each(|line| println!("{}", line));
}

/// Pure calculation: Extract agent not found error
fn agent_not_found_error(agent_id: &str) -> scp_core::Error {
    scp_core::Error::AgentNotFound(agent_id.to_string())
}

/// Show agent status
pub fn status(id: Option<&str>) -> Result<()> {
    let registry = get_agent_registry();

    match id {
        Some(agent_id) => {
            let aid = AgentId::new_checked(agent_id)?;
            match registry.get(&aid)? {
                Some(agent) => {
                    print_agent_status(agent_id, &agent);
                    Ok(())
                }
                None => {
                    eprintln!("{}", status_not_found_message(agent_id));
                    Err(agent_not_found_error(agent_id))
                }
            }
        }
        None => {
            let agents = registry.list()?;
            let active = registry.list_active()?;
            print_agents_summary(&agents, &active);
            Ok(())
        }
    }
}

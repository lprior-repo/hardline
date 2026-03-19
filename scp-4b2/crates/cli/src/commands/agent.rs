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

/// List agents
pub fn list() -> Result<()> {
    let registry = get_agent_registry();

    let agents = registry.list()?;

    if agents.is_empty() {
        println!("No agents registered");
    } else {
        println!("Agents ({} total):", agents.len());
        for agent in &agents {
            let status = agent.status();
            let activity = match &agent.activity {
                AgentActivity::Idle => "idle",
                AgentActivity::Working { session, command } => {
                    println!(
                        "  - {} [{}] working on '{}': {}",
                        agent.id, status, session, command
                    );
                    continue;
                }
            };
            println!("  - {} [{}] {}", agent.id, status, activity);
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

/// Print detailed status for a single agent
fn print_agent_status(agent_id: &str, agent: &Agent) {
    println!("Agent '{}':", agent_id);
    println!("  Status: {}", agent.status());
    println!(
        "  Registered: {}",
        agent.registered_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  Last seen: {}",
        agent.last_seen.format("%Y-%m-%d %H:%M:%S")
    );
    println!("  Actions: {}", agent.actions_count);

    match &agent.activity {
        AgentActivity::Idle => println!("  Activity: idle"),
        AgentActivity::Working { session, command } => {
            println!("  Activity: working on '{}' - {}", session, command);
        }
    }
}

/// Print summary status for all agents
fn print_agents_summary(agents: &[Agent], active: &[Agent]) {
    println!("Agent Status:");
    println!("  Total: {}", agents.len());
    println!("  Active: {}", active.len());

    if !active.is_empty() {
        println!("  Active agents:");
        for agent in active {
            println!("    - {}", agent.id);
        }
    }
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
                    eprintln!("Agent '{}' not found", agent_id);
                    Err(scp_core::Error::AgentNotFound(agent_id.to_string()))
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

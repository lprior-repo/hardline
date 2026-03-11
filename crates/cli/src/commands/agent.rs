//! Agent commands

use scp_core::{
    agent::{get_agent_registry, Agent, AgentId},
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
                scp_core::agent::AgentActivity::Idle => "idle",
                scp_core::agent::AgentActivity::Working { session, command } => {
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

/// Show agent status
pub fn status(id: Option<&str>) -> Result<()> {
    let registry = get_agent_registry();

    if let Some(agent_id) = id {
        let aid = AgentId::new_checked(agent_id)?;

        match registry.get(&aid)? {
            Some(agent) => {
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
                    scp_core::agent::AgentActivity::Idle => {
                        println!("  Activity: idle");
                    }
                    scp_core::agent::AgentActivity::Working { session, command } => {
                        println!("  Activity: working on '{}' - {}", session, command);
                    }
                }

                Ok(())
            }
            None => {
                eprintln!("Agent '{}' not found", agent_id);
                Err(scp_core::Error::AgentNotFound(agent_id.to_string()))
            }
        }
    } else {
        // Show all agents summary
        let agents = registry.list()?;
        let active = registry.list_active()?;

        println!("Agent Status:");
        println!("  Total: {}", agents.len());
        println!("  Active: {}", active.len());

        if !active.is_empty() {
            println!("  Active agents:");
            for agent in &active {
                println!("    - {}", agent.id);
            }
        }

        Ok(())
    }
}

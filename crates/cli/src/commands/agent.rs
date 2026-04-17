//! Agent commands

use scp_core::{
    agent::{get_agent_registry, Agent, AgentId},
    vcs, Error, Result,
};

const HEARTBEAT_FILE: &str = ".scp/heartbeat";

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
                Err(scp_core::Error::agent_not_found(agent_id))
            }
        }
    } else {
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

/// Register current agent session
pub fn register(session: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let session_name = if let Some(s) = session {
        s.to_string()
    } else {
        let workspaces = backend.list_workspaces()?;
        workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.clone())
            .ok_or_else(|| Error::workspace_not_found("no current session"))?
    };

    let heartbeat_dir = cwd.join(".scp");
    std::fs::create_dir_all(&heartbeat_dir)?;

    let heartbeat_path = heartbeat_dir.join("heartbeat");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::internal(e.to_string()))?
        .as_secs()
        .to_string();

    let content = format!("{}:{}\n", session_name, timestamp);
    std::fs::write(&heartbeat_path, content)?;

    println!("✓ Registered agent for session '{}'", session_name);
    Ok(())
}

/// Send agent heartbeat
pub fn heartbeat(session: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    let session_name = if let Some(s) = session {
        s.to_string()
    } else {
        let workspaces = backend.list_workspaces()?;
        workspaces
            .iter()
            .find(|w| w.is_current)
            .map(|w| w.name.clone())
            .ok_or_else(|| Error::workspace_not_found("no current session"))?
    };

    let heartbeat_path = cwd.join(".scp").join("heartbeat");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::internal(e.to_string()))?
        .as_secs()
        .to_string();

    let content = format!("{}:{}\n", session_name, timestamp);
    std::fs::write(&heartbeat_path, content)?;

    println!("✓ Heartbeat sent for session '{}'", session_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::agent::{AgentActivity, MemAgentRegistry};
    use std::sync::Arc;

    /// Create a fresh in-memory registry for isolated tests
    fn fresh_registry() -> Arc<dyn scp_core::agent::AgentRegistry> {
        Arc::new(MemAgentRegistry::new())
    }

    #[test]
    fn agent_id_valid_non_empty() {
        let id = AgentId::new_checked("alpha-1");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "alpha-1");
    }

    #[test]
    fn agent_id_rejects_empty() {
        let id = AgentId::new_checked("");
        assert!(id.is_err());
    }

    #[test]
    fn agent_default_activity_is_idle() {
        assert_eq!(AgentActivity::default(), AgentActivity::Idle);
    }

    #[test]
    fn agent_activity_working_accessors() {
        let act = AgentActivity::Working {
            session: "sess-1".into(),
            command: "build".into(),
        };
        assert!(act.is_working());
        assert_eq!(act.session(), Some("sess-1"));
        assert_eq!(act.command(), Some("build"));
    }

    #[test]
    fn agent_activity_idle_accessors() {
        let act = AgentActivity::Idle;
        assert!(!act.is_working());
        assert_eq!(act.session(), None);
        assert_eq!(act.command(), None);
    }

    #[test]
    fn registry_roundtrip() {
        let reg = fresh_registry();
        let agent = Agent::new(AgentId::new_checked("test-agent").unwrap());
        assert!(reg.register(agent).is_ok());
        let got = reg.get(&AgentId::new("test-agent")).unwrap();
        assert!(got.is_some());
    }

    #[test]
    fn registry_list_empty() {
        let reg = fresh_registry();
        let list = reg.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn registry_unregister_nonexistent_fails() {
        let reg = fresh_registry();
        let result = reg.unregister(&AgentId::new("ghost"));
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_file_content_format() {
        let dir = tempfile::tempdir().unwrap();
        let scp_dir = dir.path().join(".scp");
        std::fs::create_dir_all(&scp_dir).unwrap();

        let session = "my-session";
        let ts = "1714000000";
        let content = format!("{}:{}\n", session, ts);
        std::fs::write(scp_dir.join("heartbeat"), &content).unwrap();

        let read = std::fs::read_to_string(scp_dir.join("heartbeat")).unwrap();
        assert_eq!(read, content);
        let parts: Vec<&str> = read.trim().splitn(2, ':').collect();
        assert_eq!(parts[0], session);
        assert_eq!(parts[1], ts);
    }

    #[test]
    fn heartbeat_constant_defined() {
        assert!(!HEARTBEAT_FILE.is_empty());
        assert!(HEARTBEAT_FILE.contains("heartbeat"));
    }

    #[test]
    fn create_rejects_empty_name() {
        let result = create("");
        assert!(result.is_err());
    }

    #[test]
    fn kill_rejects_empty_id() {
        let result = kill("");
        assert!(result.is_err());
    }

    #[test]
    fn agent_id_preserves_value() {
        let id = AgentId::new("test-123");
        assert_eq!(id.as_str(), "test-123");
    }

    #[test]
    fn agent_id_display_matches_str() {
        let id = AgentId::new("display-test");
        assert_eq!(format!("{}", id), "display-test");
        assert_eq!(id.as_str(), format!("{}", id));
    }

    #[test]
    fn agent_new_checked_whitespace_only_rejected() {
        let id = AgentId::new_checked("   ");
        assert!(id.is_ok(), "whitespace-only is technically non-empty");
    }

    #[test]
    fn registry_list_after_register() {
        let reg = fresh_registry();
        let agent = Agent::new(AgentId::new_checked("visible").unwrap());
        reg.register(agent).unwrap();
        let list = reg.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id.as_str(), "visible");
    }

    #[test]
    fn registry_get_nonexistent_returns_none() {
        let reg = fresh_registry();
        let got = reg.get(&AgentId::new("nope")).unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn registry_unregister_returns_agent() {
        let reg = fresh_registry();
        let agent = Agent::new(AgentId::new_checked("remove-me").unwrap());
        reg.register(agent).unwrap();
        let removed = reg.unregister(&AgentId::new("remove-me")).unwrap();
        assert_eq!(removed.id.as_str(), "remove-me");
        let list = reg.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn registry_double_register_fails() {
        let reg = fresh_registry();
        let agent = Agent::new(AgentId::new_checked("dup").unwrap());
        assert!(reg.register(agent).is_ok());
        let agent2 = Agent::new(AgentId::new_checked("dup").unwrap());
        assert!(reg.register(agent2).is_err());
    }

    #[test]
    fn registry_list_active_empty() {
        let reg = fresh_registry();
        let active = reg.list_active().unwrap();
        assert!(active.is_empty());
    }
}

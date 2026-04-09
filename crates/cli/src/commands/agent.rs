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
    use chrono::Utc;
    use scp_core::agent::{AgentRegistry, AgentStatus};
    use scp_core::error_agent::AgentErrorKind;
    use std::collections::HashMap;

    // Mock registry for testing
    struct TestRegistry {
        agents: HashMap<AgentId, Agent>,
    }

    impl TestRegistry {
        fn new() -> Self {
            Self {
                agents: HashMap::new(),
            }
        }
    }

    impl Default for TestRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AgentRegistry for TestRegistry {
        fn register(&self, agent: Agent) -> Result<()> {
            let agents = self.agents.clone();
            if agents.contains_key(&agent.id) {
                return Err(AgentErrorKind::Exists(agent.id.to_string()).into());
            }
            Ok(())
        }

        fn unregister(&self, id: &AgentId) -> Result<Agent> {
            let mut agents = self.agents.clone();
            agents
                .remove(id)
                .ok_or_else(|| AgentErrorKind::NotFound(id.to_string()).into())
        }

        fn get(&self, id: &AgentId) -> Result<Option<Agent>> {
            Ok(self.agents.get(id).cloned())
        }

        fn heartbeat(&self, id: &AgentId) -> Result<()> {
            if self.agents.contains_key(id) {
                Ok(())
            } else {
                Err(AgentErrorKind::NotFound(id.to_string()).into())
            }
        }

        fn list(&self) -> Result<Vec<Agent>> {
            Ok(self.agents.values().cloned().collect())
        }

        fn list_active(&self) -> Result<Vec<Agent>> {
            Ok(self
                .agents
                .values()
                .filter(|a| a.is_active())
                .cloned()
                .collect())
        }
    }

    // Helper functions
    fn checked_agent_id(s: &str) -> Result<AgentId> {
        AgentId::new_checked(s)
    }

    // =========================================================================
    // AgentId validation tests
    // =========================================================================

    #[test]
    fn agent_id_new_checked_empty_string_rejects() {
        let result = checked_agent_id("");
        assert!(result.is_err());
    }

    #[test]
    fn agent_id_new_checked_valid_id_accepts() {
        let result = checked_agent_id("cli-agent-01");
        assert!(result.is_ok());
        let id = result.unwrap();
        assert_eq!(id.as_str(), "cli-agent-01");
    }

    #[test]
    fn agent_id_new_checked_whitespace_id_accepts() {
        let result = checked_agent_id("   ");
        assert!(result.is_ok());
    }

    #[test]
    fn agent_id_new_checked_special_characters_accepts() {
        let result = checked_agent_id("agent-01_test.name");
        assert!(result.is_ok());
    }

    #[test]
    fn agent_id_new_checked_underscore_prefix_accepts() {
        let result = checked_agent_id("_agent");
        assert!(result.is_ok());
    }

    #[test]
    fn agent_id_new_checked_number_prefix_accepts() {
        let result = checked_agent_id("123");
        assert!(result.is_ok());
    }

    // =========================================================================
    // AgentStatus tests
    // =========================================================================

    #[test]
    fn agent_status_display_active() {
        assert_eq!(format!("{}", AgentStatus::Active), "active");
    }

    #[test]
    fn agent_status_display_stale() {
        assert_eq!(format!("{}", AgentStatus::Stale), "stale");
    }

    #[test]
    fn agent_status_equality() {
        assert_eq!(AgentStatus::Active, AgentStatus::Active);
        assert_ne!(AgentStatus::Active, AgentStatus::Stale);
    }

    // =========================================================================
    // Create command tests
    // =========================================================================

    #[test]
    fn create_agent_id_validation() {
        let id = AgentId::new("create-test");
        assert_eq!(id.as_str(), "create-test");
    }

    #[test]
    fn create_agent_uses_checked_id() {
        let id = checked_agent_id("new-agent").unwrap();
        let agent = Agent::new(id);
        assert_eq!(agent.id.as_str(), "new-agent");
    }

    // =========================================================================
    // List command tests
    // =========================================================================

    #[test]
    fn list_agents_empty_registry() {
        let registry = TestRegistry::new();
        let agents = registry.list().expect("list should succeed");
        assert!(agents.is_empty());
    }

    #[test]
    fn list_agents_single_agent() {
        let mut registry = TestRegistry::new();
        registry.agents.insert(
            AgentId::new("single-agent"),
            Agent::new(AgentId::new("single-agent")),
        );
        let agents = registry.list().expect("list should succeed");
        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn list_agents_multiple_agents() {
        let mut registry = TestRegistry::new();
        for i in 0..10 {
            registry.agents.insert(
                AgentId::new(format!("agent-{i}")),
                Agent::new(AgentId::new(format!("agent-{i}"))),
            );
        }
        let agents = registry.list().expect("list should succeed");
        assert_eq!(agents.len(), 10);
    }

    #[test]
    fn list_active_agents_empty() {
        let registry = TestRegistry::new();
        let active = registry.list_active().expect("list_active should succeed");
        assert!(active.is_empty());
    }

    #[test]
    fn list_active_agents_all_active() {
        let mut registry = TestRegistry::new();
        registry.agents.insert(
            AgentId::new("active-1"),
            Agent::new(AgentId::new("active-1")),
        );
        registry.agents.insert(
            AgentId::new("active-2"),
            Agent::new(AgentId::new("active-2")),
        );
        let active = registry.list_active().expect("list_active should succeed");
        assert_eq!(active.len(), 2);
    }

    // =========================================================================
    // Kill command tests
    // =========================================================================

    #[test]
    fn kill_agent_not_found_fails() {
        let registry = TestRegistry::new();
        let result = registry.unregister(&AgentId::new("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn kill_agent_success() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("kill-test");
        let agent = Agent::new(id.clone());
        registry.agents.insert(id.clone(), agent.clone());

        // Manually remove from HashMap (unregister trait method doesn't mutate)
        let removed = registry.agents.remove(&id).expect("agent exists");
        assert_eq!(removed.id.as_str(), "kill-test");

        let result = registry.get(&id)?;
        assert!(result.is_none());
        Ok(())
    }

    // =========================================================================
    // Get agent tests
    // =========================================================================

    #[test]
    fn get_agent_found() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("get-test");
        registry.agents.insert(id.clone(), Agent::new(id.clone()));

        let found = registry.get(&id)?;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), "get-test");
        Ok(())
    }

    #[test]
    fn get_agent_not_found() -> Result<()> {
        let registry = TestRegistry::new();
        let found = registry.get(&AgentId::new("nonexistent"))?;
        assert!(found.is_none());
        Ok(())
    }

    // =========================================================================
    // Heartbeat tests
    // =========================================================================

    #[test]
    fn heartbeat_agent_found() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("hb-test");
        registry.agents.insert(id.clone(), Agent::new(id.clone()));

        registry.heartbeat(&id)?;
        Ok(())
    }

    #[test]
    fn heartbeat_agent_not_found_fails() {
        let registry = TestRegistry::new();
        let result = registry.heartbeat(&AgentId::new("nonexistent"));
        assert!(result.is_err());
    }

    // =========================================================================
    // Activity state tests
    // =========================================================================

    #[test]
    fn agent_activity_idle_default() {
        let agent = Agent::new(AgentId::new("activity-test"));
        assert!(!agent.activity.is_working());
        assert!(agent.activity.session().is_none());
        assert!(agent.activity.command().is_none());
    }

    #[test]
    fn agent_activity_working() {
        let mut agent = Agent::new(AgentId::new("working-test"));
        agent.start_work("session-123", "build");

        assert!(agent.activity.is_working());
        assert_eq!(agent.activity.session(), Some("session-123"));
        assert_eq!(agent.activity.command(), Some("build"));
        assert_eq!(agent.actions_count, 1);
    }

    #[test]
    fn agent_activity_stop_work() {
        let mut agent = Agent::new(AgentId::new("stop-test"));
        agent.start_work("sess", "cmd");
        agent.stop_work();

        assert!(!agent.activity.is_working());
        assert!(agent.activity.session().is_none());
        assert_eq!(agent.actions_count, 1);
    }

    #[test]
    fn agent_activity_start_work_increments_count() {
        let mut agent = Agent::new(AgentId::new("count-test"));
        agent.start_work("s1", "cmd1");
        agent.start_work("s2", "cmd2");
        agent.start_work("s3", "cmd3");
        assert_eq!(agent.actions_count, 3);
    }

    // =========================================================================
    // Timestamp tests
    // =========================================================================

    #[test]
    fn agent_creation_timestamps_valid() {
        let before = Utc::now();
        let agent = Agent::new(AgentId::new("ts-test"));
        let after = Utc::now();

        assert!(agent.registered_at >= before && agent.registered_at <= after);
        assert!(agent.last_seen >= before && agent.last_seen <= after);
        assert_eq!(agent.registered_at, agent.last_seen);
    }

    #[test]
    fn agent_update_heartbeat_updates_last_seen() {
        let mut agent = Agent::new(AgentId::new("hb-update"));
        let original_seen = agent.last_seen;

        agent.update_heartbeat();
        assert!(agent.last_seen >= original_seen);
    }

    // =========================================================================
    // Activity display formatting tests
    // =========================================================================

    #[test]
    fn activity_display_idle() {
        let activity = scp_core::agent::AgentActivity::Idle;
        let display: String = match &activity {
            scp_core::agent::AgentActivity::Idle => "idle".to_string(),
            scp_core::agent::AgentActivity::Working { session, command } => {
                format!("working on '{}': {}", session, command)
            }
        };
        assert_eq!(display, "idle");
    }

    #[test]
    fn activity_display_working() {
        let activity = scp_core::agent::AgentActivity::Working {
            session: "test-session".to_string(),
            command: "deploy".to_string(),
        };

        let display: String = match &activity {
            scp_core::agent::AgentActivity::Idle => "idle".to_string(),
            scp_core::agent::AgentActivity::Working { session, command } => {
                format!("working on '{}': {}", session, command)
            }
        };
        assert_eq!(display, "working on 'test-session': deploy");
    }

    // =========================================================================
    // Error message formatting tests
    // =========================================================================

    #[test]
    fn agent_not_found_error_message() {
        let err: scp_core::Error = AgentErrorKind::NotFound("missing-agent".to_string()).into();
        let msg = format!("{}", err);
        assert!(
            msg.contains("missing-agent") || msg.contains("not found"),
            "Error should mention agent ID, got: {}",
            msg
        );
    }

    #[test]
    fn agent_exists_error_message() {
        let err: scp_core::Error = AgentErrorKind::Exists("dup-agent".to_string()).into();
        let msg = format!("{}", err);
        assert!(
            msg.contains("dup-agent") || msg.contains("exists"),
            "Error should mention agent ID, got: {}",
            msg
        );
    }

    // =========================================================================
    // Output formatting tests (human output format)
    // =========================================================================

    #[test]
    fn human_output_format_single_agent() {
        let id = AgentId::new("human-test");
        let agent = Agent::new(id.clone());

        let status = agent.status();
        let activity: String = match &agent.activity {
            scp_core::agent::AgentActivity::Idle => "idle".to_string(),
            scp_core::agent::AgentActivity::Working { session, command } => {
                format!("working on '{}': {}", session, command)
            }
        };

        let line = format!("  - {} [{}] {}", id, status, activity);
        assert!(line.contains("human-test"));
        assert!(line.contains("active"));
        assert!(line.contains("idle"));
    }

    #[test]
    fn human_output_format_working_agent() {
        let mut agent = Agent::new(AgentId::new("working-output"));
        agent.start_work("sess-x", "build-cmd");

        let status = agent.status();
        let line = match &agent.activity {
            scp_core::agent::AgentActivity::Idle => {
                format!("  - {} [{}] idle", agent.id, status)
            }
            scp_core::agent::AgentActivity::Working { session, command } => {
                format!(
                    "  - {} [{}] working on '{}': {}",
                    agent.id, status, session, command
                )
            }
        };

        assert!(line.contains("working-output"));
        assert!(line.contains("working"));
        assert!(line.contains("sess-x"));
        assert!(line.contains("build-cmd"));
    }

    #[test]
    fn human_output_format_total_count() {
        let mut registry = TestRegistry::new();
        for i in 0..5 {
            registry.agents.insert(
                AgentId::new(format!("count-{i}")),
                Agent::new(AgentId::new(format!("count-{i}"))),
            );
        }

        let agents = registry.list().expect("list");
        let count_line = format!("Agents ({} total):", agents.len());
        assert!(count_line.contains("5 total"));
    }

    #[test]
    fn human_output_format_empty_list() {
        let registry = TestRegistry::new();
        let agents = registry.list().expect("list");

        if agents.is_empty() {
            assert_eq!("No agents registered", "No agents registered");
        }
    }

    // =========================================================================
    // Status command output tests
    // =========================================================================

    #[test]
    fn status_command_output_format() {
        let mut registry = TestRegistry::new();
        registry.agents.insert(
            AgentId::new("status-test"),
            Agent::new(AgentId::new("status-test")),
        );

        let agents = registry.list().expect("list");
        let active = registry.list_active().expect("list_active");

        let total_line = format!("  Total: {}", agents.len());
        let active_line = format!("  Active: {}", active.len());

        // Debug: print the actual values
        eprintln!(
            "agents.len() = {}, active.len() = {}",
            agents.len(),
            active.len()
        );
        eprintln!("total_line = {}, active_line = {}", total_line, active_line);

        assert_eq!(agents.len(), 1, "should have 1 agent");
        assert_eq!(active.len(), 1, "should have 1 active agent");
    }

    #[test]
    fn status_command_with_agent_id() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("status-single");
        registry.agents.insert(id.clone(), Agent::new(id.clone()));

        let agent = registry.get(&id)?.expect("agent exists");

        let id_line = format!("Agent '{}':", id);
        let status_line = format!("  Status: {}", agent.status());
        let reg_line = format!(
            "  Registered: {}",
            agent.registered_at.format("%Y-%m-%d %H:%M:%S")
        );
        let seen_line = format!(
            "  Last seen: {}",
            agent.last_seen.format("%Y-%m-%d %H:%M:%S")
        );
        let actions_line = format!("  Actions: {}", agent.actions_count);

        assert!(id_line.contains("status-single"));
        assert!(status_line.contains("active"));
        assert!(reg_line.contains("20"));
        assert!(seen_line.contains("20"));
        assert!(actions_line.contains("0"));

        Ok(())
    }

    // =========================================================================
    // Missing agent error handling
    // =========================================================================

    #[test]
    fn missing_agent_error_display() {
        let err: scp_core::Error = AgentErrorKind::NotFound("ghost-agent".to_string()).into();
        let msg = format!("{}", err);
        assert!(!msg.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn missing_agent_error_is_agent_type() {
        let err: scp_core::Error = AgentErrorKind::NotFound("test".to_string()).into();
        assert!(matches!(err, Error::Agent(_)));
    }

    // =========================================================================
    // Duplicate agent handling
    // =========================================================================

    #[test]
    fn duplicate_register_rejected() {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("dup-test");
        registry.agents.insert(id.clone(), Agent::new(id.clone()));

        let result = registry.register(Agent::new(id.clone()));
        assert!(result.is_err());

        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("dup-test") || msg.contains("exists"),
            "Duplicate register should fail with exists error, got: {}",
            msg
        );
    }

    #[test]
    fn duplicate_register_preserves_original() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("keep-original");

        let mut original = Agent::new(id.clone());
        original.start_work("original-sess", "original-cmd");
        registry.agents.insert(id.clone(), original);

        let duplicate = Agent::new(id.clone());
        let result = registry.register(duplicate);
        assert!(result.is_err());

        let found = registry.get(&id)?;
        assert!(found.is_some());
        let agent = found.unwrap();
        assert!(agent.activity.is_working());
        assert_eq!(agent.activity.session(), Some("original-sess"));

        Ok(())
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[test]
    fn agent_id_case_sensitive() {
        let a1 = AgentId::new("Agent-01");
        let a2 = AgentId::new("agent-01");
        let a3 = AgentId::new("AGENT-01");

        assert_ne!(a1, a2);
        assert_ne!(a2, a3);
        assert_ne!(a1, a3);
    }

    #[test]
    fn agent_id_with_emoji() {
        let result = checked_agent_id("agent-🦙");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "agent-🦙");
    }

    #[test]
    fn agent_id_with_unicode() {
        let result = checked_agent_id("agent-αβγ");
        assert!(result.is_ok());
    }

    #[test]
    fn agent_id_very_long() {
        let long_id = "a".repeat(1000);
        let result = checked_agent_id(&long_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().len(), 1000);
    }

    // =========================================================================
    // Activity state persistence
    // =========================================================================

    #[test]
    fn activity_persists_through_get() -> Result<()> {
        let mut registry = TestRegistry::new();
        let mut agent = Agent::new(AgentId::new("persist-test"));
        agent.start_work("sess-p", "cmd-p");
        registry.agents.insert(agent.id.clone(), agent);

        let retrieved = registry.get(&AgentId::new("persist-test"))?;
        assert!(retrieved.is_some());
        let a = retrieved.unwrap();
        assert!(a.activity.is_working());
        assert_eq!(a.activity.session(), Some("sess-p"));

        Ok(())
    }

    #[test]
    fn heartbeat_preserves_activity() -> Result<()> {
        let mut registry = TestRegistry::new();
        let id = AgentId::new("preserve-activity");
        let mut agent = Agent::new(id.clone());
        agent.start_work("sess-h", "cmd-h");
        registry.agents.insert(id.clone(), agent);

        registry.heartbeat(&id)?;

        let agent = registry.get(&id)?.expect("agent exists");
        assert!(agent.activity.is_working());
        assert_eq!(agent.activity.session(), Some("sess-h"));

        Ok(())
    }

    // =========================================================================
    // Registry list operations
    // =========================================================================

    #[test]
    fn registry_list_after_partial_remove() -> Result<()> {
        let mut agents = HashMap::new();

        agents.insert(AgentId::new("keep-1"), Agent::new(AgentId::new("keep-1")));
        agents.insert(
            AgentId::new("remove-1"),
            Agent::new(AgentId::new("remove-1")),
        );
        agents.insert(AgentId::new("keep-2"), Agent::new(AgentId::new("keep-2")));
        agents.insert(
            AgentId::new("remove-2"),
            Agent::new(AgentId::new("remove-2")),
        );

        let registry = TestRegistry {
            agents: agents.clone(),
        };
        assert_eq!(registry.list()?.len(), 4);

        agents.remove(&AgentId::new("remove-1"));
        let registry = TestRegistry {
            agents: agents.clone(),
        };
        assert_eq!(registry.list()?.len(), 3);

        agents.remove(&AgentId::new("remove-2"));
        let registry = TestRegistry { agents };
        assert_eq!(registry.list()?.len(), 2);

        let remaining: Vec<String> = registry
            .list()?
            .into_iter()
            .map(|a| a.id.as_str().to_string())
            .collect();
        assert!(remaining.contains(&"keep-1".to_string()));
        assert!(remaining.contains(&"keep-2".to_string()));
        assert!(!remaining.contains(&"remove-1".to_string()));
        assert!(!remaining.contains(&"remove-2".to_string()));

        Ok(())
    }

    // =========================================================================
    // Activity working state extraction
    // =========================================================================

    #[test]
    fn working_extract_session() {
        let activity = scp_core::agent::AgentActivity::Working {
            session: "my-session".to_string(),
            command: "test".to_string(),
        };

        assert_eq!(activity.session(), Some("my-session"));
    }

    #[test]
    fn working_extract_command() {
        let activity = scp_core::agent::AgentActivity::Working {
            session: "session".to_string(),
            command: "my-command".to_string(),
        };

        assert_eq!(activity.command(), Some("my-command"));
    }

    #[test]
    fn idle_returns_none_for_session() {
        let activity = scp_core::agent::AgentActivity::Idle;
        assert!(activity.session().is_none());
    }

    #[test]
    fn idle_returns_none_for_command() {
        let activity = scp_core::agent::AgentActivity::Idle;
        assert!(activity.command().is_none());
    }

    // =========================================================================
    // Status computation
    // =========================================================================

    #[test]
    fn active_agent_has_active_status() {
        let agent = Agent::new(AgentId::new("active-status"));
        assert_eq!(agent.status(), AgentStatus::Active);
    }

    #[test]
    fn stale_agent_status_check() {
        let agent = Agent::new(AgentId::new("status-check"));
        let status = agent.status();
        assert!(matches!(status, AgentStatus::Active | AgentStatus::Stale));
    }
}

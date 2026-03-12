//! KIRK Contracts for Agent CLI operations.
//!
//! Agents are autonomous workers that can operate on sessions.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// AGENT INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for spawning an agent.
#[derive(Debug, Clone)]
pub struct SpawnAgentInput {
    /// Session to work on
    pub session: String,
    /// Agent type (claude, cursor, etc.)
    pub agent_type: String,
    /// Task description
    pub task: String,
    /// Maximum runtime in seconds
    pub timeout: Option<u64>,
}

/// Input for listing agents.
#[derive(Debug, Clone, Default)]
pub struct ListAgentsInput {
    /// Filter by status
    pub status: Option<String>,
    /// Filter by session
    pub session: Option<String>,
}

/// Input for stopping an agent.
#[derive(Debug, Clone)]
pub struct StopAgentInput {
    /// Agent ID
    pub agent_id: String,
    /// Force stop (SIGKILL vs SIGTERM)
    pub force: bool,
}

/// Input for waiting for an agent.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WaitAgentInput {
    /// Agent ID
    pub agent_id: String,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

/// Result of agent operations.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Agent ID
    pub id: String,
    /// Agent type
    pub agent_type: String,
    /// Session being worked on
    pub session: String,
    /// Current status
    pub status: String,
    /// PID (if running)
    pub pid: Option<u32>,
}

/// Result of agent listing.
#[derive(Debug, Clone)]
pub struct AgentListResult {
    /// List of agents
    pub agents: Vec<AgentResult>,
    /// Total count
    pub total: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// AGENT CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Agent CLI operations.
pub struct AgentContracts;

impl AgentContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: session exists.
    pub const PRECOND_SESSION_EXISTS: Precondition =
        Precondition::new("session_exists", "Session must exist in the database");

    /// Precondition: session is not locked by another agent.
    pub const PRECOND_SESSION_NOT_LOCKED: Precondition = Precondition::new(
        "session_not_locked",
        "Session must not be locked by another agent",
    );

    /// Precondition: agent type is valid.
    pub const PRECOND_AGENT_TYPE_VALID: Precondition =
        Precondition::new("agent_type_valid", "Agent type must be supported");

    /// Precondition: agent exists.
    pub const PRECOND_AGENT_EXISTS: Precondition =
        Precondition::new("agent_exists", "Agent must exist in the database");

    /// Precondition: agent is running.
    pub const PRECOND_AGENT_RUNNING: Precondition =
        Precondition::new("agent_running", "Agent must be in running state");

    /// Precondition: task is not empty.
    pub const PRECOND_TASK_NOT_EMPTY: Precondition =
        Precondition::new("task_not_empty", "Task description must not be empty");

    /// Precondition: timeout is reasonable.
    pub const PRECOND_TIMEOUT_VALID: Precondition = Precondition::new(
        "timeout_valid",
        "Timeout must be between 1 second and 24 hours",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: one agent per session.
    pub const INV_ONE_AGENT_PER_SESSION: Invariant = Invariant::documented(
        "one_agent_per_session",
        "At most one agent can work on a session",
    );

    /// Invariant: agent ID is unique.
    pub const INV_AGENT_ID_UNIQUE: Invariant =
        Invariant::documented("agent_id_unique", "Each agent has a unique identifier");

    /// Invariant: agent has valid PID when running.
    pub const INV_VALID_PID: Invariant =
        Invariant::documented("valid_pid", "Running agents have a valid PID > 0");

    /// Invariant: lock is held while agent runs.
    pub const INV_LOCK_HELD: Invariant =
        Invariant::documented("lock_held", "Agent holds session lock while running");

    /// Invariant: agent releases lock on termination.
    pub const INV_LOCK_RELEASED: Invariant =
        Invariant::documented("lock_released", "Agent releases lock when terminated");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: agent was spawned.
    pub const POST_AGENT_SPAWNED: Postcondition =
        Postcondition::new("agent_spawned", "Agent is running with a valid PID");

    /// Postcondition: agent was stopped.
    pub const POST_AGENT_STOPPED: Postcondition =
        Postcondition::new("agent_stopped", "Agent is no longer running");

    /// Postcondition: session lock was acquired.
    pub const POST_LOCK_ACQUIRED: Postcondition =
        Postcondition::new("lock_acquired", "Session is locked by the agent");

    /// Postcondition: session lock was released.
    pub const POST_LOCK_RELEASED: Postcondition = Postcondition::new(
        "lock_released_post",
        "Session lock is released after agent stops",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate an agent type.
    ///
    /// # Errors
    /// Returns `ContractError` if the agent type is invalid.
    pub fn validate_agent_type(agent_type: &str) -> Result<(), ContractError> {
        match agent_type {
            "claude" | "cursor" | "aider" | "copilot" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "agent_type",
                "must be one of: claude, cursor, aider, copilot",
            )),
        }
    }

    /// Validate an agent status.
    ///
    /// # Errors
    /// Returns `ContractError` if the status is invalid.
    pub fn validate_status(status: &str) -> Result<(), ContractError> {
        match status {
            "pending" | "running" | "completed" | "failed" | "cancelled" | "timeout" => Ok(()),
            _ => Err(ContractError::invalid_input(
                "status",
                "must be one of: pending, running, completed, failed, cancelled, timeout",
            )),
        }
    }

    /// Validate a timeout value.
    ///
    /// # Errors
    /// Returns `ContractError` if the timeout is invalid.
    pub fn validate_timeout(timeout: u64) -> Result<(), ContractError> {
        const MIN_TIMEOUT: u64 = 1;
        const MAX_TIMEOUT: u64 = 24 * 60 * 60; // 24 hours

        if timeout < MIN_TIMEOUT {
            return Err(ContractError::invalid_input(
                "timeout",
                "must be at least 1 second",
            ));
        }
        if timeout > MAX_TIMEOUT {
            return Err(ContractError::invalid_input(
                "timeout",
                "cannot exceed 24 hours",
            ));
        }
        Ok(())
    }

    /// Validate a PID.
    ///
    /// # Errors
    /// Returns `ContractError` if the PID is invalid.
    pub fn validate_pid(pid: u32) -> Result<(), ContractError> {
        if pid == 0 {
            return Err(ContractError::invalid_input("pid", "must be > 0"));
        }
        Ok(())
    }
}

impl Contract<SpawnAgentInput, AgentResult> for AgentContracts {
    fn preconditions(input: &SpawnAgentInput) -> Result<(), ContractError> {
        if input.session.trim().is_empty() {
            return Err(ContractError::invalid_input("session", "cannot be empty"));
        }

        Self::validate_agent_type(&input.agent_type)?;

        if input.task.trim().is_empty() {
            return Err(ContractError::invalid_input("task", "cannot be empty"));
        }

        if let Some(timeout) = input.timeout {
            Self::validate_timeout(timeout)?;
        }

        Ok(())
    }

    fn invariants(_input: &SpawnAgentInput) -> Vec<Invariant> {
        vec![
            Self::INV_ONE_AGENT_PER_SESSION,
            Self::INV_AGENT_ID_UNIQUE,
            Self::INV_VALID_PID,
            Self::INV_LOCK_HELD,
        ]
    }

    fn postconditions(input: &SpawnAgentInput, result: &AgentResult) -> Result<(), ContractError> {
        if result.session != input.session {
            return Err(ContractError::PostconditionFailed {
                name: "session_matches",
                description: "Result session must match input",
            });
        }
        if result.agent_type != input.agent_type {
            return Err(ContractError::PostconditionFailed {
                name: "agent_type_matches",
                description: "Result agent type must match input",
            });
        }
        if result.status != "running" {
            return Err(ContractError::PostconditionFailed {
                name: "running_status",
                description: "Spawned agent must have status 'running'",
            });
        }
        if result.pid.is_none() {
            return Err(ContractError::PostconditionFailed {
                name: "has_pid",
                description: "Spawned agent must have a PID",
            });
        }
        Ok(())
    }
}

impl Contract<ListAgentsInput, AgentListResult> for AgentContracts {
    fn preconditions(input: &ListAgentsInput) -> Result<(), ContractError> {
        if let Some(ref status) = input.status {
            Self::validate_status(status)?;
        }
        Ok(())
    }

    fn invariants(_input: &ListAgentsInput) -> Vec<Invariant> {
        vec![Self::INV_ONE_AGENT_PER_SESSION, Self::INV_AGENT_ID_UNIQUE]
    }

    fn postconditions(
        input: &ListAgentsInput,
        result: &AgentListResult,
    ) -> Result<(), ContractError> {
        if let Some(ref status) = input.status {
            let all_match_status = result.agents.iter().all(|a| &a.status == status);
            if !all_match_status {
                return Err(ContractError::PostconditionFailed {
                    name: "status_filter",
                    description: "All returned agents must match the status filter",
                });
            }
        }

        if result.agents.len() > result.total {
            return Err(ContractError::PostconditionFailed {
                name: "count_consistent",
                description: "Agent count must not exceed total",
            });
        }

        Ok(())
    }
}

impl Contract<StopAgentInput, ()> for AgentContracts {
    fn preconditions(input: &StopAgentInput) -> Result<(), ContractError> {
        if input.agent_id.trim().is_empty() {
            return Err(ContractError::invalid_input("agent_id", "cannot be empty"));
        }
        Ok(())
    }

    fn invariants(_input: &StopAgentInput) -> Vec<Invariant> {
        vec![Self::INV_LOCK_RELEASED]
    }

    fn postconditions(_input: &StopAgentInput, _result: &()) -> Result<(), ContractError> {
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_agent_type_valid() {
        assert!(AgentContracts::validate_agent_type("claude").is_ok());
        assert!(AgentContracts::validate_agent_type("cursor").is_ok());
        assert!(AgentContracts::validate_agent_type("aider").is_ok());
        assert!(AgentContracts::validate_agent_type("copilot").is_ok());
    }

    #[test]
    fn test_validate_agent_type_invalid() {
        assert!(AgentContracts::validate_agent_type("gpt").is_err());
        assert!(AgentContracts::validate_agent_type("unknown").is_err());
    }

    #[test]
    fn test_validate_status_valid() {
        assert!(AgentContracts::validate_status("pending").is_ok());
        assert!(AgentContracts::validate_status("running").is_ok());
        assert!(AgentContracts::validate_status("completed").is_ok());
        assert!(AgentContracts::validate_status("failed").is_ok());
        assert!(AgentContracts::validate_status("cancelled").is_ok());
        assert!(AgentContracts::validate_status("timeout").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        assert!(AgentContracts::validate_status("done").is_err());
        assert!(AgentContracts::validate_status("stopped").is_err());
    }

    #[test]
    fn test_validate_timeout_valid() {
        assert!(AgentContracts::validate_timeout(1).is_ok());
        assert!(AgentContracts::validate_timeout(3600).is_ok());
        assert!(AgentContracts::validate_timeout(24 * 60 * 60).is_ok());
    }

    #[test]
    fn test_validate_timeout_invalid() {
        assert!(AgentContracts::validate_timeout(0).is_err());
        assert!(AgentContracts::validate_timeout(24 * 60 * 60 + 1).is_err());
    }

    #[test]
    fn test_validate_pid_valid() {
        assert!(AgentContracts::validate_pid(1).is_ok());
        assert!(AgentContracts::validate_pid(1000).is_ok());
    }

    #[test]
    fn test_validate_pid_invalid() {
        assert!(AgentContracts::validate_pid(0).is_err());
    }

    #[test]
    fn test_spawn_agent_contract_preconditions() {
        let input = SpawnAgentInput {
            session: "test-session".to_string(),
            agent_type: "claude".to_string(),
            task: "Fix the bug".to_string(),
            timeout: Some(3600),
        };
        assert!(AgentContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_spawn_agent_contract_preconditions_empty_task() {
        let input = SpawnAgentInput {
            session: "test-session".to_string(),
            agent_type: "claude".to_string(),
            task: String::new(),
            timeout: None,
        };
        assert!(AgentContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_spawn_agent_contract_postconditions() {
        let input = SpawnAgentInput {
            session: "test-session".to_string(),
            agent_type: "claude".to_string(),
            task: "Fix the bug".to_string(),
            timeout: None,
        };
        let result = AgentResult {
            id: "agent-123".to_string(),
            agent_type: "claude".to_string(),
            session: "test-session".to_string(),
            status: "running".to_string(),
            pid: Some(12345),
        };
        assert!(AgentContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_spawn_agent_contract_postconditions_no_pid() {
        let input = SpawnAgentInput {
            session: "test-session".to_string(),
            agent_type: "claude".to_string(),
            task: "Fix the bug".to_string(),
            timeout: None,
        };
        let result = AgentResult {
            id: "agent-123".to_string(),
            agent_type: "claude".to_string(),
            session: "test-session".to_string(),
            status: "running".to_string(),
            pid: None,
        };
        assert!(AgentContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_agents_contract_postconditions_filter() {
        let input = ListAgentsInput {
            status: Some("running".to_string()),
            session: None,
        };
        let result = AgentListResult {
            agents: vec![
                AgentResult {
                    id: "1".to_string(),
                    agent_type: "claude".to_string(),
                    session: "s1".to_string(),
                    status: "running".to_string(),
                    pid: Some(1),
                },
                AgentResult {
                    id: "2".to_string(),
                    agent_type: "cursor".to_string(),
                    session: "s2".to_string(),
                    status: "running".to_string(),
                    pid: Some(2),
                },
            ],
            total: 2,
        };
        assert!(AgentContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_agents_contract_postconditions_filter_mismatch() {
        let input = ListAgentsInput {
            status: Some("running".to_string()),
            session: None,
        };
        let result = AgentListResult {
            agents: vec![
                AgentResult {
                    id: "1".to_string(),
                    agent_type: "claude".to_string(),
                    session: "s1".to_string(),
                    status: "running".to_string(),
                    pid: Some(1),
                },
                AgentResult {
                    id: "2".to_string(),
                    agent_type: "cursor".to_string(),
                    session: "s2".to_string(),
                    status: "completed".to_string(), // Wrong!
                    pid: None,
                },
            ],
            total: 2,
        };
        assert!(AgentContracts::postconditions(&input, &result).is_err());
    }
}

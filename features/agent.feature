# Feature: Agent Management
#
# BDD acceptance tests for agent object lifecycle and subcommands.
# Agents are autonomous workers that can register, send heartbeats,
# and coordinate via locks.
#
# State Machine: Unregistered -> Registered -> (Active | Stale) -> Unregistered
#
# Actual CLI commands (from crates/cli/src/cli/args.rs):
#   scp agent create <name>           -> create an agent
#   scp agent list                    -> list all agents
#   scp agent kill <id>               -> kill/remove an agent
#   scp agent status [id]             -> show agent status
#   scp agent register [--session <s>]-> register current agent session
#   scp agent heartbeat [--session <s>]-> send agent heartbeat
#
# NOTE: "whoami" command does NOT exist. Use "scp agent status" to query current agent.
# NOTE: "unregister" command does NOT exist. Use "scp agent kill" to remove an agent.
# NOTE: "register" takes --session flag, not an ID directly.
#
# Actual error types (from crates/scp-error/src/lib.rs):
#   AgentNotFound   -> "Agent not found: {0}"     exit 70
#   AgentExists     -> "Agent already registered: {0}" exit 71
#   AgentTimeout    -> "Agent '{0}' heartbeat timeout" exit 72
#   ValidationError -> "Validation error: {0}"    exit 90
#
# Key Invariants:
# - Unique agent IDs (no two agents can have the same ID)
# - Agents must be registered before sending heartbeats
# - Heartbeats track liveness (stale after timeout)
#
# See: crates/core/src/domain/agent_registry/entities/agent.rs for implementation

Feature: Agent Management

  As an autonomous agent using SCP
  I want to manage my agent identity
  So that I can coordinate work with other agents

  Background:
    Given the SCP database is initialized
    And I am in a JJ repository

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # CREATE AGENT (scp agent create <name>)
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Create agent succeeds
    Given no agent with ID "agent-test-001" exists
    When I create an agent named "agent-test-001"
    Then the agent "agent-test-001" should exist
    And the agent details should be returned as JSON

  Scenario: Create duplicate agent
    Given an agent with ID "agent-duplicate" exists
    When I attempt to create an agent named "agent-duplicate"
    Then the operation should fail with error "AgentExists"
    And the error message should contain "Agent already registered"
    And the exit code should be 71

  Scenario: Create agent with invalid name fails
    Given no agent is registered
    When I attempt to create an agent with name ""
    Then the operation should fail with error "ValidationError"
    And the error message should contain "Validation error"
    And the exit code should be 90

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # REGISTER AGENT (scp agent register [--session <name>])
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Register creates agent session
    Given no agent with ID "agent-test-001" exists
    When I register an agent with session "feature-session"
    Then the agent should be registered
    And the environment variable "SCP_AGENT_ID" should be set
    And the agent details should be returned as JSON

  @pending
  # NOTE: "scp agent register" does not support auto-generated IDs.
  #       This scenario describes desired future behavior.
  Scenario: Register with auto-generated ID
    Given no agent is registered
    When I register an agent without specifying an ID
    Then an agent should be created with an auto-generated ID
    And the agent ID should match pattern "agent-XXXXXXXX-XXXX"
    And the environment variable "SCP_AGENT_ID" should be set

  Scenario: Register with whitespace name fails
    Given no agent is registered
    When I attempt to register an agent with name "   "
    Then the operation should fail with error "ValidationError"
    And the error message should contain "Validation error"
    And the exit code should be 90

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # HEARTBEAT (scp agent heartbeat [--session <name>])
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Heartbeat updates timestamp
    Given an agent with ID "agent-heartbeat" exists
    When I send a heartbeat for agent "agent-heartbeat"
    Then the agent "agent-heartbeat" should have an updated last_seen timestamp

  Scenario: Heartbeat for unknown agent fails
    Given no agent with ID "agent-ghost" exists
    When I attempt to send a heartbeat for agent "agent-ghost"
    Then the operation should fail with error "AgentNotFound"
    And the error message should contain "Agent not found"
    And the exit code should be 70

  @pending
  # NOTE: "scp agent heartbeat" does not currently accept a --command argument.
  #       This scenario describes desired future behavior.
  Scenario: Heartbeat with command updates current_command
    Given an agent with ID "agent-cmd" exists
    When I send a heartbeat with command "scp add feature-x"
    Then the agent "agent-cmd" should have current_command set to "scp add feature-x"
    And the last_seen timestamp should be updated

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # LIST AGENTS (scp agent list)
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: List shows all agents
    Given agents "agent-alpha", "agent-beta", and "agent-gamma" exist
    When I list all agents
    Then the output should contain 3 agents
    And each agent should show agent_id, registered_at, and last_seen
    And the output should be valid JSON

  Scenario: List empty returns empty array
    Given no agents exist
    When I list all agents
    Then the output should show 0 agents
    And the output should be valid JSON

  @pending
  # NOTE: "scp agent list" does not have --all or --session filter flags.
  #       This scenario describes desired future behavior.
  Scenario: List with --all shows stale agents
    Given agent "agent-active" exists and is active
    And agent "agent-stale" exists and is stale
    When I list all agents with --all flag
    Then the output should contain 2 agents
    And the total_stale count should be at least 1

  @pending
  # NOTE: "scp agent list" does not have --session filter flag.
  Scenario: List filters by session
    Given agent "agent-a" is working on session "feature-auth"
    And agent "agent-b" is working on session "feature-db"
    When I list agents filtered by session "feature-auth"
    Then the output should contain only "agent-a"
    And "agent-b" should not appear in the output

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # INVARIANT: UNIQUE AGENT IDS
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Each agent has a unique ID
    Given agents "agent-1", "agent-2", and "agent-3" exist
    When I inspect the agent IDs
    Then all agent IDs should be unique
    And no two agents should share the same ID

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # STATUS (scp agent status [id])
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Status shows agent details
    Given an agent with ID "agent-status" exists
    When I query the agent status for "agent-status"
    Then the output should show agent_id "agent-status"
    And the output should show registered_at timestamp
    And the output should show last_seen timestamp

  Scenario: Status for non-existent agent fails
    Given no agent with ID "agent-ghost" exists
    When I query the agent status for "agent-ghost"
    Then the operation should fail with error "AgentNotFound"
    And the error message should contain "Agent not found"
    And the exit code should be 70

  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  # KILL AGENT (scp agent kill <id>)
  # NOTE: There is NO "unregister" command. Use "scp agent kill" instead.
  # ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Scenario: Kill removes agent
    Given an agent with ID "agent-kill-test" exists
    When I kill the agent "agent-kill-test"
    Then the agent "agent-kill-test" should not exist

  Scenario: Kill non-existent agent fails
    Given no agent with ID "agent-ghost" exists
    When I attempt to kill agent "agent-ghost"
    Then the operation should fail with error "AgentNotFound"
    And the error message should contain "Agent not found"
    And the exit code should be 70

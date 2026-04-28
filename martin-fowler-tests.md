# Martin Fowler Test Plan: Agent Registration

## Happy Path Tests

- test_register_agent_with_explicit_id_succeeds
  Given: No agent with ID "agent-test-001" exists
  When: I register an agent with ID "agent-test-001"
  Then:
    - Agent "agent-test-001" should exist
    - Agent "agent-test-001" should be registered
    - Environment variable "ISOLATE_AGENT_ID" should be set to "agent-test-001"
    - Agent details should be returned as JSON

- test_register_agent_with_auto_generated_id_succeeds
  Given: No agent is registered
  When: I register an agent without specifying an ID
  Then:
    - An agent should be created with an auto-generated ID
    - Agent ID should match pattern "agent-XXXXXXXX-XXXX"
    - Environment variable "ISOLATE_AGENT_ID" should be set
    - Agent details should be returned as JSON

- test_register_duplicate_id_updates_last_seen
  Given: An agent with ID "agent-duplicate" exists
  When: I register an agent with ID "agent-duplicate"
  Then:
    - Operation should succeed with update semantics
    - Agent "agent-duplicate" should have an updated last_seen timestamp

## Error Path Tests

- test_register_with_empty_id_returns_validation_error
  Given: No agent is registered
  When: I attempt to register an agent with ID ""
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be empty" })`

- test_register_with_whitespace_id_returns_validation_error
  Given: No agent is registered
  When: I attempt to register an agent with ID "   "
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be empty or whitespace-only" })`

- test_register_with_reserved_keyword_returns_validation_error
  Given: No agent is registered
  When: I attempt to register an agent with ID "null"
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be a reserved keyword" })`

## Edge Case Tests

- test_register_with_max_length_id_succeeds
  Given: No agent is registered
  When: I register an agent with a 255-character ID
  Then: Registration succeeds

- test_register_reserved_keyword_undefined_fails
  Given: No agent is registered
  When: I attempt to register an agent with ID "undefined"
  Then: Returns validation error for reserved keyword

- test_register_reserved_keyword_none_fails
  Given: No agent is registered
  When: I attempt to register an agent with ID "none"
  Then: Returns validation error for reserved keyword

## Contract Verification Tests

- test_precondition_id_not_empty_enforced_at_runtime
  Given: No validation has occurred
  When: validate_agent_id("") is called
  Then: Returns ValidationError

- test_precondition_id_not_whitespace_only_enforced_at_runtime
  Given: No validation has occurred
  When: validate_agent_id("   ") is called
  Then: Returns ValidationError

- test_precondition_id_not_reserved_keyword_enforced_at_runtime
  Given: No validation has occurred
  When: validate_agent_id("null") is called
  Then: Returns ValidationError

- test_postcondition_agent_exists_in_registry
  Given: A valid registration request
  When: register_agent succeeds
  Then: get_agent(id) returns Some(agent)

- test_postcondition_environment_variable_set
  Given: A valid registration request
  When: register_agent succeeds
  Then: std::env::var("ISOLATE_AGENT_ID") == Ok(id)

- test_postcondition_agent_details_returned_as_json
  Given: A valid registration request
  When: register_agent succeeds
  Then: Return value contains valid JSON with agent details

- test_postcondition_duplicate_id_updates_timestamp
  Given: Agent "dup-agent" exists with old last_seen
  When: register_agent(Some("dup-agent")) is called
  Then: Agent's last_seen is updated to a newer timestamp

## Contract Violation Tests

- `test_p2_violation_returns_validation_error`
  Given: No agent is registered
  When: register_agent(Some("".to_string())) is called
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be empty" })` -- NOT a panic

- `test_p3_violation_returns_validation_error`
  Given: No agent is registered
  When: register_agent(Some("   ".to_string())) is called
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be empty or whitespace-only" })` -- NOT a panic

- `test_p4_violation_returns_validation_error`
  Given: No agent is registered
  When: register_agent(Some("null".to_string())) is called
  Then: Returns `Err(Error::ValidationError { message: "Agent ID cannot be a reserved keyword" })` -- NOT a panic

- `test_q1_violation_agent_not_created`
  Given: register_agent is called with valid ID
  When: Q1 is violated (agent not stored in registry)
  Then: get_agent(id) returns None

- `test_q3_violation_env_var_not_set`
  Given: register_agent succeeds
  When: Q3 is violated (env var not set)
  Then: std::env::var("ISOLATE_AGENT_ID") returns Err

## Given-When-Then Scenarios

### Scenario 1: Register creates agent (lines 30-36)
Given: No agent with ID "agent-test-001" exists
When: I register an agent with ID "agent-test-001"
Then:
- The agent "agent-test-001" should exist
- The agent "agent-test-001" should be registered
- The environment variable "ISOLATE_AGENT_ID" should be set to "agent-test-001"
- The agent details should be returned as JSON

### Scenario 2: Register with auto-generated ID (lines 38-43)
Given: No agent is registered
When: I register an agent without specifying an ID
Then:
- An agent should be created with an auto-generated ID
- The agent ID should match pattern "agent-XXXXXXXX-XXXX"
- The environment variable "ISOLATE_AGENT_ID" should be set

### Scenario 3: Duplicate ID succeeds with update semantics (lines 45-49)
Given: An agent with ID "agent-duplicate" exists
When: I attempt to register an agent with ID "agent-duplicate"
Then:
- The operation should succeed with update semantics
- The agent "agent-duplicate" should have an updated last_seen timestamp

### Scenario 4: Register with invalid empty ID fails (lines 51-55)
Given: No agent is registered
When: I attempt to register an agent with ID ""
Then:
- The operation should fail with error "VALIDATION_ERROR"
- The error message should indicate "Agent ID cannot be empty"

### Scenario 5: Register with whitespace ID fails (lines 57-61)
Given: No agent is registered
When: I attempt to register an agent with ID "   "
Then:
- The operation should fail with error "VALIDATION_ERROR"
- The error message should indicate "Agent ID cannot be empty or whitespace-only"

### Scenario 6: Register with reserved keyword fails (lines 63-67)
Given: No agent is registered
When: I attempt to register an agent with ID "null"
Then:
- The operation should fail with error "VALIDATION_ERROR"
- The error message should indicate "reserved keyword"

---

# Martin Fowler Test Plan: Session List

## Happy Path Tests

- test_list_all_sessions_returns_all
  Given: Sessions "feature-a", "feature-b", and "feature-c" exist
  When: I list all sessions with no filter
  Then:
    - The output should contain 3 sessions
    - Each session should show name, status, and workspace path
    - The output should be valid JSON lines

- test_list_empty_returns_empty_array
  Given: No sessions exist
  When: I list all sessions
  Then:
    - The output should be an empty array
    - The output should be valid JSON

- test_list_filter_by_status_returns_matching
  Given: Sessions with statuses "active", "paused", and "completed" exist
  When: I list sessions with status filter "active"
  Then:
    - Only sessions with status "active" should be shown
    - Output should be valid JSON lines

## Error Path Tests

- test_list_invalid_status_filter_returns_error
  Given: Sessions exist
  When: I list sessions with status filter "invalid-status"
  Then: Returns `Err(Error::InvalidStatusFilter)`

- test_list_storage_error_returns_error
  Given: Session storage is corrupted
  When: I list all sessions
  Then: Returns `Err(Error::SessionStorageError)`

## Edge Case Tests

- test_list_single_session_returns_json_lines
  Given: One session exists
  When: I list all sessions
  Then: Output is valid JSON lines (one line)

- test_list_all_statuses_filtered
  Given: Sessions with all status types exist (active, paused, completed)
  When: I list sessions with status filter "paused"
  Then: Only paused sessions are returned

## Contract Verification Tests

- test_precondition_status_filter_valid
  Given: A status filter string is provided
  When: list_sessions is called
  Then: Filter is validated before querying storage

- test_postcondition_all_sessions_returned
  Given: 3 sessions exist in storage
  When: list_sessions(None) is called
  Then: Returns exactly 3 sessions

- test_postcondition_filtered_sessions_match
  Given: 2 active, 1 paused session exist
  When: list_sessions(Some(Active)) is called
  Then: Returns exactly 2 sessions, all with status Active

- test_postcondition_output_includes_required_fields
  Given: Sessions exist
  When: list_sessions_json is called
  Then: Each JSON object contains name, status, workspace_path

- test_postcondition_empty_is_array
  Given: No sessions exist
  When: list_sessions_json is called
  Then: Output is "[]" (empty JSON array)

## Contract Violation Tests

- `test_p2_violation_returns_invalid_status_filter`
  Given: Status filter "invalid" is provided
  When: list_sessions(Some(InvalidStatus)) is called
  Then: Returns `Err(Error::InvalidStatusFilter)` -- NOT a panic

- `test_q1_violation_not_all_sessions_returned`
  Given: 3 sessions exist
  When: Q1 is violated (not all sessions returned)
  Then: Returns fewer than 3 sessions

- `test_q4_violation_invalid_json_output`
  Given: Session has invalid UTF-8 in path
  When: list_sessions_json is called
  Then: Returns `Err(Error::SessionStorageError)` -- NOT invalid JSON

## Given-When-Then Scenarios

### Scenario 1: List shows all sessions (lines 165-170)
Given: Sessions "feature-a", "feature-b", and "feature-c" exist
When: I list all sessions
Then:
- The output should contain 3 sessions
- Each session should show name, status, and workspace path
- The output should be valid JSON lines

### Scenario 2: List empty returns empty array (lines 172-176)
Given: No sessions exist
When: I list all sessions
Then:
- The output should be an empty array
- The output should be valid JSON

### Scenario 3: List with filter shows matching sessions (lines 178-182)
Given: Sessions with statuses "active", "paused", and "completed" exist
When: I list sessions with status filter "active"
Then:
- Only sessions with status "active" should be shown

---

# Martin Fowler Test Plan: Agent Heartbeat
- The operation should fail with error "SESSION_NOT_FOUND"

### Scenario 4: Focus with empty name fails
Given: No session name provided
When: I attempt to focus on ""
Then:
- The operation should fail with error "SESSION_NAME_REQUIRED"
- The error message should indicate "Session name is required"

### Scenario 5: Focus with whitespace-only name fails
Given: Whitespace-only session name provided
When: I attempt to focus on "   "
Then:
- The operation should fail with error "SESSION_NAME_REQUIRED"
- The error message should indicate "Session name is required"

### Scenario 6: Focus completed session fails
Given: A session named "completed-session" exists with status "completed"
When: I focus on the session "completed-session"
Then:
- The operation should fail with error "INVALID_SESSION_STATUS"
- The error message should indicate "Cannot focus completed session"

---

# Test Plan: Agent Heartbeat

## Happy Path Tests

- test_heartbeat_updates_timestamp
  Given: An agent with ID "agent-heartbeat" exists
  When: I send a heartbeat for the agent
  Then: The agent should have an updated last_seen timestamp
  And: The actions_count should be incremented
  And: The heartbeat timestamp should be returned

- test_heartbeat_with_command_updates_current_command
  Given: An agent with ID "agent-cmd" exists
  When: I send a heartbeat with command "isolate add feature-x"
  Then: The agent should have current_command set to "isolate add feature-x"
  And: The last_seen timestamp should be updated

- test_heartbeat_returns_output_with_timestamp
  Given: An agent is registered
  When: I send a heartbeat
  Then: The response should include agent_id, timestamp (RFC3339), and message

## Error Path Tests

- test_heartbeat_unknown_agent_returns_error
  Given: No agent is registered in the environment
  When: I attempt to send a heartbeat
  Then: The operation should fail with error "NO_AGENT_REGISTERED"
  And: The error message should indicate "No agent registered"

- test_heartbeat_unregistered_agent_fails
  Given: The environment variable ISOLATE_AGENT_ID is set to "agent-ghost"
  And: No agent with ID "agent-ghost" exists in the database
  When: I attempt to send a heartbeat
  Then: The operation should fail with error "AGENT_NOT_FOUND"
  And: The error message should indicate "not found"

## Edge Case Tests

- test_heartbeat_command_preserves_existing_when_none
  Given: An agent with current_command="previous-command" exists
  When: I send a heartbeat with no command
  Then: The current_command should remain "previous-command"

- test_heartbeat_multiple_increments_count
  Given: An agent with actions_count=0 exists
  When: I send 3 heartbeats
  Then: The actions_count should be 3

## Contract Verification Tests

- test_precondition_agent_exists
  Given: A heartbeat request with agent_id
  When: heartbeat is called
  Then: Agent existence is validated before update

- test_postcondition_last_seen_updated
  Given: An agent with last_seen=T1
  When: heartbeat is called at time T2 > T1
  Then: The agent's last_seen should be >= T2

- test_postcondition_actions_count_incremented
  Given: An agent with actions_count=N
  When: heartbeat is called
  Then: The agent's actions_count should be N+1

- test_postcondition_command_updated
  Given: An agent exists
  When: heartbeat is called with command="test"
  Then: The agent's current_command should be "test"

## Contract Violation Tests

- test_agent_not_found_violation_returns_error
  Given: No agent with id "nonexistent" exists
  When: heartbeat(HeartbeatRequest { agent_id: "nonexistent", command: None }) is called
  Then: Returns Err(Error::NotFound("Agent not found: nonexistent"))

- test_actions_count_not_incremented_violation
  Given: An agent with actions_count=5
  When: heartbeat is called but actions_count is not incremented
  Then: Should return Err (contract violation)

- test_command_not_updated_violation
  Given: An agent exists
  When: heartbeat(HeartbeatRequest { agent_id, command: Some("new-cmd") }) is called
  Then: Agent.current_command should equal "new-cmd" (not unchanged)

## Given-When-Then Scenarios

### Scenario 1: Basic heartbeat
Given: An agent with ID "agent-1" is registered
When: I send a heartbeat
Then:
- last_seen is updated to current time
- actions_count is incremented by 1
- HeartbeatOutput is returned with agent_id, timestamp, message

### Scenario 2: Heartbeat with command
Given: An agent with ID "agent-1" is registered
When: I send a heartbeat with command "implementing feature-x"
Then:
- current_command is set to "implementing feature-x"
- last_seen is updated
- actions_count is incremented

### Scenario 3: Heartbeat for unknown agent
Given: No agent is registered in the environment
When: I attempt to send a heartbeat
Then:
- Operation fails with NO_AGENT_REGISTERED
- Error message indicates "No agent registered"

### Scenario 4: Heartbeat for non-existent agent
Given: ISOLATE_AGENT_ID is set to "ghost-agent"
And: No agent with ID "ghost-agent" exists in database
When: I attempt to send a heartbeat
Then:
- Operation fails with AGENT_NOT_FOUND
- Error message indicates "not found"

---

# Martin Fowler Test Plan: Session Submit

## Happy Path Tests

- test_submit_session_with_synced_status_pushes_bookmark
  Given: A session named "feature-submit" exists with status "synced"
  And: The session has a bookmark named "feature-submit"
  When: I submit the session "feature-submit"
  Then:
    - The bookmark should be pushed to remote
    - The response should include the dedupe key
    - dry_run should be false

- test_submit_session_with_auto_commit_commits_and_pushes
  Given: A session named "feature-autocommit" exists with status "active"
  And: The session has uncommitted changes
  And: The session has a bookmark named "feature-autocommit"
  When: I submit the session "feature-autocommit" with auto-commit
  Then:
    - The changes should be committed automatically
    - The bookmark should be pushed to remote
    - The response should include the dedupe key

## Error Path Tests

- test_submit_nonexistent_session_returns_session_not_found
  Given: No session named "nonexistent" exists
  When: I submit the session "nonexistent"
  Then: Returns `Err(Error::SessionNotFound)` with exit code 3

- test_submit_dirty_workspace_without_auto_commit_returns_dirty_workspace
  Given: A session named "feature-dirty" exists with status "active"
  And: The session has uncommitted changes
  And: The session has a bookmark named "feature-dirty"
  When: I attempt to submit the session "feature-dirty" without auto-commit
  Then: Returns `Err(Error::DirtyWorkspace)` with exit code 3

- test_submit_session_with_no_bookmark_returns_no_bookmark
  Given: A session named "feature-nobookmark" exists with status "synced"
  And: The session has no bookmark
  When: I attempt to submit the session "feature-nobookmark"
  Then: Returns `Err(Error::NoBookmark)` with exit code 3

## Edge Case Tests

- test_submit_dry_run_does_not_push_bookmark
  Given: A session named "feature-dryrun" exists with status "synced"
  And: The session has a bookmark named "feature-dryrun"
  When: I submit the session "feature-dryrun" with dry-run
  Then:
    - No bookmark should be pushed to remote
    - Response should indicate "dry_run: true"
    - dedupe key should still be returned

## Contract Violation Tests

- test_submit_session_not_found_violation
  Given: No session exists
  When: `submit_session("nonexistent", SubmitOptions { auto_commit: false, dry_run: false })`
  Then: Returns `Err(Error::SessionNotFound)`

- test_submit_no_bookmark_violation
  Given: A session with no bookmark exists
  When: `submit_session("session-no-bookmark", SubmitOptions { auto_commit: false, dry_run: false })`
  Then: Returns `Err(Error::NoBookmark)`

- test_submit_dirty_workspace_violation
  Given: A session with status "active" and uncommitted changes
  When: `submit_session("dirty-session", SubmitOptions { auto_commit: false, dry_run: false })`
  Then: Returns `Err(Error::DirtyWorkspace)`

## Given-When-Then Scenarios

### Scenario 1: Submit pushes bookmark to remote
Given: A session named "feature-submit" exists with status "synced"
And: The session has a bookmark named "feature-submit"
When: I submit the session "feature-submit"
Then:
- The bookmark should be pushed to remote
- The response should include the dedupe key

### Scenario 2: Submit with dirty workspace fails
Given: A session named "feature-dirty" exists with status "active"
And: The session has uncommitted changes
When: I attempt to submit the session "feature-dirty"
Then:
- The operation should fail with error "DIRTY_WORKSPACE"
- The exit code should be 3

### Scenario 3: Submit with auto-commit succeeds
Given: A session named "feature-autocommit" exists with status "active"
And: The session has uncommitted changes
When: I submit the session "feature-autocommit" with auto-commit
Then:
- The changes should be committed automatically
- The bookmark should be pushed to remote

### Scenario 4: Submit with no bookmark fails
Given: A session named "feature-nobookmark" exists with status "synced"
And: The session has no bookmark
When: I attempt to submit the session "feature-nobookmark"
Then:
- The operation should fail with error "NO_BOOKMARK"
- The exit code should be 3

### Scenario 5: Submit dry-run does not modify state
Given: A session named "feature-dryrun" exists with status "synced"
And: The session has a bookmark named "feature-dryrun"
When: I submit the session "feature-dryrun" with dry-run
Then:
- No bookmark should be pushed
- The response should indicate "dry_run: true"

---

# Martin Fowler Test Plan: Doctor Command

## Happy Path Tests

- test_doctor_check_only_runs_all_diagnostics
  Given: The system is in a healthy state
  When: I run the doctor command without --fix flag
  Then:
    - All diagnostic checks should run
    - The output should contain check results
    - The output should be valid JSON
    - The exit code should be 0

- test_doctor_all_checks_pass_healthy_system
  Given: The system is in a healthy state
  And: All dependencies are installed
  And: There are no orphaned workspaces
  And: There are no stale sessions
  When: I run the doctor command
  Then:
    - All checks should pass
    - The exit code should be 0
    - The summary should show "0 error(s)"

- test_doctor_verbose_with_fix_shows_details
  Given: There are 2 orphaned workspaces
  When: I run the doctor command with --fix --verbose flags
  Then:
    - Each fix action should be reported
    - The output should include action status
    - The output should be valid JSON

## Error Path Tests

- test_doctor_detects_missing_jj
  Given: JJ is not installed
  When: I run the doctor command
  Then:
    - The "JJ Installation" check should fail
    - The output should contain suggestion "Install JJ"
    - The exit code should be 1

- test_doctor_detects_uninitialized_isolate
  Given: isolate is not initialized
  When: I run the doctor command
  Then:
    - The "isolate Initialized" check should warn
    - The suggestion should include "isolate init"

- test_doctor_detects_orphaned_workspaces
  Given: There are 2 workspaces without session records
  When: I run the doctor command
  Then:
    - The "Orphaned Workspaces" check should warn
    - The output should show 2 orphaned workspaces

- test_doctor_detects_stale_sessions
  Given: There are 3 sessions in "creating" status for over 5 minutes
  When: I run the doctor command
  Then:
    - The "Stale Sessions" check should warn
    - The output should show 3 stale sessions

- test_doctor_detects_database_corruption
  Given: The state database is corrupted
  When: I run the doctor command
  Then:
    - The "State Database" check should fail
    - The suggestion should include "doctor --fix"
    - The exit code should be 1

- test_doctor_detects_pending_add_operations
  Given: There are 5 pending add operations in the journal
  When: I run the doctor command
  Then:
    - The "Pending Add Operations" check should fail
    - The output should show 5 pending operations
    - The exit code should be 1

- test_doctor_detects_workspace_integrity_issues
  Given: Session "feature-1" has workspace at "/workspaces/feature-1"
  And: The workspace directory does not exist
  When: I run the doctor command
  Then:
    - The "Workspace Integrity" check should fail
    - The output should show the missing workspace
    - The exit code should be 1

- test_doctor_detects_workflow_violation
  Given: The current directory is the main workspace
  And: There are 2 active sessions
  When: I run the doctor command
  Then:
    - The "Workflow Health" check should warn
    - The suggestion should include "isolate attach"

- test_doctor_detects_recent_recovery
  Given: Recovery occurred in the last 5 minutes
  When: I run the doctor command
  Then:
    - The "State Database" check should warn
    - The output should indicate recovery detected
    - The suggestion should mention "recovery.log"

- test_doctor_dry_run_requires_fix
  Given: The system has issues
  When: I run the doctor command with --dry-run flag (without --fix)
  Then:
    - The command should fail
    - The output should be a consistent JSON error envelope
    - The exit code should be 1

- test_doctor_verbose_requires_fix
  Given: The system has issues
  When: I run the doctor command with --verbose flag (without --fix)
  Then:
    - The command should fail
    - The output should be a consistent JSON error envelope
    - The exit code should be 1

## Edge Case Tests

- test_doctor_fix_idempotent
  Given: There are 2 orphaned workspaces
  When: I run the doctor command with --fix flag
  And: I run the doctor command with --fix flag again
  Then:
    - The second run should report no issues to fix
    - Both runs should complete successfully
    - Exit code should be 0 on both runs

- test_doctor_dry_run_shows_what_would_be_fixed
  Given: There are 2 orphaned workspaces
  When: I run the doctor command with --fix --dry-run flags
  Then:
    - No changes should be made to the system
    - The output should show what would be fixed
    - The output should contain "Dry-run mode"

- test_doctor_beads_integration_optional
  Given: beads CLI is not installed
  When: I run the doctor command
  Then:
    - The "Beads Integration" check should pass
    - The message should include "optional"

- test_doctor_exit_code_warnings_only
  Given: The system has 0 errors and 2 warnings
  When: I run the doctor command
  Then:
    - The exit code should be 0

- test_doctor_exit_code_errors
  Given: The system has 1 error and 0 warnings
  When: I run the doctor command
  Then:
    - The exit code should be 1

- test_doctor_summary_statistics_accurate
  Given: The system has 5 passed checks
  And: The system has 2 warnings
  And: The system has 1 error
  When: I run the doctor command
  Then:
    - The summary should show "5 passed"
    - The summary should show "2 warning(s)"
    - The summary should show "1 error(s)"

## Contract Verification Tests

- test_precondition_dry_run_requires_fix
  Given: No checks have been run
  When: doctor is called with dry_run=true and fix=false
  Then: Returns Error::DryRunRequiresFix

- test_precondition_verbose_requires_fix
  Given: No checks have been run
  When: doctor is called with verbose=true and fix=false
  Then: Returns Error::VerboseRequiresFix

- test_postcondition_json_valid
  Given: Any system state
  When: doctor runs successfully
  Then: Output is valid JSON (can be parsed)

- test_postcondition_json_has_schema
  Given: Any system state
  When: doctor runs successfully
  Then: Output contains "$schema" field

- test_postcondition_json_has_schema_version
  Given: Any system state
  When: doctor runs successfully
  Then: Output contains "_schema_version" field

- test_postcondition_exit_code_zero_when_no_errors
  Given: All checks pass or only have warnings
  When: doctor runs
  Then: Exit code is 0

- test_postcondition_exit_code_one_when_errors
  Given: One or more checks have Fail status
  When: doctor runs
  Then: Exit code is 1

- test_invariant_check_only_is_readonly
  Given: System with various issues
  When: doctor runs without --fix flag
  Then:
    - No files should be modified
    - No database records should be deleted
    - Output should only report issues

- test_invariant_fix_is_idempotent
  Given: System with fixable issues
  When: doctor --fix runs twice
  Then: Both runs produce identical results (same files deleted, same DB changes)

- test_invariant_json_always_valid
  Given: Any system state (healthy, corrupted, etc.)
  When: doctor runs
  Then: Output is always valid JSON (never partial/invalid)

## Contract Violation Tests

- `test_p3_violation_dry_run_without_fix`
  Given: System with issues
  When: `run(false, false, true, false)` is called (format, fix, dry_run, verbose)
  Then: Returns `Err(Error::DryRunRequiresFix)` -- NOT a panic

- `test_p3_violation_verbose_without_fix`
  Given: System with issues
  When: `run(false, false, false, true)` is called
  Then: Returns `Err(Error::VerboseRequiresFix)` -- NOT a panic

- `test_q1_violation_invalid_json_output`
  Given: Any system state
  When: doctor runs but produces invalid JSON
  Then: Should produce valid JSON (enforced by contract)

- `test_q5_violation_exit_code_not_zero_with_no_errors`
  Given: All checks pass (no errors)
  When: doctor runs
  Then: Exit code should be 0 (not 1)

- `test_q6_violation_exit_code_not_one_with_errors`
  Given: System has 1+ checks with Fail status
  When: doctor runs
  Then: Exit code should be 1 (not 0)

- `test_q7_violation_check_mode_modifies_system`
  Given: System with issues
  When: doctor runs without --fix
  Then: No changes made to filesystem or database

- `test_q8_violation_fix_not_idempotent`
  Given: System with fixable issues
  When: doctor --fix runs twice
  Then: Second run should report no changes needed (idempotent)

## Given-When-Then Scenarios

### Scenario 1: Basic health check runs all diagnostics
Given: The system is in a healthy state
When: I run the doctor command without fix flag
Then:
- All diagnostic checks should run
- The output should contain check results
- The output should be valid JSON
- The exit code should be 0

### Scenario 2: Doctor detects missing dependencies
Given: JJ is not installed
When: I run the doctor command
Then:
- The "JJ Installation" check should fail
- The output should contain suggestion "Install JJ"
- The exit code should be 1

### Scenario 3: Doctor detects orphaned workspaces
Given: There are 2 workspaces without session records
When: I run the doctor command
Then:
- The "Orphaned Workspaces" check should warn
- The output should show 2 orphaned workspaces
- The issue should be auto-fixable

### Scenario 4: Fix mode with auto-fixable issues
Given: There are 2 orphaned workspaces
And: There are 3 stale sessions
When: I run the doctor command with --fix flag
Then:
- The orphaned workspaces should be removed
- The stale sessions should be removed
- The output should show fix results
- The output should be valid JSON
- Exit code should be 0

### Scenario 5: Fix idempotency
Given: There are 2 orphaned workspaces
When: I run the doctor command with --fix flag
And: I run the doctor command with --fix flag again
Then:
- The second run should report no issues to fix
- Both runs should complete successfully

### Scenario 6: Dry-run mode shows what would be fixed
Given: There are 2 orphaned workspaces
When: I run the doctor command with --fix --dry-run flags
Then:
- No changes should be made to the system
- The output should show what would be fixed
- The output should contain "Dry-run mode"

### Scenario 7: Safety - check mode is read-only
Given: The system has various issues
When: I run the doctor command without --fix flag
Then:
- No changes should be made to the system
- No files should be modified
- No database records should be deleted
- The output should only report issues

### Scenario 8: Database recovery with fix
Given: The state database is corrupted
When: I run the doctor command with --fix flag
Then:
- The corrupted database should be handled
- The fix result should be reported
- The output should be valid JSON

### Scenario 9: Workspace integrity fix with rebind
Given: Session "feature-1" has workspace at "/old/path/feature-1"
And: The workspace exists at "/new/path/feature-1"
When: I run the doctor command with --fix flag
Then:
- The session workspace path should be updated
- The fix should be reported

### Scenario 10: Non-auto-fixable issues remain after fix
Given: JJ is not installed
When: I run the doctor command with --fix flag
Then:
- The fix should fail with reason "Requires manual intervention"
- The exit code should be 1
- The output should be valid JSON

### Scenario 11: JSON validity invariant
Given: The system is in any state
When: I run the doctor command
Then:
- The output must be valid JSON
- The output must have a "$schema" field
- The output must have a "_schema_version" field
- The output must have a "success" field

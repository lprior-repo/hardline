# Martin Fowler Test Plan

## Overview
This test plan covers the SVT (Super Velocity Throughput) testing pipeline for the hardline repository. Tests are organized following Martin Fowler's Given-When-Then pattern with expressive names that serve as executable specifications.

## Happy Path Tests

### test_svt_runner_script_exists_and_is_executable
Given: The svt-runner.sh script exists at the expected path
When: The script is checked for execute permissions
Then: Script returns exit code 0 and has executable bit set

### test_all_required_dependencies_are_available
Given: All required dependencies (jq, curl, ss, opencode, bd) are installed
When: Dependency validation is performed
Then: Returns Ok with list of found dependencies

### test_target_directory_is_accessible
Given: Target directory `/home/lewis/src/hardline` exists and is readable
When: Directory existence is validated
Then: Path validation passes

### test_opencode_server_starts_successfully
Given: Base port 4500 is available and OPENCODE_SERVER_PASSWORD is set
When: opencode serve is started on port 4500
Then: Server process is running with valid PID

### test_session_created_for_ready_bead
Given: opencode server is running on port 4500 and a ready bead exists
When: Session creation API is called with bead title
Then: Returns valid session ID (non-empty string)

### test_agent_dispatch_succeeds
Given: Valid session exists on opencode server
When: Agent is dispatched to session with bead ID
Then: Dispatch returns success status

### test_poll_completes_within_timeout
Given: Agent is dispatched and processing bead
When: Polling for completion is performed (timeout 300s)
Then: Returns SessionStatus::Completed or SessionStatus::Failed

### test_report_generated_with_execution_matrix
Given: All sessions have completed processing
When: Report generation is triggered
Then: JSON report is produced with port, session_id, status, duration fields

### test_servers_cleaned_up_on_exit
Given: opencode servers are running
When: Script exits (success or failure)
Then: All server processes are terminated via trap handler

### test_svt_test_completes_with_zero_exit_code
Given: All preconditions are met and beads are processed
When: svt-runner.sh completes full execution
Then: Script exits with code 0

## Error Path Tests

### test_returns_error_when_svt_runner_script_missing
Given: svt-runner.sh does not exist at expected path
When: Script execution is attempted
Then: Returns `Err(Error::DependencyMissing("svt-runner.sh not found"))`

### test_returns_error_when_jq_dependency_missing
Given: jq is not installed on the system
When: Dependency validation runs
Then: Returns `Err(Error::DependencyMissing("jq not installed"))`

### test_returns_error_when_curl_dependency_missing
Given: curl is not installed on the system
When: Dependency validation runs
Then: Returns `Err(Error::DependencyMissing("curl not installed"))`

### test_returns_error_when_ss_dependency_missing
Given: ss (socket statistics) is not installed
When: Dependency validation runs
Then: Returns `Err(Error::DependencyMissing("ss not installed"))`

### test_returns_error_when_opencode_missing
Given: opencode CLI is not installed
When: Dependency validation runs
Then: Returns `Err(Error::DependencyMissing("opencode not installed"))`

### test_returns_error_when_bd_missing
Given: bd CLI is not installed
When: Dependency validation runs
Then: Returns `Err(Error::DependencyMissing("bd not installed"))`

### test_returns_error_when_no_ready_beads
Given: No beads are in ready state in the system
When: `bd ready` returns empty list
Then: Returns `Err(Error::NoReadyBeads)`

### test_returns_error_when_port_conflict_all_ports_busy
Given: All ports 4500-4529 are occupied
When: Port availability check is performed
Then: Returns `Err(Error::PortConflict("no available ports"))`

### test_returns_error_when_server_start_fails
Given: Port 4500 is in use and cannot be bound
When: opencode serve startup is attempted
Then: Returns `Err(Error::ServerStartFailed)`

### test_returns_error_when_session_creation_fails
Given: opencode server is running but returns null session ID
When: Session creation API is called
Then: Returns `Err(Error::SessionCreationFailed("null session id"))`

### test_returns_error_when_dispatch_fails
Given: Session exists but dispatch API returns error
When: Agent dispatch is attempted
Then: Returns `Err(Error::DispatchFailed)`

### test_returns_error_when_poll_times_out
Given: Agent is dispatched but never completes
When: Polling exceeds timeout (300s)
Then: Returns `Err(Error::PollTimeout)`

### test_returns_error_when_report_generation_fails
Given: No session data available for report
When: Report generation is triggered
Then: Returns `Err(Error::ReportGenerationFailed("missing required fields"))`

### test_returns_error_without_opencode_server_password
Given: OPENCODE_SERVER_PASSWORD environment variable is not set
When: opencode serve is started
Then: Server fails authentication, returns `Err(Error::ServerStartFailed)`

## Edge Case Tests

### test_handles_single_bead_batch
Given: Only one ready bead exists in the system
When: SVT runs with batch_size=1
Then: Processes exactly one bead, generates report with single entry

### test_handles_empty_bead_list_gracefully
Given: No beads are available for processing
When: SVT runs
Then: Generates empty report, exits cleanly with code 0

### test_handles_port_increment_for_multiple_servers
Given: Port 4500 is occupied but 4501 is available
When: First server fails to start on port 4500
Then: Successfully starts on port 4501

### test_handles_server_crash_during_execution
Given: opencode server crashes mid-execution
When: Server process exits unexpectedly
Then: Cleanup trap fires, remaining servers stopped, error reported

### test_handles_api_timeout_gracefully
Given: opencode server API responds slowly
When: API call exceeds reasonable timeout
Then: Returns timeout error, does not hang indefinitely

### test_handles_invalid_bead_id
Given: Dispatch is attempted with non-existent bead ID
When: Agent processes invalid bead
Then: Returns appropriate error, continues to next bead

### test_handles_concurrent_session_creation
Given: Multiple beads ready for processing
When: Sessions are created in parallel
Then: Each session gets unique ID, no collisions

### test_handles_report_with_mixed_statuses
Given: Some beads succeed, some fail during processing
When: Report is generated
Then: Report accurately reflects each bead's status

## Contract Verification Tests

### test_precondition_p1_svt_runner_exists
Given: File system state
When: Checking svt-runner.sh existence
Then: Precondition P1 enforced via file existence check

### test_precondition_p2_dependencies_available
Given: System with installed/removed tools
When: Validating dependencies
Then: Precondition P2 enforced via `command -v` checks

### test_precondition_p3_target_directory_exists
Given: Path to target directory
When: Validating directory accessibility
Then: Precondition P3 enforced via path existence check

### test_precondition_p4_password_environment_set
Given: Environment variables
When: Checking OPENCODE_SERVER_PASSWORD
Then: Precondition P4 enforced via env var presence

### test_precondition_p5_port_available
Given: Port usage state
When: Checking port availability
Then: Precondition P5 enforced via `ss -tuln` check

### test_postcondition_q1_server_started
Given: Server startup attempt
When: Checking SERVER_PIDS
Then: Postcondition Q1 verified by non-empty SERVER_PIDS

### test_postcondition_q2_sessions_created
Given: Session creation calls
When: Checking BEAD_SESSIONS
Then: Postcondition Q2 verified by populated session map

### test_postcondition_q3_report_generated
Given: Report generation call
When: Checking output
Then: Postcondition Q3 verified by JSON report presence

### test_postcondition_q4_servers_cleaned_up
Given: Script completion
When: Checking remaining processes
Then: Postcondition Q4 verified by empty process list

### test_postcondition_q5_zero_exit_code
Given: Script completion
When: Checking exit code
Then: Postcondition Q5 verified by exit code 0

### test_invariant_i1_base_port_starts_at_4500
Given: SVT execution
When: Monitoring port assignments
Then: Invariant I1 maintained: BASE_PORT=4500 initially

### test_invariant_i2_unique_port_per_bead
Given: Multiple beads
When: Assigning ports
Then: Invariant I2 maintained: no port collisions

### test_invariant_i3_provider_defaults_correctly
Given: SVT_PROVIDER not set
When: Reading configuration
Then: Invariant I3 maintained: defaults to "minimax-coding-plan"

### test_invariant_i4_model_defaults_correctly
Given: SVT_MODEL not set
When: Reading configuration
Then: Invariant I4 maintained: defaults to "MiniMax-M2.5-highspeed"

### test_invariant_i5_cleanup_on_any_exit
Given: Script running
When: Script exits with error
Then: Invariant I5 maintained: cleanup via trap EXIT

## Contract Violation Tests

### test_violation_p1_returns_dependency_missing_error
Given: svt-runner.sh script does not exist at /home/lewis/.config/opencode/skill/svt/svt-runner.sh
When: Script execution is attempted
Then: Returns `Err(Error::DependencyMissing("svt-runner.sh not found"))` - NOT a panic

### test_violation_p2_returns_dependency_missing_error
Given: jq is not installed on the system
When: Dependency validation runs via `command -v jq`
Then: Returns `Err(Error::DependencyMissing("jq not installed"))` - NOT a panic

### test_violation_p3_returns_invalid_path_error
Given: Target directory "/nonexistent/path" does not exist
When: Directory existence is validated
Then: Returns `Err(Error::InvalidPath("directory does not exist"))` - NOT a panic

### test_violation_p4_returns_server_start_failed_error
Given: OPENCODE_SERVER_PASSWORD environment variable is not set
When: opencode serve attempts to start
Then: Returns `Err(Error::ServerStartFailed)` due to authentication failure - NOT a panic

### test_violation_p5_returns_port_conflict_error
Given: All ports 4500-4529 are occupied
When: find_available_port is called
Then: Returns `Err(Error::PortConflict("no available ports"))` - NOT a panic

### test_violation_q1_returns_server_start_failed_error
Given: All port startup attempts fail
When: start_opencode_server is called for all beads
Then: Returns `Err(Error::ServerStartFailed)` - NOT a panic

### test_violation_q2_returns_session_creation_failed_error
Given: Session creation API returns null session ID
When: create_session is called
Then: Returns `Err(Error::SessionCreationFailed("null session id"))` - NOT a panic

### test_violation_q3_returns_report_generation_failed_error
Given: No session data available (empty bead results)
When: generate_report is called
Then: Returns `Err(Error::ReportGenerationFailed("missing required fields"))` - NOT a panic

## Given-When-Then Scenarios

### Scenario 1: Successful SVT Test Execution with Single Ready Bead
Given:
- svt-runner.sh exists at expected path
- All required dependencies (jq, curl, ss, opencode, bd) are installed
- Target directory exists and is readable
- OPENCODE_SERVER_PASSWORD environment variable is set
- Port 4500 is available
- At least one bead is in ready state

When:
- svt-runner.sh is executed with batch_size=1
- Server starts on port 4500
- Session is created for ready bead
- Agent is dispatched to process bead
- Poll completes successfully

Then:
- SERVER_PIDS contains one PID
- BEAD_SESSIONS contains one session entry
- JSON report is generated with execution matrix
- All servers are cleaned up
- Exit code is 0

### Scenario 2: SVT Fails Due to Missing Dependency
Given:
- svt-runner.sh script is missing from expected path

When:
- SVT test execution is attempted

Then:
- Error::DependencyMissing is returned
- No servers are started
- No sessions are created
- Exit code is non-zero

### Scenario 3: SVT Fails Due to No Ready Beads
Given:
- All dependencies are available
- Server can start successfully
- No beads are in ready state (bd ready returns empty)

When:
- SVT test execution attempts to discover beads

Then:
- Error::NoReadyBeads is returned
- Servers are cleaned up
- Exit code is non-zero

### Scenario 4: SVT Recovers from Port Conflict
Given:
- Port 4500 is occupied but port 4501 is available

When:
- Server startup fails on port 4500
- SVT automatically increments to port 4501
- Server successfully starts on port 4501

Then:
- Bead is processed on port 4501
- Report reflects correct port assignment
- Exit code is 0

### Scenario 5: SVT Handles Server Crash and Cleans Up
Given:
- opencode server starts successfully on port 4500
- Session is created and agent is dispatched

When:
- Server process crashes unexpectedly during processing

Then:
- Trap handler fires
- Any remaining server processes are terminated
- Partial results are not generated (cleanup takes precedence)
- Exit code reflects failure

## Test Execution Order

1. **Dependency Validation Tests** (prerequisite checks)
2. **Happy Path Tests** (full successful execution)
3. **Error Path Tests** (each failure mode in isolation)
4. **Edge Case Tests** (boundary conditions)
5. **Contract Verification Tests** (pre/post/invariant enforcement)
6. **Contract Violation Tests** (error handling for violations)
7. **Given-When-Then Scenarios** (end-to-end workflows)

## Success Criteria

- All happy path tests pass
- All error path tests return correct error variants (not panics)
- All edge cases are handled gracefully
- All preconditions are enforced at appropriate level
- All postconditions are verified after execution
- All invariants hold throughout execution
- All violation examples have corresponding tests
- Cleanup is verified regardless of test outcome

## Non-Goals (from contract-spec.md)

- Testing individual bead execution logic (go-skill handles this)
- Testing opencode serve internal functionality
- Testing bd CLI beyond basic ready bead discovery
- Performance benchmarking of SVT pipeline
- Testing SVT with multiple concurrent batches

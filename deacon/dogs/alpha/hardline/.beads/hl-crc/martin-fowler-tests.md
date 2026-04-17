---
bead_id: hl-crc
bead_title: svt_batch_1
bead_description: SVT Load Test 1
bead_type: task
phase: contract-synthesis
updated_at: 2026-03-12T00:00:00Z
---

# Martin Fowler Test Plan

## Overview
This test plan validates SVT (Super Velocity Throughput) Load Test 1 - single bead execution baseline test for the opencode serve infrastructure. This test validates the core SVT orchestration with minimal load (batch_size=1) to establish a baseline for subsequent larger batches.

---

## Happy Path Tests

### test_svt_batch1_completes_successfully_with_single_bead
**Given**: All dependencies are installed, svt-runner.sh exists, and at least 1 bead is ready
**When**: Running `svt-runner.sh 1 /home/lewis/src/hardline` for batch 1
**Then**:
- Returns `Ok(SvtBatchReport)` with 1 bead processed
- Bead shows completion status (success or documented failure)
- Report is generated with batch summary

### test_svt_batch1_creates_single_opencode_serve_instance
**Given**: opencode CLI is available and port 4500 is free
**When**: SVT batch 1 execution starts
**Then**:
- Exactly one opencode serve instance spawns on port 4500 (or next available)
- Instance responds to health checks

### test_svt_batch1_creates_session_for_bead
**Given**: opencode serve is running
**When**: Session creation for the single bead
**Then**:
- Session is created with title "svt-bead-{bead_id}"
- Session ID is returned and stored

### test_svt_batch1_dispatches_go_skill_to_session
**Given**: Session is created successfully
**When**: Dispatching go-skill to session
**Then**:
- Prompt is sent with model configuration (provider: minimax-coding-plan, model: MiniMax-M2.5-highspeed)
- Agent type is "build"
- Async prompt returns successfully

### test_svt_batch1_polls_for_completion
**Given**: Bead is dispatched and running
**When**: SVT polls for completion status (every 10 seconds)
**Then**:
- Polling returns status for the bead
- Completion is detected via session status and completed_time field
- Timeout is handled gracefully

### test_svt_batch1_generates_execution_matrix_report
**Given**: Single bead completes (success or failure)
**When**: Report generation phase
**Then**:
- Execution matrix is generated with per-bead results
- Summary includes batch size (1), model, completion status
- Trace file is saved to /tmp/svt_trace_{bead_id}.json

### test_svt_batch1_cleans_up_server_instance
**Given**: Test completes (success, failure, or timeout)
**When**: Cleanup phase via EXIT trap
**Then**:
- opencode serve process is terminated
- No orphan processes on test port

---

## Error Path Tests

### test_returns_error_when_svt_runner_script_missing
**Given**: svt-runner.sh does not exist at expected path
**When**: Attempting to run SVT batch
**Then**: Returns `Err(Error::DependencyMissing("svt-runner.sh not found"))`

### test_returns_error_when_jq_missing
**Given**: jq is not installed or not in PATH
**When**: SVT runner attempts to parse JSON output
**Then**: Returns `Err(Error::DependencyMissing("jq not found"))`

### test_returns_error_when_curl_missing
**Given**: curl is not installed
**When**: Attempting to communicate with opencode serve
**Then**: Returns `Err(Error::DependencyMissing("curl not found"))`

### test_returns_error_when_ss_missing
**Given**: ss (socket statistics) is not installed
**When**: Checking port availability
**Then**: Returns `Err(Error::DependencyMissing("ss not found"))`

### test_returns_error_when_opencode_missing
**Given**: opencode CLI is not installed
**When**: Attempting to start opencode serve
**Then**: Returns `Err(Error::DependencyMissing("opencode not found"))`

### test_returns_error_when_bd_missing
**Given**: bd CLI is not installed
**When**: Checking for ready beads
**Then**: Returns `Err(Error::DependencyMissing("bd not found"))`

### test_returns_error_when_target_directory_invalid
**Given**: Target directory does not exist
**When**: Running SVT with invalid target_dir
**Then**: Returns `Err(Error::InvalidPath("target directory does not exist"))`

### test_returns_error_when_no_ready_beads
**Given**: No beads are in ready state
**When**: Running `bd ready --json` returns empty
**Then**: Returns `Err(Error::NoReadyBeads("no ready beads found"))`

### test_returns_error_when_port_blocked
**Given**: Port 4500 (and subsequent ports) are occupied
**When**: Attempting to start opencode serve instance
**Then**: Returns `Err(Error::PortUnavailable("port 4500 in use"))`

### test_returns_error_when_server_fails_to_start
**Given**: Port is available but opencode serve fails to start
**When**: Starting opencode serve instance
**Then**: Returns `Err(Error::ServerStartFailed("failed to start on port 4500: ..."))`

### test_returns_error_when_session_creation_fails
**Given**: opencode serve is running but session creation fails
**When**: Creating session for the bead
**Then**: Returns `Err(Error::SessionCreationFailed("bead-id: failed to create session"))`

### test_returns_error_when_dispatch_fails
**Given**: Session is created but go-skill dispatch fails
**When**: Dispatching go-skill for bead execution
**Then**: Returns `Err(Error::DispatchFailed("bead-id: dispatch failed"))`

### test_returns_error_on_poll_timeout
**Given**: Bead is dispatched but does not complete within polling threshold
**When**: Polling loop exits without detecting completion
**Then**: Returns `Err(Error::PollTimeout("bead-id: polling timed out"))`

### test_returns_error_when_completion_check_fails
**Given**: Session status check fails
**When**: Polling for bead status
**Then**: Returns `Err(Error::CompletionCheckFailed("bead-id: status check failed"))`

### test_returns_error_when_report_generation_fails
**Given**: Bead completes but report file cannot be written
**When**: Generating execution matrix report
**Then**: Returns `Err(Error::ReportGenerationFailed("failed to write report"))`

---

## Edge Case Tests

### test_handles_batch_size_zero_uses_default
**Given**: batch_size=0 is passed
**When**: Running svt-runner.sh with 0
**Then**: Uses default batch size (30) or exits gracefully

### test_handles_empty_target_directory
**Given**: Target directory exists but is empty (no .beads directory)
**When**: Running SVT batch
**Then**: Returns `Err(Error::NoReadyBeads)` as no beads exist

### test_handles_server_crash_during_execution
**Given**: opencode serve crashes mid-execution
**When**: Serve instance terminates unexpectedly
**Then**:
- Cleanup is attempted for crashed instance
- Report reflects failure with error status

### test_handles_session_id_null_response
**Given**: Session creation returns null/empty session ID
**When**: Parsing curl response
**Then**:
- Marks bead as "failed_to_start"
- Continues to report generation

### test_handles_missing_environment_variables
**Given**: SVT_PROVIDER or SVT_MODEL env vars not set
**When**: Running SVT batch
**Then**: Uses sensible defaults (minimax-coding-plan, MiniMax-M2.5-highspeed)

### test_handles_session_busy_incorrectly_reported
**Given**: Session status check returns unexpected format
**When**: Parsing JSON response
**Then**: Gracefully handles parse error, marks as failed

### test_handles_idle_session_without_completion_time
**Given**: Session is idle but missing completion_time
**When**: Polling detects session not busy
**Then**: Marks bead as "failed" (not "completed")

### test_handles_trace_file_write_failure
**Given**: Cannot write to /tmp/svt_trace_{bead_id}.json
**When**: Saving session trace
**Then**: Continues execution, reports error in output

---

## Contract Verification Tests

### test_precondition_p1_svt_runner_exists
**Given**: svt-runner.sh location
**When**: Checking existence at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
**Then**: Returns `Ok(true)` if exists, `Err(Error::DependencyMissing)` if not

### test_precondition_p2_svt_runner_executable
**Given**: svt-runner.sh exists
**When**: Checking execute permissions
**Then**: Returns `Ok(true)` if executable, `Err(Error::DependencyMissing)` otherwise

### test_precondition_p3_p7_cli_tools_available
**Given**: System with CLI tools
**When**: Checking jq, curl, ss, opencode, bd availability
**Then**: Returns `Ok(Vec<DependencyStatus>)` with all available tools

### test_precondition_p8_target_directory_valid
**Given**: Target directory path
**When**: Validating directory exists and is readable
**Then**: Returns `Ok(PathBuf)` if valid, `Err(Error::InvalidPath)` if not

### test_precondition_p9_ready_beads_exist
**Given**: Bead repository
**When**: Running `bd ready --json`
**Then**: Returns `Ok(Vec<Bead>)` if beads ready, `Err(Error::NoReadyBeads)` if empty

### test_precondition_p10_port_available
**Given**: Port 4500 and subsequent ports
**When**: Checking port availability with ss
**Then**: Returns `Ok(u16)` with available port, `Err(Error::PortUnavailable)` if none

### test_postcondition_q1_report_generated
**Given**: SVT batch execution completes
**When**: Checking for report output
**Then**: Report file exists with valid JSON containing bead results

### test_postcondition_q2_bead_completes
**Given**: SVT batch execution completes
**When**: Inspecting bead completion status
**Then**: Bead has terminal status (success/failure/timeout/failed_to_start)

### test_postcondition_q3_serve_instance_cleaned_up
**Given**: SVT batch execution completes (success or failure)
**When**: Checking running processes
**Then**: No orphan opencode serve process on test port

### test_postcondition_q4_summary_produced
**Given**: SVT batch execution completes
**When**: Inspecting report summary
**Then**: Summary contains batch_size, model, completion status

### test_postcondition_q5_trace_captured
**Given**: SVT batch execution completes
**When**: Checking trace file
**Then**: /tmp/svt_trace_{bead_id}.json exists and contains messages

### test_invariant_i1_no_orphan_processes
**Given**: Any SVT execution scenario
**When**: After completion (success, failure, or timeout)
**Then**: No stray opencode serve processes remain

### test_invariant_i2_bead_state_consistent
**Given**: SVT execution
**When**: Checking bd state after completion
**Then**: Bead states are consistent, no corruption

### test_invariant_i3_resource_cleanup
**Given**: Any SVT execution outcome
**When**: Cleanup phase
**Then**: Temp files and processes cleaned regardless of outcome

### test_invariant_i4_single_server_instance
**Given**: batch_size=1
**When**: Starting servers
**Then**: Exactly one opencode serve instance is started

### test_invariant_i5_cleanup_trap_active
**Given**: Script execution begins
**When**: Script receives interrupt signal (SIGINT/SIGTERM)
**Then**: EXIT trap fires and cleanup executes

---

## Contract Violation Tests

### test_p1_violation_returns_dependency_missing
**Given**: svt-runner.sh does not exist
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("svt-runner.sh not found"))` - NOT a panic

### test_p3_violation_returns_dependency_missing
**Given**: jq is not installed
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("jq not found"))` - NOT a panic

### test_p4_violation_returns_dependency_missing
**Given**: curl is not installed
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("curl not found"))` - NOT a panic

### test_p5_violation_returns_dependency_missing
**Given**: ss is not installed
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("ss not found"))` - NOT a panic

### test_p6_violation_returns_dependency_missing
**Given**: opencode CLI is missing
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("opencode not found"))` - NOT a panic

### test_p7_violation_returns_dependency_missing
**Given**: bd CLI is missing
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("bd not found"))` - NOT a panic

### test_p8_violation_returns_invalid_path
**Given**: Target directory does not exist
**When**: Running SVT with invalid path
**Then**: Returns `Err(Error::InvalidPath("target directory does not exist"))` - NOT a panic

### test_p9_violation_returns_no_ready_beads
**Given**: No beads in ready state
**When**: Running SVT batch
**Then**: Returns `Err(Error::NoReadyBeads("no ready beads found"))` - NOT a panic

### test_p10_violation_returns_port_unavailable
**Given**: Port 4500 is already in use
**When**: Starting opencode serve
**Then**: Returns `Err(Error::PortUnavailable("port 4500 in use"))` - NOT a panic

### test_q1_violation_returns_report_generation_failed
**Given**: SVT runs but output is missing
**When**: Report not generated
**Then**: Returns `Err(Error::ReportGenerationFailed("report not found"))` - NOT a panic

### test_q2_violation_returns_poll_timeout
**Given**: Bead incomplete after polling loop
**When**: No completion detected
**Then**: Returns `Err(Error::PollTimeout(...))` - NOT a panic

### test_q3_violation_returns_cleanup_failed
**Given**: Process cleanup fails
**When**: Orphan process remains
**Then**: Returns `Err(Error::CleanupFailed("process still running on port 4500"))` - NOT a panic

### test_q4_violation_returns_report_generation_failed
**Given**: Report generated but summary missing
**When**: Inspecting report
**Then**: Returns `Err(Error::ReportGenerationFailed("summary missing"))` - NOT a panic

### test_q5_violation_returns_report_generation_failed
**Given**: Trace file not created
**When**: Saving session trace
**Then**: Returns `Err(Error::ReportGenerationFailed("trace file not found"))` - NOT a panic

### test_i1_violation_returns_cleanup_failed
**Given**: Test completes but process still running
**When**: Checking processes after test
**Then**: Returns `Err(Error::CleanupFailed("orphan process"))` - NOT a panic

### test_i4_violation_returns_server_start_failed
**Given**: batch_size=1
**When**: Multiple servers started
**Then**: Returns `Err(Error::ServerStartFailed("unexpected instance count"))` - NOT a panic

---

## Given-When-Then Scenarios

### Scenario 1: Successful SVT Batch 1 Execution
**Given**:
- svt-runner.sh exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
- All dependencies (jq, curl, ss, opencode, bd) are installed
- At least 1 bead is in ready state
- Port 4500 is available
- Target directory `/home/lewis/src/hardline` exists

**When**:
```bash
svt-runner.sh 1 /home/lewis/src/hardline
```

**Then**:
- 1 opencode serve instance starts on port 4500
- go-skill dispatch occurs for the single bead
- Polling completes for the bead
- Execution matrix report is generated
- Serve instance is cleaned up
- Returns `Ok(SvtBatchReport)` with 1 completed bead

### Scenario 2: Dependency Missing - jq
**Given**: jq is not installed on the system
**When**: Running SVT batch 1
**Then**:
- Execution fails early in check_deps phase
- Returns `Err(Error::DependencyMissing("jq not found"))`
- No serve instances are started
- No cleanup needed

### Scenario 3: Session Creation Failure
**Given**: opencode serve is running but session API returns error
**When**: Creating session for bead
**Then**:
- Bead marked as "failed_to_start" in BEAD_STATUSES
- Report generation proceeds with error status
- Server is still cleaned up

### Scenario 4: Bead Dispatch Success But Poll Timeout
**Given**: Session created and prompt dispatched successfully
**When**: Polling exceeds threshold without completion detection
**Then**:
- Bead marked as "failed" in report
- Server is cleaned up via EXIT trap
- Report includes timeout details

### Scenario 5: Script Interrupted During Execution
**Given**: SVT is running and receives SIGINT
**When**: User presses Ctrl-C or process killed
**Then**:
- EXIT trap fires
- opencode serve process is terminated
- Partial results may be available

### Scenario 6: Single Bead Baseline Validation
**Given**: All preconditions met, single ready bead
**When**: Running svt_batch_1 to establish baseline
**Then**:
- Validates core SVT infrastructure works
- Single session created with correct configuration
- Completion detection works for single bead
- Report format validated for single entry
- Serves as baseline for svt_batch_5 and larger tests

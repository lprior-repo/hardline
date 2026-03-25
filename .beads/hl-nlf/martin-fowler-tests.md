---
bead_id: hl-nlf
bead_title: svt_batch_5
phase: contract-synthesis
updated_at: 2026-03-12T00:00:00Z
---

# Martin Fowler Test Plan

## Overview
This test plan validates SVT (Super Velocity Throughput) Load Test 5 - batch execution of multiple beads through the opencode serve infrastructure.

---

## Happy Path Tests

### test_svt_batch5_completes_successfully_with_5_beads
**Given**: All dependencies are installed, svt-runner.sh exists, and at least 5 beads are ready
**When**: Running `svt-runner.sh 5 /home/lewis/src/hardline` for batch 5
**Then**:
- Returns `Ok(SvtBatchReport)` with 5 beads processed
- All beads show completion status (success or documented failure)
- Report is generated with batch summary

### test_svt_batch5_creates_opencode_serve_instances
**Given**: opencode CLI is available and ports are free
**When**: SVT batch 5 execution starts
**Then**:
- Multiple opencode serve instances are spawned on available ports
- Each instance responds to health checks

### test_svt_batch5_dispatches_go_skill_for_each_bead
**Given**: go-skill is available and beads are ready
**When**: Processing each bead in batch 5
**Then**:
- go-skill is dispatched for each bead
- Each dispatch returns a session ID

### test_svt_batch5_polls_for_completion
**Given**: Beads are dispatched and running
**When**: SVT polls for completion status
**Then**:
- Polling returns status for each bead
- Timeout is handled gracefully per bead

### test_svt_batch5_generates_execution_matrix_report
**Given**: All beads in batch complete (success or failure)
**When**: Report generation phase
**Then**:
- Execution matrix is generated with per-bead results
- Summary includes batch size, model, completion rates

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

### test_returns_error_when_all_ports_blocked
**Given**: All ports in range 3000-3010 are occupied
**When**: Attempting to start opencode serve instances
**Then**: Returns `Err(Error::PortUnavailable("no ports available"))`

### test_returns_error_when_server_fails_to_start
**Given**: Port is available but opencode serve fails to start
**When**: Starting opencode serve instance
**Then**: Returns `Err(Error::ServerStartFailed("failed to start on port 3000: ..."))`

### test_returns_error_when_session_creation_fails
**Given**: opencode serve is running but session creation fails
**When**: Creating session for a bead
**Then**: Returns `Err(Error::SessionCreationFailed("bead-id: failed to create session"))`

### test_returns_error_when_dispatch_fails
**Given**: Session is created but go-skill dispatch fails
**When**: Dispatching go-skill for bead execution
**Then**: Returns `Err(Error::DispatchFailed("bead-id: dispatch failed"))`

### test_returns_error_on_poll_timeout
**Given**: Bead is dispatched but does not complete within timeout
**When**: Polling exceeds threshold (default 300s)
**Then**: Returns `Err(Error::PollTimeout("bead-id: polling timed out after 300s"))`

### test_returns_error_when_report_generation_fails
**Given**: All beads complete but report file cannot be written
**When**: Generating execution matrix report
**Then**: Returns `Err(Error::ReportGenerationFailed("failed to write report"))`

---

## Edge Case Tests

### test_handles_zero_batch_size_gracefully
**Given**: batch_size=0 is passed
**When**: Running svt-runner.sh with 0
**Then**: Uses default batch size (30) or returns error appropriately

### test_handles_empty_target_directory
**Given**: Target directory exists but is empty (no .beads directory)
**When**: Running SVT batch
**Then**: Returns `Err(Error::NoReadyBeads)` as no beads exist

### test_handles_server_crash_during_execution
**Given**: opencode serve crashes mid-batch
**When**: One or more serve instances terminate unexpectedly
**Then**: 
- Cleanup is attempted for crashed instances
- Report reflects partial completion with error status

### test_handles_partial_completion
**Given**: Some beads complete, others fail
**When**: Batch execution ends with mixed results
**Then**:
- Report shows successful beads and failed beads
- Cleanup is performed for all instances

### test_handles_missing_environment_variables
**Given**: Required env vars are not set
**When**: Running SVT batch
**Then**: Returns appropriate error or uses sensible defaults

### test_handles_concurrent_batch_runs
**Given**: Another SVT batch is already running
**When**: Starting new batch
**Then**: Returns `Err(Error::PortUnavailable)` or queues appropriately

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

### test_precondition_p3_p4_p5_cli_tools_available
**Given**: System with CLI tools
**When**: Checking jq, opencode, bd availability
**Then**: Returns `Ok(Vec<DependencyStatus>)` with all available tools

### test_precondition_p6_target_directory_valid
**Given**: Target directory path
**When**: Validating directory exists and is readable
**Then**: Returns `Ok(PathBuf)` if valid, `Err(Error::InvalidPath)` if not

### test_precondition_p7_ready_beads_exist
**Given**: Bead repository
**When**: Running `bd ready --json`
**Then**: Returns `Ok(Vec<Bead>)` if beads ready, `Err(Error::NoReadyBeads)` if empty

### test_precondition_p8_ports_available
**Given**: Port range 3000-3010
**When**: Checking port availability
**Then**: Returns `Ok(PortPool)` with available ports, `Err(Error::PortUnavailable)` if none

### test_postcondition_q1_report_generated
**Given**: SVT batch execution completes
**When**: Checking for report output
**Then**: Report file exists with valid JSON

### test_postcondition_q2_beads_complete
**Given**: SVT batch execution completes
**When**: Inspecting bead completion status
**Then**: All beads have terminal status (success/failure/timeout)

### test_postcondition_q3_serve_instances_cleaned_up
**Given**: SVT batch execution completes (success or failure)
**When**: Checking running processes
**Then**: No orphan opencode serve processes on test ports

### test_postcondition_q4_summary_produced
**Given**: SVT batch execution completes
**When**: Inspecting report summary
**Then**: Summary contains batch_size, model, completion counts

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
**Given**: opencode CLI is missing
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("opencode not found"))` - NOT a panic

### test_p5_violation_returns_dependency_missing
**Given**: bd CLI is missing
**When**: Running SVT batch
**Then**: Returns `Err(Error::DependencyMissing("bd not found"))` - NOT a panic

### test_p6_violation_returns_invalid_path
**Given**: Target directory does not exist
**When**: Running SVT with invalid path
**Then**: Returns `Err(Error::InvalidPath("target directory does not exist"))` - NOT a panic

### test_p7_violation_returns_no_ready_beads
**Given**: No beads in ready state
**When**: Running SVT batch
**Then**: Returns `Err(Error::NoReadyBeads("no ready beads found"))` - NOT a panic

### test_p8_violation_returns_port_unavailable
**Given**: All ports blocked
**When**: Starting opencode serve
**Then**: Returns `Err(Error::PortUnavailable("no ports available"))` - NOT a panic

### test_q1_violation_returns_report_generation_failed
**Given**: SVT runs but output is missing
**When**: Report not generated
**Then**: Returns `Err(Error::ReportGenerationFailed("report not found"))` - NOT a panic

### test_q2_violation_returns_poll_timeout
**Given**: Beads incomplete after timeout
**When**: Polling exceeds threshold
**Then**: Returns `Err(Error::PollTimeout(...))` - NOT a panic

### test_q3_violation_returns_cleanup_failed
**Given**: Process cleanup fails
**When**: Orphan processes remain
**Then**: Returns `Err(Error::CleanupFailed("processes still running"))` - NOT a panic

---

## Given-When-Then Scenarios

### Scenario 1: Successful SVT Batch 5 Execution
**Given**:
- svt-runner.sh exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
- All dependencies (jq, opencode, bd) are installed
- At least 5 beads are in ready state
- Ports 3000-3004 are available
- Target directory `/home/lewis/src/hardline` exists

**When**:
```bash
svt-runner.sh 5 /home/lewis/src/hardline
```

**Then**:
- 5 opencode serve instances start on ports 3000-3004
- 5 go-skill dispatches occur (one per bead)
- Polling completes for all beads
- Execution matrix report is generated
- All serve instances are cleaned up
- Returns `Ok(SvtBatchReport)` with 5 completed beads

### Scenario 2: Dependency Missing - jq
**Given**: jq is not installed on the system
**When**: Running SVT batch 5
**Then**:
- Execution fails early
- Returns `Err(Error::DependencyMissing("jq not found"))`
- No serve instances are started
- No cleanup needed

### Scenario 3: Partial Failure During Execution
**Given**: 5 beads ready, but bead-3 causes dispatch failure
**When**: SVT batch runs and bead-3 fails during dispatch
**Then**:
- Beads 1, 2, 4, 5 complete (success or own failures)
- Bead 3 shows dispatch failure in report
- All serve instances still cleaned up
- Report reflects partial completion

### Scenario 4: Cleanup After Timeout
**Given**: One bead exceeds poll timeout
**When**: Timeout occurs during batch execution
**Then**:
- Timeout bead is marked as timed out
- Other beads continue/complete
- All started serve instances are cleaned up
- Report includes timeout error details

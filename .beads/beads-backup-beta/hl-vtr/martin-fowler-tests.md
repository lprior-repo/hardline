---
bead_id: hl-vtr
bead_title: svt_batch_2
phase: contract-synthesis
updated_at: 2026-03-12T00:00:00Z
---

# Martin Fowler Test Plan

## Overview
This test plan validates SVT (Super Velocity Throughput) Load Test 2 - batch execution of 2 beads through the opencode serve infrastructure.

---

## Happy Path Tests

### test_svt_batch2_completes_successfully_with_2_beads
**Given**: All dependencies are installed, svt-runner.sh exists, and at least 2 beads are ready
**When**: Running `svt-runner.sh 2 /home/lewis/src/hardline` for batch 2
**Then**:
- Returns `Ok(SvtBatchReport)` with 2 beads processed
- All beads show completion status (success or documented failure)
- Report is generated with batch summary

### test_svt_batch2_creates_two_opencode_serve_instances
**Given**: opencode CLI is available and at least 2 ports are free
**When**: SVT batch 2 execution starts
**Then**:
- Two opencode serve instances are spawned on available ports
- Each instance responds to health checks

### test_svt_batch2_dispatches_go_skill_for_each_bead
**Given**: go-skill is available and 2 beads are ready
**When**: Processing each bead in batch 2
**Then**:
- go-skill is dispatched for each bead
- Each dispatch returns a session ID
- Exactly 2 dispatches occur

### test_svt_batch2_polls_for_completion
**Given**: Beads are dispatched and running
**When**: SVT polls for completion status
**Then**:
- Polling returns status for each bead
- Timeout is handled gracefully per bead

### test_svt_batch2_generates_execution_matrix_report
**Given**: All beads in batch complete (success or failure)
**When**: Report generation phase
**Then**:
- Execution matrix is generated with per-bead results
- Summary includes batch size (2), model, completion rates

### test_svt_batch2_respects_exact_batch_size
**Given**: More than 2 beads are ready (e.g., 5 beads)
**When**: Running batch 2
**Then**:
- Only 2 beads are processed
- Remaining beads remain in ready state
- No overflow to batch 3

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

### test_returns_error_when_only_one_bead_ready
**Given**: Only 1 bead is in ready state (need 2 for batch 2)
**When**: Running `bd ready --json` returns 1 bead
**Then**: Returns `Err(Error::InsufficientReadyBeads { requested: 2, available: 1 })`

### test_returns_error_when_no_beads_ready
**Given**: No beads are in ready state
**When**: Running `bd ready --json` returns empty
**Then**: Returns `Err(Error::InsufficientReadyBeads { requested: 2, available: 0 })`

### test_returns_error_when_only_one_port_available
**Given**: Only 1 port in range 3000-3010 is available (need 2)
**When**: Attempting to start opencode serve instances
**Then**: Returns `Err(Error::PortUnavailable { requested: 2, available: 1 })`

### test_returns_error_when_no_ports_available
**Given**: All ports in range 3000-3010 are occupied
**When**: Attempting to start opencode serve instances
**Then**: Returns `Err(Error::PortUnavailable { requested: 2, available: 0 })`

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

### test_handles_batch_size_of_one_gracefully
**Given**: batch_size=1 is passed to batch 2 runner
**When**: Running svt-runner.sh with 1
**Then**: Returns error or uses minimum batch size of 2

### test_handles_negative_batch_size
**Given**: batch_size=-1 is passed
**When**: Running svt-runner.sh with negative
**Then**: Returns error for invalid batch size

### test_handles_empty_target_directory
**Given**: Target directory exists but is empty (no .beads directory)
**When**: Running SVT batch
**Then**: Returns `Err(Error::InsufficientReadyBeads { requested: 2, available: 0 })`

### test_handles_server_crash_during_execution
**Given**: opencode serve crashes mid-batch
**When**: One or more serve instances terminate unexpectedly
**Then**: 
- Cleanup is attempted for crashed instances
- Report reflects partial completion with error status

### test_handles_partial_completion
**Given**: One bead completes, other fails
**When**: Batch execution ends with mixed results
**Then**:
- Report shows 1 successful bead and 1 failed bead
- Cleanup is performed for all instances

### test_handles_missing_environment_variables
**Given**: Required env vars are not set
**When**: Running SVT batch
**Then**: Returns appropriate error or uses sensible defaults

### test_handles_concurrent_batch_runs
**Given**: Another SVT batch is already running
**When**: Starting batch 2
**Then**: Either runs concurrently or returns port conflict error

### test_handles_bead_failure_gracefully
**Given**: One bead in batch 2 fails during execution
**When**: Bead execution fails
**Then**:
- Other bead continues execution
- Failed bead is marked as failed in report
- Batch completes with partial success

### test_handles_rapid_port_reuse
**Given**: Previous test just finished and ports are in TIME_WAIT
**When**: Starting new batch 2
**Then**: Successfully binds to ports after brief wait

---

## Contract Verification Tests

### Precondition Tests

- `test_precondition_p1_script_exists`
- `test_precondition_p2_script_is_executable`
- `test_precondition_p3_jq_available`
- `test_precondition_p4_opencode_available`
- `test_precondition_p5_bd_available`
- `test_precondition_p6_target_directory_valid`
- `test_precondition_p7_at_least_2_beads_ready`
- `test_precondition_p8_at_least_2_ports_available`

### Postcondition Tests

- `test_postcondition_q1_report_generated`
- `test_postcondition_q2_all_beads_complete`
- `test_postcondition_q3_instances_cleaned_up`
- `test_postcondition_q4_summary_produced`
- `test_postcondition_q5_exactly_2_beads_processed`

### Invariant Tests

- `test_invariant_i1_no_orphan_processes`
- `test_invariant_i2_bead_state_consistent`
- `test_invariant_i3_resource_cleanup`
- `test_invariant_i4_batch_size_respected`

---

## Contract Violation Tests

### P1 Violation Tests

```
test_p1_violation_missing_script_returns_dependency_missing
  Given: svt-runner.sh does not exist at /home/lewis/.config/opencode/skill/svt/svt-runner.sh
  When: Attempting to run SVT batch 2
  Then: Returns Err(Error::DependencyMissing("svt-runner.sh not found"))
  And: NOT a panic
```

### P3 Violation Tests

```
test_p3_violation_missing_jq_returns_dependency_missing
  Given: jq is not installed or not in PATH
  When: SVT runner attempts to parse JSON output
  Then: Returns Err(Error::DependencyMissing("jq not found"))
  And: NOT a panic
```

### P4 Violation Tests

```
test_p4_violation_missing_opencode_returns_dependency_missing
  Given: opencode CLI is not installed
  When: Attempting to start opencode serve
  Then: Returns Err(Error::DependencyMissing("opencode not found"))
  And: NOT a panic
```

### P5 Violation Tests

```
test_p5_violation_missing_bd_returns_dependency_missing
  Given: bd CLI is not installed
  When: Checking for ready beads
  Then: Returns Err(Error::DependencyMissing("bd not found"))
  And: NOT a panic
```

### P6 Violation Tests

```
test_p6_violation_invalid_target_directory_returns_invalid_path
  Given: Target directory "/nonexistent/path" does not exist
  When: Running SVT batch 2 with invalid target_dir
  Then: Returns Err(Error::InvalidPath("target directory does not exist"))
  And: NOT a panic
```

### P7 Violation Tests

```
test_p7_violation_only_one_bead_ready_returns_insufficient_ready_beads
  Given: Only 1 bead is in ready state
  When: Running bd ready --json returns 1 bead
  Then: Returns Err(Error::InsufficientReadyBeads { requested: 2, available: 1 })
  And: NOT a panic
```

```
test_p7_violation_no_beads_ready_returns_insufficient_ready_beads
  Given: No beads are in ready state
  When: Running bd ready --json returns empty
  Then: Returns Err(Error::InsufficientReadyBeads { requested: 2, available: 0 })
  And: NOT a panic
```

### P8 Violation Tests

```
test_p8_violation_only_one_port_available_returns_port_unavailable
  Given: Only 1 port in range 3000-3010 is available
  When: Attempting to start 2 opencode serve instances
  Then: Returns Err(Error::PortUnavailable { requested: 2, available: 1 })
  And: NOT a panic
```

```
test_p8_violation_no_ports_available_returns_port_unavailable
  Given: All ports in range 3000-3010 are occupied
  When: Attempting to start opencode serve instances
  Then: Returns Err(Error::PortUnavailable { requested: 2, available: 0 })
  And: NOT a panic
```

### Q1 Violation Tests

```
test_q1_violation_report_not_generated_returns_report_generation_failed
  Given: SVT execution completes but report file is missing
  When: Checking for report output
  Then: Returns Err(Error::ReportGenerationFailed("report not found"))
  And: NOT a panic
```

### Q2 Violation Tests

```
test_q2_violation_bead_incomplete_after_timeout_returns_poll_timeout
  Given: Bead is dispatched but does not complete within timeout
  When: Polling exceeds threshold
  Then: Returns Err(Error::PollTimeout("bead-id: polling timed out after 300s"))
  And: NOT a panic
```

### Q3 Violation Tests

```
test_q3_violation_processes_remain_running_returns_cleanup_failed
  Given: opencode serve processes remain running after test completion
  When: Cleanup verification runs
  Then: Returns Err(Error::CleanupFailed("processes still running: [pid1, pid2]"))
  And: NOT a panic
```

### Q5 Violation Tests

```
test_q5_violation_batch_processes_more_than_2_beads_returns_batch_size_violation
  Given: More than 2 beads are ready (e.g., 5 beads)
  When: Running batch 2
  Then: Returns Err(Error::BatchSizeViolation("expected 2, processed 5"))
  And: NOT a panic
```

---

## Given-When-Then Scenarios

### Scenario 1: Successful SVT Batch 2 Execution
**Given**: All dependencies are installed, svt-runner.sh exists, and at least 2 beads are ready
**When**: Running `svt-runner.sh 2 /home/lewis/src/hardline` for batch 2
**Then**:
- Exactly 2 opencode serve instances are spawned
- go-skill is dispatched for each of the 2 beads
- Both beads complete (success or failure)
- Execution report is generated with 2 results
- All processes are cleaned up

### Scenario 2: SVT Fails Due to Insufficient Ready Beads
**Given**: Only 1 bead is in ready state
**When**: Running batch 2
**Then**:
- Returns `Err(Error::InsufficientReadyBeads { requested: 2, available: 1 })`
- No opencode serve instances are started
- No report is generated

### Scenario 3: SVT Fails Due to Port Conflict
**Given**: Only 1 port is available in the port range
**When**: Attempting to start 2 opencode serve instances
**Then**:
- Returns `Err(Error::PortUnavailable { requested: 2, available: 1 })`
- No serve instances are started

### Scenario 4: SVT Handles Bead Failure Gracefully
**Given**: 2 beads are ready, but one fails during execution
**When**: One bead's go-skill dispatch fails
**Then**:
- Failed bead is marked as failed in report
- Other bead continues execution
- Batch completes with 1 success, 1 failure
- Cleanup is performed

### Scenario 5: SVT Batch 2 Recovers from Server Crash
**Given**: opencode serve crashes during batch execution
**When**: One serve instance terminates unexpectedly
**Then**:
- Cleanup is attempted for crashed instance
- Remaining bead continues if possible
- Report reflects partial completion with error status
- No orphan processes remain

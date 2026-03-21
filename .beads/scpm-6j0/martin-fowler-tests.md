# Martin Fowler Test Plan: Atomic Batch Execution

## Test Metadata
- **Feature**: cli: implement atomic batch execution
- **Bead ID**: scpm-6j0
- **Updated**: 2026-03-20

## Happy Path Tests

### test_batch_executes_all_commands_and_commits
**Given**: Workspace is ready, batch has 3 valid commands
**When**: `execute_batch(ws, [cmd1, cmd2, cmd3])` is called
**Then**:
- Returns `BatchResult::Committed { checkpoint_id, results: [r1, r2, r3] }`
- All 3 command results present in order
- Workspace state reflects all 3 command changes

### test_batch_creates_checkpoint_on_success
**Given**: Workspace is ready, batch has 2 valid commands
**When**: `execute_batch()` succeeds
**Then**:
- New checkpoint created in database
- `checkpoint_id` in result matches database entry

## Error Path Tests

### test_returns_empty_batch_error_when_no_commands
**Given**: Empty commands array `[]`
**When**: `execute_batch(ws, batch_id, [])` is called
**Then**: Returns `Err(BatchError::EmptyBatch)`

### test_returns_workspace_not_ready_when_locked
**Given**: Workspace is locked by another operation
**When**: `execute_batch(ws, batch_id, [cmd1])` is called
**Then**: Returns `Err(BatchError::WorkspaceNotReady("workspace locked"))`

### test_returns_execution_failed_when_command_fails
**Given**: Batch has valid commands, but command 2 will fail
**When**: `execute_batch(ws, batch_id, [cmd1, failing_cmd, cmd3])` is called
**Then**: 
- Returns `BatchResult::RolledBack { error: BatchError::ExecutionFailed {...} }`
- Workspace state is identical to pre-batch

### test_rollback_failure_is_propagated_not_silently_ignored
**Given**: Batch execution fails AND rollback fails
**When**: `execute_batch()` is called
**Then**:
- Returns `Err(BatchError::RollbackFailed { checkpoint_id, underlying })`
- Error is NOT silently ignored
- Log contains "RollbackFailed" warning

### test_returns_batch_in_progress_when_another_batch_running
**Given**: Batch A is currently executing
**When**: Batch B `execute_batch()` is called
**Then**: Returns `Err(BatchError::BatchInProgress)`

### test_commit_failure_returns_error
**Given**: All commands succeeded, but commit fails
**When**: `execute_batch()` reaches commit phase
**Then**: Returns `Err(BatchError::CommitFailed {...})`

## Edge Case Tests

### test_single_command_batch_works
**Given**: Batch with exactly 1 command
**When**: `execute_batch(ws, batch_id, [single_cmd])` is called
**Then**:
- Returns `BatchResult::Committed { results: [single_result] }`
- Checkpoint created

### test_batch_command_results_preserve_order
**Given**: Batch with 5 commands
**When**: All commands succeed
**Then**: Results array is in exact command execution order

### test_empty_command_args_handled
**Given**: Command with empty args array
**When**: `execute_batch()` runs this command
**Then**: Command executes with no arguments (not error)

### test_workspace_state_identical_after_failed_batch
**Given**: Workspace has initial state S0, commands would change to S1
**When**: Batch fails mid-execution
**Then**: Workspace state is back to S0 (not S1, not intermediate)

## Contract Verification Tests

### test_precondition_p2_empty_batch_rejected
**Given**: An empty batch
**When**: `execute_batch()` is invoked
**Then**: Returns `Err(BatchError::EmptyBatch)` (NOT a panic)

### test_precondition_p3_workspace_ready_enforced
**Given**: Workspace in dirty state
**When**: `execute_batch()` is invoked
**Then**: Returns `Err(BatchError::WorkspaceNotReady(...))`

### test_invariant_i3_single_batch_enforced
**Given**: A batch is already running
**When**: A second batch starts
**Then**: Returns `Err(BatchError::BatchInProgress)`

### test_postcondition_q2_rollback_restores_state
**Given**: Pre-batch workspace state is W0
**When**: Batch fails at step 2
**Then**: Workspace state is W0 (fully restored)

## Given-When-Then Scenarios

### Scenario 1: Successful Atomic Batch
**Given**: Workspace "hl-test" is ready, no locks
**When**: User runs `scp batch --workspace hl-test --commands "jj status, jj log -r @, jj diff"`
**Then**:
- All 3 commands execute in sequence
- Output shows all 3 command results
- Exit code is 0
- Checkpoint "auto-{timestamp}" created

### Scenario 2: Failed Batch Triggers Rollback
**Given**: Workspace "hl-test" has 2 commits ready
**When**: User runs `scp batch --commands "jj status, invalid-command, jj log"`
**Then**:
- First command executes
- Second command fails with parse error
- Third command does NOT execute
- Workspace state unchanged from pre-batch
- Returns error with `BatchError::ExecutionFailed`

### Scenario 3: Rollback Failure Is Critical Error
**Given**: Workspace "hl-test" in corrupted state, batch will fail
**When**: Batch fails AND subsequent rollback fails
**Then**:
- Returns `Err(BatchError::RollbackFailed { checkpoint_id, underlying })`
- Error is logged at WARN level
- CLI exit code is non-zero
- Error message clearly explains rollback failure

### Scenario 4: Concurrent Batch Rejected
**Given**: Batch A is running (slow command in progress)
**When**: User attempts to start Batch B
**Then**:
- Batch B rejected immediately
- Returns `Err(BatchError::BatchInProgress)`
- Batch A continues uninterrupted

## End-to-End Pipeline Test

### test_full_batch_pipeline_with_jj_workspace
**Given**: JJ workspace "hl-batch-test" exists and is clean
**When**: Running: `scp batch --workspace hl-batch-test --commands "jj new, jj describe -m 'batch change', jj log -r @"`
**Then**:
- Creates new commit
- Sets description
- Shows new commit in log
- Exit code 0
- `jj log` shows the batch commit

# Martin Fowler Test Plan: CLI Wait and Batch Commands

## Happy Path Tests

### Wait Command Happy Path Tests

- **test_wait_returns_success_when_session_exists_immediately**
  - Given: Session "test-session" already exists in registry
  - When: Running `wait test-session --mode session-exists`
  - Then: Returns `Ok(WaitResult::ConditionMet { session: "test-session", state: Created })` immediately

- **test_wait_returns_success_when_session_becomes_healthy**
  - Given: Session "test-session" starts as unhealthy, will become healthy after 2 seconds
  - When: Running `wait test-session --mode healthy --timeout 10s`
  - Then: Returns `Ok(WaitResult::ConditionMet { session: "test-session", state: Healthy })` after ~2 seconds

- **test_wait_returns_success_when_session_reaches_target_status**
  - Given: Session "test-session" in Created state, will transition to Active after 1 second
  - When: Running `wait test-session --mode status=Active --timeout 5s`
  - Then: Returns `Ok(WaitResult::ConditionMet { session: "test-session", state: Active })` after ~1 second

- **test_wait_respects_custom_poll_interval**
  - Given: Session "test-session" will become healthy after 5 seconds
  - When: Running `wait test-session --mode healthy --poll-interval 2s --timeout 10s`
  - Then: Polling occurs at 2-second intervals, returns after ~5 seconds

### Batch Command Happy Path Tests

- **test_batch_executes_single_command_successfully**
  - Given: Valid command "workspace list"
  - When: Running `batch "workspace list"`
  - Then: Returns `Ok(BatchResult::Completed { checkpoint_id, results: [CommandResult::Success] })`

- **test_batch_executes_multiple_commands_in_order**
  - Given: Commands ["workspace list", "queue list", "session list"]
  - When: Running `batch "workspace list" "queue list" "session list"`
  - Then: Returns `Ok(BatchResult::Completed { checkpoint_id, results: [Success, Success, Success] })` in order

- **test_batch_creates_checkpoint_before_execution**
  - Given: Valid commands
  - When: Running `batch "workspace list"`
  - Then: Checkpoint is created before any command runs, `checkpoint_id` is present in result

- **test_batch_dry_run_does_not_execute_commands**
  - Given: Valid commands
  - When: Running `batch --dry-run "workspace list" "queue list"`
  - Then: Returns `Ok(BatchResult::Completed { checkpoint_id: None, results: [] })` - no execution

- **test_batch_with_checkpoint_file_path**
  - Given: Valid commands
  - When: Running `batch --checkpoint /tmp/batch.cp "workspace list"`
  - Then: Checkpoint saved to specified path, result includes checkpoint_id

---

## Error Path Tests

### Wait Command Error Path Tests

- **test_wait_returns_error_when_session_not_found**
  - Given: No session "nonexistent" exists
  - When: Running `wait nonexistent --mode session-exists --timeout 2s`
  - Then: Returns `Err(Error::SessionNotFound("nonexistent".into()))`

- **test_wait_returns_timeout_error_when_condition_not_met**
  - Given: Session "test-session" exists but never becomes healthy
  - When: Running `wait test-session --mode healthy --timeout 1s`
  - Then: Returns `Err(Error::WaitTimeout { session: "test-session", expected: Healthy })`

- **test_wait_returns_error_for_invalid_wait_mode**
  - Given: Valid session
  - When: Running `wait test-session --mode invalid-mode`
  - Then: Returns `Err(Error::InvalidWaitMode("unknown mode: invalid-mode".into()))`

- **test_wait_returns_error_for_zero_timeout**
  - Given: Valid session
  - When: Running `wait test-session --mode healthy --timeout 0`
  - Then: Returns `Err(Error::ValidationError("timeout must be > 0".into()))`

- **test_wait_returns_error_for_empty_session_name**
  - Given: Nothing
  - When: Running `wait "" --mode session-exists`
  - Then: Returns `Err(Error::InvalidSessionName("session name cannot be empty".into()))`

### Batch Command Error Path Tests

- **test_batch_returns_error_for_empty_command_list**
  - Given: Empty command list
  - When: Running `batch`
  - Then: Returns `Err(Error::BatchEmpty("batch must contain at least one command".into()))`

- **test_batch_returns_error_for_invalid_command**
  - Given: Invalid command "invalid-cmd"
  - When: Running `batch "invalid-cmd"`
  - Then: Returns `Err(Error::BatchCommandFailed("unknown command: invalid-cmd".into()))`

- **test_batch_rolls_back_on_command_failure**
  - Given: Commands ["workspace list", "invalid-cmd", "queue list"]
  - When: Running `batch "workspace list" "invalid-cmd" "queue list"`
  - Then: Returns `Err(Error::BatchCommandFailed)` and checkpoint is rolled back

- **test_batch_returns_error_when_rollback_fails**
  - Given: Command fails, but checkpoint restore fails
  - When: Running batch with corrupted checkpoint
  - Then: Returns `Err(Error::BatchRollbackFailed("failed to restore checkpoint".into()))`

- **test_batch_returns_error_when_size_exceeds_limit**
  - Given: More than 100 commands
  - When: Running batch with 101 commands
  - Then: Returns `Err(Error::BatchSizeExceeded("batch size exceeds maximum of 100".into()))`

---

## Edge Case Tests

### Wait Command Edge Case Tests

- **test_wait_handles_rapid_state_changes**
  - Given: Session rapidly toggles between healthy/unhealthy
  - When: Running `wait test-session --mode healthy --timeout 5s`
  - Then: Returns on first healthy state, does not race

- **test_wait_with_very_large_timeout**
  - Given: Session will become healthy after very long time
  - When: Running `wait test-session --mode healthy --timeout 86400s` (24 hours)
  - Then: Polls correctly, returns when condition met

- **test_wait_with_minimum_poll_interval**
  - Given: Session becomes healthy immediately
  - When: Running `wait test-session --mode healthy --poll-interval 100ms`
  - Then: Returns immediately, no unnecessary waiting

- **test_wait_handles_all_session_states**
  - Given: Session can be in Created, Active, Syncing, Synced, Paused, Completed, Failed states
  - When: Running `wait test-session --mode status=Active`
  - Then: Correctly identifies matching state

### Batch Command Edge Case Tests

- **test_batch_handles_single_command**
  - Given: Single valid command
  - When: Running `batch "workspace list"`
  - Then: Executes successfully, no unnecessary checkpoint complexity

- **test_batch_handles_command_with_special_characters**
  - Given: Commands with special characters in args
  - When: Running `batch "workspace spawn 'my-workspace'" --sync`
  - Then: Correctly parses and executes with quoted args

- **test_batch_handles_empty_args**
  - Given: Command with no additional args
  - When: Running `batch "queue list"`
  - Then: Executes correctly

- **test_batch_handles_duplicate_commands**
  - Given: Same command specified multiple times
  - When: Running `batch "workspace list" "workspace list"`
  - Then: Executes each command in order, returns combined results

---

## Contract Verification Tests

### Wait Command Contract Tests

- **test_precondition_session_name_not_empty**
  - Given: Empty session name
  - When: Parsing wait command arguments
  - Then: Returns `Err(Error::InvalidSessionName)` before any wait

- **test_precondition_wait_mode_valid**
  - Given: Invalid wait mode string
  - When: Parsing wait mode from CLI
  - Then: Returns `Err(Error::InvalidWaitMode)`

- **test_precondition_timeout_positive**
  - Given: Timeout value of 0
  - When: Validating timeout duration
  - Then: Returns `Err(Error::ValidationError("timeout must be > 0"))`

- **test_postcondition_returns_on_condition_met**
  - Given: Session meets condition during wait
  - When: Wait loop detects condition met
  - Then: Returns immediately with `WaitResult::ConditionMet`

- **test_invariant_poll_interval_within_bounds**
  - Given: Poll interval < 100ms or > 60s
  - When: Validating poll interval
  - Then: Returns `Err(Error::ValidationError)` (or uses clamped value with warning)

### Batch Command Contract Tests

- **test_precondition_commands_not_empty**
  - Given: Empty command vector
  - When: Validating batch input
  - Then: Returns `Err(Error::BatchEmpty)`

- **test_precondition_each_command_valid**
  - Given: Vector with invalid command
  - When: Parsing commands
  - Then: Returns `Err(Error::BatchCommandFailed)` for first invalid command

- **test_precondition_batch_size_within_limit**
  - Given: 101+ commands
  - When: Validating batch size
  - Then: Returns `Err(Error::BatchSizeExceeded)`

- **test_postcondition_all_or_nothing_semantics**
  - Given: Batch where command 2 of 3 fails
  - When: Batch execution
  - Then: Either all 3 succeed or all are rolled back

- **test_invariant_checkpoint_created_before_first_command**
  - Given: Valid batch
  - When: Before executing first command
  - Then: Checkpoint exists and is valid

---

## Contract Violation Tests

(One test per violation example in contract-spec.md)

- `test_p1_violation_empty_session_name_returns_invalid_session_name_error`
  - Given: Empty session name input
  - When: `wait(session_name="", mode=SessionExists, timeout=None)`
  - Then: Returns `Err(Error::InvalidSessionName("session name cannot be empty".into()))`

- `test_p2_violation_invalid_wait_mode_returns_invalid_wait_mode_error`
  - Given: Invalid wait mode
  - When: `wait(session_name="test", mode=InvalidMode, timeout=None)`
  - Then: Returns `Err(Error::InvalidWaitMode("unknown mode: InvalidMode".into()))`

- `test_p3_violation_zero_timeout_returns_validation_error`
  - Given: Zero timeout
  - When: `wait(session_name="test", mode=Healthy, timeout=Some(0))`
  - Then: Returns `Err(Error::ValidationError("timeout must be > 0".into()))`

- `test_q2_violation_timeout_expired_returns_wait_timeout_error`
  - Given: Session never becomes healthy
  - When: `wait(session_name="test", mode=Healthy, timeout=Some(Duration::from_secs(1)))`
  - Then: Returns `Err(Error::WaitTimeout { session: "test", expected: Healthy })` after timeout

- `test_p4_violation_empty_batch_returns_batch_empty_error`
  - Given: Empty command list
  - When: `batch(commands=[], checkpoint_path=None, dry_run=false)`
  - Then: Returns `Err(Error::BatchEmpty("batch must contain at least one command".into()))`

- `test_p5_violation_invalid_command_returns_batch_command_failed_error`
  - Given: Invalid command
  - When: `batch(commands=[BatchCommand { subcommand: "invalid-cmd", args: [] }], ...)`
  - Then: Returns `Err(Error::BatchCommandFailed("unknown command: invalid-cmd".into()))`

- `test_q5_violation_rollback_failure_returns_batch_rollback_failed_error`
  - Given: Command fails and checkpoint is corrupted
  - When: Batch execution with failed rollback
  - Then: Returns `Err(Error::BatchRollbackFailed("failed to restore checkpoint: ...".into()))`

- `test_p6_violation_empty_checkpoint_path_returns_validation_error`
  - Given: Empty checkpoint path
  - When: `batch(commands=[cmd], checkpoint_path=Some(PathBuf::from("")), ...)`
  - Then: Returns `Err(Error::ValidationError("checkpoint path cannot be empty".into()))`

---

## Given-When-Then Scenarios

### Scenario 1: Wait for session to exist
- **Scenario**: Wait for a new session to be created by another process
- **Given**: No session "new-session" exists initially
- **When**: Running `wait new-session --mode session-exists --timeout 30s`
- **Then**:
  - Polls every 1 second (default)
  - Returns success when session is created by external process
  - Returns timeout error if session not created within 30 seconds

### Scenario 2: Wait for session health
- **Scenario**: Wait for a session to become healthy after initialization
- **Given**: Session "test-session" starts in "Created" state, will become "Healthy" after 5 seconds
- **When**: Running `wait test-session --mode healthy --timeout 60s`
- **Then**:
  - Polls session health every 1 second
  - Returns success when session reports "Healthy" state
  - Returns timeout if health not achieved within 60 seconds

### Scenario 3: Atomic batch execution
- **Scenario**: Execute multiple commands atomically with rollback on failure
- **Given**: Commands "workspace list" succeeds, "queue list" succeeds
- **When**: Running `batch "workspace list" "queue list"`
- **Then**:
  - Checkpoint created before execution
  - Both commands execute in order
  - Both succeed, result shows both success
  - Checkpoint ID returned in result

### Scenario 4: Batch rollback on failure
- **Scenario**: Batch fails mid-way and rolls back
- **Given**: Commands "workspace list" succeeds, "invalid-cmd" fails
- **When**: Running `batch "workspace list" "invalid-cmd" "queue list"`
- **Then**:
  - Checkpoint created
  - "workspace list" executes successfully
  - "invalid-cmd" fails
  - Rollback initiated
  - Returns error with `BatchResult::RolledBack` variant
  - Checkpoint restored

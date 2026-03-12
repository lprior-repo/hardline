# Contract Specification: CLI Wait and Batch Commands

## Context
- **Feature**: Add `wait` and `batch` commands to the SCP CLI
- **Bead**: scp-4b2
- **Domain terms**:
  - `wait` - blocking primitive that polls until a condition is met
  - `batch` - atomic execution of multiple commands with checkpoint rollback on failure
  - `session` - workspace/session entity that can be in various states
  - `health` - session health status (healthy/unhealthy/unknown)
  - `checkpoint` - saved state for rollback capability
- **Assumptions**:
  - Session entities already exist (see `crates/session/src/domain/entities/session.rs`)
  - Agent registry exists for tracking agent status
  - The CLI uses clap for argument parsing
  - Error types follow the pattern in `crates/core/src/error.rs`

## Open Questions
- Should `wait` support timeout duration? (Yes, add `--timeout` flag)
- What is the default polling interval? (1 second)
- Should `batch` support dry-run mode? (Yes, add `--dry-run` flag)
- How is checkpoint stored? (In-memory for now, persisted in future)

---

## Preconditions

### Wait Command Preconditions

- **[P1]**: Target session name must be provided
  - Enforcement: CLI argument is required (String, non-empty)
  - Type encoding: `String` argument, validated with `.is_empty()` check

- **[P2]**: Wait mode must be valid (session-exists, healthy, status)
  - Enforcement: Enum-based validation
  - Type encoding: `WaitMode` enum with variants `SessionExists`, `Healthy`, `Status`

- **[P3]**: Timeout value, if provided, must be positive
  - Enforcement: Runtime check via `Duration::from_secs`
  - Type encoding: `Option<u64>` with validation

### Batch Command Preconditions

- **[P4]**: At least one command must be provided in the batch
  - Enforcement: CLI requires non-empty vector
  - Type encoding: `Vec<BatchCommand>` with length check

- **[P5]**: Each command must be valid (non-empty, recognized subcommand)
  - Enforcement: Each command parsed as CLI subcommand
  - Type encoding: `BatchCommand` enum with variants matching existing commands

- **[P6]**: Checkpoint file path, if provided, must be valid path
  - Enforcement: `PathBuf` validation
  - Type encoding: `Option<PathBuf>`

---

## Postconditions

### Wait Command Postconditions

- **[Q1]**: Returns success when the wait condition is met
  - Type: `Result<WaitResult, Error>`
  - Returns `WaitResult::ConditionMet { session: SessionName, state: SessionState }`

- **[Q2]**: Returns error with `WaitTimeout` if timeout expires before condition met
  - Type: `Result<WaitResult, Error::WaitTimeout>`

- **[Q3]**: Returns error with `SessionNotFound` if session does not exist (for session-exists mode)
  - Type: `Result<WaitResult, Error::SessionNotFound>`

### Batch Command Postconditions

- **[Q4]**: All commands execute atomically (all succeed or all rollback)
  - Type: `Result<BatchResult, Error>`
  - Returns `BatchResult::Completed { results: Vec<CommandResult> }`

- **[Q5]**: On failure, all changes are rolled back to checkpoint
  - Type: `Result<BatchResult, Error>`
  - Returns `BatchResult::RolledBack { checkpoint_id: CheckpointId }`

- **[Q6]**: Checkpoint is created before batch execution starts
  - Type: `CheckpointId` returned in `BatchResult::Completed` or `BatchResult::RolledBack`

---

## Invariants

### Wait Command Invariants

- **[I1]**: Polling interval is always >= 100ms and <= 60s
  - Enforced: `MIN_POLL_INTERVAL <= interval <= MAX_POLL_INTERVAL`

- **[I2]**: Once condition is met, wait returns immediately (no extra waits)
  - Enforced: Check condition before each sleep cycle

### Batch Command Invariants

- **[I3]**: If any command fails, all prior commands in batch are rolled back
  - Enforced: Checkpoint created, rollback on `Err`

- **[I4]**: Batch size is limited to prevent resource exhaustion
  - Enforced: `MAX_BATCH_SIZE = 100` commands

---

## Error Taxonomy

### Wait Command Errors

| Error Variant | Code | Condition |
|---|---|---|
| `WaitTimeout` | 55 | Timeout expired before condition met |
| `SessionNotFound` | 14 | Session does not exist |
| `InvalidWaitMode` | 80 | Invalid wait mode specified |
| `InvalidSessionName` | 82 | Session name is empty/invalid |

### Batch Command Errors

| Error Variant | Code | Condition |
|---|---|---|
| `BatchEmpty` | 80 | No commands provided |
| `BatchCommandFailed` | 56 | Individual command failed |
| `BatchRollbackFailed` | 57 | Rollback to checkpoint failed |
| `CheckpointError` | 58 | Checkpoint creation/storage failed |
| `BatchSizeExceeded` | 80 | More than MAX_BATCH_SIZE commands |

---

## Contract Signatures

### Wait Command

```rust
/// Wait modes for the wait command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitMode {
    SessionExists,  // Wait until session exists
    Healthy,        // Wait until session is healthy
    Status(String), // Wait until session reaches specific status
}

/// Result of wait operation
#[derive(Debug, Clone)]
pub enum WaitResult {
    ConditionMet { session: SessionName, state: SessionState },
    Timeout { session: SessionName, expected: WaitMode },
}

/// Wait for a condition to be met
pub fn wait(
    session: SessionName,
    mode: WaitMode,
    timeout: Option<Duration>,
    poll_interval: Duration,
) -> Result<WaitResult, Error>;
```

### Batch Command

```rust
/// Individual command in a batch
#[derive(Debug, Clone)]
pub struct BatchCommand {
    pub subcommand: String,
    pub args: Vec<String>,
}

/// Result of batch operation
#[derive(Debug, Clone)]
pub enum BatchResult {
    Completed {
        checkpoint_id: CheckpointId,
        results: Vec<CommandResult>,
    },
    RolledBack {
        checkpoint_id: CheckpointId,
        error: Error,
    },
}

/// Execute batch of commands atomically with checkpoint rollback
pub fn batch(
    commands: Vec<BatchCommand>,
    checkpoint_path: Option<PathBuf>,
    dry_run: bool,
) -> Result<BatchResult, Error>;
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Session name provided | Compile-time | Required CLI argument |
| Wait mode valid | Compile-time | `enum WaitMode { ... }` |
| Timeout positive | Runtime-checked constructor | `Duration::from_secs(timeout).unwrap()` (returns Err if 0) |
| Commands non-empty | Runtime check | `if commands.is_empty()` returns `Err(Error::BatchEmpty)` |
| Command valid | Runtime parse | `BatchCommand::parse()` |
| Batch size limit | Runtime check | `if commands.len() > MAX_BATCH_SIZE` returns `Err(Error::BatchSizeExceeded)` |

---

## Violation Examples

### Wait Command Violations

- **VIOLATES P1**: `wait(session_name="", mode=SessionExists, timeout=None)`
  - Expected: `Err(Error::InvalidSessionName("session name cannot be empty".into()))`

- **VIOLATES P2**: `wait(session_name="test", mode=InvalidMode, timeout=None)`
  - Expected: `Err(Error::InvalidWaitMode("unknown mode: InvalidMode".into()))`

- **VIOLATES P3**: `wait(session_name="test", mode=Healthy, timeout=Some(0))`
  - Expected: `Err(Error::ValidationError("timeout must be > 0".into()))`

- **VIOLATES Q2**: `wait(session_name="test", mode=Healthy, timeout=Some(Duration::from_secs(1)))` with session never becoming healthy
  - Expected: `Err(Error::WaitTimeout { session: "test", expected: Healthy })` after timeout

### Batch Command Violations

- **VIOLATES P4**: `batch(commands=[], checkpoint_path=None, dry_run=false)`
  - Expected: `Err(Error::BatchEmpty("batch must contain at least one command".into()))`

- **VIOLATES P5**: `batch(commands=[BatchCommand { subcommand: "invalid-cmd", args: [] }], ...)`
  - Expected: `Err(Error::BatchCommandFailed("unknown command: invalid-cmd".into()))`

- **VIOLATES P6**: `batch(commands=[...], checkpoint_path=Some(PathBuf::from("")), ...)`
  - Expected: `Err(Error::ValidationError("checkpoint path cannot be empty".into()))`

- **VIOLATES Q5**: Rollback fails after command 2 of 3 fails
  - Expected: `Err(Error::BatchRollbackFailed("failed to restore checkpoint: ...".into()))`

---

## Ownership Contracts

### Wait Command

- `session: SessionName` - Borrowed, read-only for session lookup
- `timeout: Option<Duration>` - Borrowed, no mutation
- `poll_interval: Duration` - Borrowed, used for timing

### Batch Command

- `commands: Vec<BatchCommand>` - Owned, consumed during execution
- `checkpoint_path: Option<PathBuf>` - Borrowed, used for checkpoint file
- `dry_run: bool` - Borrowed, read-only flag

---

## Non-goals

- [ ] Persistence of checkpoints across CLI invocations (future work)
- [ ] Distributed transactions across multiple machines (future work)
- [ ] Real-time event subscription instead of polling (future work)
- [ ] Complex workflow definitions (future work)

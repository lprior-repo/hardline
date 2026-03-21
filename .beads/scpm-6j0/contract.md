# Contract Specification: Atomic Batch Execution

## Context
- **Feature**: cli: implement atomic batch execution
- **Bead ID**: scpm-6j0
- **Domain terms**:
  - `BatchCommand`: A single command in a batch with name and arguments
  - `BatchId`: Unique identifier for a batch execution
  - `CheckpointGuard`: RAII guard for transactional semantics
  - `WorkspaceState`: Pre-batch vs post-batch state
- **Assumptions**:
  - JJ backend is used for workspace management
  - SQLite checkpoints track batch transaction state
  - Batch commands are self-contained workspace operations
- **Open questions**: None

## Preconditions
- [P1] `BatchId` must be unique and non-empty
- [P2] `commands` array must contain at least one valid `BatchCommand`
- [P3] Workspace must be in a `ready` state (not locked, not dirty pending conflict)
- [P4] All commands in batch must be syntactically valid (parseable)

## Postconditions
- [Q1] On SUCCESS: All commands executed sequentially, workspace state reflects all changes, new checkpoint created
- [Q2] On FAILURE: Workspace state identical to pre-batch (rolled back), all intermediate changes undone
- [Q3] On ROLLBACK_FAILURE: Error propagated to caller with `RollbackFailed` variant, workspace left in indeterminate state (logged)
- [Q4] `BatchResult` contains either `Committed` with all results, or `RolledBack` with original error

## Invariants
- [I1] A batch execution strictly enforces atomic "all-or-nothing" semantics
- [I2] No partial state visible to other operations during batch execution
- [I3] Only one batch can execute in a workspace at a time

## Error Taxonomy
```rust
pub enum BatchError {
    EmptyBatch,                      // No commands provided
    WorkspaceNotReady(String),        // Workspace locked or dirty
    CommandParseFailed(String),       // Command could not be parsed
    ExecutionFailed { cmd: String, underlying: String },  // Command execution failed
    RollbackFailed { checkpoint_id: String, underlying: String },  // Critical: rollback failed
    CommitFailed { checkpoint_id: String, underlying: String },   // Failed to finalize
    BatchInProgress,                  // Another batch is running
}
```

## Contract Signatures
```rust
pub trait BatchExecutor {
    async fn execute_batch(
        &self,
        workspace_id: &WorkspaceId,
        commands: Vec<BatchCommand>,
    ) -> Result<BatchResult, BatchError>;
}

pub enum BatchResult {
    Committed { checkpoint_id: String, results: Vec<CommandResult> },
    RolledBack { error: BatchError },
}
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| P1: BatchId non-empty | Compile-time | `NonEmptyString` newtype |
| P2: commands non-empty | Runtime-checked constructor | `BatchCommands::new() -> Result` |
| P3: workspace ready | Runtime-checked | `Workspace::validate_ready()` |
| P4: valid commands | Runtime-checked parser | `BatchCommand::parse()` |
| I3: single batch | Runtime-lock | `Mutex<Option<BatchInProgress>>` |

## Violation Examples (REQUIRED)

### VIOLATES P1 (Empty BatchId)
```rust
execute_batch(workspace_id, "", vec![...])
// Expected: Err(BatchError::EmptyBatch)
```

### VIOLATES P2 (No Commands)
```rust
execute_batch(workspace_id, "batch-1", vec![])
// Expected: Err(BatchError::EmptyBatch)
```

### VIOLATES Q2 (Rollback after failure)
```rust
// Given: Batch execution fails at step 3
// When: Rollback to checkpoint fails due to corrupted state
// Then: Err(BatchError::RollbackFailed { checkpoint_id: "auto-123", underlying: "..." })
// NOT: Silent ignore, NOT: panic
```

### VIOLATES Q3 (Silent rollback failure)
```rust
// Given: Rollback fails
// When: System silently ignores
// Then: VIOLATION - system MUST propagate RollbackFailed error
```

### VIOLATES I3 (Concurrent batch)
```rust
// Given: Batch A is executing
// When: Batch B starts
// Then: Err(BatchError::BatchInProgress)
```

## Ownership Contracts
- `BatchCommand { name: String, args: Vec<String> }` - Owned, cloneable
- `BatchResult` - Owned, no shared mutation
- `CheckpointGuard` - Exclusive borrow, lifecycle-managed
- `BatchExecutor` trait - `&self` borrows, stateless operations

## Non-goals
- [ ] Batch commands spanning multiple workspaces (not atomic cross-workspace)
- [ ] Partial rollback with some commands committed (purest atomic only)
- [ ] Compensating transactions (strict rollback only)

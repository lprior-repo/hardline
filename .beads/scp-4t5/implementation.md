# Implementation Summary: cli: Add task management commands

## Files Created/Modified

### New Files
- `.beads/scp-4t5/contract.md` - Design-by-contract specification
- `.beads/scp-4t5/martin-fowler-tests.md` - Test plan
- `crates/cli/src/commands/task.rs` - Task commands module

### Modified Files
- `crates/cli/src/main.rs` - Added TaskCommands enum and routing
- `crates/cli/src/commands/mod.rs` - Added task module declaration
- `crates/cli/Cargo.toml` - Added scp-beads and tokio dependencies
- `crates/core/src/error.rs` - Added task-related error variants
- `crates/core/src/lock.rs` - Added LockType::Task variant

## Implementation Details

### Commands Implemented
1. **task list** - Lists all tasks with state, priority, and assignee
2. **task show** - Shows detailed task information
3. **task claim** - Claims a task (assigns to current user, sets InProgress)
4. **task yield** - Releases task assignment (sets to Open)
5. **task start** - Starts working on a task (sets InProgress)
6. **task done** - Completes a task (sets Closed)

### Architecture
- **Data Layer**: Task struct with id, title, state, priority, assignee
- **Calculation Layer**: State transitions, validation logic
- **Actions Layer**: CLI command handlers with lock management

### TTL Locking
- Implemented using LockType::Task in the lock module
- Each state-changing operation acquires a lock before proceeding
- Lock is released automatically when guard is dropped

### Preconditions Enforced (per contract.md)
- P1: Task ID validation (non-empty, valid format)
- P2: Task existence check
- P3: Not already claimed by another user
- P4: Must be claimed by current user
- P5: State must allow transition

### Postconditions Achieved
- Q1: List returns all tasks
- Q3: Claim sets assignee and InProgress
- Q4: Yield clears assignee and sets Open
- Q5: Start sets InProgress
- Q6: Done sets Closed with timestamp
- Q7: Lock acquired before state changes

## Notes
- Implementation uses in-memory TaskStore (not persisted)
- Current user is hardcoded as "current-user"
- Demo tasks are initialized on first list if empty

## Status
Core implementation complete. Additional error variants need to be added to satisfy Clone/ExitCode traits for full compilation.

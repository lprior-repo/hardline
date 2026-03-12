# Contract Specification: cli: Add task management commands

## Context
- **Feature**: Add task management commands to SCP CLI
- **Bead ID**: scp-4t5
- **Domain terms**:
  - Task = Bead in the beads domain
  - TTL = Time-To-Live for locks
  - Claim = Assign task to current user
  - Yield = Release assignment
  - Start = Transition task to InProgress
  - Done = Transition task to Closed
- **Assumptions**:
  - Task operations use in-memory bead repository
  - TTL locking uses MemLockManager with TTL support
  - Current user is identified by CLI context
- **Open questions**:
  - What is the default TTL for locks?
  - What is the format for task IDs?

## Preconditions
- [P1] Task ID must be valid (non-empty, alphanumeric with - or _)
- [P2] Task must exist for show/claim/yield/start/done operations
- [P3] Task must not be claimed by another user for claim operation
- [P4] Task must be claimed by current user for yield/start/done operations
- [P5] Task state must allow transition (not already Closed for start/done)

## Postconditions
- [Q1] List returns all tasks with their current state and assignee
- [Q2] Show returns complete task details including state, assignee, priority
- [Q3] Claim sets assignee to current user, state to InProgress
- [Q4] Yield clears assignee, state to Open
- [Q5] Start sets state to InProgress, preserves assignee
- [Q6] Done sets state to Closed with closed_at timestamp
- [Q7] Lock is acquired before state-changing operations, released after

## Invariants
- [I1] Task ID format must match BeadId validation rules
- [I2] State transitions must be valid per BeadState rules
- [I3] Lock must be released even on error

## Error Taxonomy
- Error::InvalidInput - when task ID is malformed
- Error::NotFound - when task does not exist
- Error::TaskNotFound - alias for NotFound for task-specific errors
- Error::TaskAlreadyClaimed - when task is claimed by another user
- Error::TaskNotClaimed - when task is not claimed by current user
- Error::InvalidStateTransition - when state transition is not allowed
- Error::TaskLocked - when TTL lock cannot be acquired

## Contract Signatures
```rust
// CLI command handlers
fn task_list() -> Result<()>
fn task_show(task_id: &str) -> Result<()>
fn task_claim(task_id: &str) -> Result<()>
fn task_yield(task_id: &str) -> Result<()>
fn task_start(task_id: &str) -> Result<()>
fn task_done(task_id: &str) -> Result<()>

// Domain service functions
fn list_tasks(repo: &dyn BeadRepository) -> Result<Vec<Bead>>
fn get_task(repo: &dyn BeadRepository, id: &BeadId) -> Result<Bead>
fn claim_task(repo: &mut dyn BeadRepository, lock: &dyn LockManager, id: &BeadId, holder: &str) -> Result<Bead>
fn yield_task(repo: &mut dyn BeadRepository, lock: &dyn LockManager, id: &BeadId, holder: &str) -> Result<Bead>
fn start_task(repo: &mut dyn BeadRepository, lock: &dyn LockManager, id: &BeadId, holder: &str) -> Result<Bead>
fn complete_task(repo: &mut dyn BeadRepository, lock: &dyn LockManager, id: &BeadId, holder: &str) -> Result<Bead>
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| task_id non-empty | Compile-time | `BeadId::new() -> Result<BeadId>` |
| task exists | Runtime-checked | `Result<T, Error::NotFound>` |
| task not claimed by other | Runtime-checked | `Result<T, Error::TaskAlreadyClaimed>` |
| task claimed by current user | Runtime-checked | `Result<T, Error::TaskNotClaimed>` |
| valid state transition | Runtime-checked | `Result<T, Error::InvalidStateTransition>` |
| lock acquired | Runtime-checked | `Result<LockGuard, Error::TaskLocked>` |

## Violation Examples (REQUIRED)
- VIOLATES P1: `BeadId::new("")` -- should produce `Err(BeadError::InvalidId("ID cannot be empty"))`
- VIOLATES P1: `BeadId::new("bad id!")` -- should produce `Err(BeadError::InvalidId(...))`
- VIOLATES P2: `get_task(&repo, &BeadId::new("nonexistent").unwrap())` -- should produce `Err(Error::NotFound)`
- VIOLATES P3: `claim_task(&mut repo, &lock, &id, "user2")` when claimed by "user1" -- should produce `Err(Error::TaskAlreadyClaimed)`
- VIOLATES P4: `yield_task(&mut repo, &lock, &id, "user1")` when not claimed -- should produce `Err(Error::TaskNotClaimed)`
- VIOLATES P5: `start_task(&mut repo, &lock, &id, "user1")` when already Closed -- should produce `Err(Error::InvalidStateTransition)`

## Ownership Contracts (Rust-specific)
- `repo: &dyn BeadRepository` - shared borrow, read-only for list/show
- `repo: &mut dyn BeadRepository` - exclusive borrow for state changes, mutates state and assignee fields
- `lock: &dyn LockManager` - shared borrow for lock acquisition, releases on drop
- `holder: &str` - borrowed string, represents current user identity

## Non-goals
- [ ] Persistent storage (in-memory only)
- [ ] Real-time TTL expiration (mock/placeholder)
- [ ] Concurrent distributed locking
- [ ] Task creation/deletion commands

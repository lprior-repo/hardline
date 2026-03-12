# Contract Specification

## Context
- **Feature**: cli: Add task management commands
- **Bead ID**: scp-4t5
- **Domain terms**: Task, TaskState, TaskLock, TTL, Claim, Assignee
- **Assumptions**: 
  - Tasks are stored in memory (per current implementation)
  - LockManager trait exists and provides acquire/release
  - TTL locking needs to be implemented (current lock has no TTL)
- **Open questions**: 
  - Should tasks be persisted? (assumed: no, in-memory for this bead)
  - What is the default TTL? (assumed: 5 minutes)
  - Is this for single-process or multi-agent? (assumed: single-process)

## Preconditions

### P1: Task ID Validation
- Task ID must be non-empty string
- Task ID must exist in task store for operations that require it (show, claim, yield_task, start, done)

### P2: Lock Acquisition
- Lock must be acquirable for the task (not already held by another holder)
- For claim: task can be unclaimed OR claimed by current user
- For yield_task/start/done: task must be claimed by current user

### P3: State Transition Validity
- `start` requires task to be in Open or InProgress state and claimed by current user
- `done` requires task to be in Open or InProgress state and claimed by current user

## Postconditions

### Q1: Task State Transitions
- `claim`: state becomes InProgress, assignee set to current user
- `yield_task`: state becomes Open, assignee cleared
- `start`: state becomes InProgress (must already have assignee)
- `done`: state becomes Closed { closed_at: Utc::now() }

### Q2: Timestamp Updates
- `updated_at` must be set to Utc::now() on any state transition

### Q3: Lock Management
- Lock is acquired at start of operation
- Lock is released when LockGuard drops (end of operation)
- Lock has TTL (expires after configured duration)

### Q4: Task ID Format Enforcement
- Task IDs follow pattern: `task-XXX` (e.g., task-001)
- Enforcement level: Runtime check via Result error

## Invariants

### I1: Task ID Uniqueness
- No two tasks can have the same ID in the store

### I2: State Consistency
- If assignee is Some, state must be InProgress
- If state is Closed, assignee must be Some

### I3: Lock Consistency
- A task can only be locked by one holder at a time
- Lock holder must match task assignee for modifying operations

## Error Taxonomy

### Existing Errors (already in error.rs)
- `Error::TaskNotFound(String)` - Task ID not found in store
- `Error::TaskAlreadyClaimed(String, String)` - Task already claimed by another user
- `Error::TaskNotClaimed(String)` - Task not claimed by current user
- `Error::TaskLocked(String)` - Task lock acquisition failed
- `Error::InvalidTaskId(String)` - Task ID format validation failed
- `Error::InvalidTaskStateTransition(String, String)` - Invalid state transition

### New Errors Needed
- `Error::LockTtlExpired(String)` - Lock TTL has expired (if implementing TTL locking)

## Contract Signatures

```rust
// Task management functions
pub fn list() -> Result<()>
pub fn show(task_id: &str) -> Result<()>
pub fn claim(task_id: &str) -> Result<()>
pub fn yield_task(task_id: &str) -> Result<()>
pub fn start(task_id: &str) -> Result<()>
pub fn done(task_id: &str) -> Result<()>

// Supporting functions (for TTL locking)
fn validate_task_id(task_id: &str) -> Result<()>
fn acquire_task_lock(task_id: &str, holder: &str, ttl: Duration) -> Result<LockGuard>
fn transition_task_state(task_id: &str, new_state: TaskState) -> Result<()>
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| task_id non-empty | Runtime-checked constructor | `validate_task_id() -> Result<()>` |
| task_id exists | Runtime check | `store.get()` returns Option |
| task not locked | Runtime check | `LockManager::acquire()` returns Result |
| task claimed by current user | Runtime check | Compare assignee field |
| state allows transition | Runtime check | Match on TaskState enum |

## Violation Examples

### VIOLATES P1 (empty task_id)
```rust
show("")
// Expected: Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()))
```

### VIOLATES P1 (task not found)
```rust
show("nonexistent-task")
// Expected: Err(Error::TaskNotFound("nonexistent-task".to_string()))
```

### VIOLATES P2 (lock held by another)
```rust
// Task "task-001" is locked by "user-a"
claim("task-001")  // called by "user-b"
// Expected: Err(Error::TaskLocked("task-001".to_string()))
```

### VIOLATES P2 (task claimed by another)
```rust
// Task "task-001" is claimed by "user-a"
claim("task-001")  // called by "user-b"
// Expected: Err(Error::TaskAlreadyClaimed("task-001".to_string(), "user-a".to_string()))
```

### VIOLATES P3 (not claimed by current user)
```rust
// Task "task-001" is not claimed
start("task-001")  // current user is "user-b"
// Expected: Err(Error::TaskNotClaimed("task-001".to_string()))
```

### VIOLATES P3 (already closed)
```rust
// Task "task-001" is in Closed state
done("task-001")
// Expected: Err(Error::InvalidTaskStateTransition("task-001".to_string(), "Task is already closed".to_string()))
```

### VIOLATES Q1 (state transition failure)
```rust
// Task "task-001" in Open state, no assignee
done("task-001")
// Expected: Err(Error::TaskNotClaimed("task-001".to_string()))
```

## Ownership Contracts

### TaskStore (in-memory storage)
- Uses `RwLock<HashMap<String, Task>>` for interior mutability
- All operations return `Result<()>` - zero panics
- `list()`: shared borrow, returns cloned Vec<Task>
- `get()`: shared borrow, returns Option<Task>
- `update()`: exclusive borrow, requires task exists
- `insert()`: exclusive borrow, requires task doesn't exist

### LockManager
- Trait provides `acquire()`, `try_acquire()`, `release()`
- Returns `Result<LockGuard>` - lock guard released on drop
- For TTL: requires `acquire_with_ttl(lock, holder, ttl)` method

## Non-goals
- [ ] Persistent task storage (in-memory only)
- [ ] Multi-machine/distributed locking
- [ ] Task CRUD (create, delete) - only read and state transition
- [ ] Priority queue ordering

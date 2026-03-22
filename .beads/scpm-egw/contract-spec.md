# Contract Specification: Task Claim and Yield Commands

## Context

**Feature:** CLI task claim and yield commands  
**Domain Terms:** Task, Agent, Claim, TTL Lock, Exclusive Ownership  
**Assumptions:**
- Tasks are stored in a persistent JSON file store
- Locks are managed via LockManager trait with TTL support
- Each agent has a unique identifier (user string)
- TTL (Time-To-Live) ensures locks are automatically released if agent dies

**Open Questions:**
- Should TTL be configurable? (Assumed: yes, via DEFAULT_TTL_SECS)
- What happens if agent claims task but never yields? (Assumed: lock auto-expires after TTL)

---

## Preconditions

| ID | Description | Enforcement Level | Type/Pattern |
|----|-------------|-------------------|--------------|
| P1 | Task ID must be valid (non-empty, alphanumeric with - or _) | Compile-time | `TaskId::new() -> Result<Self, Error>` |
| P2 | Task must exist in the system | Runtime-checked | `validate_task_exists()` returns `Result<Task, Error::TaskNotFound>` |
| P3 | Task must not be claimed by another agent | Runtime-checked | `validate_not_claimed_by_other()` returns `Result<(), Error::TaskAlreadyClaimed>` |
| P4 | Agent must hold the claim to yield | Runtime-checked | `validate_claimed_by_user()` returns `Result<(), Error::TaskNotClaimed>` |

---

## Postconditions

| ID | Description |
|----|-------------|
| Q1 | `task claim` grants TTL lock on the task |
| Q2 | `task claim` sets assignee to current agent |
| Q3 | `task claim` transitions task state to `InProgress` |
| Q4 | `task yield` releases TTL lock on the task |
| Q5 | `task yield` clears assignee (sets to None) |
| Q6 | `task yield` transitions task state to `Open` |

---

## Invariants

| ID | Description |
|----|-------------|
| I1 | A task can only be claimed by ONE agent at a time |
| I2 | Only the claiming agent can yield the task |
| I3 | Lock TTL prevents indefinite blocking (300 seconds default) |
| I4 | Task state transitions are valid (Open → InProgress → Closed) |

---

## Error Taxonomy

| Error Variant | When Triggered | Exit Code |
|---------------|-----------------|-----------|
| `Error::TaskNotFound(id)` | Task with given ID does not exist | 60 |
| `Error::TaskAlreadyClaimed(id, holder)` | Another agent already holds the claim | 61 |
| `Error::TaskNotClaimed(id)` | Agent attempts to yield unclaimed task | 62 |
| `Error::TaskLocked(id)` | Lock acquisition fails | 63 |
| `Error::InvalidTaskId(msg)` | Task ID validation fails | 64 |
| `Error::InvalidTaskStateTransition(id, msg)` | Invalid state transition attempted | 65 |

---

## Contract Signatures

### Core Functions

```rust
// CLI Commands (in task.rs)
pub fn claim(task_id: &str, user: &str) -> CoreResult<()>
pub fn yield_task(task_id: &str, user: &str) -> CoreResult<()>

// Validation (in task_validation.rs)
pub fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task>
pub fn validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()>
pub fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()>
pub fn validate_not_closed(task: &Task) -> CoreResult<()>
pub fn acquire_task_lock(lock: &dyn LockManager, task_id: &str, holder: &str) -> CoreResult<LockGuard>

// State Transitions (pure functions)
pub fn transition_to_claimed(task: Task, user: &str) -> Task
pub fn transition_to_yielded(task: Task) -> Task
pub fn transition_to_started(task: Task) -> Task
pub fn transition_to_done(task: Task) -> Task
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| P1: Valid TaskId | Compile-time (strongest) | `TaskId::new(String) -> Result<Self, Error>` - validates regex `^[a-zA-Z0-9_-]+$` |
| P2: Task exists | Runtime-checked constructor | `validate_task_exists(Option<Task>) -> Result<Task, Error>` |
| P3: Not claimed by other | Runtime-checked | `validate_not_claimed_by_other(&Task, &str) -> Result<()>` |
| P4: Claimed by user | Runtime-checked | `validate_claimed_by_user(&Task, &str) -> Result<()>` |
| Lock acquisition | Runtime-checked | `LockManager::acquire(LockType, &str) -> Result<LockGuard>` |

---

## Violation Examples (REQUIRED)

### VIOLATES P1: Empty Task ID
```rust
// Call: TaskId::new("")
// Expected: Err(Error::InvalidTaskId("Task ID cannot be empty"))
```

### VIOLATES P1: Malformed Task ID
```rust
// Call: TaskId::new("bad id!")
// Expected: Err(Error::InvalidTaskId("Task ID must be alphanumeric with - or _"))
```

### VIOLATES P2: Task Not Found
```rust
// Call: validate_task_exists(None, "nonexistent")
// Expected: Err(Error::TaskNotFound("nonexistent"))
```

### VIOLATES P3: Already Claimed By Other
```rust
// Given: Task claimed by "agent-a"
// Call: validate_not_claimed_by_other(&task, "agent-b")
// Expected: Err(Error::TaskAlreadyClaimed("task-001", "agent-a"))
```

### VIOLATES P4: Not Claimed By User
```rust
// Given: Task with no assignee (Open state)
// Call: validate_claimed_by_user(&task, "agent-b")
// Expected: Err(Error::TaskNotClaimed("task-001"))
```

### VIOLATES Q2/Q3: Claim Postconditions
```rust
// Given: Open task with no assignee
// Call: transition_to_claimed(task, "agent-a")
// Expected post-state:
//   - assignee == Some(Assignee("agent-a"))
//   - state == InProgress
```

### VIOLATES Q5/Q6: Yield Postconditions
```rust
// Given: Task with assignee="agent-a", state=InProgress
// Call: transition_to_yielded(task)
// Expected post-state:
//   - assignee == None
//   - state == Open
```

---

## Ownership Contracts

### LockManager Trait
- `acquire(lock: LockType, holder: &str) -> Result<LockGuard>`: Acquires exclusive lock
- Lock is automatically released when `LockGuard` is dropped
- TTL-based locks auto-expire after DEFAULT_TTL_SECS (300 seconds)

### Task Transitions
- `transition_to_claimed(task, user)`: Creates NEW Task instance with updated assignee/state
- `transition_to_yielded(task)`: Creates NEW Task instance (immutable, no mutation)
- All transitions are pure functions returning new instances

### Mutability Contract
- No `let mut` in source code - use persistent data structures
- `TaskStore` uses `RwLock` for interior mutability at shell boundary only
- Domain logic (validation, transitions) is pure and immutable

---

## Non-goals

- Task creation (only claim/yield for existing tasks)
- Task deletion
- Task assignment by administrators (only self-claim)
- Persistent lock storage (in-memory only for single-process)
- Distributed locking (would require coordination service)

---

## TTL Lock Specification

The lock system uses TTL (Time-To-Live) to prevent indefinite blocking:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| DEFAULT_TTL_SECS | 300 (5 min) | Balance between responsiveness and fault tolerance |
| Lock Type | Task(String) | Task-specific locking for granular control |
| Automatic Release | On Drop | LockGuard releases lock when dropped |
| TTL Extension | Heartbeat | Agent can extend lock via heartbeat |

---

## State Machine

```
    ┌──────────────────────────────────────────────────────────────┐
    │                                                              │
    ▼                                                              │
  OPEN ──────► INPROGRESS ──────► CLOSED                           │
    ▲              │                                                 │
    │              │                                                 │
    │              │ yield                                           │
    │              ▼                                                 │
    └──────────────────────────────────────────────────────────────┘

claim: Open → InProgress (sets assignee, acquires lock)
yield: InProgress → Open (clears assignee, releases lock)
start: Open → InProgress (if already claimed)
done: InProgress → Closed (with timestamp)
```

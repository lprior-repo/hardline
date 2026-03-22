# Contract Specification: task start and done commands

## Context

- **Feature:** CLI task lifecycle management - start and done commands
- **Domain terms:** Task, TaskId, TaskState, InProgress, Closed, Agent, Claim
- **Assumptions:** Tasks exist in a task store, users are identified by strings
- **Open questions:** None - implementation exists and follows existing patterns

## Scope Map (What/Where)

| Component | Location | Responsibility |
|-----------|----------|----------------|
| CLI Commands | `crates/cli/src/commands/task.rs` | `start()` and `done()` entry points |
| Validation | `crates/cli/src/commands/task_validation.rs` | Precondition checks, state transitions |
| Types | `crates/cli/src/commands/task_types.rs` | Task, TaskState, TaskId, etc. |

## Contract Clauses

### Preconditions

| ID | Clause | Enforcement Level | Type/Pattern |
|----|--------|-------------------|--------------|
| P1 | TaskId must be non-empty and alphanumeric (with - or _) | Runtime-checked constructor | `TaskId::new() -> Result<Self, Error>` |
| P2 | Task must exist in store | Runtime-checked | `validate_task_exists() -> CoreResult<Task>` |
| P3 | Task must be claimed by the executing user | Runtime-checked | `validate_claimed_by_user() -> CoreResult<()>` |
| P4 | For `done`: Task must not already be closed | Runtime-checked | `validate_not_closed() -> CoreResult<()>` |

### Postconditions

| ID | Clause | Enforcement |
|----|--------|-------------|
| Q1 | `start`: Task state transitions to `InProgress` | `transition_to_started()` returns `Task { state: TaskState::InProgress, ... }` |
| Q2 | `start`: Task assignee remains unchanged | `transition_to_started()` preserves `assignee` field |
| Q3 | `start`: `updated_at` is set to current time | `transition_to_started()` sets `updated_at: chrono::Utc::now()` |
| Q4 | `done`: Task state transitions to `Closed { closed_at }` | `transition_to_done()` returns `Task { state: TaskState::Closed { closed_at }, ... }` |
| Q5 | `done`: `updated_at` is set to current time | `transition_to_done()` sets `updated_at: chrono::Utc::now()` |

### Invariants

| ID | Clause |
|----|--------|
| I1 | TaskId format: alphanumeric with - or _ only |
| I2 | Task state machine: Open -> InProgress -> Closed (no backward transitions) |
| I3 | Only the claiming agent can transition a task |
| I4 | Closed is a terminal state - no transitions out |

## Error Taxonomy

| Error | When Returned |
|-------|---------------|
| `Error::InvalidTaskId` | TaskId creation fails validation |
| `Error::TaskNotFound` | Task does not exist in store |
| `Error::TaskNotClaimed` | Task not claimed by current user |
| `Error::InvalidTaskStateTransition` | Task already closed (for done) |
| `Error::TaskLocked` | Could not acquire task lock |

## Contract Signatures

```rust
// crates/cli/src/commands/task.rs
pub fn start(task_id: &str, user: &str) -> CoreResult<()>
pub fn done(task_id: &str, user: &str) -> CoreResult<()>

// crates/cli/src/commands/task_validation.rs
pub fn validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task>
pub fn validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()>
pub fn validate_not_closed(task: &Task) -> CoreResult<()>
pub fn transition_to_started(task: Task) -> Task
pub fn transition_to_done(task: Task) -> Task
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| TaskId non-empty | Compile-time via constructor | `TaskId::new() -> Result<Self, Error>` |
| TaskId alphanumeric | Compile-time via regex | `TASK_ID_PATTERN.is_match()` |
| Task exists | Runtime via Option | `Option<Task>` + `validate_task_exists()` |
| Task claimed by user | Runtime via validation | `validate_claimed_by_user()` |
| Task not closed | Runtime via validation | `validate_not_closed()` |

## Violation Examples (REQUIRED)

- VIOLATES P1: `TaskId::new("")` -- produces `Err(Error::InvalidTaskId("Task ID cannot be empty"))`
- VIOLATES P1: `TaskId::new("bad id!")` -- produces `Err(Error::InvalidTaskId(...))`
- VIOLATES P2: `validate_task_exists(None, "nonexistent")` -- produces `Err(Error::TaskNotFound("nonexistent"))`
- VIOLATES P3: `validate_claimed_by_user(&unclaimed_task, "user")` -- produces `Err(Error::TaskNotClaimed(...))`
- VIOLATES P4: `validate_not_closed(&closed_task)` -- produces `Err(Error::InvalidTaskStateTransition(...))`

## Violation Test Parity

| Violation | Test Name |
|-----------|-----------|
| VIOLATES P1 (empty) | `test_precondition_p1_empty_id_rejected_at_type_level` |
| VIOLATES P1 (malformed) | `test_precondition_p1_malformed_id_rejected_at_type_level` |
| VIOLATES P2 | `test_precondition_p2_nonexistent_task_returns_not_found` |
| VIOLATES P3 | `test_precondition_p4_must_be_claimed_before_yield` |
| VIOLATES P4 | `test_task_start_returns_error_for_closed_task` |

## Ownership Contracts

| Function | Ownership | Mutation Contract |
|----------|-----------|-------------------|
| `start(task_id, user)` | `task_id` borrowed, `user` borrowed | No mutation - returns `CoreResult<()>` |
| `done(task_id, user)` | `task_id` borrowed, `user` borrowed | No mutation - returns `CoreResult<()>` |
| `transition_to_started(task)` | `task` consumed, new Task returned | `state`, `updated_at` fields change |
| `transition_to_done(task)` | `task` consumed, new Task returned | `state`, `updated_at` fields change |
| `TaskStore::update(task)` | `task` cloned, stored | `tasks` RwLock write |

## Non-goals

- Task creation (separate feature)
- Task deletion (not in scope)
- Task assignment to different user during start/done
- Persistence to database (uses JSON file store)

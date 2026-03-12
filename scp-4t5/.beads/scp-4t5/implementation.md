# Implementation Summary: CLI Task Management Commands

## Bead: scp-4t5
## Feature: cli: Add task management commands with TTL locking

## Changes Made

### 1. Task Store (`crates/cli/src/commands/task.rs`)

Refactored `TaskStore` to eliminate `.unwrap()` calls:

**Before:**
```rust
fn list(&self) -> Vec<Task> {
    let tasks = self.tasks.read().unwrap();
    tasks.values().cloned().collect()
}

fn get(&self, id: &str) -> Option<Task> {
    let tasks = self.tasks.read().unwrap();
    tasks.get(id).cloned()
}

fn update(&self, task: Task) -> CoreResult<()> {
    let mut tasks = self.tasks.write().unwrap();
    // ...
}

fn insert(&self, task: Task) -> CoreResult<()> {
    let mut tasks = self.tasks.write().unwrap();
    // ...
}
```

**After:**
```rust
fn list(&self) -> Vec<Task> {
    self.tasks
        .read()
        .map(|tasks| tasks.values().cloned().collect())
        .unwrap_or_default()
}

fn get(&self, id: &str) -> Option<Task> {
    self.tasks
        .read()
        .ok()
        .and_then(|tasks| tasks.get(id).cloned())
}

fn update(&self, task: Task) -> CoreResult<()> {
    let mut tasks = self.tasks.write().map_err(|e| Error::Internal(e.to_string()))?;
    // ...
}

fn insert(&self, task: Task) -> CoreResult<()> {
    let mut tasks = self.tasks.write().map_err(|e| Error::Internal(e.to_string()))?;
    // ...
}
```

### 2. Demo Task Initialization

Changed `init_demo_tasks` to return `CoreResult<()>` instead of silently ignoring errors.

### 3. Function Size Refactoring (Black Hat Fix)

Extracted pure validation and transition functions to reduce command function size:

**Extracted functions:**
- `validate_task_id(task_id: &str) -> CoreResult<()>` - validates non-empty task ID
- `validate_task_exists(task: Option<Task>, task_id: &str) -> CoreResult<Task>` - returns task or error
- `validate_not_claimed_by_other(task: &Task, current_user: &str) -> CoreResult<()>` - checks ownership
- `validate_claimed_by_user(task: &Task, current_user: &str) -> CoreResult<()>` - checks claim
- `validate_not_closed(task: &Task) -> CoreResult<()>` - checks not already closed
- `acquire_task_lock(lock: &dyn LockManager, task_id: &str, holder: &str) -> CoreResult<LockGuard>` - acquires lock
- `transition_to_claimed(task: Task, user: &str) -> Task` - state transition
- `transition_to_yielded(task: Task) -> Task` - state transition
- `transition_to_started(task: Task) -> Task` - state transition
- `transition_to_done(task: Task) -> Task` - state transition

**Command function sizes after refactoring:**
| Function | Lines |
|----------|-------|
| `claim()` | 14 |
| `yield_task()` | 13 |
| `start()` | 14 |
| `done()` | 14 |

All functions are now ≤ 25 lines (was 35-44 lines).

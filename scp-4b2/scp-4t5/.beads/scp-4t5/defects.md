# Black Hat Review Defects

## Bead: scp-4t5

## Status: REJECTED

### Defect D1: Function Size Exceeds 25 Lines

**Location**: `crates/cli/src/commands/task.rs`

| Function | Lines | Limit |
|----------|-------|-------|
| `claim()` | 40 | 25 |
| `done()` | 44 | 25 |
| `yield_task()` | 35 | 25 |
| `start()` | 34 | 25 |

**Required Fix**: Extract validation logic and lock acquisition into separate pure functions to reduce each command to ≤25 lines.

Example refactor for `claim()`:
```rust
// Pure validation function (moves 10 lines out)
fn validate_claim(task_id: &str) -> CoreResult<()> {
    if task_id.is_empty() {
        return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
    }
    Ok(())
}

// Pure lock acquisition (moves 5 lines out)  
fn acquire_task_lock(lock: &dyn LockManager, task_id: &str, holder: &str) -> CoreResult<LockGuard> {
    let lock_type = LockType::Task(task_id.to_string());
    lock.acquire(lock_type, holder)
        .map_err(|_| Error::TaskLocked(task_id.to_string()))
}

// Command function now ~20 lines
pub fn claim(task_id: &str) -> CoreResult<()> {
    validate_claim(task_id)?;
    let store = get_task_store();
    let lock = get_lock_manager();
    let _guard = acquire_task_lock(&*lock, task_id, "current-user")?;
    
    let mut task = store.get(task_id)
        .ok_or_else(|| Error::TaskNotFound(task_id.to_string()))?;
    
    if let Some(assignee) = &task.assignee {
        if assignee != "current-user" {
            return Err(Error::TaskAlreadyClaimed(task_id.to_string(), assignee.clone()));
        }
    }
    
    task.assignee = Some("current-user".to_string());
    task.state = TaskState::InProgress;
    task.updated_at = chrono::Utc::now();
    
    store.update(task)?;
    println!("Task {} claimed", task_id);
    Ok(())
}
```

### Defect D2: Mixed I/O (Functional Core / Imperative Shell Violation)

**Location**: All command functions in `crates/cli/src/commands/task.rs`

**Issue**: Functions contain both business logic and `println!` I/O.

**Required Fix**: Separate pure logic from I/O:

```rust
// Pure function - no I/O
fn transition_to_in_progress(task: Task, user: &str) -> Task {
    let mut t = task;
    t.assignee = Some(user.to_string());
    t.state = TaskState::InProgress;
    t.updated_at = chrono::Utc::now();
    t
}

// Imperative shell - I/O only
pub fn claim(task_id: &str) -> CoreResult<()> {
    // ... validate and acquire lock ...
    let task = store.get(task_id)?;
    let updated_task = transition_to_in_progress(task, "current-user");
    store.update(updated_task)?;
    println!("Task {} claimed", task_id);  // I/O at boundary
    Ok(())
}
```

### Defect D3: Primitive Obsession

**Location**: `Task` struct fields

**Issue**: Uses `String` for id, title instead of newtype wrappers.

**Required Fix** (optional, lower priority):
```rust
// Newtypes for type safety
struct TaskId(String);
struct TaskTitle(String);
```

## Resolution Required

Fix D1 (function size) is mandatory. D2 (I/O separation) and D3 (newtypes) are recommended but not blocking if function size is addressed.

**Total violations**: 1 mandatory

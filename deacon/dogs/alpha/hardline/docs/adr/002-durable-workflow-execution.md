# ADR-002: Durable Workflow Execution

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs durable workflow execution for:
1. Multi-step operations that survive crashes (like Restate/Temporal)
2. Saga pattern with automatic compensation on failure
3. Workspace isolation operations (spawn, switch, sync) that are resumable
4. Queue processing that can recover from mid-processing failures

This ADR documents the implementation approach based on verified patterns from hardline (prior codebase) and seshat.

---

## 1. Core Concept: The Step Journal

### The Problem

When executing multi-step operations, failures can occur at any point:
- SIGKILL during workspace creation
- Database corruption mid-transaction
- Network failure during sync
- Process crash during commit

### The Solution: Step Journal

Every durable operation maintains a **journal** of steps. On restart, the system replays the journal, skipping completed steps.

```
┌─────────────────────────────────────────────────────┐
│ STEP JOURNAL (Append-only)                          │
│ 1. StepRecord { name: "create-db-record", status: completed }
│ 2. StepRecord { name: "create-workspace", status: completed }
│ 3. StepRecord { name: "update-metadata", status: running }
│ 4. StepRecord { name: "create-git-workspace", status: pending }
└─────────────────────────────────────────────────────┘
```

### Journal States

From `seshat/durable_types.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,  // Skipped due to earlier failure (compensation)
}
```

---

## 2. Operation State Machine

### States

From `seshat/durable_types.rs` and `orchestrator/state.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Started,      // Operation created, not yet running
    InProgress,   // Currently executing
    Completed,    // Successfully finished
    Failed,       // Permanently failed (no more retries)
}
```

### hardline Operation States

For hardline specifically:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Operation created, waiting to start
    Pending,
    /// Currently executing steps
    Running,
    /// All steps completed successfully
    Completed,
    /// Failed with error (may have partial compensation)
    Failed,
    /// Waiting on external input (promise/awakeable)
    Suspended,
    /// Compensation in progress (rolling back)
    Compensating,
}
```

---

## 3. Saga Pattern with Compensation

### The Problem

Multi-step operations may need to roll back if a later step fails.

Example: `workspace spawn`
1. Create DB record (succeeds)
2. Create workspace directory (fails)
3. NOW: Need to delete the DB record

### The Solution: Two-Phase Compensation

From `hardline prior codebase commands/add/atomic.rs`:

```rust
const JOURNAL_PENDING_EXTERNAL: &str = "pending_external";
const JOURNAL_COMPENSATING: &str = "compensating";
const JOURNAL_DONE: &str = "done";
const JOURNAL_FAILED_COMPENSATION: &str = "failed_compensation";
```

### Atomic Session Creation Pattern

```rust
pub(super) async fn atomic_create_session(
    name: &str,
    workspace_path: &std::path::Path,
    repo_root: &std::path::Path,
    db: &SessionDb,
    bead_metadata: Option<serde_json::Value>,
    create_command_id: Option<&str>,
) -> Result<()> {
    let operation_id = add_operation_id(name, create_command_id);

    // STEP 1: Create DB record FIRST with 'creating' status
    let session = db.create(name, &workspace_path_str).await?;

    // Journal: Mark as pending external work
    db.upsert_add_operation_journal(
        &operation_id,
        name,
        &workspace_path_str,
        create_command_id,
        JOURNAL_PENDING_EXTERNAL,
        None,
    ).await?;

    // STEP 2: Create Git workspace (can be interrupted by SIGKILL)
    let workspace_result = create_git_workspace(name, workspace_path, repo_root).await;

    match workspace_result {
        Ok(()) => {
            // Success path
            db.upsert_add_operation_journal(
                &operation_id, name, &workspace_path_str,
                create_command_id, JOURNAL_DONE, None
            ).await?;
            Ok(())
        }
        Err(workspace_error) => {
            // COMPENSATION PATH
            db.upsert_add_operation_journal(
                &operation_id, name, &workspace_path_str,
                create_command_id, JOURNAL_COMPENSATING,
                Some(&workspace_error.to_string())
            ).await?;

            // Roll back both sides
            rollback_workspace_state(name, workspace_path).await?;
            rollback_database_state(name, db, create_command_id).await?;

            Err(workspace_error)
        }
    }
}
```

### Compensation States

```rust
enum CompensationState {
    NoCompensationNeeded,      // Operation succeeded
    CompensationInProgress,    // Currently rolling back
    CompensationCompleted,     // Rollback succeeded
    CompensationFailed,         // Rollback failed (needs manual intervention)
}
```

---

## 4. Journal Structure

### OperationRecord

From `seshat/durable_types.rs`:

```rust
pub struct OperationRecord {
    pub operation_id: String,
    pub state: OperationState,
    pub current_step: u32,
    pub total_steps: u32,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub final_revision: Option<i64>,  // For optimistic concurrency
    pub error_message: Option<String>,
    pub author_id: String,
    pub description: String,
}
```

### StepRecord

```rust
pub struct StepRecord {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub status: StepStatus,
    pub event_revision: Option<i64>,  // DB revision at step time
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}
```

---

## 5. Recovery Procedures

### On Startup: Scan for Incomplete Operations

From `hardline prior codebase isolate-core/src/recovery.rs`:

```rust
pub async fn recover_incomplete_operations(db: &Database) -> Vec<RecoveryTask> {
    // 1. Find all operations not in terminal state
    let incomplete = sqlx::query_as!(
        OperationRecord,
        "SELECT * FROM operations WHERE state NOT IN ('completed', 'failed')"
    ).fetch_all(db.pool()).await?;

    // 2. For each incomplete, find last completed step
    for operation in incomplete {
        let last_step = sqlx::query_as!(
            StepRecord,
            "SELECT * FROM steps
             WHERE operation_id = $1
             ORDER BY step_index DESC LIMIT 1",
            operation.id
        ).fetch_one(db.pool()).await?;

        yield RecoveryTask {
            operation_id: operation.id,
            resume_from_step: last_step.step_index + 1,
        };
    }
}
```

### Recovery Policies

From `hardline prior codebase isolate-core/src/recovery.rs`:

```rust
pub enum RecoveryPolicy {
    /// Fail immediately on corruption (strict mode)
    FailFast,
    /// Log and attempt repair (warn + fix)
    Warn,
    /// Silently repair without logging (production)
    Silent,
}
```

### Recovery Scanner

```rust
pub async fn scan_and_recover(db: &Database, policy: RecoveryPolicy) -> Result<RecoveryReport> {
    // 1. Check database integrity
    validate_database(db.path(), policy).await?;

    // 2. Find incomplete operations
    let tasks = recover_incomplete_operations(db).await;

    // 3. For each incomplete, replay journal
    let mut recovered = Vec::new();
    for task in tasks {
        match replay_operation(db, task).await {
            Ok(()) => recovered.push(task.operation_id),
            Err(e) => {
                match policy {
                    RecoveryPolicy::FailFast => return Err(e),
                    RecoveryPolicy::Warn | RecoveryPolicy::Silent => {
                        // Log and continue
                    }
                }
            }
        }
    }

    Ok(RecoveryReport {
        total_incomplete: tasks.len(),
        recovered: recovered.len(),
        failed: tasks.len() - recovered.len(),
    })
}
```

---

## 6. Implementation in hardline

### Workspace Operations with Journaling

For hardline, the key operations that need durability:

| Operation | Steps | Compensation |
|-----------|-------|--------------|
| `workspace spawn` | 1. Create DB record, 2. Create workspace dir, 3. Init Git | Delete DB, remove dir |
| `workspace switch` | 1. Save current state, 2. Update refs, 3. Update working copy | Restore previous state |
| `sync pull` | 1. Fetch, 2. Rebase, 3. Update working copy | Abort rebase |
| `operation checkpoint` | 1. Serialize state, 2. Write checkpoint | Delete checkpoint |

### Domain Events

From `hardline prior codebase isolate-core/src/domain/events.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum DomainEvent {
    SessionCreated(Box<SessionCreatedEvent>),
    SessionCompleted(Box<SessionCompletedEvent>),
    SessionFailed(Box<SessionFailedEvent>),
    WorkspaceCreated(Box<WorkspaceCreatedEvent>),
    WorkspaceRemoved(Box<WorkspaceRemovedEvent>),
    BeadCreated(Box<BeadCreatedEvent>),
    BeadClosed(Box<BeadClosedEvent>),
}
```

---

## 7. Pipeline State Machine (Orchestrator)

From `orchestrator/src/state.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Pending,              // Initial - pipeline created but not started
    SpecReview,           // Running linter on spec
    UniverseSetup,        // Deploying twin/universe
    AgentDevelopment,     // Agent working (with iteration count)
    Validation,           // Running scenarios for validation
    Accepted,             // All scenarios passed - artifact ready
    Escalated,            // Human intervention needed
    Failed,               // Validation failed permanently
}
```

### State Transitions

```
Pending → SpecReview → UniverseSetup → AgentDevelopment
                                           ↓
                                      Validation
                                           ↓
                          ┌───────────────┼───────────────┐
                          ↓               ↓               ↓
                      Accepted        AgentDevelopment    Failed
                          (terminal)    (retry loop)     (terminal)
                          ↓
                      Escalated
                    (human needed)
```

### Transition Validation

```rust
impl Pipeline {
    pub fn transition_to(&mut self, new_state: PipelineState)
        -> Result<(), TransitionError>
    {
        // Validate transition is legal
        match (&self.state, &new_state) {
            (Pending, SpecReview) => {},
            (SpecReview, UniverseSetup) => {},
            // ... etc
            (state, _) if state.is_terminal() => {
                return Err(TransitionError::AlreadyTerminal { current: *state });
            }
            _ => {
                return Err(TransitionError::InvalidTransition {
                    from: self.state,
                    to: new_state,
                });
            }
        }
        self.state = new_state;
        Ok(())
    }
}
```

---

## 8. Journaling Strategy Summary

### Append-Only Journal

1. Every step write is journaled BEFORE execution
2. Step completion updates status in journal
3. On crash, scan journal to find last completed step
4. Resume from next pending step

### Idempotency

Each operation has a unique `operation_id`:
```rust
fn add_operation_id(name: &str, command_id: Option<&str>) -> String {
    command_id.map_or_else(
        || format!("add:{name}"),
        |id| format!("add:{name}:{id}")
    )
}
```

### Rollback Safety

From `atomic.rs`:
```rust
/// This function NEVER panics - all cleanup failures are logged
/// and handled gracefully. Partial state is always detectable.
pub(super) async fn rollback_partial_state(
    name: &str,
    workspace_path: &std::path::Path,
) -> Result<()> {
    // Uses remove_dir_all directly without checking existence first
    // This prevents TOCTOU - if it doesn't exist, we get an error we ignore
    match tokio::fs::remove_dir_all(workspace_dir).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // Already gone
        Err(e) => Err(e),
    }
}
```

---

## 9. Implementation Priority

### Phase 1: Core Journal
1. **StepRecord/OperationRecord types** - Port from seshat
2. **Journal table** in SQLite - Append-only step log
3. **OperationState enum** - State machine for operations

### Phase 2: Atomic Operations
4. **atomic_create_workspace** - Port from hardline prior codebase atomic.rs
5. **Compensation logic** - Rollback on failure
6. **Recovery scanner** - On startup, find incomplete ops

### Phase 3: Durable Context
7. **DurableContext trait** - run(), sleep(), get_state(), set_state()
8. **Promise/Awakeable** - Workflow signaling
9. **TerminalError** - Non-retryable errors

### Phase 4: Orchestrator Integration
10. **PipelineState machine** - Port from orchestrator
11. **State transitions** - Validated state changes
12. **Recovery procedures** - Full journal replay

---

## 10. Key Files to Port

| Source | File | Purpose |
|--------|------|---------|
| hardline (prior) | `isolate-core/src/domain/events.rs` | DomainEvent enum, event sourcing |
| hardline (prior) | `isolate/src/commands/add/atomic.rs` | Two-phase compensation, atomic create |
| hardline (prior) | `isolate-core/src/recovery.rs` | Recovery policies, validation, cleanup |
| seshat | `diagram_tool/src/store/types/durable_types.rs` | OperationRecord, StepRecord |
| orchestrator | `orchestrator/src/state.rs` | PipelineState machine |

---

## 11. Related ADRs

- **ADR-001**: Hardline CLI Architecture - Command hierarchy, AI-first output
- **ADR-003**: Restate Feature Parity - Verified Restate SDK API

---

## 12. Decision Summary

1. **Journal-based durability**: Every operation logs steps before execution
2. **Two-phase compensation**: On failure, roll back in reverse order
3. **State machine**: Operations have explicit Pending → Running → Completed/Failed states
4. **Recovery on startup**: Scan for incomplete ops and replay journal
5. **Never panic in cleanup**: All rollback failures are handled gracefully

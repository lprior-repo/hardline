# Red Queen Test Plan: scp-cli Adversarial Test Suite

**Bead**: ha-qaw3
**Target**: `crates/cli/src/commands/` (command handlers, task_types, task_validation, queue)
**Method**: Deterministic adversarial evolution — exit codes are ground truth

## Attack Dimensions

### DIM-1: QueueItem State Machine — Unguarded Transitions

**Severity**: CRITICAL
**Finding**: `QueueItem::start_processing`, `complete`, `fail`, `cancel` have NO state guards.
Any transition is valid from any state. A Completed item can be re-processed, a Failed item
can be marked Completed without fixing, `start_processing` increments `attempt_count`
without bound.

**Tests**:
- Re-process a Completed item (Complete -> Processing)
- Fail a Completed item (Complete -> Failed)
- Complete a Failed item without retry (Failed -> Complete)
- Cancel an already-Completed item
- `start_processing` on Completed increments attempt_count past 1
- `start_processing` overflow: call 1000 times, verify attempt_count wraps or panics
- Complete -> Cancel -> Complete round-trip
- Full cycle: Pending -> Processing -> Failed -> Processing -> Completed (skip normal flow)

### DIM-2: TaskId Serde Bypass — Construction-Time Validation Evasion

**Severity**: CRITICAL
**Finding**: `TaskId::new()` validates `^[a-zA-Z0-9_-]+$` but `#[derive(Deserialize)]` uses
serde's default string deserialization which does NOT call `new()`. A `TaskId` constructed via
JSON deserialization bypasses all validation.

**Tests**:
- Deserialize `{"id": "bad id!"}` into Task — verify invalid ID is accepted
- Deserialize `{"id": "task; DROP TABLE"}` — SQL injection string bypasses validation
- Deserialize `{"id": "../../../etc/passwd"}` — path traversal string bypasses validation
- Deserialize `{"id": ""}` — empty string bypasses validation
- Deserialize `{"id": "task\nwith\nnewlines"}` — newline injection bypasses validation
- Full Task serde roundtrip with invalid ID — serialize with valid, deserialize with invalid

### DIM-3: Title/Priority/Assignee Injection — No Input Validation

**Severity**: MAJOR
**Finding**: `Title::new()`, `Priority::new()`, `Assignee::new()` accept ANY string including
empty strings, newlines, control characters, and SQL injection payloads.

**Tests**:
- Title with newline characters breaks line-oriented output
- Title with null bytes (`\x00`)
- Priority containing arbitrary strings (not mapped to queue Priority enum)
- Assignee with newlines/control characters
- Empty Title/Priority/Assignee — accepted without error
- Title with JSON-breaking characters (`"`, `\`)
- Task serialization roundtrip preserves injection payloads

### DIM-4: TaskStore Silent Error Swallowing

**Severity**: MAJOR
**Finding**: `TaskStore::load()` silently returns empty store on parse errors.
`TaskStore::list()` returns empty on poisoned RwLock. `TaskStore::get()` returns None
on poisoned lock. These mask data loss.

**Tests**:
- Verify `list()` on poisoned lock returns empty (document behavior)
- Verify `get()` on poisoned lock returns None (document behavior)
- Task JSON with extra unknown fields — verify accepted (serde lenient)
- Task JSON with missing required fields — verify error handling
- Malformed JSON in task store file — verify load returns empty (not panic)

### DIM-5: Queue Priority Edge Cases

**Severity**: MINOR
**Finding**: `MemQueue::enqueue` uses `unwrap_or(items.len())` for priority insertion
position. Same-priority items maintain FIFO order but the unwrap_or silently swallows
any position error. Queue skips non-Pending items during dequeue.

**Tests**:
- Enqueue 100 items with same priority — verify FIFO order preserved
- Dequeue skips Completed/Failed/Cancelled items
- Dequeue on queue with only non-Pending items returns None
- Enqueue with same priority multiple times — stable sort verification
- Priority tie-breaking: items enqueued at same priority maintain insertion order

### DIM-6: Task State Consistency Invariants

**Severity**: MAJOR
**Finding**: Task state and assignee can become inconsistent. A Closed task can be
re-opened via raw transition functions. The `execute_done` handler checks for
"task disappeared after completion" — a race condition that can occur in theory.

**Tests**:
- Closed task can be claimed via transition_to_claimed (no validate_not_closed)
- Blocked task can be yielded (no state guard)
- Deferred task can be started (no state guard)
- Task assignee cleared but state remains InProgress (inconsistency via transition_to_yielded + manual state set)
- Double-close produces monotonic timestamps
- transition_to_done on already-Closed overwrites closed_at timestamp

### DIM-7: UUID Generation Collision

**Severity**: MINOR
**Finding**: The simplified UUID module in `core/src/queue.rs` uses `SystemTime::now()`
with `unwrap_or(0)`. If `SystemTime::now()` fails, all items get all-zero UUIDs.

**Tests**:
- Two QueueItems created in rapid succession have different IDs
- UUID format is valid (8-4-4-4-12 hex groups)
- UUID string is 36 characters long

## Non-Goals

- UI rendering tests (covered by existing tests)
- Integration tests requiring git workspace (covered by lock_integration.rs)
- Fuzzing (covered by fuzz/ directory)

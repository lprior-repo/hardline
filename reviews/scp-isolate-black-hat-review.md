# BLACK HAT REVIEW: scp-isolate

**Reviewer**: Polecat eta (adversarial)
**Target**: `crates/isolate/` (18 files, ~11,457 lines incl. tests)
**Date**: 2026-04-17
**Verdict**: **REJECT** — 2 CRITICAL, 6 HIGH, 4 MEDIUM, 3 LOW

---

## PHASE 1: Contract & Bead Parity

### CRITICAL-1: Events define states that don't exist in the state machine

`events.rs` defines four event variants for states that have **no corresponding `WorkspaceState` variant**:

| Event Variant | Expected State | Exists in `WorkspaceState`? |
|---|---|---|
| `WorkspaceSyncing` | Syncing | **NO** |
| `WorkspaceSynced` | Synced | **NO** |
| `WorkspacePaused` | Paused | **NO** |
| `WorkspaceResumed` | Resumed | **NO** |

The `WorkspaceState` enum (types.rs:14) has: `Created`, `Working`, `Ready`, `Merged`, `Abandoned`, `Conflict`. The architecture spec (section 2.2) defines `SessionState: Created -> Active -> Syncing -> Synced -> Paused -> Completed/Failed` — but the isolate domain uses `WorkspaceState`, not `SessionState`. The spec explicitly lists `WorkspaceState: Created -> Working -> Ready -> Merged/Abandoned/Conflict` (architecture-spec.md:125). The events are lying about states that the state machine cannot represent.

**Impact**: These events can never be emitted by the state machine. They are dead code that creates a false contract. Any consumer testing for these events will never receive them.

### HIGH-1: `IsolateError` lacks error codes from the architecture spec

The architecture spec (section 4.1) defines error code ranges:
- 1xxx: Workspace errors (`WorkspaceNotFound`, `WorkspaceLocked`, `WorkspaceCorrupt`)
- 9xxx: Internal errors

`IsolateError` (error.rs) provides only 6 variants, none with numeric codes. Missing critical spec variants:
- `WorkspaceLocked` (1xxx) — essential for concurrent agent isolation
- `WorkspaceCorrupt` (1xxx) — durability guarantee
- `WorkspaceNotFound` (1xxx) — only `InvalidWorkspaceId` exists

The spec also defines a `ScpError` trait with `code()`, `category()`, `fix()`, and `is_retryable()` methods (architecture-spec.md:274-279). `IsolateError` implements none of this.

### HIGH-2: `CheckpointRecord.id` is a bare `String`

`checkpoint_types.rs:69` — `pub id: String`. This is a domain identifier that should be a newtype (`CheckpointId(String)`) per the architecture spec's DDD principles (architecture-spec.md:457: `struct BeadId(String); not String`). The `id` field is even documented with a format convention: `"auto-1234567890"`.

---

## PHASE 2: Farley Engineering Rigor

### CRITICAL-2: `events.rs` is 754 lines (2.5x the 300-line limit)

The architecture spec enforces a **300-line maximum per file** (architecture-spec.md:422, section 13.2). `events.rs` is 754 lines — **151% over the limit**. The `events.rs` file contains:
- 4 type definitions (EventType, EventContext, IsolateEvent, all accessor methods)
- 398 lines of tests

This is a monolith. The events, event context, and event type should be separate files. The test module alone is larger than most source files.

### HIGH-3: `guard.rs` is 555 lines (1.85x the 300-line limit)

`guard.rs` at 555 lines violates the file limit. The test module (lines 222-555) is 333 lines by itself — larger than the entire allowed file budget. The `WorkspaceGuard` and `CommittedGuard` types should be in separate files.

### HIGH-4: `IsolateEvent::event_type()`, `workspace()`, `context()`, `timestamp()` — 4 identical mega-match arms

`events.rs:236-352` has four methods that each match on all 17 `IsolateEvent` variants to extract a single field. This is 116 lines of boilerplate that could be eliminated with a struct-of-fields approach (store `name`, `context`, `timestamp` as common fields instead of duplicating them in every variant).

**Farley constraint violation**: This is unnecessary complexity that serves no domain purpose. It's pure ceremony.

### MEDIUM-1: `checkpoint.rs` tests swallow errors silently

`checkpoint.rs:242-248` — The `committed_guard_state_is_committed` test uses `.ok().flatten()` to silently discard SQL errors. If the query fails, the test passes vacuously:

```rust
let row: Option<(String,)> =
    sqlx::query_as("SELECT state FROM checkpoints WHERE id = ?")
        .bind(&id)
        .fetch_optional(&pool)
        .await
        .ok()           // <-- SWALLOWS ERROR
        .flatten();
assert_eq!(row.map(|(s,)| s), Some("committed".to_string()));
```

This pattern appears in 3 tests. Tests should assert behavior (WHAT), not implementation details (HOW) — but they also shouldn't pass when the database query fails.

---

## PHASE 3: NASA-Level Functional Rust (The Big 6)

### HIGH-5: Massive primitive obsession in `IsolateEvent` and `EventContext`

`events.rs` uses bare `String` for **every domain identifier**:

- `EventContext.workspace: String` (line 48) — should be `WorkspaceId`
- `EventContext.agent_id: Option<String>` (line 50) — should be `AgentId` (newtype)
- `EventContext.session_id: Option<String>` (line 52) — should be `SessionId` (newtype)
- `IsolateEvent` variants: `name: String`, `source: String`, `branch: String`, `reason: String`, `agent_id: String`, `session_id: String` — all should be newtypes

The crate already defines `WorkspaceId` and `BeadId` as proper newtypes (types.rs:92, 128). But `IsolateEvent` and `EventContext` don't use them. This violates the architecture spec's "No primitive obsession" rule (architecture-spec.md:456-458).

`hints/types.rs` has the same problem:
- `WorkspaceInfo.id: String` (line 50) — should be `WorkspaceId`
- `WorkspaceInfo.name: String` (line 51) — should be a `WorkspaceName` newtype
- `NextAction.action: String` (line 72) — at minimum needs justification
- `CommandContext.command: String` (line 83) — should be `CommandName` newtype

### MEDIUM-2: `WorkspaceStateMachine` is a unit struct with all-static methods

`state_machine.rs:14` — `pub struct WorkspaceStateMachine;` with only associated functions. This is a Java-style utility class antipattern. The state machine has no state — it's just a namespace for free functions. Per functional Rust principles, these should be plain functions on the `WorkspaceState` type (which already has `can_transition_to`, `is_terminal`, `is_active`, `is_complete`).

The `WorkspaceStateMachine::transition()` is a thin wrapper around `from.can_transition_to(to)`. The `can_transition`, `is_terminal`, `is_active`, `is_complete` methods are pure delegation to the type itself. This is unnecessary indirection.

### MEDIUM-3: `let mut` in non-test source code

`dag/calc.rs:133,156` — `let mut bfs = Bfs::new(...)` in the `ancestors()` and `descendants()` methods. While technically necessary for the BFS iterator API, this indicates the code is using an imperative traversal pattern. The `build_graph()` method (calc.rs:19-46) correctly avoids `mut` by using `fold` — the traversal methods should follow the same pattern.

### LOW-1: `BeadId` has no `generate()` method

`WorkspaceId::generate()` exists (types.rs:97), but `BeadId` has only `parse()`. The asymmetry suggests incomplete domain modeling. In the architecture spec, bead IDs have a specific format (`bd-xxx`, architecture-spec.md:144).

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### HIGH-6: DDD boundary violation — `checkpoint.rs` lives in the domain module's parent

`checkpoint.rs` is at the crate root level (not under `domain/`), but it imports from `domain::checkpoint_types` and `domain::WorkspaceState`. It uses `sqlx::SqlitePool` directly. This is correct placement for infrastructure — **but** `checkpoint_types.rs` and `checkpoint_calc.rs` are under `domain/` while being tightly coupled to the infrastructure checkpoint system.

The `CheckpointState::from_db()` and `CheckpointState::as_db()` methods (checkpoint_types.rs:43-63) are **persistence concerns leaking into the domain layer**. The domain should not know about "db" representations. This violates the architecture spec's DDD layer enforcement (architecture-spec.md:644-654): "No tokio, sqlx, reqwest imports allowed" in domain.

While `from_db`/`as_db` don't import sqlx directly, they encode database schema knowledge into domain types — which is the same violation in spirit.

### MEDIUM-4: `BranchDag` uses `pub(crate)` fields instead of constructor encapsulation

`data.rs:20-26` — `parents`, `children`, `branches` are `pub(crate)`. While this limits visibility to the crate, it still exposes mutable interior state. The `add_branch` and `remove_branch` methods in `calc.rs` directly mutate these fields. A pure functional approach would return a new `BranchDag` with the operation applied.

The `build_graph()` method (calc.rs:19) is called by `ancestors`, `descendants`, `would_create_cycle`, and `topological_sort` — rebuilding the entire petgraph on every call. This is O(V+E) on every traversal. For a domain that needs to handle "1000+ stacked branches" (architecture-spec.md:1362), this is a performance concern masked by functional purity.

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### LOW-2: `generation.rs` `extract_workspace_name` is fragile string parsing

`generation.rs:182-184` — Splits on single quotes to extract a workspace name from error messages. This couples hint generation to the exact format of error messages. If any error message format changes, hints silently break. This should use structured error types instead.

### LOW-3: `hints/next_actions.rs` — 216 lines of command-dispatch boilerplate

Every function (`next_after_init`, `next_after_create`, `next_after_destroy`, etc.) follows the exact same pattern: construct `Vec<NextAction>`, push items, return. This is 216 lines of code that could be a data table (map of command → list of actions). The current approach violates Unix philosophy — it's not composable, it's a giant match statement.

### LOW-4: `CommittedGuard` is a data class with trivial accessors

`guard.rs:176-219` — `CommittedGuard` has 5 getter methods and 2 boolean helpers for a 4-field struct. This is Java bean territory. In Rust, the fields should be public (or `pub(crate)`) and the struct should derive the relevant traits. The `is_ready()` and `is_abandoned()` methods exist solely because `state` is private — make it public and delete the boilerplate.

---

## SUMMARY

| Severity | Count | Key Issues |
|---|---|---|
| CRITICAL | 2 | Events define non-existent states (dead contract); events.rs 754 lines |
| HIGH | 6 | Error codes missing; file limits exceeded; primitive obsession; DDD leak |
| MEDIUM | 4 | Utility struct antipattern; mut in source; fragile string parsing; mutable DAG |
| LOW | 4 | Missing BeadId::generate(); boilerplate next_actions; CommittedGuard bean; extract_workspace_name |
| **TOTAL** | **16** | |

## VERDICT: **REJECT**

The crate has solid fundamentals — good state machine, proper RAII guards, zero `unwrap`/`panic`/`unsafe` in source code, and excellent test coverage. But it fails on contract parity (events lie about states that don't exist) and file size limits (the two largest files are 2.5x and 1.85x over budget). The primitive obsession in events and hints is a systemic DDD violation that will compound as the crate grows.

**Must fix before merge:**
1. Remove or justify the 4 ghost event variants (`Syncing`, `Synced`, `Paused`, `Resumed`) — they correspond to no state
2. Split `events.rs` (754 lines) into `events/types.rs`, `events/context.rs`, `events/mod.rs`
3. Split `guard.rs` (555 lines) into `guard.rs` + `committed_guard.rs`
4. Replace bare `String` with proper newtypes in `EventContext` and `IsolateEvent`
5. Move `CheckpointState::from_db()`/`as_db()` out of the domain layer

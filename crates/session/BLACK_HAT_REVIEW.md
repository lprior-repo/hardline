# Black Hat Review: scp-session

**Reviewer**: Black Hat (adversarial)  
**Target**: `crates/session/` (crate name: `scp-session`)  
**Date**: 2026-04-17  
**Verdict**: **REJECT — CRITICAL findings require rewrite**

---

## PHASE 1: Contract & Bead Parity

### FINDING: C1-CRITICAL — Type Duplication Crisis (5 Conflicting BeadId Definitions)

There are **five separate `BeadId` types** in this crate, each with different validation rules:

| Location | Type | Validation |
|----------|------|-----------|
| `domain/bead_value.rs:16` | `BeadId(String)` | non-empty, ≤100 chars, alphanumeric/hyphen/underscore |
| `domain/value_objects/session.rs:121` | `BeadId(String)` | `bd-` prefix + hex suffix |
| `domain/value_objects/task.rs:47` | `TaskId(String)` | `bd-` prefix + hex suffix (same as above but different name) |
| `domain/value_objects/metadata.rs:95` | `DependsOn(String)` | `bd-` prefix + hex suffix (same again, third name) |
| `lib.rs:15` re-exports | `BeadId as BdId` from `bead_value` | alphanumeric (NOT hex!) |

**This is a contract violation.** The `BeadId` from `bead_value.rs` accepts `bd-123!@#$` while the one from `session.rs` rejects it. Consumers have no idea which one they're using. The `lib.rs` re-export aliases it to `BdId` to dodge the collision — a naming hack, not a fix.

**Same disease infects `WorkspaceId`, `WorkspaceName`, `Priority`, and `Title`:**
- `WorkspaceId` in `workspace.rs` — just non-empty
- `WorkspaceId` in `session.rs` — just non-empty  
- `WorkspaceName` in `workspace.rs` — non-empty, trimmed, ≤100 chars
- `WorkspaceName` in `metadata.rs` — non-empty, trimmed, ≤100 chars (duplicate)
- `Priority` in `bead_types.rs` — `Priority(u8)` with 0-4 range
- `Priority` in `metadata.rs` — `Priority(u8)` with 0-4 range (duplicate)
- `Title` in `task.rs` — non-empty, trimmed, ≤200 chars
- `BeadTitle` in `bead_value.rs` — non-empty, trimmed, ≤200 chars (duplicate)

**Rule violated**: Contract Parity — types do not match runtime expectations across module boundaries.

### FINDING: C2-HIGH — SessionError InvalidTransition Uses Wrong State Types

`bead.rs:171-206`: Three methods (`validate_closed_state_transition`, `try_transition_to_closed`, `validate_state_transition`) return `SessionError::InvalidTransition { from: WorkspaceState, to: WorkspaceState }` for **Bead** state transitions.

A bead's state machine (`BeadState`) has nothing to do with `WorkspaceState`. The error silently reports `Working → Working` for every bead transition failure — completely wrong context, impossible to debug.

**Lines**: `bead.rs:174-176`, `bead.rs:190-195`, `bead.rs:201-206`

---

## PHASE 2: Farley Engineering Rigor

### FINDING: F1-CRITICAL — SQL Injection via String Interpolation

`sqlite_session_repository.rs:163-191`: The `save` method builds SQL via `format!()` with string interpolation:

```rust
let query = format!(
    r#"INSERT INTO sessions (id, name, workspace, ...) VALUES ('{}', '{}', ...)"#,
    escape_sql_string(&row.id), ...
);
```

The `escape_sql_string` function (line 150-152) only escapes single quotes (`'` → `''`). This is **not sufficient**:
- Backslash escapes are not handled
- NULL bytes could truncate strings
- The `workspace` column receives values like `/tmp/test` which could contain crafted input from the `WorkspacePath` validator that only checks for `"/"` or `"."` prefix — no metacharacter filtering

The `find_by_id`, `find_by_name`, and `delete` methods (lines 200-291) use the same pattern.

**Lines**: `sqlite_session_repository.rs:150-191`, `sqlite_session_repository.rs:206-213`, `sqlite_session_repository.rs:273-287`

**Contrast**: The migration module (`migration.rs`) correctly uses parameterized queries (`sqlx::query(...).bind(...)`). The repository should do the same.

### FINDING: F2-HIGH — SessionService is a Stateless Passthrough

`session_service.rs`: The entire service is 31 lines. Every method is a direct delegation:

```rust
pub fn create_session(name: SessionName) -> Result<Session<Created>> {
    Session::create(name)
}
```

`list_sessions()` returns `Ok(Vec::new())` — hardcoded empty. `get_session()` returns `Err(NotFound("not implemented"))`. This is not a service — it's a useless abstraction layer with two stub methods that will compile-happy but lie at runtime.

**Lines**: `session_service.rs:24-31`

### FINDING: F3-MEDIUM — workspace.rs Uses `let mut` for State Transitions

`workspace.rs:161-256`: Five transition methods (`start_working`, `mark_ready`, `merge`, `mark_conflict`, `abandon`) all clone then mutate:

```rust
let mut new_state = self.clone();
new_state.state = WorkspaceState::Working;
new_state.updated_at = Utc::now();
Ok(new_state)
```

The `Bead` aggregate in `bead.rs` uses struct update syntax (`Self { state, updated_at, ..self }`) — the correct, immutable approach. `Workspace` should do the same.

**Lines**: `workspace.rs:168-171`, `workspace.rs:189-192`, `workspace.rs:210-213`, `workspace.rs:231-234`, `workspace.rs:252-255`

---

## PHASE 3: NASA-Level Functional Rust (The Big 6)

### FINDING: N1-CRITICAL — Session transition_impl Allows Illegal Transitions

`entities/session.rs:265-276`: `transition_impl` is an unconditional move:

```rust
fn transition_impl<T: StateInfo>(self) -> Result<Session<T>, SessionError> {
    Ok(Session {
        id: self.id, name: self.name, ...
        _state: PhantomData,
    })
}
```

This means **any state can transition to any other state**. The typestate pattern is purely cosmetic — `Created → Completed` is allowed, `Failed → Active` is allowed, `Completed → Syncing` is allowed. The compiler enforces nothing because `transition_impl` accepts *any* `T: StateInfo`.

The individual `impl` blocks (e.g., `impl Session<Active>`) restrict which methods are *callable*, but `transition_impl` itself performs zero validation. If anyone adds a `complete()` method to `impl Session<Created>`, it compiles and works — no state machine enforcement.

**Lines**: `entities/session.rs:265-276`

### FINDING: N2-HIGH — Session Fields Are Public

`entities/session.rs:177-186`:

```rust
pub struct Session<S = Created> {
    pub id: SessionId,
    pub name: SessionName,
    pub last_synced: Option<DateTime<Utc>>,
    ...
}
```

All fields are `pub`. Anyone can construct a `Session<Completed>` with `last_synced = None` and `created_at` set to any value. The typestate boundary is porous — internal invariants are exposed.

**Lines**: `entities/session.rs:178-185`

### FINDING: N3-MEDIUM — Boolean Parameter Anti-Pattern

`bead.rs:117-118`:

```rust
if depends_on != self.id && !self.depends_on.contains(&depends_on) {
```

This is fine as an internal check but the `add_dependency` method silently swallows invalid input (self-reference or duplicate) instead of returning an error. The return type is `Self` (not `Result<Self, _>`), so the caller has no way to know the operation was a no-op.

**Lines**: `bead.rs:116-128`, `bead.rs:135-147`

### FINDING: N4-MEDIUM — SessionEvent Fields Use Raw String for session_id

`events/mod.rs:31-32`:

```rust
pub struct SessionCreatedEvent {
    pub session_id: String,
```

The rest of the crate uses `SessionId(String)` as a newtype. Events bypass this — raw `String` means no validation at the boundary.

**Lines**: `events/mod.rs:31`, `events/mod.rs:47`, `events/mod.rs:64`

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### FINDING: D1-CRITICAL — DDD Boundary Violations

The crate has a clean layered architecture (domain → application → infrastructure) on paper. In practice:

1. **Domain depends on infrastructure error types**: `domain/bead.rs` imports `crate::error::SessionError` — which is defined at the crate root, not in domain. But `error.rs` imports `domain::entities::session::SessionState` and `domain::workspace_state::WorkspaceState`. This is a circular dependency that works only because Rust allows it within the same crate. If domain and infrastructure were separate crates (as DDD demands), this would not compile.

2. **BeadState transitions are validated in two places**: `bead.rs` (via `BeadState::can_transition_to`) AND `bead.rs` transition method (via `try_transition_to_closed` overriding the state machine). The `can_transition_to` method says Closed→Closed is invalid, but `transition` allows it via the early return in `try_transition_to_closed`. Two sources of truth.

3. **WorkspaceStateMachine is a stateless utility**: `workspace_state.rs:114-152` — every method is `pub fn transition(from, to)` or `pub fn is_terminal(state)`. These are all already methods on `WorkspaceState` itself. The `WorkspaceStateMachine` struct exists for no reason — it has no state and adds no capability.

### FINDING: D2-HIGH — Test Bloat (3500+ Lines of Boilerplate)

The test-to-production code ratio is obscene:

| File | Production Lines | Test Lines | Ratio |
|------|-----------------|-----------|-------|
| `error.rs` | 116 | 318 | 2.7x |
| `bead_state.rs` | 88 | 255 | 2.9x |
| `bead_types.rs` | 63 | 157 | 2.5x |
| `bead_value.rs` | 128 | 193 | 1.5x |
| `workspace.rs` | 321 | 462 | 1.4x |
| `workspace_state.rs` | 152 | 393 | 2.6x |
| `entities/session.rs` | 397 | 1068 | 2.7x |
| `events/mod.rs` | 88 | 336 | 3.8x |
| `value_objects/session.rs` | 145 | 425 | 2.9x |
| `value_objects/task.rs` | 188 | 495 | 2.6x |
| `value_objects/metadata.rs` | 228 | 597 | 2.6x |
| `sqlite_session_repository.rs` | 333 | 227 | 0.7x |
| `migration.rs` | 465 | 675 | 1.5x |

The test boilerplate is predominantly:
- Display format tests (`assert_eq!(format!("{x}"), "expected")`) — 50+ tests
- Serde roundtrip tests (`to_string → from_str → assert_eq`) — 40+ tests
- "Extended tests" that test the same invariant with slightly different values
- "Edge case tests" that test boundary values already covered by proptests

This is test theater, not test coverage. The proptests already cover the property space. The hand-written tests are redundant.

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### FINDING: B1-HIGH — YAGNI Violations

1. **`SessionService`** (`session_service.rs`): 31 lines of pure delegation. Two methods are stubs. This adds a layer of indirection with zero value.

2. **`WorkspaceStateMachine`** (`workspace_state.rs:114`): A struct with no fields whose every method delegates to `WorkspaceState`. Delete it.

3. **`serialize_event` / `deserialize_event`** (`events/mod.rs:82-88`): Two free functions that wrap `serde_json::to_string` / `from_str`. Anyone can call these directly.

4. **`DependsOn`** (`metadata.rs:95`): A newtype wrapping `String` with the exact same validation as `BeadId`/`TaskId`. This is `BeadId` with a different name.

5. **`IssueType`** (`metadata.rs:188`): A string-based enum validated at runtime against `["bug", "feature", "task", "epic", "chore"]`. The `BeadType` enum in `bead_types.rs` represents the exact same concept as a proper enum. Two representations of the same thing.

### FINDING: B2-MEDIUM — `BeadDescription` Wraps `Option<String>` for No Reason

`bead_value.rs:97`:

```rust
pub struct BeadDescription(Option<String>);
```

This type wraps `Option<String>` inside a struct. The `as_option()` method returns `Option<&String>`. There is no invariant being enforced that `Option<String>` doesn't already provide. The constructor converts empty strings to `None` — but that's a parsing concern, not a type-level invariant.

### FINDING: B3-LOW — Proptest Strategy is Enum Indexing

Throughout the test suite, proptests use `u8` index into arrays:

```rust
proptest! {
    fn prop_something(state_idx in 0u8..5u8) {
        let states = BeadState::all();
        let state = states[state_idx as usize];
    }
}
```

This is fine for small enums but brittle — adding a variant changes the range silently. `proptest::arbitrary::Arbitrary` or a custom strategy would be more robust.

---

## Summary of Findings

| ID | Severity | Phase | Finding |
|----|----------|-------|---------|
| C1 | CRITICAL | 1 | 5 conflicting BeadId types, similar duplication for WorkspaceId/Name/Priority/Title |
| C2 | HIGH | 1 | Bead transition errors report WorkspaceState instead of BeadState |
| F1 | CRITICAL | 2 | SQL injection via string interpolation in repository |
| F2 | HIGH | 2 | SessionService is a useless passthrough with stub methods |
| F3 | MEDIUM | 2 | Workspace uses `let mut` instead of struct update syntax |
| N1 | CRITICAL | 3 | Session typestate is cosmetic — no actual state machine enforcement |
| N2 | HIGH | 3 | Session fields are all `pub` — invariants bypassable |
| N3 | MEDIUM | 3 | `add_dependency`/`add_blocker` silently swallow invalid input |
| N4 | MEDIUM | 3 | Event fields use raw String instead of newtypes |
| D1 | CRITICAL | 4 | Circular domain→error dependency; dual state machine truth |
| D2 | HIGH | 4 | 3500+ lines of redundant test boilerplate |
| B1 | HIGH | 5 | 5 YAGNI violations (SessionService, WorkspaceStateMachine, DependsOn, IssueType, serialize_event) |
| B2 | MEDIUM | 5 | BeadDescription wraps Option<String> for no invariant |
| B3 | LOW | 5 | Proptest enum indexing is fragile |

---

## VERDICT: REJECT

**4 CRITICAL, 4 HIGH, 4 MEDIUM, 1 LOW findings.**

The crate has good bones — the typestate pattern for Session, the state machines for BeadState and WorkspaceState, the newtype discipline in value objects. But the implementation has rotted from the inside:

1. **Type duplication makes the crate impossible to use correctly.** Five different `BeadId` types with different validation means every consumer is a coin flip.

2. **SQL injection in the repository** is a showstopper. The migration module demonstrates the correct approach (parameterized queries) — the repository should follow it.

3. **The typestate pattern is theater.** `transition_impl` accepts any target state. The compile-time guarantees are illusory.

4. **Test bloat is hiding the real problems.** 3500+ lines of "display format" and "serde roundtrip" tests give a false sense of coverage while the actual logic (state transitions, error handling, boundary enforcement) goes under-tested.

**Mandate**: Fix C1 (consolidate to single BeadId/WorkspaceId/Priority/Title), F1 (parameterized queries), N1 (enforce transitions in transition_impl), and D2 (halve the test count). Then re-submit for review.

# Black-Hat Defects: scp-hky

## Phase 1: Contract Violations (CRITICAL)

### Defect 1: Missing `AgentStateMachine` Struct
- **Severity**: CRITICAL
- **Location**: `crates/core/src/domain/agent.rs`
- **Contract**: Lines 55-60 explicitly define `AgentStateMachine` with methods:
  - `transition(from: AgentState, to: AgentState) -> Result<AgentState, Error>`
  - `can_transition(from: AgentState, to: AgentState) -> bool`
  - `is_terminal(state: AgentState) -> bool`
  - `is_available(state: AgentState) -> bool`
- **Actual**: Methods exist on `AgentState` directly (e.g., `AgentState::transition_to`, `AgentState::can_transition_to`). No `AgentStateMachine` struct exists.
- **Fix**: Create `AgentStateMachine` struct with static methods matching contract signatures.

### Defect 2: Error Type Regression (String vs Enum)
- **Severity**: CRITICAL
- **Location**: `crates/session/src/error.rs` line 14-15
- **Contract**: Lines 41-42 specify `Error::InvalidTransition { from: WorkspaceState, to: WorkspaceState }` using raw enum types
- **Actual**: Uses `String` types, losing compile-time type safety
- **Fix**: Change error fields to use enum types for type-safe error handling.

### Defect 3: Missing Error Types
- **Severity**: HIGH
- **Location**: `crates/session/src/error.rs`
- **Contract**: Lines 43-44 require:
  - `Error::TerminalStateReached { state: State }`
  - `Error::PreconditionViolation { message: String }`
- **Actual**: Neither error variant exists
- **Fix**: Add missing error variants to `SessionError` enum.

### Defect 4: Zero Tests for AgentState
- **Severity**: HIGH
- **Location**: `crates/core/src/domain/agent.rs`
- **Contract**: Lines 74-76 violation examples require tests:
  - `transition(Error, Active)` → should Err
  - `transition(Idle, Error)` → should Ok
- **Actual**: File has NO tests (ends at line 133 with no `#[cfg(test)]` module)
- **Compare**: `workspace_state.rs` has 18 tests, `agent.rs` has 0
- **Fix**: Add comprehensive tests matching workspace_state.rs test patterns.

---

## Phase 2: Farley Rigor
- **PASS**: All functions <25 lines
- **PASS**: All functions have <5 params
- **PASS**: Pure functions only

---

## Phase 3: Functional Rust Flaws

### Defect 5: Mutation in Builder Pattern
- **Severity**: LOW
- **Location**: `crates/core/src/domain/agent.rs` line 129
- **Code**: `pub const fn with_last_seen(mut self, last_seen: chrono::DateTime<chrono::Utc>) -> Self`
- **Issue**: Uses `mut self` when functional style should return new value
- **Fix**: Change to `self` and construct new `AgentInfo` with `last_seen: Some(last_seen)`

---

## Phase 4: Simplicity & DDD
- **PASS**: No Option-based state machines
- **PASS**: State enums properly exclusive
- **PASS**: No boolean parameters

---

## Phase 5: The Bitter Truth
- **PASS**: Code is readable
- **PASS**: No clever tricks
- **PASS**: No YAGNI violations

---

## Summary

| Phase | Defects | Critical |
|-------|---------|----------|
| Phase 1: Contract | 4 | 3 |
| Phase 2: Farley | 0 | 0 |
| Phase 3: Functional Rust | 1 | 0 |
| Phase 4: Simplicity | 0 | 0 |
| Phase 5: Bitter Truth | 0 | 0 |

**Total Critical**: 3
**Total High**: 1
**Total Low**: 1

**RECOMMENDATION**: REJECT. The AgentState implementation must be refactored to match contract structure with separate `AgentStateMachine` struct, proper enum error types, and comprehensive tests.

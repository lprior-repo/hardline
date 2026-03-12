# Contract Specification: WorkspaceState and AgentState State Machines

## Context

- **Feature**: Session state machines for workspace lifecycle and agent lifecycle
- **Domain terms**:
  - WorkspaceState: Controls workspace lifecycle (Created → Working → Ready → Merged/Conflict/Abandoned)
  - AgentState: Controls agent availability (Idle ↔ Active, any → Offline, any → Error)
- **Assumptions**:
  - State machines must be pure, returning new states without mutation
  - All transitions must be validated before execution
  - Terminal states for workspace are Merged, Conflict, Abandoned. AgentState has no terminal states (any state can transition to Offline or Error, and recovery is possible).
- **Open Questions**:
  - Should WorkspaceState include a "Locked" state for concurrent access control?
  - What triggers the transition from Ready to Merged/Conflict?

## Preconditions

- **P1**: `WorkspaceStateMachine::transition` requires valid source-to-target transition per defined rules
- **P2**: `AgentStateMachine::transition` requires valid source-to-target transition per defined rules
- **P3**: Workspace state transitions must preserve workspace invariants (valid ID, non-empty name)
- **P4**: Agent state transitions must preserve agent invariants (valid ID, valid timestamp)

## Postconditions

- **Q1**: `WorkspaceStateMachine::transition` returns `Ok(new_state)` only if transition is valid
- **Q2**: `WorkspaceStateMachine::transition` returns `Err(InvalidTransition)` for invalid transitions
- **Q3**: `AgentStateMachine::transition` returns `Ok(new_state)` only if transition is valid
- **Q4**: `AgentStateMachine::transition` returns `Err(InvalidTransition)` for invalid transitions
- **Q5**: Terminal states for workspace (Merged, Conflict, Abandoned) cannot transition to any other state. AgentState has no terminal states in this spec.

## Invariants

- **I1**: WorkspaceState enum variants are mutually exclusive (Created, Working, Ready, Merged, Conflict, Abandoned)
- **I2**: AgentState enum variants are mutually exclusive (Idle, Active, Offline, Error)
- **I3**: `can_transition_to` returns true only for valid state pairs
- **I4**: `is_terminal` returns true only for terminal states

## Error Taxonomy

- `Error::InvalidTransition { from: WorkspaceState, to: WorkspaceState }` - when transition is not allowed
- `Error::InvalidTransition { from: AgentState, to: AgentState }` - when transition is not allowed
- `Error::TerminalStateReached { state: State }` - when attempting to transition from terminal state
- `Error::PreconditionViolation { message: String }` - when precondition is not met

## Contract Signatures

```rust
// WorkspaceStateMachine
pub fn transition(from: WorkspaceState, to: WorkspaceState) -> Result<WorkspaceState, Error>
pub fn can_transition(from: WorkspaceState, to: WorkspaceState) -> bool
pub fn is_terminal(state: WorkspaceState) -> bool
pub fn is_ready(state: WorkspaceState) -> bool

// AgentStateMachine  
pub fn transition(from: AgentState, to: AgentState) -> Result<AgentState, Error>
pub fn can_transition(from: AgentState, to: AgentState) -> bool
pub fn is_terminal(state: AgentState) -> bool
pub fn is_available(state: AgentState) -> bool
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Valid workspace state enum | Compile-time | `enum WorkspaceState { ... }` |
| Valid agent state enum | Compile-time | `enum AgentState { ... }` |
| Valid transition pair | Runtime (Result) | `Result<State, Error::InvalidTransition>` |
| Non-terminal check | Runtime (Result) | `Result<State, Error::TerminalStateReached>` |

## Violation Examples (REQUIRED)

- **VIOLATES P1**: `WorkspaceStateMachine::transition(WorkspaceState::Created, WorkspaceState::Merged)` -- should produce `Err(Error::InvalidTransition)`
- **VIOLATES P2**: `AgentStateMachine::transition(AgentState::Error, AgentState::Active)` -- should produce `Err(Error::InvalidTransition)`
- **VIOLATES Q2**: `WorkspaceStateMachine::transition(WorkspaceState::Ready, WorkspaceState::Created)` -- should produce `Err(Error::InvalidTransition)`
- **VIOLATES Q4**: `AgentStateMachine::transition(AgentState::Idle, AgentState::Error)` -- should produce `Ok(AgentState::Error)` (valid transition per "any→Error" rule)

## Ownership Contracts

- All state machine functions take values by ownership (no &mut)
- State transitions return new state values (immutable, functional style)
- No mutation of internal state - pure functions only

## Non-goals

- Persistence of state to database
- Event sourcing or state history tracking
- Concurrent access control via locking

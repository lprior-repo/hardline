---
bead_id: scp-1xw
bead_title: Create BeadState enum in domain
phase: 1
updated_at: "2026-03-13T00:00:00Z"
---

# Contract Specification

## Context
- **Feature**: Create BeadState enum in domain
- **Domain terms**:
  - Bead: atomic unit of work in the system
  - BeadState: lifecycle state of a bead
  - State machine: enforced progression through valid states
- **Assumptions**:
  - The bead crate does not yet exist or needs a new BeadState definition
  - This is for the SCP (Source Control Plane) project
  - Following DDD principles: pure domain types, no I/O
- **Open questions**:
  - Should Claimed be separate from InProgress? (Architecture spec says yes)
  - Are there additional states needed for blocking/deferring?

## Preconditions
- [P1] `BeadState::new()` requires a valid state variant input
- [P2] `BeadState::transition()` requires current_state.can_transition_to(target) == true
- [P3] Creating a new Bead requires state == Open (initial state)
- [P4] Claiming a bead requires current state == Open

## Postconditions
- [Q1] `Bead::create()` returns Bead with state == Open
- [Q2] `Bead::transition()` returns Err on invalid transition (no panic)
- [Q3] `Bead::transition()` returns Ok(new_bead) with new state on valid transition
- [Q4] When transitioning to Merged or Abandoned, state is terminal (can_transition_to returns false for all)
- [Q5] Bead.claimed_by is Some when state is Claimed or InProgress or Ready

## Invariants
- [I1] BeadState variants are exhaustive and mutually exclusive
- [I2] Once in Merged or Abandoned, bead cannot transition to any other state
- [I3] State transitions follow: Open -> Claimed -> InProgress -> Ready -> (Merged | Abandoned)
- [I4] No self-loops allowed (transitioning to same state returns Err)

## Error Taxonomy
- `Error::InvalidStateTransition { from: BeadState, to: BeadState }` - when transition is not allowed by state machine
- `Error::BeadNotFound { bead_id: BeadId }` - when bead does not exist
- `Error::BeadAlreadyClaimed { bead_id: BeadId }` - when claiming already claimed bead
- `Error::PreconditionViolated { message: String }` - when precondition check fails

## Contract Signatures
```rust
// In domain layer - pure, no I/O
pub enum BeadState { Open, Claimed, InProgress, Ready, Merged, Abandoned }

impl BeadState {
    pub fn can_transition_to(&self, target: BeadState) -> bool;
    pub fn is_terminal(&self) -> bool;
    pub fn valid_transitions(&self) -> Vec<BeadState>;
}

pub struct Bead { /* ... */ }

impl Bead {
    pub fn create(id: BeadId, title: BeadTitle) -> Self;
    pub fn transition(&self, new_state: BeadState) -> Result<Self, Error>;
    pub fn can_transition_to(&self, target: BeadState) -> bool;
    pub fn claim(&self, by: AgentId) -> Result<Self, Error>;
    pub fn mark_ready(&self) -> Result<Self, Error>;
    pub fn merge(&self) -> Result<Self, Error>;
    pub fn abandon(&self) -> Result<Self, Error>;
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Valid state variant | Compile-time | `enum BeadState { ... }` - only valid variants exist |
| Valid transition | Runtime-checked constructor | `can_transition_to()` returns bool |
| State == Open for claim | Runtime-checked | `claim()` checks and returns Result |
| Initial state == Open | Compile-time | `Bead::create()` always sets Open |
| Terminal state detection | Compile-time | `is_terminal()` const method |
| No self-loops | Runtime-checked | `transition()` rejects same-state |

## Violation Examples (REQUIRED)

- VIOLATES P2: `bead.transition(BeadState::Open)` when already Open -- should produce `Err(Error::InvalidStateTransition { from: Open, to: Open })`
- VIOLATES P2: `bead.transition(BeadState::Ready)` when state is Open (skipping Claimed, InProgress) -- should produce `Err(Error::InvalidStateTransition { from: Open, to: Ready })`
- VIOLATES P2: `bead.transition(BeadState::Open)` when state is Merged (terminal) -- should produce `Err(Error::InvalidStateTransition { from: Merged, to: Open })`
- VIOLATES P4: `bead.claim(agent_id)` when state is Claimed -- should produce `Err(Error::BeadAlreadyClaimed { ... })`

## Ownership Contracts (Rust-specific)
- `Bead::create()`: Takes ownership of BeadId and BeadTitle, transfers to new Bead
- `Bead::transition()`: Takes &self (immutable borrow), returns new Bead with updated state (functional style, no mut)
- `Bead::claim()`: Takes &self, returns Result<Self, Error> - creates new Bead with updated state and claimed_by field

## Non-goals
- [ ] Persistence layer (SQLite storage)
- [ ] Repository trait implementations
- [ ] Event sourcing / domain events
- [ ] Serialization (JSON/DB) - keep as plain enum

# Contract Specification

## Context
- **Feature:** Add Workspace and Bead aggregates with full lifecycle
- **Domain terms:**
  - `Workspace` - isolated execution environment with lifecycle states
  - `Bead` - atomic unit of work with state transitions
  - `WorkspaceState` - Initializing → Active → Locked → Corrupted → Deleted
  - `BeadState` - Open → InProgress → Blocked → Deferred → Closed
- **Assumptions:** This is an extension of existing domain types; the aggregates must integrate with existing error handling
- **Open questions:** None

## Preconditions

### Workspace Preconditions
- [P1] `Workspace::create` requires non-empty name and valid path
- [P2] `Workspace::activate` requires workspace is in Initializing state
- [P3] `Workspace::lock` requires workspace is in Active state and lock_holder is non-empty
- [P4] `Workspace::unlock` requires workspace is in Locked state
- [P5] `Workspace::mark_corrupted` can only transition from non-terminal states
- [P6] `Workspace::delete` cannot transition from Deleted state

### Bead Preconditions
- [P7] `Bead::create` requires non-empty id (alphanumeric, hyphen, underscore only, max 100 chars) and non-empty title (max 200 chars)
- [P8] `Bead::transition` requires valid state transition according to state machine rules
- [P9] `Bead::add_dependency` requires non-empty dependency id
- [P10] `Bead::add_blocker` requires non-empty blocker id

## Postconditions

### Workspace Postconditions
- [Q1] After `create`, workspace has state = Initializing and created_at = updated_at
- [Q2] After `activate`, state = Active
- [Q3] After `lock`, state = Locked and lock_holder = Some(holder)
- [Q4] After `unlock`, state = Active and lock_holder = None
- [Q5] After `mark_corrupted`, state = Corrupted and lock_holder = None
- [Q6] After `delete`, state = Deleted
- [Q7] After any state transition, updated_at > created_at
- [Q8] `is_locked` returns true iff state == Locked
- [Q9] `is_active` returns true iff state == Active
- [Q10] `is_terminal` returns true iff state ∈ {Deleted, Corrupted}

### Bead Postconditions
- [Q11] After `create`, state = Open and created_at = updated_at
- [Q12] After `transition` to Closed, state includes closed_at timestamp
- [Q13] After `transition`, updated_at is updated to current time
- [Q14] `is_blocked` returns true iff blocked_by is non-empty
- [Q15] `can_transition_to` returns false when transitioning from Closed to any other state
- [Q16] `can_transition_to` returns true for any transition TO Closed

## Invariants

### Workspace Invariants
- [I1] Workspace ID is always non-empty
- [I2] Workspace path is always valid and accessible
- [I3] Terminal states (Deleted, Corrupted) cannot transition to any other state
- [I4] Locked workspaces have a lock_holder
- [I5] Non-terminal workspaces can be locked/unlocked

### Bead Invariants
- [I6] Bead ID is always non-empty, ≤100 chars, alphanumeric/hyphen/underscore only
- [I7] Bead title is always non-empty, ≤200 chars
- [I8] Closed beads cannot transition to any other state
- [I9] blocked_by list contains no self-references
- [I10] depends_on list contains no self-references

## Error Taxonomy

### WorkspaceError Variants
- `WorkspaceError::WorkspaceNotFound` - when workspace does not exist
- `WorkspaceError::WorkspaceExists` - when creating duplicate workspace
- `WorkspaceError::WorkspaceLocked` - when operations require Active state
- `WorkspaceError::InvalidStateTransition` - when transition is not allowed
- `WorkspaceError::InvalidWorkspaceId` - when ID is empty
- `WorkspaceError::InvalidWorkspaceName` - when name is invalid
- `WorkspaceError::InvalidWorkspacePath` - when path is invalid
- `WorkspaceError::OperationFailed` - generic operation failure
- `WorkspaceError::RepositoryError` - persistence layer failure

### BeadError Variants
- `BeadError::NotFound` - when bead does not exist
- `BeadError::AlreadyExists` - when creating duplicate bead
- `BeadError::InvalidId` - when ID is empty or exceeds max length
- `BeadError::InvalidTitle` - when title is empty or exceeds max length
- `BeadError::InvalidStateTransition` - when transition is not allowed
- `BeadError::DependencyCycle` - when dependency would create cycle
- `BeadError::BlockedBy` - when bead has blockers
- `BeadError::InvalidDependency` - when dependency is invalid
- `BeadError::Database` - persistence layer failure
- `BeadError::Serialization` - serialization/deserialization failure

## Contract Signatures

### Workspace Functions
```rust
fn create(name: WorkspaceName, path: WorkspacePath) -> Result<Workspace, WorkspaceError>
fn activate(&self) -> Result<Workspace, WorkspaceError>
fn lock(&self, holder: String) -> Result<Workspace, WorkspaceError>
fn unlock(&self) -> Result<Workspace, WorkspaceError>
fn mark_corrupted(&self) -> Result<Workspace, WorkspaceError>
fn delete(&self) -> Result<Workspace, WorkspaceError>
```

### Bead Functions
```rust
fn create(id: BeadId, title: BeadTitle, description: Option<BeadDescription>) -> Bead
fn with_priority(self, priority: Priority) -> Bead
fn with_type(self, bead_type: BeadType) -> Bead
fn with_assignee(self, assignee: impl Into<String>) -> Bead
fn with_parent(self, parent: BeadId) -> Bead
fn add_dependency(self, depends_on: BeadId) -> Bead
fn add_blocker(self, blocked_by: BeadId) -> Bead
fn transition(&self, new_state: BeadState) -> Result<Bead, BeadError>
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Workspace name non-empty | Runtime-checked constructor | `WorkspaceName::new() -> Result` |
| Workspace path valid | Runtime-checked constructor | `WorkspacePath::new() -> Result` |
| Bead ID alphanumeric + valid length | Compile-time | `BeadId::new() -> Result` |
| Bead title non-empty + valid length | Compile-time | `BeadTitle::new() -> Result` |
| State transition valid | Runtime-checked | `Result<Workspace, WorkspaceError>` |
| Lock holder non-empty | Runtime-checked | `Result<Workspace, WorkspaceError>` |

## Violation Examples (REQUIRED)

### Workspace Violations
- VIOLATES P2: `workspace.activate()` when state=Active → returns `Err(InvalidStateTransition { from: "Active", to: "Active" })`
- VIOLATES P3: `workspace.lock("".into())` when state=Active → returns `Err(InvalidStateTransition)` (empty holder)
- VIOLATES P4: `workspace.unlock()` when state=Initializing → returns `Err(InvalidStateTransition { from: "Initializing", to: "Active" })`
- VIOLATES P5: `workspace.delete()` when state=Deleted → returns `Err(InvalidStateTransition { from: "Deleted", to: "Deleted" })`
- VIOLATES Q3: `workspace.lock("holder".into())` when state=Active → returns Ok but lock_holder should be Some("holder")

### Bead Violations
- VIOLATES P7: `Bead::create(BeadId::new("".into()).unwrap(), ...)` → should return Err (via BeadId::new)
- VIOLATES P7: `Bead::create(BeadId::new("bead!".into()).unwrap(), ...)` → should return Err (invalid characters)
- VIOLATES P8: `bead.transition(Open)` when state=Closed → returns `Err(InvalidStateTransition)`
- VIOLATES Q11: After create, state should be Open but test should verify exact equality

## Ownership Contracts (Rust-specific)

### Workspace
- `create` transfers ownership of name and path into the Workspace
- `lock` takes ownership of holder String, clones internally
- All state transition methods clone internally, return new Workspace (immutable)

### Bead
- `create` transfers ownership of id and title into the Bead
- Builder methods (with_priority, with_type, etc.) consume and return Self
- `transition` borrows self immutably, returns new Bead with updated state
- All collections (depends_on, blocked_by) are cloned on modification

## Non-goals
- [ ] Implementing repository/persistence layer (separate concern)
- [ ] Adding event sourcing (future enhancement)
- [ ] Supporting workspace templates (future enhancement)
- [ ] Implementing bead swimlanes/columns (future enhancement)

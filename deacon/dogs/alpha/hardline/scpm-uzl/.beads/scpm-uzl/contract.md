# Contract Specification: Queue Service Wiring

## Context

- **Feature**: Wire queue application services together
- **Bead ID**: scpm-uzl
- **Bead Title**: queue: wire application services
- **Domain Terms**:
  - `QueueEntry` - Main entity representing a job in the queue
  - `QueueEntryId` - Unique identifier with validation
  - `QueueStatus` - State enum: Pending, Claimed, Rebasing, Testing, ReadyToMerge, Merging, Merged, FailedRetryable, FailedTerminal, Cancelled
  - `QueueRepository` - Port trait for persistence
  - `QueueService` - Application service orchestrating job processing
  - `Priority` - Value object for job priority (Critical, High, Default, Low)
  - `QueuePosition` - Value object for ordering
- **Assumptions**:
  - Repository trait is already defined and implemented
  - Domain entities and state machine are already implemented
  - Application service is partially implemented but needs proper wiring
- **Open Questions**: None

## Preconditions

- [ ] **P1**: QueueService MUST receive a valid `QueueRepository` implementation at construction
- [ ] **P2**: All service methods MUST accept validated domain objects (non-empty session_id, valid Priority)
- [ ] **P3**: Repository MUST be available (not poisoned) for all operations
- [ ] **P4**: Concurrent dequeue requests MUST be handled safely (mutex protection in repository)

## Postconditions

- [ ] **Q1**: `enqueue` MUST persist a new QueueEntry with status `Pending` and return its ID
- [ ] **Q2**: `dequeue` MUST return `Some(QueueEntry)` with status `Claimed` OR `None` if queue is empty
- [ ] **Q3**: `complete_job` MUST transition entry through valid state machine states
- [ ] **Q4**: State transitions MUST be validated against `QueueStateMachine::can_transition`
- [ ] **Q5**: Service methods MUST return `Result<T, QueueError>` for all fallible operations
- [ ] **Q6**: A single job can only be processed by one worker at a time (atomic dequeue)

## Invariants

- [ ] **I1**: A `QueueEntry` in `Pending` status is the only entry that can be dequeued
- [ ] **I2**: Once `Merged`, `FailedTerminal`, or `Cancelled`, a job cannot change state
- [ ] **I3**: `retry_count` MUST NOT exceed 3 for `FailedRetryable` entries
- [ ] **I4**: All timestamps (`enqueued_at`, `updated_at`) MUST be in UTC

## Error Taxonomy

```rust
pub enum QueueError {
    QueueEntryNotFound(String),           // When entry ID not found
    QueueEmpty,                           // When dequeue on empty queue
    InvalidStateTransition { from: String, to: String }, // Invalid state change
    InvalidQueueEntryId(String),          // Validation failure for entry ID
    InvalidPriority(String),              // Validation failure for priority
    InvalidQueuePosition(String),         // Validation failure for position
    OperationFailed(String),               // Generic operation failure
    RepositoryError(String),               // Repository-level failures
}
```

## Contract Signatures

```rust
// Repository trait (already defined)
pub trait QueueRepository: Send + Sync {
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError>;
    fn dequeue(&self) -> Result<Option<QueueEntry>, ValidationError>;
    fn get(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, ValidationError>;
    fn update(&self, entry: QueueEntry) -> Result<QueueEntry, ValidationError>;
    fn list_pending(&self) -> Result<Vec<QueueEntry>, ValidationError>;
    fn list_all(&self) -> Result<Vec<QueueEntry>, ValidationError>;
    fn remove(&self, id: &QueueEntryId) -> Result<(), ValidationError>;
}

// Service wiring (THE CONTRACT)
pub struct QueueService<R: QueueRepository> {
    repository: R,
}

impl<R: QueueRepository> QueueService<R> {
    pub fn new(repository: R) -> Self;
    pub fn enqueue(&self, session_id: String, bead_id: Option<String>, priority: Priority) -> Result<QueueEntry, QueueError>;
    pub fn dequeue(&self) -> Result<Option<QueueEntry>, QueueError>;
    pub fn get_job(&self, id: &QueueEntryId) -> Result<Option<QueueEntry>, QueueError>;
    pub fn update_job(&self, entry: QueueEntry) -> Result<QueueEntry, QueueError>;
    pub fn claim_job(&self, id: &QueueEntryId) -> Result<QueueEntry, QueueError>;
    pub fn complete_job(&self, id: &QueueEntryId, success: bool) -> Result<QueueEntry, QueueError>;
    pub fn cancel_job(&self, id: &QueueEntryId) -> Result<QueueEntry, QueueError>;
    pub fn list_pending(&self) -> Result<Vec<QueueEntry>, QueueError>;
    pub fn list_active(&self) -> Result<Vec<QueueEntry>, QueueError>;
    pub fn list_all(&self) -> Result<Vec<QueueEntry>, QueueError>;
    pub fn remove_job(&self, id: &QueueEntryId) -> Result<(), QueueError>;
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| session_id non-empty | Runtime-checked constructor | `QueueEntry::enqueue()` validates |
| priority is valid | Compile-time | `Priority` enum with `new()` validation |
| repository not poisoned | Runtime-checked | `Mutex::lock().map_err()` |
| state transition valid | Runtime-checked | `QueueStateMachine::validate_transition()` |
| entry ID is valid | Runtime-checked | `QueueEntryId::parse()` |
| job not already processing | Runtime-checked | Dequeue returns Pending only |

## Violation Examples (REQUIRED)

- VIOLATES P1: `QueueService::new(poisoned_repo)` where Mutex is poisoned → returns `Err(RepositoryError(...))`
- VIOLATES P2: `service.enqueue("", None, Priority::default())` → returns `Err(InvalidQueueEntryId("empty id"))`
- VIOLATES P3: Concurrent calls during repo lock → handled by Mutex, serializes access
- VIOLATES Q1: `enqueue` without persist → entry MUST be stored via `repo.enqueue()`
- VIOLATES Q2: `dequeue` when no pending → returns `Ok(None)`, not error
- VIOLATES Q6: `dequeue` twice simultaneously → Mutex ensures atomicity

## Ownership Contracts

- **repository**: `QueueService` takes ownership of the repository. The service owns and manages the repository lifetime.
- **QueueEntry**: Cloned on input/output. Repository stores owned entries. Service returns clones.
- **No mutation of entry in place**: All state transitions create new entries (functional style)

## Non-goals

- [ ] Direct database/sql implementation (use repository trait)
- [ ] Job execution/processing logic (only state transitions)
- [ ] Scheduling or priority queue implementation (simple FIFO with priority ordering)

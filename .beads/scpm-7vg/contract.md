# Contract Specification: scpm-7vg - Queue Domain Models

## Context
- **Feature**: Queue domain model definition
- **Bead ID**: scpm-7vg
- **Domain terms**: Queue, Job, JobId, QueueId, JobStatus, Payload, Priority
- **Assumptions**: 
  - Jobs are created with a payload (JSON blob) and integer priority
  - Priority range is 0-255 (u8)
  - Queue entries are ordered by priority ascending
- **Open questions**: None

## Preconditions
- P1: `Job::new()` requires payload to be non-empty valid JSON
- P2: `Job::new()` requires priority to be in range 0-255
- P3: `Queue::enqueue()` requires a valid Job instance
- P4: `JobStatus::transition()` requires a valid state transition per the state machine

## Postconditions
- Q1: `Job::new()` returns `Ok(Job)` with valid JobId when inputs are valid
- Q2: `Job::new()` returns `Err(JobCreationError::InvalidPayload)` when payload is empty
- Q3: `Job::new()` returns `Err(JobCreationError::InvalidPayload)` when payload is malformed JSON
- Q4: `Job::new()` returns `Err(JobCreationError::InvalidPriority)` when priority > 255
- Q5: `Queue::enqueue()` returns new Queue with Job added at correct priority position
- Q6: `Queue::dequeue()` returns `(Queue, Option<Job>)` where Job has lowest priority number

## Invariants
- I1: All Jobs in a Queue have unique JobIds
- I2: All Jobs in a Queue are sorted by priority ascending
- I3: Job.status is always a valid JobStatus variant
- I4: JobId and QueueId are always non-empty strings

## Error Taxonomy
```rust
pub enum JobCreationError {
    InvalidPayload(PayloadError),   // 4001
    InvalidPriority(u8),            // 4002
}

pub enum QueueError {
    QueueEmpty,                      // 4003
    JobNotFound(JobId),              // 4004
    InvalidTransition { from: JobStatus, to: JobStatus }, // 4005
}
```

## Contract Signatures
```rust
impl Job {
    pub fn new(id: JobId, payload: Payload, priority: u8) -> Result<Job, JobCreationError>;
    pub fn status(&self) -> JobStatus;
    pub fn transition_to(&self, status: JobStatus) -> Result<Job, QueueError>;
}

impl JobStatus {
    pub fn transition(&self, to: JobStatus) -> Result<JobStatus, QueueError>;
}

impl Queue {
    pub fn enqueue(self, job: Job) -> Queue;
    pub fn dequeue(self) -> (Queue, Option<Job>);
}
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| payload non-empty | Compile-time | `NonEmptyString` wrapper |
| payload valid JSON | Runtime-checked constructor | `serde_json::Value` validation |
| priority 0-255 | Compile-time (strongest) | `u8` with validation at construction |
| JobId non-empty | Compile-time | `JobId` newtype with private constructor |
| QueueId non-empty | Compile-time | `QueueId` newtype with private constructor |
| Valid status transition | Runtime-checked | `JobStatus::transition()` returns Result |

## Violation Examples (REQUIRED)
- VIOLATES P1: `Job::new(JobId::new("j-1"), Payload::from(""), 100)` -- should produce `Err(JobCreationError::InvalidPayload(PayloadError::Empty))`
- VIOLATES P1: `Job::new(JobId::new("j-1"), Payload::from("not json{{"), 100)` -- should produce `Err(JobCreationError::InvalidPayload(PayloadError::MalformedJson))`
- VIOLATES P2: `Job::new(JobId::new("j-1"), Payload::from("{}"), 256)` -- should produce `Err(JobCreationError::InvalidPriority(256))`
- VIOLATES P4: `JobStatus::Pending.transition_to(JobStatus::Pending)` -- should produce `Err(QueueError::InvalidTransition { from: Pending, to: Pending })`

## Ownership Contracts
- `Job::new()` takes ownership of `JobId`, `Payload` - clones if needed for storage
- `Queue::enqueue()` consumes `self` and `Job`, returns new `Queue` - immutable append
- `Job::transition_to()` borrows `self`, returns new `Job` with updated status - no mutation

## Non-goals
- Persistence of Queue or Job to database
- Actual job execution/processing
- Queue worker loop implementation

# Contract Specification: Job Processing Loop

## Context
- Feature: queue: implement job processing loop
- Domain terms: Job, JobQueue, JobProcessor, Priority, Poll, Execute
- Bead ID: scpm-wlx
- Assumptions:
  - Jobs are persisted in a repository (BeadRepository pattern already exists)
  - Priority ordering: P0 > P1 > P2 > P3 > P4 (lower value = higher priority)
  - Job processing is async and interruptible
  - The loop runs continuously until stopped

## Contract Clauses

### Preconditions
- P1: JobProcessor must be created with a valid repository reference
- P2: JobProcessor must have a non-zero poll interval
- P3: Jobs can only be polled if they are in the "pending" state

### Postconditions
- Q1: Upon successful poll, exactly one pending job (or none) is returned
- Q2: Jobs are always returned in priority order (highest priority first)
- Q3: After execution, the job state transitions to either Completed or Failed
- Q4: The job processor maintains internal state tracking of processed jobs

### Invariants
- I1: The total number of running jobs never exceeds the configured concurrency limit
- I2: A job cannot be processed more than once simultaneously
- I3: Priority ordering is preserved across all poll operations

## Error Taxonomy
- Error::QueueEmpty - No pending jobs available when polled
- Error::NoRepository - Repository not configured or connection failed
- Error::JobNotFound - Referenced job does not exist in repository
- Error::InvalidJobState - Job is not in a processable state
- Error::ExecutionFailed - Job execution returned an error
- Error::ShutdownRequested - Processor was signaled to stop

## Type Encoding
| Constraint | Enforcement Level | Type / Pattern |
|---|---|---|
| Repository is valid | Compile-time | `&dyn BeadRepository` trait bound |
| Poll interval > 0 | Runtime-checked constructor | `NonZeroU64` for interval |
| Priority ordering | Compile-time | `Priority: Ord` trait |
| Job state transitions | Runtime-checked | `BeadState` enum with validation |
| Concurrency limit | Runtime-enforced | `Semaphore` or similar |

## Violation Examples (REQUIRED)

### Precondition Violations
- VIOLATES P1: `JobProcessor::new(nullptr, interval)` -- should produce `Err(Error::NoRepository)`
- VIOLATES P2: `JobProcessor::new(repo, Duration::ZERO)` -- should produce `Err(Error::InvalidConfiguration)`
- VIOLATES P3: `poll_job(BlockedJob)` -- should produce `Err(Error::InvalidJobState)`

### Postcondition Violations
- VIOLATES Q1: `poll_pending_job()` returns 2 jobs -- should return max 1
- VIOLATES Q2: `poll_pending_job()` returns lower priority before higher -- should sort by priority descending
- VIOLATES Q3: After execution, job remains in Pending state -- should transition to Completed or Failed

### Invariant Violations
- VIOLATES I1: 10 jobs running with concurrency limit of 5 -- should enforce max 5 concurrent
- VIOLATES I2: Same job executed twice concurrently -- should prevent duplicate execution
- VIOLATES I3: Poll returns jobs out of priority order -- should maintain sorted order

## Ownership Contracts
- `JobProcessor::new(repo, interval)` -- Takes ownership of repository reference, caller retains ownership of interval
- `poll_pending_job(&self)` -- Shared borrow of self, no mutation to processor state during poll
- `execute_job(&self, job_id)` -- Exclusive borrow, modifies job state in repository

## Non-goals
- Job retry logic (handled by separate retry policy)
- Job timeout enforcement (handled by timeout policy)
- Dead letter queue handling (future enhancement)

## Function Signatures (Railway-Oriented)

```rust
pub trait JobRepository: Send + Sync {
    async fn poll_pending_jobs(&self, limit: usize) -> Result<Vec<Job>, Error>;
    async fn update_job_state(&self, job_id: &JobId, state: JobState) -> Result<(), Error>;
}

pub struct JobProcessor<R: JobRepository> {
    repository: R,
    poll_interval: Duration,
    concurrency_limit: usize,
}

impl<R: JobRepository> JobProcessor<R> {
    pub fn new(repository: R, poll_interval: Duration, concurrency_limit: usize) -> Result<Self, Error>;
    pub async fn run(&self, stop_signal: ShutdownToken) -> Result<(), Error>;
    async fn poll_once(&self) -> Result<Option<Job>, Error>;
    async fn execute_job(&self, job: Job) -> Result<JobResult, Error>;
}

pub enum JobState {
    Pending,
    Running { started_at: DateTime<Utc> },
    Completed { finished_at: DateTime<Utc> },
    Failed { error: String, failed_at: DateTime<Utc> },
}

#[derive(Clone)]
pub struct Job {
    pub id: JobId,
    pub priority: Priority,
    pub payload: JobPayload,
    pub created_at: DateTime<Utc>,
}

pub enum JobPayload {
    // Domain-specific payloads
}

pub struct JobResult {
    pub job_id: JobId,
    pub outcome: JobOutcome,
    pub execution_time_ms: u64,
}

pub enum JobOutcome {
    Success,
    Failure { error: String },
}
```

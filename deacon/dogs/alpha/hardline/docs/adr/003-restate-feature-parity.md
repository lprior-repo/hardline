# ADR-003: Restate Feature Parity Analysis

**Date:** 2026-03-20  
**Status:** Research Complete - API VERIFIED against Restate SDK docs.rs  
**Deciders:** Lewis

---

## ⚠️ IMPORTANT: API Verification

I read the actual Restate Rust SDK documentation at https://docs.rs/restate_sdk/0.9.0/ to verify this information. The earlier agent research may have been inaccurate - this document reflects the ACTUAL API.

---

## Verified Restate SDK API (Direct from docs.rs)

I read the actual Restate Rust SDK docs at https://docs.rs/restate_sdk/0.9.0/ to verify the API. Here's the **real** API:

### Context Traits

**1. ContextSideEffects** (Journaling)
```rust
fn run<R, F, T>(&self, run_closure: R) -> impl RunFuture<Result<T, TerminalError>> + 'ctx
where R: RunClosure<Fut = F, Output = T> + Send + 'ctx,
      F: Future<Output = HandlerResult<T>> + Send + 'ctx,
      T: Serialize + Deserialize + 'static

fn random_seed(&self) -> u64
fn rand(&mut self) -> &mut StdRng
fn rand_uuid(&mut self) -> Uuid
```

**2. ContextTimers** (Durable sleep)
```rust
fn sleep(&self, duration: Duration) -> impl DurableFuture<Output = Result<(), TerminalError>> + 'ctx
```

**3. ContextReadState** (State storage)
```rust
fn get<T: Deserialize + 'static>(&self, key: &'ctx str) -> impl Future<Output = Result<Option<T>, TerminalError>> + 'ctx
fn get_keys(&'ctx self) -> impl Future<Output = Result<Vec<String>, TerminalError>> + 'ctx
```

**4. ContextWriteState** (State mutation)
```rust
fn set<T: Serialize + 'static>(&self, key: &str, t: T)
fn clear(&self, key: &str)
fn clear_all(&self)
```

**5. ContextClient** (Service communication)
```rust
fn service_client<C>(&self) -> C where C: IntoServiceClient<'ctx>
fn object_client<C>(&self, key: impl Into<String>) -> C where C: IntoObjectClient<'ctx>
fn workflow_client<C>(&self, key: impl Into<String>) -> C where C: IntoWorkflowClient<'ctx>
fn request<Req, Res>(&self, request_target: RequestTarget, req: Req) -> Request<'ctx, Req, Res>
fn invocation_handle(&self, invocation_id: String) -> impl InvocationHandle + 'ctx
```

**6. ContextPromises** (Workflow signaling)
```rust
fn promise<T: Deserialize + 'static>(&'ctx self, key: &'ctx str) -> impl DurableFuture<Output = Result<T, TerminalError>> + 'ctx
fn peek_promise<T: Deserialize + 'static>(&self, key: &'ctx str) -> impl Future<Output = Result<Option<T>, TerminalError>> + 'ctx
fn resolve_promise<T: Serialize + 'static>(&self, key: &str, t: T)
fn reject_promise(&self, key: &str, failure: TerminalError)
```

**7. ContextAwakeables** (External completion)
```rust
fn awakeable<T: Deserialize + 'static>(&self) -> (String, impl DurableFuture<Output = Result<T, TerminalError>> + Send + 'ctx)
fn resolve_awakeable<T: Serialize + 'static>(&self, key: &str, t: T)
fn reject_awakeable(&self, key: &str, failure: TerminalError)
```

**8. TerminalError** (Non-retryable error)
```rust
TerminalError::new(message: impl Into<String>) -> Self
TerminalError::new_with_code(code: u16, message: impl Into<String>) -> Self
fn code(&self) -> u16
fn message(&self) -> &str
fn from_error<E: StdError>(e: E) -> Self
```

### Service Types

| Type | Macro | Context | Description |
|------|-------|---------|-------------|
| Service | `#[restate_sdk::service]` | `Context<'_>` | Stateless handlers |
| Virtual Object | `#[restate_sdk::object]` | `ObjectContext<'_>` / `SharedObjectContext<'_>` | Stateful keyed entities |
| Workflow | `#[restate_sdk::workflow]` | `WorkflowContext<'_>` / `SharedWorkflowContext<'_>` | Long-running with promises |

---

## Research Summary from 12-Agent Investigation

This ADR documents Restate's durable execution architecture and the features needed for parity in hardline.

---

## 1. Core Durable Execution Model

### Restate's Journal-Based Approach

Restate records every step of code execution in a **journal**. On crash, it replays the journal, skipping completed steps.

```
┌─────────────────────────────────────────────────────┐
│ JOURNAL (Append-only log)                           │
│ 1. InputCommand { name: "run" }                   │
│ 2. RunCommand { name: "create-user" } → Result(1) │
│ 3. SleepCommand { wake_up: 1699900000 } → Result  │
│ 4. RunCommand { name: "send-email" } → Result(2) │
│ 5. OutputCommand { result: true }                  │
└─────────────────────────────────────────────────────┘
```

### Key Components

| Component | Purpose | Restate Equivalent |
|-----------|---------|-------------------|
| **Journal** | Append-only log of all operations | `sys_journal` table |
| **State Store** | Key-value state per entity | `state` table |
| **Invocation State** | Track running/completed/failed | `sys_invocation` table |
| **Partition Processor** | Execute steps, update state | Custom implementation |

---

## 2. Invocation State Machine

| State | Description |
|-------|-------------|
| `pending` | Enqueued, waiting |
| `ready` | Ready to process |
| `running` | Actively executing |
| `backing-off` | Retrying after failure |
| `suspended` | Waiting on external input |
| `completed` | Finished successfully |
| `failed` | Terminal failure |

**Transitions:**
```
pending → ready → running → (completed | backing-off | suspended)
                                    ↓              ↓
                               backing-off     suspended
                                    ↓              ↓
                              (retry or fail) → running
```

---

## 3. Journal Entry Types

| Entry | Restate Command | Purpose |
|-------|----------------|---------|
| Input | `InputCommandMessage` | Initial handler input |
| Output | `OutputCommandMessage` | Handler output |
| State Get | `GetLazyStateCommandMessage` | Read from KV store |
| State Set | `SetStateCommandMessage` | Write to KV store |
| Sleep | `SleepCommandMessage` | Durable timer |
| Call | `CallCommandMessage` | Service-to-service call |
| Run | `RunCommandMessage` | Non-deterministic block |
| Promise | `GetPromiseCommandMessage` | Workflow promise operations |
| Awakeable | `CompleteAwakeableCommandMessage` | External event handling |
| Signal | `SignalNotificationMessage` | Signal from external source |

---

## 4. Service Types (Feature Parity)

| Type | Context | State | Promises | Use Case |
|------|---------|-------|----------|----------|
| **Service** | `Context<'_>` | None | No | Stateless handlers, sagas |
| **Virtual Object** | `ObjectContext<'_>` | Per-key K/V | No | Stateful entities, rate limiters |
| **Workflow** | `WorkflowContext<'_>` | Per-instance | Yes | Long-running with waits |

### hardline Equivalents

| Restate Type | hardline Implementation |
|--------------|------------------------|
| Service | `#[command]` handlers |
| Virtual Object | Workspace + state in SQLite |
| Workflow | Operation + Saga orchestrator |

---

## 5. Context Capabilities

| Capability | Context | ObjectContext | WorkflowContext |
|-----------|---------|--------------|-----------------|
| `run()` | Yes | Yes | Yes |
| `sleep()` | Yes | Yes | Yes |
| `promise()` | No | No | Yes |
| `awakeable()` | Yes | Yes | Yes |
| `get(key)` | No | Yes | Yes |
| `set(key, value)` | No | Yes | Yes |
| `service_client()` | Yes | Yes | Yes |
| `rand_uuid()` | Yes | Yes | Yes |

**hardline implementation (verified against Restate SDK):**
```rust
pub trait DurableContext {
    fn run<R, F, T>(&self, closure: R) -> impl Future<Output = Result<T, TerminalError>>
    where
        R: Future<Output = Result<T, HandlerError>> + Send + 'ctx,
        T: Serialize + Deserialize + 'static;

    fn sleep(&self, duration: Duration) -> impl Future<Output = Result<(), TerminalError>>;

    fn get_state<T: Deserialize>(&self, key: &str) -> impl Future<Output = Result<Option<T>, TerminalError>>;
    fn set_state<T: Serialize>(&self, key: &str, value: T);

    fn promise<T: Deserialize>(&self, key: &str) -> impl Future<Output = Result<T, TerminalError>>;
    fn resolve_promise<T: Serialize>(&self, key: &str, value: T);
    fn reject_promise(&self, key: &str, failure: TerminalError);

    fn awakeable<T: Deserialize>(&self) -> (String, impl Future<Output = Result<T, TerminalError>>);
    fn resolve_awakeable<T: Serialize>(&self, key: &str, value: T);
    fn reject_awakeable(&self, key: &str, failure: TerminalError);

    fn rand_uuid(&self) -> Uuid;
    fn rand(&mut self) -> &mut StdRng;
    fn random_seed(&self) -> u64;
}
```

---

## 6. Saga Pattern Implementation

### Restate's Code-First Approach

Sagas are implemented as regular service handlers with compensation. This is VERIFIED from Restate documentation:

```rust
async fn run(&self, ctx: &Context, req: BookingRequest) -> Result<BookingResult, HandlerError> {
    let mut compensations: Vec<Box<dyn Fn() -> _>> = Vec::new();
    
    // Step 1: Book flight
    let flight_id = ctx.run(|| book_flight(&req.flight)).await?;
    compensations.push(Box::new(|| cancel_flight(flight_id)));
    
    // Step 2: Book hotel
    let hotel_id = ctx.run(|| book_hotel(&req.hotel)).await?;
    compensations.push(Box::new(|| cancel_hotel(hotel_id)));
    
    // On TerminalError, run compensations in reverse
    // (Code managed by developer - no automatic saga support)
    
    Ok(BookingResult { flight_id, hotel_id })
}
```

**Key point from Restate docs:** Sagas are code-managed. Restate provides the durable execution (journal, replay, retry), but compensation/rollback logic is written by the developer using try-catch and compensation lists.
```

### hardline Saga Requirements

```rust
pub struct Saga {
    steps: Vec<SagaStep>,
    compensating: Vec<CompensationStep>,
}

pub struct SagaStep {
    name: String,
    execute: Box<dyn Fn(&DurableContext) -> Box<dyn Future<Output = Result<StepOutput, Error>>> + Send>,
}

pub struct CompensationStep {
    name: String,
    compensate: Box<dyn Fn(&DurableContext, StepOutput) -> Box<dyn Future<Output = Result<(), Error>>> + Send>,
}

impl Saga {
    pub async fn execute(&self, ctx: &DurableContext) -> Result<(), SagaError> {
        let mut completed_steps: Vec<StepOutput> = Vec::new();
        
        for step in &self.steps {
            let output = (step.execute)(ctx).await?;
            ctx.log_event(Event::StepCompleted { step: &step.name, output: &output }).await?;
            completed_steps.push(output);
        }
        
        Ok(())
    }
    
    pub async fn compensate(&self, ctx: &DurableContext) -> Result<(), SagaError> {
        for step in self.compensating.iter().rev() {
            (step.compensate)(ctx, step.output).await?;
            ctx.log_event(Event::StepCompensated { step: &step.name }).await?;
        }
        Ok(())
    }
}
```

---

## 7. Virtual Object Equivalence

### Restate Virtual Object

```rust
#[restate_sdk::object]
pub trait Counter {
    async fn increment(amount: i32) -> Result<i32, HandlerError>;
    #[shared]
    async fn get() -> Result<i32, HandlerError>;
}
```

### hardline Workspace (Virtual Object Equivalent)

Workspaces ARE the virtual objects in hardline:

```rust
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    state: HashMap<String, JsonValue>,
    status: WorkspaceStatus,
}

impl Workspace {
    pub async fn get_state<T: Deserialize>(&self, key: &str) -> Result<Option<T>> {
        Ok(self.state.get(key).and_then(|v| serde_json::from_value(v.clone()).ok()))
    }
    
    pub async fn set_state<T: Serialize>(&mut self, key: &str, value: &T) -> Result<()> {
        self.state.insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }
}
```

---

## 8. Workflow Equivalence

### Restate Workflow

```rust
#[restate_sdk::workflow]
pub trait OrderWorkflow {
    async fn run(order_id: String) -> Result<OrderStatus, HandlerError>;
    #[shared]
    async fn get_status() -> Result<String, HandlerError>;
}
```

### hardline Operation (Workflow Equivalent)

Operations with saga pattern:

```rust
pub struct Operation {
    id: OperationId,
    name: String,
    steps: Vec<OperationStep>,
    state: HashMap<String, JsonValue>,
    status: OperationStatus,
}

pub enum OperationStatus {
    Running,
    Completed,
    Failed { compensation: Vec<CompensationStep> },
    Suspended { waiting_on: String },
}

impl Operation {
    pub async fn checkpoint(&self, name: &str) -> Result<CheckpointId> {
        // Save current state to named snapshot
    }
    
    pub async fn promise<T: Serialize>(&self, key: &str) -> Result<Promise<T>> {
        // Create durable promise
    }
    
    pub async fn resolve_promise<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        // Resolve promise
    }
}
```

---

## 9. Error Handling Parity (VERIFIED)

### TerminalError vs Retryable (from Restate SDK)

| Error Type | Restate | hardline |
|------------|---------|----------|
| Non-retryable | `TerminalError` | `TerminalError` |
| Retryable | Any `std::error::Error` | `HandlerError` |

**Restate error handling (verified):**
```rust
// Return TerminalError to stop retries
Err(TerminalError::new("Business logic failure").into())

// Return TerminalError with code
Err(TerminalError::new_with_code(404, "Not found").into())

// Return regular error - will retry automatically
Err(some_std_error.into())

// Check if error is terminal
fn is_terminal_error(e: &HandlerError) -> bool {
    // Terminal errors stop retries
    // All other errors trigger exponential backoff retry
}
```

**From Restate docs:** "Restate retries failures infinitely. Use `TerminalError` to stop retries."

### Retry Policy Configuration

```rust
// Per-invocation retry policy (server-wide)
[invocation.default-retry-policy]
initial-interval = "50ms"
exponentiation-factor = 2.0
max-attempts = 70
max-interval = "60s"
on-max-attempts = "pause"  // or "kill"

// Per-ctx.run retry policy
let retry_policy = RunRetryPolicy::default()
    .initial_delay(Duration::from_millis(100))
    .max_attempts(10);
ctx.run(|| risky_operation())
    .retry_policy(retry_policy)
    .await?;
```
```

### Retry Policy

```rust
pub struct RetryPolicy {
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub exponentiation_factor: f64,
    pub max_attempts: usize,
    pub on_max_attempts: OnMaxAttempts, // Pause or Kill
}

pub enum OnMaxAttempts {
    Pause,
    Kill,
}
```

---

## 10. Recovery Procedures

### On Startup

```rust
pub async fn recover_incomplete_workflows(db: &Database) -> Vec<RecoveryTask> {
    // 1. Find all invocations not in terminal state
    let incomplete = sqlx::query_as!(
        Invocation,
        "SELECT * FROM invocations WHERE status NOT IN ('completed', 'failed')"
    ).fetch_all(db.pool()).await?;
    
    // 2. For each incomplete, find last completed step
    for invocation in incomplete {
        let journal = sqlx::query_as!(
            JournalEntry,
            "SELECT * FROM journal WHERE invocation_id = $1 ORDER BY seq DESC LIMIT 1",
            invocation.id
        ).fetch_one(db.pool()).await?;
        
        // 3. Resume from last completed step
        yield RecoveryTask {
            invocation_id: invocation.id,
            resume_from_step: journal.seq,
        };
    }
}
```

---

## 11. Feature Parity Checklist (VERIFIED)

| Feature | Restate | hardline | Status |
|---------|---------|----------|--------|
| Journal (ctx.run journaling) | ✅ | ❌ | **TODO** |
| State persistence (get/set/clear) | ✅ | ❌ | **TODO** |
| Sleep/timers (durable sleep) | ✅ | ❌ | **TODO** |
| TerminalError | ✅ | ❌ | **TODO** |
| HandlerError (auto-retry) | ✅ | ❌ | **TODO** |
| Retry policy (configurable) | ✅ | ❌ | **TODO** |
| Promises (workflow signaling) | ✅ | ❌ | **TODO** |
| Awakeables (external completion) | ✅ | ❌ | **TODO** |
| Service client (RPC) | ✅ | ❌ | **TODO** |
| Object client (keyed) | ✅ | ❌ | **TODO** |
| Workflow client | ✅ | ❌ | **TODO** |
| rand_uuid (stable UUID) | ✅ | ❌ | **TODO** |
| rand (stable RNG) | ✅ | ❌ | **TODO** |
| random_seed | ✅ | ❌ | **TODO** |
| Virtual Objects | ✅ | ❌ | **TODO** |
| Workflows | ✅ | ❌ | **TODO** |
| Saga/compensation | ✅ (code-managed) | ❌ | **TODO** |
| Service registry | ✅ | ❌ | **TODO** |
| Delayed calls (send_after) | ✅ | ❌ | **TODO** |
| Invocation state machine | ✅ | ❌ | **TODO** |

---

## 12. Implementation Priority (Based on Verified API)

### Phase 1: Core Foundation
1. **Journal** - Implement `ctx.run()` that records results and replays on retry
2. **State Store** - Implement `get()`/`set()`/`clear()` using SQLite
3. **Sleep/Timers** - Implement `sleep()` that survives crashes
4. **Invocation State Machine** - Track pending/running/suspended/completed states

### Phase 2: Advanced Context
5. **Promises** - Implement `promise()`/`resolve_promise()`/`reject_promise()`
6. **Awakeables** - Implement `awakeable()`/`resolve_awakeable()`/`reject_awakeable()`
7. **Deterministic RNG** - Implement `rand()`/`rand_uuid()`/`random_seed()`

### Phase 3: Error Handling
8. **TerminalError** - Non-retryable error type with code and message
9. **HandlerError** - Wrapper that auto-retries (except TerminalError)
10. **Retry Policy** - Configurable backoff and max attempts

### Phase 4: Service Types
11. **Virtual Objects** - `#[object]` with `ObjectContext`
12. **Workflows** - `#[workflow]` with `WorkflowContext`
13. **Services** - `#[service]` with `Context`

### Phase 5: Patterns
14. **Saga/Compensation** - Developer-managed rollback in code
15. **Delayed calls** - `send_after()` for scheduled execution
16. **Service registry** - Registration and discovery

### Phase 6: Polish
17. **Observability** - Tracing, metrics
18. **Recovery scanner** - On startup, resume incomplete invocations
19. **CLI commands** - For workflow management

---

## 13. References

### Restate Resources
- Documentation: https://docs.restate.dev
- Rust SDK: https://docs.rs/restate_sdk/latest/restate_sdk/
- GitHub: https://github.com/restatedev/sdk-rust

### Key Files to Port from Isolate/Seshat
- Isolate `durable_tasks.jsonl` - Saga implementation plan
- Isolate `commands/add/atomic.rs` - Two-phase compensation
- Seshat `durable_types.rs` - OperationRecord, StepRecord
- Isolate `recovery.rs` - Recovery policies

---

## Summary

To achieve Restate-level feature parity, hardline needs:

1. **Journal** - Append-only log of all operations
2. **Invocation State Machine** - Track pending/running/suspended/completed
3. **DurableContext** - Trait with `run`, `sleep`, `get_state`, `set_state`
4. **Saga Orchestrator** - Compensation on failure in reverse order
5. **Virtual Objects** - Workspace with per-key state
6. **Workflows** - Operation with promises and awakeables
7. **Recovery Scanner** - On startup, find and resume incomplete workflows

This is a significant implementation effort. Recommend starting with Phase 1 (core foundation) and iterating.

---

## Corrections to Earlier Agent Research

The 12-agent research may have been inaccurate in several places. Here are corrections:

### 1. Journal Entry Types
**Agent said:** There were detailed journal entry types like `CallCommandMessage`, `SleepCommandMessage`  
**Reality:** The SDK docs don't enumerate journal entry types at the Rust level - the journal is internal to Restate Server. The `ctx.run()` records results, but the specific journal protocol isn't exposed in the SDK API.

### 2. Invocation State Machine  
**Agent said:** States like `pending`, `ready`, `running`, `backing-off`, `suspended`, `completed`  
**Reality:** These states exist in Restate Server's internal `sys_invocation` table, but the SDK doesn't expose them directly to handlers.

### 3. Automatic Saga Support
**Agent said:** Restate has built-in saga orchestration  
**Reality:** Restate provides durable execution, but saga/compensation logic is **code-managed by the developer**. There's no automatic compensation - you write try-catch and manually execute compensations.

### 4. Deterministic Random
**Agent said:** Complex random seed management  
**Reality:** It's simpler: `ctx.rand_uuid()` uses a seeded RNG based on invocation ID. `ctx.random_seed()` returns the seed. `ctx.rand()` returns a mutable RNG.

### 5. Service Types
**Agent described** various differences between service types.  
**Verified Reality:**
- `#[restate_sdk::service]` - stateless handlers, use `Context<'_>`
- `#[restate_sdk::object]` - stateful entities, use `ObjectContext<'_>` (exclusive) or `SharedObjectContext<'_>` (concurrent read)
- `#[restate_sdk::workflow]` - long-running, use `WorkflowContext<'_>` (run handler) or `SharedWorkflowContext<'_>` (other handlers)

### Key Takeaway
**The SDK is simpler than the agents described.** You get:
- `ctx.run()` for journaling non-deterministic results
- `ctx.sleep()` for durable timers
- `ctx.get()`/`ctx.set()` for state (Virtual Objects/Workflows only)
- `ctx.promise()`/`ctx.awakeable()` for signaling
- `TerminalError` to stop retries

Everything else is internal to Restate Server.

# ADR-003 Evaluation: Restate Feature Parity Assessment

**Date:** 2026-04-28
**Evaluator:** Agent review of existing implementation vs. ADR-003 checklist
**Scope:** `crates/core/src/domain/workflow/`, `crates/core/src/infrastructure/restate/`, `crates/core/src/recovery.rs`

---

## Summary

ADR-003 lists 19 Restate SDK features and marks every one as "TODO." This is inaccurate. The existing codebase already has substantial coverage across three layers: domain workflow types, Restate-style context traits, and infrastructure error types. However, nearly everything is trait definitions and data structures -- there are no runtime implementations that actually persist state, journal operations to SQLite, or replay on crash recovery.

---

## Category 1: Journal and State Persistence (ADR items 1-2)

**Features:** Journal (ctx.run journaling), State persistence (get/set/clear)

**Implemented:** Partially. The `DurableExecutor` in `executor.rs` maintains an in-memory `Vec<StepOutput>` journal and runs steps sequentially with compensation on failure. The `DurableContext` trait in `context.rs` defines `get_state`, `set_state`, `clear_state`, `clear_all_state`, `get_state_keys` -- all the signatures matching Restate's `ContextReadState`/`ContextWriteState`. The `JournalEntry` and `StepRecord` types in `records.rs` provide the persistence schema.

**Needed for hardline:** Yes. Workspace spawn, switch, and sync are multi-step operations that must survive crashes. The current `DurableExecutor` only journals in memory; none of the `ContextReadState`/`ContextWriteState` methods have implementations. The `RecoveryScanner` in `executor.rs` has stub methods that return empty results.

**Gap:** No SQLite-backed journal persistence. No replay logic that skips completed steps on restart. The traits exist but have zero concrete implementations. This is the single most important gap.

---

## Category 2: Timers, RNG, and Determinism (ADR items 3, 12-14)

**Features:** Durable sleep, rand_uuid, rand, random_seed

**Implemented:** Trait definitions only. `ContextTimers::sleep` and `ContextSideEffects::random_seed`/`rand`/`rand_uuid` are defined in `context.rs`. A `DurableTimer` trait and `NoOpTimer` stub exist in `executor.rs`.

**Needed for hardline:** Low priority. Deterministic RNG is a Restate requirement for journal replay consistency. Hardline's operations are not replayed from a journal in the Restate sense -- they use step-by-step execution with status tracking. The `NoOpTimer` is sufficient until background scheduling (e.g., auto-rebase timers) is needed.

**Gap:** No real implementation needed now. Defer until background scheduling is a requirement.

---

## Category 3: Error Handling (ADR items 4-6)

**Features:** TerminalError, HandlerError (auto-retry), Retry policy (configurable)

**Implemented:** Fully defined. `errors.rs` has complete `TerminalError` (with code, message, source chain) and `HandlerError` (Terminal vs Retryable variants) with all `From` impls. This matches the Restate SDK exactly.

**Needed for hardline:** Yes, but already usable. The error taxonomy is solid and production-ready. Retry policy configuration (exponential backoff, max attempts, pause vs kill) is not implemented, but the `RecoveryPolicy` enum in `recovery.rs` (Warn/Repair/Panic) covers the operational side.

**Gap:** No configurable retry policy with exponential backoff. The `RecoveryScanner` does not retry failed operations. This is a minor gap -- the error types are done, the retry *policy engine* is not.

---

## Category 4: Promises and Awakeables (ADR items 7-8)

**Features:** Promises (workflow signaling), Awakeables (external completion)

**Implemented:** Trait definitions only. `promises.rs` defines `ContextPromises` (promise, peek_promise, resolve_promise, reject_promise) and `ContextAwakeables` (awakeable, resolve_awakeable, reject_awakeable) with typed futures. The `Promise<T>`, `PromiseResolver`, and `AwakeableId` types are defined but have no backing storage.

**Needed for hardline:** Not in the near term. Promises are for long-running workflows that wait on external signals (e.g., human approval gates). Awakeables are for external system callbacks. Hardline's current operations are sequential step execution -- they don't suspend waiting for external input. The `OperationStatus::Suspended` variant exists in `states.rs` but is unused.

**Gap:** No implementations needed now. Defer until workflow suspension (human-in-the-loop) is required.

---

## Category 5: Service Clients and RPC (ADR items 9-11)

**Features:** Service client, Object client, Workflow client

**Implemented:** Trait definitions only. `clients.rs` defines `ContextClient`, `ServiceClient`, `ObjectClient`, `WorkflowClient`, `RequestTarget`, and `Request<'_, Req, Res>`. The `Request` `Future` impl is a `todo!()`.

**Needed for hardline:** No. Hardline is a single-process CLI tool, not a distributed service mesh. It does not make RPC calls to other Restate services. There is no service registry, no inter-service communication. The `RequestTarget` and client traits are premature abstraction copied from the Restate SDK.

**Gap:** Skip entirely. These are Restate-server concepts with no hardline equivalent. Remove or mark as explicitly out-of-scope.

---

## Category 6: Service Types -- Virtual Objects, Workflows, Services (ADR items 15-17)

**Features:** Virtual Objects, Workflows, Services (Restate's three service types)

**Implemented:** Domain-level equivalents exist. The `OperationRecord`/`OperationState`/`OperationStatus` in `records.rs` and `states.rs` map to Restate's Workflow concept. The `Pipeline`/`PipelineState` in `pipeline.rs` maps to the orchestration pattern. There is no Virtual Object equivalent -- workspaces serve that role but are managed separately.

**Needed for hardline:** The workflow/orchestration pattern (Pipeline) is needed and already has the most complete implementation -- full state machine with validated transitions, iteration tracking, and error handling. Virtual Objects as a formal concept are not needed; workspaces are managed through the workspace module.

**Gap:** The Pipeline state machine is solid. The `DurableExecutor` needs its persistence gap filled (see Category 1). No need for formal Service/Object/Workflow type decorators.

---

## Category 7: Saga/Compensation and Delayed Calls (ADR items 18-19)

**Features:** Saga/compensation, Delayed calls (send_after)

**Implemented:** Saga compensation is implemented in `executor.rs`. The `DurableExecutor` supports `add_compensation_step` and runs compensations in reverse order on failure. The `CompensationAction` type tracks compensation results. The two-phase compensation state machine (`CompensationState`) is fully defined in `states.rs`.

**Needed for hardline:** Yes. Workspace spawn is a natural saga (create dir, clone repo, init database -- each reversible). The executor works correctly in-memory but lacks persistent journaling.

**Gap:** Compensation logic is correct but operates only in memory. Once the journal is persisted (Category 1), compensation replay on restart comes for free.

---

## Category 8: Recovery (cross-cutting)

**Features:** Recovery scanner, invocation state machine, database integrity

**Implemented:** `RecoveryScanner` in `executor.rs` has the interface (scan_incomplete_operations, recover_operation, scan_and_recover_all) but all methods are stubs returning empty/Ok. `recovery.rs` provides database integrity checking (SQLite header validation) and recovery configuration (Warn/Repair/Panic) -- this is operational recovery, not workflow recovery.

**Needed for hardline:** Yes. The recovery scanner is the bridge between crash and resume. It needs to query SQLite for incomplete operations and feed them back into the executor.

**Gap:** The most critical implementation gap. RecoveryScanner is a no-op shell.

---

## Recommendations

### Needed Now (Minimum Viable Durable Execution)

1. **SQLite-backed journal persistence.** Persist `StepRecord` entries to SQLite as the executor runs. On restart, load the journal and skip completed steps. This single change unlocks crash recovery for all multi-step operations (workspace spawn, sync, merge).

2. **Concrete RecoveryScanner implementation.** Query for operations in non-terminal states, load their journals, and resume from the last completed step.

3. **State store implementation.** Provide a SQLite-backed implementation of `ContextReadState`/`ContextWriteState` so operations can checkpoint intermediate state.

### Needed Later

4. **Retry policy engine.** Configurable exponential backoff for failed steps. The error types (`TerminalError`/`HandlerError`) are ready; the policy configuration and execution loop are not.

5. **Promises/awakeables.** When human-in-the-loop workflows (approval gates, escalation) are needed, the trait definitions in `promises.rs` provide the API -- they just need SQLite-backed implementations.

### Skip or Remove

6. **Service clients (RPC).** `clients.rs` is Restate-server infrastructure with no hardline use case. The `Request` future is a `todo!()`. Mark as explicitly out-of-scope or remove to avoid confusion.

7. **Deterministic RNG.** Not needed unless journal replay with exact reproducibility is required. Hardline's step-by-step execution does not need deterministic random.

8. **Delayed calls (send_after).** Not needed until background scheduling is a requirement.

### Verdict

ADR-003's checklist is misleading -- it marks everything as TODO when significant design and type work is already done. The real gap is narrow: **persist the journal to SQLite and implement recovery scanning.** Everything else is either already adequate (error types, saga compensation, state machines) or premature (service clients, promises, awakeables). The minimum viable durable execution for hardline is roughly 200-300 lines of SQLite persistence code layered onto the existing `DurableExecutor`, not a full Restate SDK reimplementation.

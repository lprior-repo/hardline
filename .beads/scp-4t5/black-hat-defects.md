# Black Hat Defects: scp-4t5

## 🔴 PHASE 1: Contract Violations

1. **P1 Incomplete Validation** (task_validation.rs:11-16)
   - Contract P1 requires: "Task ID must be valid (non-empty, alphanumeric with - or _)"
   - Actual: `validate_task_id()` only checks `task_id.is_empty()` - does NOT validate alphanumeric format
   - Violates: Test `test_task_show_returns_error_for_malformed_task_id` expects InvalidInput for "bad id!" but code accepts it

2. **State Persistence Broken** (task.rs:66-68)
   - CRITICAL: `get_task_store()` creates a NEW `Arc<TaskStore>` on EVERY call
   - Each CLI invocation loses all tasks - `list()`, `claim()`, `show()` operate on isolated stores
   - This breaks ALL postconditions - tasks cannot persist between operations

3. **Q1/Q3/Q4 Postconditions Violated** (task.rs:129-141)
   - When `list()` is called on empty store, it calls `init_demo_tasks()` to a NEW store, but subsequent `show()` calls create ANOTHER new store - demo tasks never visible to other operations

## 🟠 PHASE 2: Farley Rigor Flaws

1. **Functions Exceed 25 Lines** (task.rs:129-222)
   - `list()`: 14 lines - OK
   - `show()`: 14 lines - OK
   - `claim()`: 14 lines - OK
   - `yield_task()`: 14 lines - OK
   - `start()`: 14 lines - OK
   - `done()`: 14 lines - OK

2. **Global Mutable State in Functional Core** (task.rs:16-18, 20-64)
   - `TaskStore` with `RwLock<HashMap>` is imperative shell leaking into domain logic
   - Pure validation functions (`validate_*`, `transition_*`) should not depend on this

## 🟡 PHASE 3: Functional Rust Flaws (The Big 6)

1. **Mutation in Pure Functions** (task_validation.rs:64-97)
   - All `transition_*` functions use `let mut t = task` instead of returning new instances
   - Violates: "Data → Calc → Actions" pattern - calc should be pure, not mutate

2. **No Newtypes** (task_types.rs:19-28)
   - `id: String`, `title: String`, `priority: Option<String>`, `assignee: Option<String>`
   - All primitives - no type safety for domain concepts

3. **Parse Not Validate Not Enforced** (task_validation.rs:11-16)
   - Task ID validation happens at runtime, not at boundary parsing

## 🔵 PHASE 4: Simplicity & DDD Failures

1. **Primitive Obsession** (task_types.rs:19-28)
   - `String` for all domain fields instead of newtypes
   - No `TaskId`, `Title`, `Priority`, `Assignee` types

2. **Panic Vector Present** (task.rs:29-32)
   - `.unwrap_or_default()` on line 31 - silent failure on poisoned lock

3. **Option-Based State Machine** (task_types.rs:9-15)
   - `TaskState` is a proper enum - this is GOOD
   - But `assignee: Option<String>` (line 25) represents state implicitly

## 🟣 PHASE 5: The Bitter Truth (Cleverness & Bloat)

1. **Broken Architecture Disguised as Cleverness** (task.rs:66-72)
   - `get_task_store()` and `get_lock_manager()` create new instances each call
   - Appears "clean" with Arc/RwLock but is fundamentally broken
   - This is not "boring" - it's a catastrophic bug

2. **Demo Task Anti-Pattern** (task.rs:74-85, 129-138)
   - `init_demo_tasks()` populates store on first list() call
   - But each command creates new store - demo tasks only exist in that one call's memory space

3. **YAGNI Violation** (task_types.rs:9-15)
   - `TaskState::Blocked` and `TaskState::Deferred` defined but never used in this bead
   - Should be added when needed, not preemptively

## Summary

The code FAILS Phase 1 catastrophically due to the `get_task_store()` anti-pattern - every CLI invocation creates a fresh in-memory store, making task persistence impossible. This breaks the fundamental contract that tasks can be listed, claimed, and manipulated across operations. Additionally, P1 task ID validation is incomplete (missing alphanumeric format check). The code appears well-structured but has a critical architectural flaw that makes it non-functional.

STATUS: REJECTED

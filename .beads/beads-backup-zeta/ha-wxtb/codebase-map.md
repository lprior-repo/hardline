# Codebase Map for ha-wxtb: Unify duplicate Priority types

## Priority Types Found

### 1. `crates/queue/src/domain/job_priority.rs` — **TO ELIMINATE**
- `pub struct Priority(u8)` — newtype over u8, no validation (always Ok)
- Derives: Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize
- NO Ord/PartialOrd
- Methods: `new(u8) -> Result<Self, JobCreationError>`, `value() -> u8`, Display
- Only used by: `job.rs` (within the same job module)
- Re-exported via `domain/mod.rs` as `job::Priority` alongside other job types

### 2. `crates/queue/src/domain/value_objects/priority.rs` — **CANONICAL (KEEP)**
- `pub struct Priority(u8)` — newtype over u8, named presets
- Derives: Debug, Clone, Copy, PartialEq, Eq, **PartialOrd, Ord**, Serialize, Deserialize
- Methods: `new(u8) -> Self`, `low() -> Self(100)`, `normal() -> Self(200)`, `high() -> Self(230)`, `critical() -> Self(255)`, `value() -> u8`, `parse(u8) -> Result<Self, QueueError>`
- Implements: Default (normal=200), From<u8>, Display
- Used by: `entities/queue_entry.rs` (typestate QueueEntry), `application/queue_service.rs`, `infrastructure/queue_repository.rs`, `domain/tests/ports_tests.rs`

### 3. `crates/orchestrator/src/queue/types.rs` — **SEPARATE (NOT IN SCOPE)**
- `pub enum JobPriority { P0, P1, P2, P3, P4 }` — enum, not a newtype
- Completely different type in a different crate (orchestrator)
- Has its own Ord via derive, maps to u8 values 0-4
- **NOT a duplicate** — different domain model, leave alone

### 4. `MAX_PRIORITY` constant — **TO RESOLVE**
- `crates/queue/src/domain/queue/status.rs`: `pub const MAX_PRIORITY: u32 = 100`
- `crates/core/src/domain/queue/status.rs`: `pub const MAX_PRIORITY: u32 = 100`
- Used by: `queue/entry.rs`, `use_cases/queue_use_cases.rs` for validation (priority 0-100)
- **Conflict**: value_objects::Priority has low=100, normal=200, high=230 — all exceed MAX_PRIORITY=100

## Module Structure

```
crates/queue/src/domain/
├── mod.rs              — re-exports job::Priority (SHADOWS value_objects::Priority)
├── job.rs              — Job, JobQueue (uses job_priority::Priority)
├── job_priority.rs     — THE DUPLICATE to eliminate
├── job_id.rs           — JobId, QueueId, JobCreationError
├── job_status.rs       — JobStatus, QueueError
├── payload.rs          — Payload (JSON wrapper)
├── value_objects/
│   ├── mod.rs          — re-exports priority::Priority
│   └── priority.rs     — THE CANONICAL Priority
├── entities/
│   └── queue_entry.rs  — typestate QueueEntry (uses value_objects::Priority)
├── queue/
│   ├── mod.rs          — re-exports QueueEntry, QueueStatus, MAX_PRIORITY
│   ├── entry.rs        — simple QueueEntry (uses raw u32 + MAX_PRIORITY)
│   ├── status.rs       — QueueStatus enum + MAX_PRIORITY=100
│   ├── queue.rs        — Queue collection
│   └── validation.rs   — validate_range helper
└── identifiers.rs      — QueueEntryId, SessionName

crates/core/src/domain/queue/
├── mod.rs
├── entry.rs            — duplicate of queue/entry.rs (uses raw u32 + MAX_PRIORITY)
└── status.rs           — duplicate of queue/status.rs (MAX_PRIORITY=100)

crates/orchestrator/src/queue/
├── mod.rs
├── types.rs            — JobPriority enum (P0-P4), separate domain
├── processor.rs
└── repository.rs
```

## Impact Analysis

### Files to modify:
1. `crates/queue/src/domain/mod.rs` — remove job_priority re-export, remove job module re-exports
2. `crates/queue/src/domain/job_priority.rs` — DELETE
3. `crates/queue/src/domain/job.rs` — needs job_priority::Priority replaced (or module eliminated)
4. `crates/queue/src/domain/queue/status.rs` — MAX_PRIORITY alignment
5. `crates/core/src/domain/queue/status.rs` — MAX_PRIORITY alignment

### Files that import from job module (via domain/mod.rs re-exports):
- Only `job.rs` itself uses `job_priority::Priority`
- The re-export `pub use job::{..., Priority, ...}` in `domain/mod.rs` creates shadowing

### MAX_PRIORITY conflict resolution options:
- Option A: Raise MAX_PRIORITY to 255 (align with u8 range and value_objects presets)
- Option B: Keep MAX_PRIORITY=100 and adjust value_objects presets (low=25, normal=50, high=75, critical=100)
- **Recommendation**: Option A — raise MAX_PRIORITY to u8::MAX (255). The queue::QueueEntry uses u32 for priority anyway, and the constraint should match value_objects::Priority's full range.

## Existing Test Coverage
- `job_priority.rs` — 24 tests (unit + proptest) — ALL to be deleted with module
- `value_objects/priority.rs` — 14 tests — KEEP, may need expansion
- `job.rs` — 30+ tests using job_priority::Priority — will need updating if job module stays
- `queue/entry.rs` — 30+ tests using MAX_PRIORITY — will need updating
- `use_cases/queue_use_cases.rs` — 20+ tests using MAX_PRIORITY — will need updating

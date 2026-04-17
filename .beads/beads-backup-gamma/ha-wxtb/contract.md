# Contract: ha-wxtb — Unify duplicate Priority types

bead_id: ha-wxtb
bead_title: Unify duplicate Priority types — eliminate job_priority::Priority and align priority scales
phase: 3
updated_at: 2026-04-17T00:00:00Z

## Types

### Canonical Priority (KEEP)
```rust
// crates/queue/src/domain/value_objects/priority.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(u8);
// Methods: new(u8), low()=100, normal()=200, high()=230, critical()=255, value()->u8, parse(u8)->Result
// Traits: Default (normal), From<u8>, Display
```

### Eliminated
- `crates/queue/src/domain/job_priority::Priority` — DELETE entire file
- `crates/queue/src/domain/mod.rs` re-export of `job::Priority` — REMOVE from pub use

### Resolved
- `MAX_PRIORITY: u32 = 100` → change to `255` in both:
  - `crates/queue/src/domain/queue/status.rs`
  - `crates/core/src/domain/queue/status.rs`

## Invariants

1. **Single Priority type**: After this change, `domain::value_objects::Priority` is the ONLY Priority type in the `queue` crate
2. **No orphan references**: No code references `job_priority::Priority` after deletion
3. **Ordinal semantics preserved**: value_objects::Priority's Ord (higher u8 = higher priority) is unchanged
4. **MAX_PRIORITY alignment**: MAX_PRIORITY=255 covers the full u8 range, compatible with all value_objects presets (low=100, normal=200, high=230, critical=255)
5. **No behavioral change**: The queue::QueueEntry uses raw u32 priority — raising MAX_PRIORITY from 100 to 255 only relaxes the constraint, doesn't change existing valid priorities
6. **job module intact**: The `job` module (job.rs, job_id.rs, job_status.rs, payload.rs) stays — only `job_priority.rs` is deleted. job.rs's internal usage of Priority switches to value_objects::Priority.

## Preconditions

- `job_priority.rs` exists at expected path
- `value_objects/priority.rs` exists with Ord derive
- `MAX_PRIORITY` constant exists in both status.rs files

## Postconditions

- `job_priority.rs` deleted
- `domain/mod.rs` no longer re-exports `job::Priority`
- `job.rs` imports from `value_objects::Priority` instead
- `MAX_PRIORITY = 255` in both queue status modules
- All tests pass
- No new clippy warnings

## Error Taxonomy

- **Compile Error**: If job.rs references are broken after job_priority removal
- **Test Failure**: If MAX_PRIORITY change breaks validation tests that expected rejection at 101

## Scope (what this bead does NOT do)

- Does NOT eliminate the job module (that's ha-omw8)
- Does NOT touch orchestrator's JobPriority enum (different domain)
- Does NOT change value_objects::Priority's preset values
- Does NOT refactor queue::QueueEntry to use value_objects::Priority instead of raw u32

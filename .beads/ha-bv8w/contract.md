---
bead_id: ha-bv8w
bead_title: Fix broken InMemoryQueueRepository in infrastructure — mutations are lost
phase: contract
updated_at: 2026-04-17T03:15:00Z
---

# Contract: Remove Dead infrastructure::queue_repository

## Problem
`crates/queue/src/infrastructure/queue_repository.rs` contains:
1. A duplicate `QueueRepository` trait (identical to `domain::ports::QueueRepository`)
2. A broken `InMemoryQueueRepository` using `VecDeque` with `&self` — clone-modify-discard means ALL mutations are silently lost
3. Tests that cannot detect the bug (each test creates a fresh repo, never verifies cross-call persistence)

## Resolution
**Delete the file entirely.** The canonical implementation lives in `domain/ports.rs` and is correctly used by all consumers. The infrastructure file is dead code — zero imports reference it.

## Invariants (Post-Change)
- I1: Single `QueueRepository` trait definition in the crate (in `domain/ports.rs`)
- I2: Single `InMemoryQueueRepository` implementation (in `domain/ports.rs`, uses `Arc<Mutex<VecDeque>>`)
- I3: All existing tests continue to pass (they use `domain::ports::InMemoryQueueRepository`)
- I4: `infrastructure/mod.rs` has no reference to deleted module

## Preconditions
- None (file is dead code)

## Postconditions
- `infrastructure/queue_repository.rs` does not exist
- `moon run :ci` passes

## Error Taxonomy
N/A — this is a dead-code deletion, no new error paths introduced.

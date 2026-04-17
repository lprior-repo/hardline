# Codebase Map for ha-bv8w

## Bug Summary
infrastructure/queue_repository.rs has a BROKEN InMemoryQueueRepository using plain VecDeque with &self — all mutations are silently lost (clone-modify-discard pattern).
domain/ports.rs has a CORRECT InMemoryQueueRepository using Arc<Mutex<VecDeque>>.

## Key Files

### The Broken Implementation
- `crates/queue/src/infrastructure/queue_repository.rs` — defines BOTH QueueRepository trait AND broken InMemoryQueueRepository with VecDeque (no interior mutability). Tests in this file would NEVER catch the mutation-loss bug because each test creates a fresh repo.

### The Correct Implementation
- `crates/queue/src/domain/ports.rs` — defines QueueRepository trait AND correct InMemoryQueueRepository with Arc<Mutex<VecDeque>>. Has Clone impl, with_entries() test helper, proper mutex poisoning handling.

### Module Wiring
- `crates/queue/src/domain/mod.rs` — re-exports InMemoryQueueRepository from ports
- `crates/queue/src/infrastructure/mod.rs` — re-exports QueueRepository from domain (canonical), NOT infrastructure trait
- `crates/queue/src/lib.rs` — re-exports from domain (InMemoryQueueRepository, QueueRepository, QueueEntry, etc.)

### Consumers
- `crates/queue/src/application/queue_service.rs` — uses domain::ports::{InMemoryQueueRepository, QueueRepository}
- `crates/queue/src/domain/tests/ports_tests.rs` — comprehensive tests for domain InMemoryQueueRepository (uses Arc<Mutex> version)

### Duplicate Trait Problem
The infrastructure/queue_repository.rs defines its own QueueRepository trait (lines 5-13) that duplicates domain::ports::QueueRepository. The infrastructure/mod.rs re-exports the domain version, making the infrastructure trait unreachable from external code. However, the infrastructure file still defines both the trait AND the broken implementation.

## Resolution Strategy
The infrastructure/queue_repository.rs file should:
1. Remove the duplicate QueueRepository trait (use domain::ports version)
2. Remove the broken InMemoryQueueRepository (use domain::ports version)
3. Keep only the sqlite_migration code (if any exists — checking: it doesn't in this file)
4. The file may become empty or can be deleted entirely

Actually: infrastructure/queue_repository.rs ONLY contains the trait + broken impl + tests. It can be deleted entirely since infrastructure/mod.rs already re-exports from domain.

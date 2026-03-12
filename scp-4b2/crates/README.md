# SCP Domain-Driven Design Structure

This document describes the Domain-Driven Design (DDD) architecture used across the SCP crates, following Scott Wlaschin's functional DDD patterns.

## Overview

The codebase follows a three-layer architecture:

```
┌─────────────────────────────────────────────────────┐
│                  Application Layer                   │
│              (Use Cases / Services)                  │
├─────────────────────────────────────────────────────┤
│                    Domain Layer                      │
│         (Entities, Value Objects, Events)           │
├─────────────────────────────────────────────────────┤
│                Infrastructure Layer                 │
│            (Repositories, External Services)         │
└─────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### Domain Layer

The domain layer is the core of the application. It contains:

- **Entities**: Objects with identity that persists over time
- **Value Objects**: Immutable objects defined by their attributes
- **Events**: Domain events representing something that happened
- **State Machines**: Explicit state transitions with validation

#### Entities

Entities in this codebase follow Scott Wlaschin's pattern:
- Contain an identity (`Id` type)
- Have methods that return new instances (immutable updates)
- Validate state transitions explicitly
- Include business rules within the entity

Example from `queue/src/domain/entities/queue_entry.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub session_id: String,
    pub status: QueueStatus,
    // ...
}

impl QueueEntry {
    pub fn enqueue(session_id: String, ...) -> Self { ... }
    
    pub fn claim(&self) -> Result<Self, QueueError> {
        if self.status != QueueStatus::Pending {
            return Err(QueueError::InvalidStateTransition {...});
        }
        Ok(self.transition_to(QueueStatus::Claimed))
    }
}
```

Key principles:
- Methods return `Result` for fallible operations
- State transitions are validated before occurring
- Immutable updates (each method returns a new instance)
- Illegal states are unrepresentable

#### Value Objects

Value objects are immutable and defined by their attributes rather than identity:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(value: u8) -> Self { Self(value) }
    pub fn low() -> Self { Self(100) }
    pub fn normal() -> Self { Self(200) }
    pub fn high() -> Self { Self(300) }
    pub fn critical() -> Self { Self(255) }
}
```

Key principles:
- No identity - two value objects with same attributes are equal
- Immutable after creation
- Include validation in constructors
- Provide factory methods for common values

#### Domain Events

Events represent something that happened in the domain:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueEvent {
    EntryEnqueued { entry_id: QueueEntryId },
    EntryClaimed { entry_id: QueueEntryId },
    EntryMerged { entry_id: QueueEntryId },
    EntryFailed { entry_id: QueueEntryId, error: String },
}
```

### Application Layer

The application layer orchestrates domain objects and contains use cases. It:

- Depends only on the domain layer
- Contains services that coordinate domain operations
- Does not contain business logic (that's in the domain)
- Defines ports/interfaces for infrastructure

Example from `queue/src/application/queue_service.rs`:

```rust
pub struct QueueService;

impl QueueService {
    pub fn enqueue(session_id: String, bead_id: Option<String>, priority: Priority) -> QueueEntry {
        QueueEntry::enqueue(session_id, bead_id, priority)
    }

    pub fn claim(entry: &QueueEntry) -> Result<QueueEntry> {
        entry.claim()
    }

    pub fn process_entry(entry: &QueueEntry) -> Result<QueueEntry> {
        let claimed = entry.claim()?;
        let rebasing = claimed.start_rebase()?;
        rebasing.start_testing()
    }
}
```

Key principles:
- Thin layer - delegates to domain entities
- Contains application-specific logic (workflows, orchestration)
- Defines the public API for the domain
- Returns domain types, not infrastructure types

### Infrastructure Layer

The infrastructure layer implements interfaces defined in the application/domain layers:

- **Repositories**: Persist and retrieve domain entities
- **External Services**: Git, Jujutsu, HTTP clients
- **Serialization**: JSON, TOML handling

Example from `beads/src/infrastructure/repository.rs`:

```rust
#[async_trait]
pub trait BeadRepository: Send + Sync {
    async fn insert(&self, bead: &Bead) -> Result<()>;
    async fn update(&self, bead: &Bead) -> Result<()>;
    async fn delete(&self, id: &BeadId) -> Result<()>;
    async fn find(&self, id: &BeadId) -> Result<Option<Bead>>;
    async fn find_all(&self) -> Result<Vec<Bead>>;
    async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>>;
    async fn exists(&self, id: &BeadId) -> bool;
}

pub struct InMemoryBeadRepository { ... }
```

Key principles:
- Implement trait-defined interfaces
- Concrete implementations for persistence, external APIs
- Can be swapped (e.g., in-memory vs. database repository)
- Handle technical concerns (connection pooling, serialization)

## Error Handling

Following functional DDD patterns, errors are represented as explicit types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Invalid queue entry id: {0}")]
    InvalidQueueEntryId(String),
    
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
    
    #[error("Invalid priority: {0}")]
    InvalidPriority(String),
}
```

The `Result` type is used throughout:
- Domain methods return `Result<Self, Error>`
- Application services return `Result<DomainType, Error>`
- Infrastructure implements `Result` for persistence operations

## Module Structure

Each crate follows this structure:

```
crate/
├── src/
│   ├── domain/
│   │   ├── mod.rs          # Domain exports
│   │   ├── entities/       # Entity types
│   │   ├── value_objects/  # Value object types
│   │   ├── events/         # Domain events
│   │   └── state/          # State machines
│   ├── application/
│   │   ├── mod.rs          # Application service exports
│   │   └── *_service.rs    # Use case implementations
│   ├── infrastructure/
│   │   ├── mod.rs          # Infrastructure exports
│   │   └── *_repository.rs # Repository implementations
│   ├── error.rs            # Error types
│   └── lib.rs              # Crate root
```

## Key DDD Principles Applied

1. **Make Illegal States Unrepresentable**: Types enforce invariants
2. **Parse, Don't Validate**: Validation at construction time
3. **Command Query Separation**: Read and write operations are distinct
4. **Domain Objects Are Pure**: No side effects in domain logic
5. **Explicit State Transitions**: Every change goes through validated methods
6. **Dependencies Point Inward**: Infrastructure depends on domain, not vice versa

## Testing Strategy

- **Domain**: Unit tests for entities, value objects, state machines
- **Application**: Tests for service orchestration
- **Infrastructure**: Integration tests for repository implementations

Tests are co-located with the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn queue_entry_when_created_then_has_pending_status() {
        let entry = QueueEntry::enqueue("session-1".into(), None, Priority::default());
        assert_eq!(entry.status, QueueStatus::Pending);
    }
}
```

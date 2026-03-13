# Contract Specification

## Context
- **Feature**: SessionRepository trait in domain layer
- **Bead ID**: scp-snt
- **Domain terms**: 
  - `Session` - aggregate root for session data
  - `SessionId` - validated unique identifier (non-empty ASCII)
  - `SessionName` - validated human-readable name (1-63 chars, starts with letter, alphanumeric/hyphen/underscore)
  - `BranchState` - session branch state (detached or on branch)
  - `RepositoryError` - common error type for all repository operations
- **Assumptions**:
  - SessionId and SessionName are validated at construction (type-system enforced)
  - Repository implementations live in infrastructure layer
  - Trait is sync (not async) - implementations handle async internally
- **Open questions**: None - domain types fully defined

## Preconditions

| ID | Precondition | Enforcement Level |
|----|--------------|-------------------|
| P1 | `load(id: &SessionId)` - id must be valid | Compile-time: `SessionId::parse()` returns `Result`, callers must handle |
| P2 | `load_by_name(name: &SessionName)` - name must be valid | Compile-time: `SessionName::parse()` returns `Result`, callers must handle |
| P3 | `save(session: &Session)` - session must be valid | Compile-time: `Session` constructed via validated `SessionId`/`SessionName` |
| P4 | `delete(id: &SessionId)` - id must be valid | Compile-time: `SessionId::parse()` returns `Result` |
| P5 | `set_current(id: &SessionId)` - id must be valid | Compile-time: `SessionId::parse()` returns `Result` |
| P6 | `list_all()` - no preconditions | N/A (pure read) |
| P7 | `list_sorted_by_name()` - no preconditions | N/A (pure read) |
| P8 | `exists(id: &SessionId)` - id must be valid | Compile-time: `SessionId::parse()` |
| P9 | `get_current()` - no preconditions | N/A (pure read) |
| P10 | `clear_current()` - no preconditions | N/A (pure state change) |

## Postconditions

| ID | Postcondition | Enforcement Level |
|----|---------------|-------------------|
| Q1 | `load(id)` returns `Ok(session)` where `session.id == *id` | Runtime: implementation contract |
| Q2 | `load_by_name(name)` returns `Ok(session)` where `session.name == *name` | Runtime: implementation contract |
| Q3 | `save(session)` - session exists in repository after call | Runtime: implementation contract |
| Q4 | `save(session)` - if new ID, creates; if existing ID, updates | Runtime: implementation contract |
| Q5 | `delete(id)` - session with ID no longer exists | Runtime: implementation contract |
| Q6 | `delete(id)` - returns `Ok(())` even if session didn't exist (idempotent) OR returns `NotFound` | Runtime: implementation decision documented |
| Q7 | `list_all()` - returns all sessions | Runtime: implementation contract |
| Q8 | `list_sorted_by_name()` - returns all sessions sorted ascending by name | Runtime: implementation contract |
| Q9 | `exists(id)` - returns `true` iff session with ID exists | Runtime: implementation contract |
| Q10 | `get_current()` - returns `Some(session)` if current set, else `None` | Runtime: implementation contract |
| Q11 | `set_current(id)` - after call, `get_current()` returns `Some(session)` with that ID | Runtime: implementation contract |
| Q12 | `clear_current()` - after call, `get_current()` returns `None` | Runtime: implementation contract |

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| I1 | No two sessions can have the same SessionName | Runtime: `save` returns `Conflict` if duplicate name |
| I2 | No two sessions can have the same SessionId | Runtime: `save` acts as upsert by ID |
| I3 | Repository remains in consistent state after any operation | Runtime: implementation must be atomic |

## Error Taxonomy

All repository operations return `Result<T, RepositoryError>`:

```rust
pub enum RepositoryError {
    /// Entity not found in repository
    NotFound(String),
    /// Conflict with existing data (duplicate name, constraint violation)
    Conflict(String),
    /// Invalid input for domain operation  
    InvalidInput(String),
    /// Underlying storage failure (corruption, permissions, I/O)
    StorageError(String),
    /// Operation not supported by repository implementation
    NotSupported(String),
    /// Concurrent modification conflict
    ConcurrentModification(String),
}
```

### Error Mapping by Operation

| Operation | Success | Failure Conditions |
|-----------|---------|-------------------|
| `load(id)` | `Ok(Session)` | `NotFound` - ID doesn't exist; `StorageError` - I/O failure |
| `load_by_name(name)` | `Ok(Session)` | `NotFound` - name doesn't exist; `StorageError` - I/O failure |
| `save(session)` | `Ok(())` | `Conflict` - duplicate name (new session); `InvalidInput` - invalid session data; `StorageError` - write failure |
| `delete(id)` | `Ok(())` | `NotFound` - ID doesn't exist (non-idempotent); `StorageError` - deletion failure |
| `list_all()` | `Ok(Vec<Session>)` | `StorageError` - read failure |
| `list_sorted_by_name()` | `Ok(Vec<Session>)` | `StorageError` - read failure |
| `exists(id)` | `Ok(bool)` | `StorageError` - I/O failure |
| `get_current()` | `Ok(Option<Session>)` | `StorageError` - read failure |
| `set_current(id)` | `Ok(())` | `NotFound` - session ID doesn't exist; `StorageError` - write failure |
| `clear_current()` | `Ok(())` | `StorageError` - write failure |

## Contract Signatures

```rust
pub trait SessionRepository: Send + Sync {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session>;
    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session>;
    fn save(&self, session: &Session) -> RepositoryResult<()>;
    fn delete(&self, id: &SessionId) -> RepositoryResult<()>;
    fn list_all(&self) -> RepositoryResult<Vec<Session>>;
    fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>>;
    fn exists(&self, id: &SessionId) -> RepositoryResult<bool>;
    fn get_current(&self) -> RepositoryResult<Option<Session>>;
    fn set_current(&self, id: &SessionId) -> RepositoryResult<()>;
    fn clear_current(&self) -> RepositoryResult<()>;
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| SessionId valid | Compile-time | `SessionId::parse(s) -> Result<Self, IdentifierError>` |
| SessionName valid | Compile-time | `SessionName::parse(s) -> Result<Self, IdentifierError>` |
| Session valid | Compile-time | Constructed via validated components |
| Repository state consistent | Debug-only | `debug_assert!(check_invariants(self))` |
| Storage available | Runtime | `Result<T, RepositoryError::StorageError>` |

## Violation Examples

### Precondition Violations

- **VIOLATES P1**: `repo.load(&SessionId::parse("").unwrap_err())` - compile-time: caller must handle parse error, can't pass invalid ID
- **VIOLATES P2**: `repo.load_by_name(&SessionName::parse("123-invalid").unwrap_err())` - compile-time: caller must handle parse error
- **VIOLATES P3**: `save(&Session { id: invalid_id, ... })` - compile-time: Session construction requires valid ID
- **VIOLATES P4**: `repo.delete(&SessionId::parse("").unwrap_err())` - compile-time: same as P1

### Postcondition Violations

- **VIOLATES Q1**: `repo.load(&valid_id)` returns `Ok(session)` where `session.id != *id` - returns wrong session
- **VIOLATES Q3**: `repo.save(&session)` then `repo.exists(&session.id)` returns `false` - save didn't persist
- **VIOLATES Q5**: `repo.delete(&id)` returns `Ok(())` but `repo.exists(&id)` returns `true` - delete didn't remove

### Invariant Violations

- **VIOLATES I1**: After `save(new_session)`, calling `save(other_session_with_same_name)` succeeds - duplicate names allowed
- **VIOLATES I2**: After operations, repository in inconsistent state (e.g., list_all returns partial results)

## Ownership Contracts

| Parameter | Ownership Model | Mutates | Rationale |
|-----------|----------------|---------|-----------|
| `id: &SessionId` | Shared borrow | No | Read-only lookup key |
| `name: &SessionName` | Shared borrow | No | Read-only lookup key |
| `session: &Session` | Shared borrow | No | Read-only data for save |
| `self` | Shared reference | No | Repository is immutable facade to storage |

**Clone Policy**: Domain types (`SessionId`, `SessionName`, `Session`) implement `Clone`. Repository trait uses borrow (`&T`) to avoid unnecessary cloning. Callers clone if they need ownership.

## Non-goals
- [ ] Async variants (use `async_trait` at implementation site if needed)
- [ ] Caching layer (implementation detail)
- [ ] Transaction support (future enhancement)
- [ ] Migration between storage backends (future enhancement)

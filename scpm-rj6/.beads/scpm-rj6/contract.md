# Contract Specification

## Context
- Feature: Session Repository - SQLite persistence layer
- Bead ID: scpm-rj6
- Domain terms: Session, SessionRepository, SqliteSessionRepository
- Assumptions: SqliteDatabaseService exists and provides async SQLite with WAL mode

## Preconditions
- P1: SqliteDatabaseService must be initialized and connected before repository operations
- P2: Database schema must be migrated (sessions table created)
- P3: Session ID must be valid (non-empty, valid UUID format)
- P4: Session name must be valid (parseable as SessionName)

## Postconditions
- Q1: Session is persisted to SQLite database and immediately queryable
- Q2: Database connections are returned to pool after operation completes
- Q3: All session fields (id, name, workspace, bead, branch, state, created_at) are correctly stored and retrieved
- Q4: Save operations are serialized via WAL mode (SqliteDatabaseService guarantee)

## Invariants
- I1: All sessions must have a valid UUID as their primary key (SessionId)
- I2: Only one SqliteDatabaseService writer instance exists per process (SqliteDatabaseService guarantee)
- I3: Session state transitions are validated per SessionState transition rules

## Error Taxonomy
```
Error::NotFound(String)           - Session does not exist in database
Error::AlreadyActive(String)     - Session already exists with that name
Error::SessionExpired(String)    - Session has expired (if TTL implemented)
Error::InvalidTransition         - Invalid state transition attempted
Error::RepositoryError(String)   - Database operation failed
Error::DatabaseError(String)     - SQLite/database layer error
Error::SerializationError(String) - JSON serialization failed
Error::InvalidIdentifier(String) - Session ID validation failed
```

## Contract Signatures
```rust
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), SessionError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Session>, SessionError>;
    async fn find_by_name(&self, name: &SessionName) -> Result<Option<Session>, SessionError>;
    async fn list(&self) -> Result<Vec<Session>, SessionError>;
    async fn delete(&self, id: &str) -> Result<(), SessionError>;
}
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| P1: DB connected | Runtime | Result error on failed connection |
| P2: Schema migrated | Runtime | Result error if table doesn't exist |
| P3: Valid session ID | Compile-time | SessionId newtype validates on parse |
| P4: Valid session name | Compile-time | SessionName newtype validates on parse |
| Q1: Immediate queryability | Runtime | sqlx query returns after commit |
| Q2: Connection return | Runtime | sqlx returns connection to pool on drop |

## Violation Examples
- VIOLATES P1: `save()` on uninitialized SqliteDatabaseService -> `Err(SessionError::RepositoryError("database not initialized"))`
- VIOLATES P2: `save()` before migration -> `Err(SessionError::DatabaseError("no such table: sessions"))`
- VIOLATES P3: `find_by_id("")` with empty string -> `Err(SessionError::InvalidIdentifier("SessionId cannot be empty"))`
- VIOLATES P4: `find_by_name(SessionName::parse("").unwrap())` -> `Err(IdentifierError::Empty)`
- VIOLATES Q1: `save()` followed by immediate `find_by_id()` returns `None` -> indicates write failure (not expected with WAL)
- VIOLATES Q2: Connection leak if pool exhausted -> `Err(SessionError::RepositoryError("connection pool exhausted"))`

## Ownership Contracts
- `save(&self, session: &Session)` - borrows Session immutably, no ownership transfer
- `find_by_id(&self, id: &str)` - borrows str for lookup key
- `find_by_name(&self, name: &SessionName)` - borrows SessionName for lookup
- `list(&self)` - returns owned Vec<Session> clone
- `delete(&self, id: &str)` - borrows str for deletion key, no mutation of Session

## Non-goals
- Connection pool management (handled by SqliteDatabaseService)
- Session lifecycle management (handled by Session domain entity)
- State machine transitions (handled by Session::transition)

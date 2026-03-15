# Contract Specification

## Context
- **Feature**: SQLite migration for `queue_entries` table
- **Bead ID**: scp-09f
- **Domain terms**:
  - `QueueEntry` - an item in the merge queue with lifecycle states
  - `QueueEntryId` - unique identifier (format: `queue-{uuid}`)
  - `QueueStatus` - state machine: Pending, Claimed, Rebasing, Testing, ReadyToMerge, Merging, Merged, FailedRetryable, FailedTerminal, Cancelled
  - `Priority` - u8 value (0-255, lower = higher priority)
  - `QueuePosition` - ordering value within same priority
- **Assumptions**:
  - SQLite is the target database (existing pattern in project)
  - Migration must be idempotent/reversible
  - Table supports the domain model defined in `crates/queue/src/domain/entities/queue_entry.rs`
- **Open questions**:
  - What is the database connection/discovery mechanism? (assumed: existing SQLite setup in project)
  - Are there existing migrations to follow as pattern? (none found)

## Preconditions
- **[P1]** SQLite database connection is valid and writable
  - **Enforcement Level**: Runtime (Result<T, Error::DatabaseError>)
  - **Violation Example**: `rusqlite::Connection::open("/nonexistent/path.db")` -> `Err(rusqlite::Error::InvalidPath)`

- **[P2]** No existing `queue_entries` table with conflicting schema
  - **Enforcement Level**: Runtime (Result<T, Error::MigrationError>)
  - **Violation Example**: Running migration twice -> `Err(Error::MigrationError("table already exists"))`

- **[P3]** Migration SQL is syntactically valid SQLite DDL
  - **Enforcement Level**: Compile-time validation (Rust compile + integration test)
  - **Violation Example**: Invalid SQL like `CREAT TABLE` (typo) -> SQLite syntax error at migration time

## Postconditions
- **[Q1]** Table `queue_entries` exists with all required columns
  - Columns: `id` (TEXT PRIMARY KEY), `session_id` (TEXT NOT NULL), `bead_id` (TEXT), `priority` (INTEGER NOT NULL), `position` (INTEGER NOT NULL), `status` (TEXT NOT NULL), `enqueued_at` (TEXT NOT NULL), `updated_at` (TEXT NOT NULL), `retry_count` (INTEGER NOT NULL DEFAULT 0), `error_message` (TEXT)
  - **Enforcement Level**: Runtime (verify via `SELECT * FROM sqlite_master`)
  - **Violation Example**: After migration, `SELECT name FROM pragma_table_info('queue_entries')` should contain all columns

- **[Q2]** Primary key constraint on `id` column enforces uniqueness
  - **Enforcement Level**: Runtime (database constraint)
  - **Violation Example**: Insert two entries with same ID -> `Err(DatabaseError::UniqueConstraintViolation)`

- **[Q3]** `session_id` column is NOT NULL
  - **Enforcement Level**: Runtime (database constraint)
  - **Violation Example**: Insert entry with NULL session_id -> `Err(DatabaseError::NotNullConstraintViolation)`

- **[Q4]** `priority` column has default value of 128 (medium priority)
  - **Enforcement Level**: Runtime (database constraint)
  - **Violation Example**: Insert without priority -> row has correct default

- **[Q5]** Indexes exist for common query patterns
  - Index on `(status, priority, position)` for dequeue operations
  - Index on `(session_id)` for session lookups
  - **Enforcement Level**: Runtime (verify via `SELECT * FROM pragma_index_list('queue_entries')`)

## Invariants
- **[I1]** Every row must have a valid status value from the QueueStatus enum
  - **Enforcement Level**: Runtime (CHECK constraint on status column)
  - **Violation Example**: Insert status 'InvalidStatus' -> `Err(DatabaseError::CheckConstraintViolation)`

- **[I2]** retry_count must be >= 0
  - **Enforcement Level**: Runtime (CHECK constraint)
  - **Violation Example**: Insert retry_count = -1 -> `Err(DatabaseError::CheckConstraintViolation)`

- **[I3]** priority must be 0-255
  - **Enforcement Level**: Runtime (CHECK constraint)
  - **Violation Example**: Insert priority = 256 -> `Err(DatabaseError::CheckConstraintViolation)`

## Error Taxonomy
```rust
#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Database connection failed: {0}")]
    DatabaseError(String),
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Table already exists with incompatible schema")]
    SchemaConflict,
    
    #[error("Invalid migration: {0}")]
    InvalidMigration(String),
    
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}
```

## Contract Signatures
```rust
/// Runs all queue_entries migrations
/// Returns Ok(()) on success, Err(MigrationError) on failure
fn run_migrations(connection: &rusqlite::Connection) -> Result<(), MigrationError>;

/// Verifies the migration was applied correctly
/// Returns Ok(true) if table exists with correct schema
fn verify_migration(connection: &rusqlite::Connection) -> Result<bool, MigrationError>;

/// Rolls back the migration (drops table)
fn rollback_migration(connection: &rusqlite::Connection) -> Result<(), MigrationError>;
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Database connection valid | Runtime | `Result<Connection, rusqlite::Error>` |
| No conflicting table | Runtime | Check `sqlite_master` before CREATE |
| Valid SQL syntax | Compile-time | Rust compile + test execution |
| Unique id constraint | Runtime | PRIMARY KEY in DDL |
| session_id NOT NULL | Runtime | NOT NULL constraint in DDL |
| Valid status values | Runtime | CHECK constraint in DDL |
| Priority bounds | Runtime | CHECK constraint in DDL |

## Violation Examples (REQUIRED)
- **VIOLATES P1**: `rusqlite::Connection::open("/invalid/path.db")` -> `Err(MigrationError::DatabaseError("no such file or directory"))`
- **VIOLATES P2**: Run migration twice on fresh database -> `Err(MigrationError::SchemaConflict("table queue_entries already exists"))`
- **VIOLATES Q1**: After migration, query `SELECT sql FROM sqlite_master WHERE name='queue_entries'` returns incorrect column list
- **VIOLATES Q2**: Insert duplicate IDs -> `UNIQUE constraint failed: queue_entries.id`
- **VIOLATES Q3**: Insert NULL session_id -> `NOT NULL constraint failed: queue_entries.session_id`
- **VIOLATES I1**: Insert status 'InvalidStatus' -> `CHECK constraint failed: status in ('Pending','Claimed',...)`
- **VIOLATES I2**: Insert retry_count = -1 -> `CHECK constraint failed: retry_count >= 0`
- **VIOLATES I3**: Insert priority = 300 -> `CHECK constraint failed: priority >= 0 AND priority <= 255`

## Ownership Contracts (Rust-specific)
- `connection: &rusqlite::Connection` - shared borrow, read-only queries + write operations on same connection
- No ownership transfer - connection lifecycle managed by caller
- Migration function does not clone or store the connection

## Non-goals
- [ ] Schema migration between versions (v1 -> v2) - not in scope
- [ ] Data migration/transform from other formats - not in scope
- [ ] Multi-database support (PostgreSQL, MySQL) - SQLite only
- [ ] Migration history table - future work

# Contract Specification

## Context
- **Feature**: Create SQLite schema for sessions
- **Bead ID**: scp-6n6
- **Description**: Create SQL migration for sessions table
- **Domain terms**:
  - `Session`: Represents a single execution context/workspace
  - `Migration`: Versioned schema change for database evolution
  - `SessionStatus`: Lifecycle state of a session (creating, active, completed, failed)
  - `SessionState`: Detailed state within the lifecycle
- **Assumptions**:
  - Project uses sqlx for SQLite operations
  - Migrations follow a sequential versioned approach
  - Sessions table is the core persistence layer
- **Open questions**:
  - What additional session metadata fields are required?
  - Should migrations be auto-discovered or explicitly listed?
  - Is there an existing migration framework (sqlx-migrate)?

---

## Preconditions

- **P1**: `migration_version > 0` - Migration version must be positive integer
- **P2**: `db_connection.is_valid()` - Database connection must be valid before migration
- **P3**: `!table_exists("sessions")` - Sessions table must not already exist (idempotent migrations handle this via IF NOT EXISTS)
- **P4**: `migration_name.is_valid_identifier()` - Migration name must be valid SQL identifier

---

## Postconditions

- **Q1**: After successful migration, `sessions` table exists in database schema
- **Q2**: Sessions table has all required columns with correct types and constraints
- **Q3**: Required indexes are created for query performance
- **Q4**: Migration record is inserted into `schema_migrations` tracking table
- **Q5**: Function returns `Ok(())` on success, indicating schema is ready

---

## Invariants

- **I1**: Schema migrations are idempotent - running twice produces same result
- **I2**: Sessions table schema is backward-compatible after migration
- **I3**: Primary key is immutable after row creation
- **I4**: Timestamps are in UTC and stored as Unix epoch or ISO 8601

---

## Error Taxonomy

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationError {
    /// Database connection is invalid or closed
    InvalidConnection { reason: String },
    /// Migration version conflict - version already applied
    VersionConflict { version: i64, table_name: String },
    /// Sessions table already exists (non-idempotent migration)
    TableExists { table_name: String },
    /// Invalid migration name or version format
    InvalidMigrationFormat { migration: String, reason: String },
    /// SQL execution failure during migration
    SchemaCreationFailed { operation: String, source: String },
    /// Migration tracking table access failed
    TrackingTableError { operation: String, source: String },
}
```

---

## Contract Signatures

### Migration Module
```rust
/// Apply the sessions table migration
///
/// # Errors
/// Returns MigrationError if:
/// - Database connection is invalid
/// - Table already exists (non-idempotent mode)
/// - Schema creation fails
/// - Migration tracking fails
pub async fn migrate_sessions_table(pool: &SqlitePool) -> Result<(), MigrationError>;

/// Check if sessions table exists
///
/// # Errors
/// Returns error if query fails
pub async fn sessions_table_exists(pool: &SqlitePool) -> Result<bool, MigrationError>;

/// Get the current schema version for sessions table
///
/// # Errors
/// Returns error if migration tracking table doesn't exist or query fails
pub async fn get_migration_version(pool: &SqlitePool) -> Result<Option<i64>, MigrationError>;
```

### Sessions Table Schema
```sql
-- Core sessions table
CREATE TABLE IF NOT EXISTS sessions (
    -- Primary key: unique session identifier
    id TEXT PRIMARY KEY,
    
    -- Human-readable session name (unique, indexed)
    name TEXT NOT NULL UNIQUE,
    
    -- Session lifecycle status
    status TEXT NOT NULL DEFAULT 'creating'
        CHECK(status IN ('creating', 'active', 'completed', 'failed', 'cancelled')),
    
    -- Detailed state within the lifecycle
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK(state IN ('pending', 'working', 'waiting', 'stopping', 'terminated')),
    
    -- Workspace filesystem path
    workspace_path TEXT NOT NULL,
    
    -- Timestamps (UTC, Unix epoch seconds)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    
    -- Optional: session metadata as JSON
    metadata TEXT,
    
    -- Optional: user/agent that created the session
    owner TEXT
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);

-- Migration tracking table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| migration_version > 0 | Runtime-checked | `Migrate::new(version).run()` validates version |
| db_connection.valid | Runtime-checked | `pool.acquire().await` with timeout |
| !table_exists("sessions") | Runtime-checked | `IF NOT EXISTS` in SQL makes idempotent |
| migration_name.valid | Compile-time | `&str` with regex validation |

---

## Violation Examples (REQUIRED)

### Precondition Violations
- **VIOLATES P1**: `migrate_sessions_table(pool, version=0)` → returns `Err(MigrationError::InvalidMigrationFormat { version: 0, reason: "version must be positive" })`
- **VIOLATES P2**: Call with closed connection pool → returns `Err(MigrationError::InvalidConnection { reason: "pool is closed" })`
- **VIOLATES P4**: `migrate_with_name(pool, "invalid-name-with-dashes")` → returns `Err(MigrationError::InvalidMigrationFormat { migration: "invalid-name-with-dashes", reason: "must be valid SQL identifier" })`

### Postcondition Violations
- **VIOLATES Q1**: After migration returns Ok, query `SELECT name FROM sqlite_master WHERE type='table' AND name='sessions'` returns 0 rows → indicates invariant violation
- **VIOLATES Q2**: Query table schema and find missing columns or wrong types → returns schema mismatch error

---

## Ownership Contracts (Rust-specific)

- `pool: &SqlitePool` - Shared read-only borrow, no mutation to pool itself
- `MigrationError` - Cloneable error type for propagation across async boundaries
- No ownership transfer; all operations borrow from connection pool

---

## Non-goals

- [ ] Session data CRUD operations (separate bead)
- [ ] Session state machine logic (separate bead)
- [ ] Database connection pooling configuration
- [ ] Migration rollback/downgrade support
- [ ] Multi-database support (PostgreSQL, MySQL)

---

## Design Rationale

1. **Idempotent migrations**: Using `CREATE TABLE IF NOT EXISTS` ensures migrations can be safely re-run
2. **Unix timestamps**: Using INTEGER with Unix epoch is more efficient than TEXT timestamps and avoids timezone issues
3. **CHECK constraints**: SQLite supports CHECK constraints for status/state validation at write time
4. **Indexes**: Created `IF NOT EXISTS` for idempotency and query performance on common access patterns
5. **Tracking table**: Simple version tracking without external dependencies

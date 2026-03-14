# Martin Fowler Test Plan

## Happy Path Tests

### test_migration_creates_sessions_table_successfully
Given: A valid SQLite connection pool
When: `migrate_sessions_table(pool)` is called
Then: 
- Returns `Ok(())`
- Sessions table exists in database schema
- All expected columns are present (id, name, status, state, workspace_path, created_at, updated_at, metadata, owner)

### test_migration_creates_required_indexes
Given: A valid SQLite connection pool after migration
When: Migration completes successfully
Then:
- Index `idx_sessions_name` exists
- Index `idx_sessions_status` exists
- Index `idx_sessions_created_at` exists
- Index `idx_sessions_updated_at` exists

### test_migration_creates_tracking_table
Given: A valid SQLite connection pool after migration
When: Migration completes successfully
Then:
- `schema_migrations` table exists
- Can insert migration record
- Can query migration version

### test_migration_is_idempotent
Given: Sessions table already exists from previous migration
When: `migrate_sessions_table(pool)` is called again
Then:
- Returns `Ok(())` without error
- Table schema remains unchanged

### test_sessions_table_schema_has_correct_constraints
Given: Sessions table exists after migration
When: Attempting to insert invalid status value
Then:
- Insert fails with CHECK constraint violation
- Error indicates invalid status

---

## Error Path Tests

### test_returns_error_when_connection_invalid
Given: A closed or invalid connection pool
When: `migrate_sessions_table(invalid_pool)` is called
Then: Returns `Err(MigrationError::InvalidConnection { reason: ... })`

### test_returns_error_for_invalid_migration_version_zero
Given: A valid connection pool
When: Migration is called with version = 0
Then: Returns `Err(MigrationError::InvalidMigrationFormat { version: 0, ... })`

### test_returns_error_for_invalid_migration_version_negative
Given: A valid connection pool
When: Migration is called with version = -1
Then: Returns `Err(MigrationError::InvalidMigrationFormat { version: -1, ... })`

### test_handles_database_permission_errors
Given: A connection pool with insufficient permissions
When: Migration attempts to create table
Then: Returns `Err(MigrationError::SchemaCreationFailed { operation: "CREATE TABLE", source: ... })`

---

## Edge Case Tests

### test_handles_empty_database
Given: A fresh SQLite database with no tables
When: Migration is executed
Then:
- Sessions table is created
- All indexes are created
- Migration tracking works

### test_migration_with_unicode_session_name
Given: Sessions table exists after migration
When: Inserting session with Unicode characters in name
Then: Insert succeeds if name is valid UTF-8

### test_migration_with_long_workspace_path
Given: Sessions table exists after migration
When: Inserting session with very long workspace path (1000+ chars)
Then: Insert succeeds, path stored correctly

### test_migration_handles_null_optional_fields
Given: Sessions table exists after migration
When: Inserting session with NULL metadata and owner
Then: Insert succeeds, NULL values stored correctly

### test_migration_with_json_metadata
Given: Sessions table exists after migration
When: Inserting session with valid JSON metadata
Then: Insert succeeds, JSON stored as TEXT

---

## Contract Verification Tests

### test_precondition_version_positive
Given: A valid connection pool
When: `migrate_sessions_table(pool, version=1)` is called
Then: Precondition P1 is satisfied (version > 0)

### test_precondition_connection_valid
Given: A connection pool obtained from `SqlitePool::connect()`
When: Before migration is called
Then: Precondition P2 is satisfied (connection is valid)

### test_precondition_table_not_exists_or_idempotent
Given: A fresh database
When: First migration is called
Then: Precondition P3 is satisfied (table doesn't exist OR idempotent)

### test_postcondition_table_exists
Given: Migration returned Ok
When: Querying `sqlite_master` for sessions table
Then: Postcondition Q1 is satisfied (table exists)

### test_postcondition_columns_present
Given: Migration returned Ok
When: Querying table schema via `PRAGMA table_info(sessions)`
Then: Postcondition Q2 is satisfied (all required columns present)

### test_postcondition_indexes_created
Given: Migration returned Ok
When: Querying `sqlite_master` for indexes on sessions
Then: Postcondition Q3 is satisfied (indexes exist)

### test_invariant_idempotent_migration
Given: Migration has been run once
When: Migration is run again
Then: Invariant I1 is preserved (same result, no errors)

---

## Contract Violation Tests

### test_p1_version_zero_violation_returns_error
Given: A valid connection pool
When: `migrate_sessions_table(pool, version=0)` is called
Then: Returns `Err(MigrationError::InvalidMigrationFormat { ... })` -- NOT a panic, NOT unwrap

### test_p2_invalid_connection_violation_returns_error
Given: An invalid/closed connection pool
When: `migrate_sessions_table(invalid_pool)` is called
Then: Returns `Err(MigrationError::InvalidConnection { ... })` -- NOT a panic, NOT unwrap

### test_p4_invalid_name_violation_returns_error
Given: A valid connection pool
When: `migrate_with_name(pool, "invalid-migration-name")` is called
Then: Returns `Err(MigrationError::InvalidMigrationFormat { ... })` -- NOT a panic

### test_q1_table_not_created_violation
Given: Migration returns Ok
When: Verifying sessions table existence fails
Then: Postcondition violation detected - should not reach this state

---

## Given-When-Then Scenarios

### Scenario 1: Fresh Database Migration
**Given**: A new SQLite database with no existing tables
**When**: Running the sessions table migration
**Then**:
- Sessions table is created with all columns
- Indexes are created for name, status, created_at, updated_at
- Migration version is recorded in schema_migrations
- Subsequent queries on sessions table succeed

### Scenario 2: Repeated Migration (Idempotency)
**Given**: Sessions table already exists from previous run
**When**: Running the migration again
**Then**:
- No error is returned
- Existing data is preserved
- Schema remains unchanged

### Scenario 3: Query Session by Name
**Given**: Sessions table exists with sample data
**When**: Querying for a session by name
**Then**:
- Index on name column is used for fast lookup
- Result is returned quickly even with large dataset

### Scenario 4: Query Sessions by Status
**Given**: Sessions table exists with sessions in various statuses
**When**: Querying sessions by status (e.g., "active")
**Then**:
- Index on status column is used
- Results filtered correctly

---

## Integration Test Scenarios

### End-to-End: Create and Query Session
**Given**: Database with migrated sessions table
**When**:
1. Insert a new session with name="test-session", status="creating", state="pending"
2. Update status to "active", state to "working"
3. Query session by name
**Then**:
- Session is created with correct initial values
- Update succeeds
- Final query returns updated session with correct values

### End-to-End: Session Lifecycle
**Given**: Database with migrated sessions table
**When**: Creating multiple sessions and transitioning through lifecycle
**Then**:
- Each status/state transition is persisted correctly
- Querying by status returns correct sessions
- Timestamps are recorded and monotonically increasing

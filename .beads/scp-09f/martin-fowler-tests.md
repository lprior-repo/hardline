# Martin Fowler Test Plan

## Document Purpose

> **IMPORTANT: This is a TEST SPECIFICATION document in Gherkin/BDD (Given-When-Then) format.**
> This document is NOT executable Rust test code. It is a specification for test engineers to implement.
> Each test case below describes the behavior to be verified; implementation is left to the test engineer.

---

## Test Category Overview

| Category | Type | Description |
|----------|------|-------------|
| Happy Path Tests | Integration | Full database migration flow with SQLite |
| Error Path Tests | Integration | Error handling with actual database operations |
| Edge Case Tests | Integration | Boundary conditions against real database |
| Contract Verification Tests | Integration | Preconditions/postconditions validated via DB |
| Property-Based Tests | Integration | Invariant validation across wide input ranges |
| Contract Violation Tests | Integration | Each violation example mapped to executable test |

---

## Happy Path Tests (Integration Tests)

> These tests execute against a real SQLite database. Each test creates an in-memory
> database connection, runs the migration, and verifies the schema.

- **test_migration_creates_queue_entries_table**
  - Given: A valid SQLite database connection
  - When: Running `run_migrations(connection)`
  - Then: Table `queue_entries` exists with all required columns

- **test_migration_creates_unique_primary_key_on_id**
  - Given: A fresh database with migration applied
  - When: Inserting two entries with the same ID
  - Then: Second insert fails with `UNIQUE constraint failed`

- **test_migration_creates_not_null_constraint_on_session_id**
  - Given: A fresh database with migration applied
  - When: Inserting an entry with NULL session_id
  - Then: Insert fails with `NOT NULL constraint failed`

- **test_migration_creates_default_priority_value**
  - Given: A fresh database with migration applied
  - When: Inserting an entry without specifying priority
  - Then: The row has priority = 128 (medium)

- **test_migration_creates_status_check_constraint**
  - Given: A fresh database with migration applied
  - When: Inserting an entry with invalid status 'InvalidStatus'
  - Then: Insert fails with `CHECK constraint failed`

- **test_migration_creates_priority_bounds_check**
  - Given: A fresh database with migration applied
  - When: Inserting an entry with priority = 300
  - Then: Insert fails with `CHECK constraint failed`

- **test_migration_creates_retry_count_check**
  - Given: A fresh database with migration applied
  - When: Inserting an entry with retry_count = -1
  - Then: Insert fails with `CHECK constraint failed`

- **test_migration_creates_composite_index_for_dequeue**
  - Given: A fresh database with migration applied
  - When: Querying `pragma_index_list('queue_entries')`
  - Then: An index exists on columns (status, priority, position)

- **test_migration_creates_session_id_index**
  - Given: A fresh database with migration applied
  - When: Querying `pragma_index_list('queue_entries')`
  - Then: An index exists on column (session_id)

---

## Error Path Tests (Integration Tests)

> All error tests operate on actual SQLite databases to verify real error propagation.

- **test_migration_fails_with_invalid_database_path**
  - Given: An invalid database path "/nonexistent/path.db"
  - When: Attempting to open connection
  - Then: Returns `Err(MigrationError::DatabaseError(...))`

- **test_migration_fails_when_table_already_exists**
  - Given: A database where migration already ran
  - When: Running `run_migrations(connection)` again
  - Then: Returns `Err(MigrationError::SchemaConflict)`

- **test_migration_fails_with_invalid_sql**
  - Given: A valid connection but corrupted migration SQL
  - When: Running migration with invalid DDL
  - Then: Returns `Err(MigrationError::MigrationFailed(...))`

---

## Edge Case Tests (Integration Tests)

> Tests against real database edge cases including empty states, read-only mode, and rollback.

- **test_migration_handles_empty_database_gracefully**
  - Given: An empty SQLite database file
  - When: Running `run_migrations(connection)`
  - Then: Migration succeeds, table is created

- **test_migration_handles_readonly_database**
  - Given: A database opened in read-only mode
  - When: Running `run_migrations(connection)`
  - Then: Returns `Err(MigrationError::DatabaseError(...))`

- **test_verify_migration_returns_true_when_table_exists**
  - Given: A database with migration applied
  - When: Running `verify_migration(connection)`
  - Then: Returns `Ok(true)`

- **test_verify_migration_returns_false_when_table_missing**
  - Given: A fresh database without migration
  - When: Running `verify_migration(connection)`
  - Then: Returns `Ok(false)`

- **test_rollback_removes_table**
  - Given: A database with migration applied
  - When: Running `rollback_migration(connection)`
  - Then: Table `queue_entries` no longer exists

---

## Contract Verification Tests (Integration Tests)

> These tests verify preconditions and postconditions from the contract using actual database operations.

### Precondition Verification Tests

- **test_precondition_p1_database_connection_valid**
  - Given: Invalid path "/invalid/path.db"
  - When: `run_migrations(connection)` is called
  - Then: Returns `Err(MigrationError::DatabaseError(_))`

- **test_precondition_p2_no_conflicting_table**
  - Given: Database with existing queue_entries table
  - When: `run_migrations(connection)` is called
  - Then: Returns `Err(MigrationError::SchemaConflict)`

- **test_precondition_p3_valid_sql_syntax**
  - Given: Migration SQL with syntax error "CREAT TABLE"
  - When: `run_migrations(connection)` is called
  - Then: Returns `Err(MigrationError::MigrationFailed(_))`

### Postcondition Verification Tests

- **test_postcondition_q1_table_exists_with_columns**
  - Given: Database with migration applied
  - When: Querying `pragma_table_info('queue_entries')`
  - Then: Returns all expected columns: id, session_id, bead_id, priority, position, status, enqueued_at, updated_at, retry_count, error_message

- **test_postcondition_q2_primary_key_unique**
  - Given: Database with migration applied
  - When: Inserting duplicate IDs
  - Then: Returns `Err(MigrationError::MigrationFailed("UNIQUE constraint failed"))`

- **test_postcondition_q3_session_id_not_null**
  - Given: Database with migration applied
  - When: Inserting with NULL session_id
  - Then: Returns `Err(MigrationError::MigrationFailed("NOT NULL constraint failed"))`

- **test_postcondition_q5_indexes_exist**
  - Given: Database with migration applied
  - When: Querying `pragma_index_list('queue_entries')`
  - Then: Returns at least 2 indexes

### Invariant Verification Tests

- **test_invariant_i1_valid_status_values**
  - Given: Database with migration applied
  - When: Inserting status 'InvalidStatus'
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed"))`

- **test_invariant_i2_retry_count_non_negative**
  - Given: Database with migration applied
  - When: Inserting retry_count = -1
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed"))`

- **test_invariant_i3_priority_within_bounds**
  - Given: Database with migration applied
  - When: Inserting priority = 256
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed"))`

---

## Property-Based Testing for Invariants (Integration Tests)

> Property-based tests verify invariants hold across a wide range of values using proptest/quickcheck.
> Each test generates many random inputs to ensure the invariant is never violated.

### I1: Valid Status Values

- **test_invariant_status_property_all_valid_values_accepted**
  - Given: Database with migration applied
  - When: Inserting entries with each valid status: Pending, Claimed, Rebasing, Testing, ReadyToMerge, Merging, Merged, FailedRetryable, FailedTerminal, Cancelled
  - Then: All inserts succeed

- **test_invariant_status_property_invalid_values_rejected**
  - Given: Database with migration applied
  - When: Inserting entries with random invalid status strings (e.g., "invalid", "PENDING", "Failed", "")
  - Then: All inserts fail with CHECK constraint violation

### I2: Retry Count Non-Negative

- **test_invariant_retry_count_property_valid_range**
  - Given: Database with migration applied
  - When: Inserting entries with retry_count values: 0, 1, 10, 100, 1000, MAX_INT
  - Then: All inserts succeed

- **test_invariant_retry_count_property_negative_rejected**
  - Given: Database with migration applied
  - When: Inserting entries with negative retry_count values: -1, -100, MIN_INT
  - Then: All inserts fail with CHECK constraint violation

### I3: Priority Within Bounds

- **test_invariant_priority_property_boundary_values**
  - Given: Database with migration applied
  - When: Inserting entries with priority: 0, 1, 127, 128, 254, 255
  - Then: All inserts succeed

- **test_invariant_priority_property_out_of_bounds_rejected**
  - Given: Database with migration applied
  - When: Inserting entries with priority: -1, 256, 1000, MAX_INT
  - Then: All inserts fail with CHECK constraint violation

---

## Contract Violation Tests (Integration Tests)

> One test per violation example in contract-spec.md. Each test verifies the exact error behavior.

- **test_violation_p1_invalid_database_path_returns_database_error**
  - Given: Path "/nonexistent/path.db"
  - When: `rusqlite::Connection::open(path)`
  - Then: Returns `Err(MigrationError::DatabaseError("no such file or directory"))`

- **test_violation_p2_duplicate_migration_returns_schema_conflict**
  - Given: Database where migration already ran
  - When: `run_migrations(connection)` called again
  - Then: Returns `Err(MigrationError::SchemaConflict)`

- **test_violation_q2_duplicate_id_violates_unique_constraint**
  - Given: Database with migration applied
  - When: Inserting two entries with id='queue-123' 
  - Then: Second insert returns `Err(MigrationError::MigrationFailed("UNIQUE constraint failed: queue_entries.id"))`

- **test_violation_q3_null_session_id_violates_not_null_constraint**
  - Given: Database with migration applied
  - When: Inserting entry with session_id=NULL
  - Then: Returns `Err(MigrationError::MigrationFailed("NOT NULL constraint failed: queue_entries.session_id"))`

- **test_violation_i1_invalid_status_violates_check_constraint**
  - Given: Database with migration applied
  - When: Inserting status='InvalidStatus'
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed: status in (...)"))`

- **test_violation_i2_negative_retry_count_violates_check_constraint**
  - Given: Database with migration applied
  - When: Inserting retry_count=-1
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed: retry_count >= 0"))`

- **test_violation_i3_priority_out_of_bounds_violates_check_constraint**
  - Given: Database with migration applied
  - When: Inserting priority=300
  - Then: Returns `Err(MigrationError::MigrationFailed("CHECK constraint failed: priority >= 0 AND priority <= 255"))`

---

## Given-When-Then Scenarios (End-to-End Integration)

> Full scenarios demonstrating complete user workflows with actual database operations.

### Scenario 1: Successful Migration Application
- **Given**: A valid SQLite connection to an empty database
- **When**: Running `run_migrations(connection)`
- **Then**:
  - Returns `Ok(())`
  - Table `queue_entries` exists
  - All required columns are present
  - Indexes are created
  - Constraints are enforced

### Scenario 2: Idempotent Migration Check
- **Given**: A database where migration already ran successfully
- **When**: Running `run_migrations(connection)` again
- **Then**: Returns `Err(MigrationError::SchemaConflict)`

### Scenario 3: Rollback Migration
- **Given**: A database with migration applied and populated data
- **When**: Running `rollback_migration(connection)`
- **Then**:
  - Returns `Ok(())`
  - Table `queue_entries` no longer exists

### Scenario 4: Insert Valid Queue Entry
- **Given**: A database with migration applied
- **When**: Inserting a valid queue entry with all fields
  ```sql
  INSERT INTO queue_entries (id, session_id, bead_id, priority, position, status, enqueued_at, updated_at, retry_count)
  VALUES ('queue-123', 'session-1', 'bead-abc', 100, 0, 'Pending', datetime('now'), datetime('now'), 0);
  ```
- **Then**: Insert succeeds, row is queryable

### Scenario 5: Query Entries by Status
- **Given**: Database with multiple queue entries in different states
- **When**: Querying `SELECT * FROM queue_entries WHERE status = 'Pending' ORDER BY priority ASC, position ASC`
- **Then**: Returns only pending entries ordered by priority (lower first)

---

## Error Type Alignment Notes

> **CRITICAL**: All error types in this test plan align with the `MigrationError` enum defined in contract.md.
> 
> SQLite constraint violations (UNIQUE, NOT NULL, CHECK) manifest as `rusqlite::Error` variants at the database layer.
> These errors MUST be wrapped/converted to `MigrationError::MigrationFailed(...)` in the implementation to maintain
> contract consistency.
>
> The test assertions expect `MigrationError` variants because that's what the public API returns per the contract:
> - `run_migrations(...) -> Result<(), MigrationError>`
> - `verify_migration(...) -> Result<bool, MigrationError>`
> - `rollback_migration(...) -> Result<(), MigrationError>`

---

## Test Fixture Specification

> Test engineers should implement the following shared fixtures:

```rust
// Suggested test fixture - implementation detail left to test engineer
fn test_db() -> Connection {
    // Creates fresh in-memory SQLite database for each test
    let conn = Connection::open_in_memory();
    run_migrations(&conn).expect("migration setup failed");
    conn
}
```

---

## Implementation Notes for Test Engineers

1. **All tests are integration tests** - they require actual SQLite database execution
2. **Use `rusqlite::Connection::open_in_memory()`** for isolated test databases
3. **Wrap SQLite errors into MigrationError** as per contract signature
4. **Use proptest/quickcheck** for property-based invariant tests
5. **Each test is independent** - no shared state between tests
6. **Tests should NOT use mocks** - verify actual database behavior

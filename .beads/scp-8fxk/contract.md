# Contract Specification: SQLite Schema for operation_log

## Context
- **Feature**: Create SQLite schema for operation_log table (event sourcing)
- **Bead**: scp-8fxk
- **Domain terms**:
  - `operation_log` - Append-only event store for all state changes
  - `stream_id` - Identifier for grouping related events (e.g., "session-123")
  - `stream_version` - Version number for optimistic concurrency control
  - `event_type` - Type of domain event (e.g., "session_created")
  - `payload` - JSON-serialized event data

## Preconditions

### Schema Creation Preconditions

- **[P1]**: Database pool must be valid and connected
  - Enforcement: `SqlitePool` passed as parameter, validated by sqlx

- **[P2]**: Schema creation must be idempotent
  - Enforcement: Uses `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`

### Insert Operation Preconditions

- **[P3]**: event_type must be non-empty
  - Enforcement: Runtime validation returns `OperationLogError::ValidationFailed`

- **[P4]**: stream_id must be non-empty
  - Enforcement: Runtime validation returns `OperationLogError::ValidationFailed`

- **[P5]**: payload must be valid JSON
  - Enforcement: Caller responsible; stored as TEXT

## Postconditions

### Schema Creation Postconditions

- **[Q1]**: Table `operation_log` exists after function completes
  - Type: `Result<(), OperationLogError>`

- **[Q2]**: All required indexes are created
  - Type: `Result<(), OperationLogError>`

### Insert Operation Postconditions

- **[Q3]**: Returns the inserted entry with assigned ID
  - Type: `Result<OperationLogEntry, OperationLogError>`
  - Returns entry with populated `id` field

- **[Q4]**: Entry is queryable immediately after insert
  - Verified by tests querying after insert

### Query Operations Postconditions

- **[Q5]**: Stream events returned in ascending version order
  - Type: `Result<Vec<OperationLogEntry>, OperationLogError>`

- **[Q6]**: Stream version returns max version or 0 if empty
  - Type: `Result<i64, OperationLogError>`

## Invariants

- **[I1]**: operation_log is append-only (no UPDATE or DELETE in this module)
  - Enforced: Only INSERT and SELECT operations exposed

- **[I2]**: stream_version is monotonically increasing per stream
  - Enforced: Caller responsible; no auto-increment at DB level

- **[I3]**: created_at is always valid RFC3339 format
  - Enforced: Uses chrono::DateTime::to_rfc3339() for serialization

- **[I4]**: Indexes enable efficient temporal and stream queries
  - Enforced: Three indexes created on creation

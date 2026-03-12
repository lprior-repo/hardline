# Martin Fowler Tests: operation_log Schema

This document defines acceptance tests in Given-When-Then format following Martin Fowler's testing patterns.

## Schema Creation Tests

### Happy Path: Create Schema Successfully

**Test**: `given_valid_database_pool_when_ensure_schema_then_succeeds`

- **Given**: A valid `SqlitePool` connected to a fresh database
- **When**: `ensure_operation_log_schema()` is called
- **Then**: Returns `Ok(())`
- **And**: Table `operation_log` exists
- **And**: All three indexes exist

---

### Happy Path: Schema Creation is Idempotent

**Test**: `given_schema_exists_when_create_again_then_succeeds`

- **Given**: Schema has been created once
- **When**: `ensure_operation_log_schema()` is called again
- **Then**: Returns `Ok(())` (no error)

---

## Entry Creation Tests

### Happy Path: Create Valid Entry

**Test**: `given_valid_entry_data_when_create_then_succeeds`

- **Given**: Valid event_type, payload, stream_id, and stream_version
- **When**: `OperationLogEntry::new()` is called
- **Then**: Returns `Ok(Entry)` with all fields populated

---

### Error Path: Empty Event Type

**Test**: `given_empty_event_type_when_create_then_returns_validation_error`

- **Given**: Empty string for event_type
- **When**: `OperationLogEntry::new()` is called
- **Then**: Returns `Err(OperationLogError::ValidationFailed)`

---

### Error Path: Empty Stream ID

**Test**: `given_empty_stream_id_when_create_then_returns_validation_error`

- **Given**: Empty string for stream_id
- **When**: `OperationLogEntry::new()` is called
- **Then**: Returns `Err(OperationLogError::ValidationFailed)`

---

## Database Insert Tests

### Happy Path: Insert Entry

**Test**: `given_valid_entry_when_insert_then_succeeds`

- **Given**: Database pool with schema created, and a valid `OperationLogEntry`
- **When**: `insert_operation_log()` is called
- **Then**: Returns `Ok(Entry)` with id > 0

---

### Error Path: Insert with Empty Event Type

**Test**: `given_empty_event_type_when_insert_then_returns_error`

- **Given**: Database pool with schema created, and entry with empty event_type
- **When**: `insert_operation_log()` is called
- **Then**: Returns `Err(OperationLogError::ValidationFailed)`

---

## Query Tests

### Happy Path: Query Stream Events

**Test**: `given_multiple_events_when_query_stream_then_returns_all`

- **Given**: Database with 3 events for stream "session-s1" (versions 1, 2, 3)
- **When**: `query_stream_events("session-s1")` is called
- **Then**: Returns 3 entries
- **And**: First entry has stream_version = 1
- **And**: Entries are in ascending version order

---

### Happy Path: Get Stream Version

**Test**: `given_events_when_get_stream_version_then_returns_max`

- **Given**: Database with events for stream "stream-1" (versions 1, 2, 3)
- **When**: `get_stream_version("stream-1")` is called
- **Then**: Returns `3`

---

### Happy Path: Empty Stream Version

**Test**: `given_no_events_when_get_stream_version_then_returns_zero`

- **Given**: Empty database
- **When**: `get_stream_version("nonexistent")` is called
- **Then**: Returns `0`

---

### Happy Path: Query All with Limit

**Test**: `given_many_events_when_query_with_limit_then_respects_limit`

- **Given**: Database with 10 events across 3 streams
- **When**: `query_all_operations(Some(5))` is called
- **Then**: Returns exactly 5 entries

---

## Integration Tests

### Full Event Sourcing Flow

**Test**: `given_complete_event_sourcing_flow_then_works`

- **Given**: Fresh database with schema
- **When**: 
  1. Insert "session_created" event (version 1)
  2. Insert "session_activated" event (version 2)
  3. Insert "session_completed" event (version 3)
  4. Query stream events
  5. Get stream version
- **Then**: 
  - Returns 3 events in order
  - Version is 3
  - All events have correct stream_id

---

## Error Handling Tests

### Error Path: Query Failed

**Test**: `given_invalid_pool_when_query_then_returns_error`

- **Given**: Disconnected or invalid pool
- **When**: `query_stream_events()` is called
- **Then**: Returns `Err(OperationLogError::DatabaseError)` or `Err(OperationLogError::QueryFailed)`

---

### Error Path: Parse Invalid Row

**Test**: `given_malformed_row_when_parse_then_returns_error`

- **Given**: Database with corrupted row data
- **When**: Query returns the row
- **Then**: Parse function returns `Err(OperationLogError::QueryFailed)`

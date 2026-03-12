# Contract Specification

## Context
- **Feature**: Create SQLite database schema for beads/tracking
- **Domain terms**:
  - **Bead**: A task/issue tracked in the system with lifecycle states
  - **Dependency**: A directed edge between beads indicating prerequisite relationships
  - **State History**: Immutable audit trail of bead status changes
  - **Metadata**: JSON-encodable flexible key-value data attached to beads
- **Assumptions**:
  - SQLite with WAL mode for durability
  - Uses existing error types from `crates/core/src/beads/types.rs`
  - Compatible with existing bead domain model
- **Open questions**:
  - Should metadata be stored as JSON text or separate key-value table?
  - What is the expected granularity of state history (per-field vs per-status)?

## Preconditions

- **P1**: Database pool must be valid and connected
  - Type enforcement: `SqlitePool` is already a validated connection pool
  - Violation: Passing `None` or unconnected pool returns `Err(BeadsError::DatabaseError)`

- **P2**: Bead ID must be non-empty
  - Type enforcement: Runtime check with `NonEmptyString` pattern
  - Violation: `insert_bead(BeadIssue { id: "", ... })` returns `Err(BeadsError::ValidationFailed("ID cannot be empty"))`

- **P3**: Bead title must be non-empty
  - Type enforcement: Runtime check
  - Violation: `insert_bead(BeadIssue { title: "", ... })` returns `Err(BeadsError::ValidationFailed("Title cannot be empty"))`

- **P4**: Status must be valid enum value
  - Type enforcement: Compile-time via `IssueStatus` enum
  - Violation: N/A - compile-time enforced

- **P5**: Priority must be valid if provided
  - Type enforcement: Runtime check via `Priority::from_u32`
  - Violation: `Priority::from_u32(99)` returns `None`, leading to validation failure

- **P6**: If status is `Closed`, `closed_at` must be set
  - Type enforcement: Runtime check
  - Violation: `insert_bead(BeadIssue { status: IssueStatus::Closed, closed_at: None, ... })` returns `Err(BeadsError::ValidationFailed("closed_at must be set when status is 'closed'"))`

- **P7**: Dependency references must point to existing beads
  - Type enforcement: Runtime validation against database
  - Violation: `insert_bead(BeadIssue { depends_on: Some(vec!["non-existent-id"]), ... })` returns `Err(BeadsError::ValidationFailed("Dependency target does not exist"))`

- **P8**: No circular dependencies allowed
  - Type enforcement: Runtime graph cycle detection
  - Violation: Creating A depends_on B, B depends_on A returns `Err(BeadsError::ValidationFailed("Circular dependency detected"))`

## Postconditions

- **Q1**: Schema creation succeeds with all required tables
  - Tables exist: `beads`, `bead_dependencies`, `bead_state_history`
  - Indexes created for performance

- **Q2**: Inserted bead is queryable by ID
  - After `insert_bead(bead)`, `get_bead(id)` returns `Ok(bead)`

- **Q3**: Updated bead reflects changes
  - After `update_bead(id, new_data)`, `get_bead(id)` returns updated data

- **Q4**: State history recorded on status change
  - After `update_bead(id, bead with new status)`, `get_history(id)` includes new entry

- **Q5**: Dependencies are queryable
  - After `add_dependency(parent_id, child_id)`, `get_dependencies(child_id)` includes parent_id

- **Q6**: Deleted bead removes from beads table
  - After `delete_bead(id)`, `get_bead(id)` returns `Err(BeadsError::NotFound)`

- **Q7**: Deleted bead removes associated dependencies
  - After `delete_bead(id)`, no dependency entries reference deleted bead

## Invariants

- **I1**: Every bead has unique ID (PRIMARY KEY constraint)
- **I2**: `created_at` is immutable after creation
- **I3**: `updated_at` is always >= `created_at`
- **I4**: State history entries are immutable (no UPDATE/DELETE on history table)
- **I5**: Dependency graph remains acyclic after any operation
- **I6**: All foreign key references point to existing beads

## Error Taxonomy

| Error Variant | Trigger Condition |
|--------------|-------------------|
| `BeadsError::DatabaseError` | SQLite connection failure, schema creation failure |
| `BeadsError::NotFound` | Bead with given ID does not exist |
| `BeadsError::ValidationFailed` | Invalid input (empty ID/title, invalid status, circular deps) |
| `BeadsError::DuplicateId` | Inserted bead ID already exists |
| `BeadsError::QueryFailed` | SQL query execution failure |
| `BeadsError::InvalidFilter` | Invalid query parameters |

## Contract Signatures

```rust
/// Initialize database schema (creates tables and indexes)
pub async fn ensure_schema(pool: &SqlitePool) -> Result<(), BeadsError>

/// Insert a new bead
pub async fn insert_bead(pool: &SqlitePool, bead: &BeadIssue) -> Result<(), BeadsError>

/// Query bead by ID
pub async fn get_bead(pool: &SqlitePool, id: &str) -> Result<BeadIssue, BeadsError>

/// Query all beads with optional filters
pub async fn query_beads(pool: &SqlitePool, filter: Option<BeadFilter>) -> Result<Vec<BeadIssue>, BeadsError>

/// Update existing bead
pub async fn update_bead(pool: &SqlitePool, id: &str, bead: &BeadIssue) -> Result<BeadIssue, BeadsError>

/// Delete bead and its dependencies
pub async fn delete_bead(pool: &SqlitePool, id: &str) -> Result<(), BeadsError>

/// Add dependency (parent -> child)
pub async fn add_dependency(pool: &SqlitePool, parent_id: &str, child_id: &str) -> Result<(), BeadsError>

/// Remove dependency
pub async fn remove_dependency(pool: &SqlitePool, parent_id: &str, child_id: &str) -> Result<(), BeadsError>

/// Get all dependencies for a bead (both depends_on and blocked_by)
pub async fn get_dependencies(pool: &SqlitePool, bead_id: &str) -> Result<BeadDependencies, BeadsError>

/// Record state change in history
pub async fn record_state_change(pool: &SqlitePool, bead_id: &str, old_status: IssueStatus, new_status: IssueStatus) -> Result<(), BeadsError>

/// Get state history for a bead
pub async fn get_history(pool: &SqlitePool, bead_id: &str) -> Result<Vec<StateHistoryEntry>, BeadsError>
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| Pool connected | Compile-time | `SqlitePool` (valid connection) |
| ID non-empty | Runtime-validated | `NonEmptyString` or explicit check |
| Title non-empty | Runtime-validated | Explicit check |
| Valid status | Compile-time | `enum IssueStatus` |
| Valid priority | Runtime-validated | `Priority::from_u32() -> Option` |
| closed_at for Closed | Runtime-validated | Explicit check |
| Dependency exists | Runtime-validated | DB lookup |
| No cycles | Runtime-validated | Graph traversal |

## Violation Examples

- **VIOLATES P2**: `insert_bead(pool, &BeadIssue { id: "".into(), title: "Test".into(), status: IssueStatus::Open, .. })` -> returns `Err(BeadsError::ValidationFailed("ID cannot be empty"))`

- **VIOLATES P3**: `insert_bead(pool, &BeadIssue { id: "test-1".into(), title: "".into(), status: IssueStatus::Open, .. })` -> returns `Err(BeadsError::ValidationFailed("Title cannot be empty"))`

- **VIOLATES P6**: `insert_bead(pool, &BeadIssue { id: "test-1".into(), title: "Test".into(), status: IssueStatus::Closed, closed_at: None, .. })` -> returns `Err(BeadsError::ValidationFailed("closed_at must be set when status is 'closed'"))`

- **VIOLATES P7**: `insert_bead(pool, &BeadIssue { id: "test-1".into(), title: "Test".into(), status: IssueStatus::Open, depends_on: Some(vec!["non-existent".into()]), .. })` -> returns `Err(BeadsError::ValidationFailed("Dependency target does not exist: non-existent"))`

- **VIOLATES P8**: First insert beads A and B, then `add_dependency(pool, "A", "B")` then `add_dependency(pool, "B", "A")` -> returns `Err(BeadsError::ValidationFailed("Circular dependency detected"))`

- **VIOLATES Q1**: `ensure_schema(pool)` with invalid path -> returns `Err(BeadsError::DatabaseError(...))`

- **VIOLATES Q2**: After `insert_bead(pool, &bead)`, immediately call `get_bead(pool, "wrong-id")` -> returns `Err(BeadsError::NotFound("wrong-id"))`

- **VIOLATES Q6**: After `delete_bead(pool, id)`, call `get_bead(pool, id)` -> returns `Err(BeadsError::NotFound(id))`

## Ownership Contracts

| Function | Parameter | Ownership Policy |
|----------|-----------|------------------|
| `insert_bead` | `bead: &BeadIssue` | Shared borrow - reads data, does not mutate caller |
| `update_bead` | `id: &str`, `bead: &BeadIssue` | Shared borrow for both - reads ID and data |
| `get_bead` | `id: &str` | Shared borrow - read-only query |
| `delete_bead` | `id: &str` | Shared borrow - read ID, deletes from DB |

## Schema Design

### beads table
```sql
CREATE TABLE beads (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'in_progress', 'blocked', 'deferred', 'closed')),
    priority TEXT,
    type TEXT,
    description TEXT,
    labels TEXT,
    assignee TEXT,
    parent TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    CHECK (closed IS NOT NULL OR closed_at IS NULL)
);
```

### bead_dependencies table
```sql
CREATE TABLE bead_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id TEXT NOT NULL REFERENCES beads(id) ON DELETE CASCADE,
    child_id TEXT NOT NULL REFERENCES beads(id) ON DELETE CASCADE,
    dependency_type TEXT NOT NULL DEFAULT 'depends_on' CHECK (dependency_type IN ('depends_on', 'blocked_by')),
    created_at TEXT NOT NULL,
    UNIQUE(parent_id, child_id, dependency_type)
);
```

### bead_state_history table
```sql
CREATE TABLE bead_state_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bead_id TEXT NOT NULL REFERENCES beads(id) ON DELETE CASCADE,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    changed_by TEXT
);
```

### Indexes
```sql
CREATE INDEX idx_beads_status ON beads(status);
CREATE INDEX idx_beads_priority ON beads(priority);
CREATE INDEX idx_beads_created_at ON beads(created_at);
CREATE INDEX idx_dependencies_child ON bead_dependencies(child_id);
CREATE INDEX idx_dependencies_parent ON bead_dependencies(parent_id);
CREATE INDEX idx_history_bead ON bead_state_history(bead_id);
```

## Non-goals
- [ ] Multi-tenant isolation (future feature)
- [ ] Real-time sync/pubsub (future feature)
- [ ] Full-text search (future feature)
- [ ] Bead merging/splitting (future feature)

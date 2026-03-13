---
bead_id: scp-fim
bead_title: Create SQLite schema for workspaces
phase: contract
updated_at: 2026-03-12T00:00:00Z
---

# Contract Specification: SQLite Schema for workspaces

## Context

- **Feature**: Create SQLite schema for workspaces table (SQL migration)
- **Bead**: scp-fim
- **Domain terms**:
  - `Workspace` - An isolated working directory for an AI agent
  - `WorkspaceId` - Unique identifier (format: `ws-<uuid>`)
  - `WorkspaceName` - User-friendly name for the workspace
  - `WorkspacePath` - Absolute filesystem path to the workspace directory
  - `WorkspaceState` - Lifecycle state: Initializing, Active, Locked, Corrupted, Deleted
  - `VcsType` - Version control system: Jj, Git, Both
  - `WorkspaceConfig` - Configuration: vcs_type, default_branch, auto_sync
  - `lock_holder` - Optional agent ID holding exclusive lock on workspace

## Preconditions

### Schema Migration Preconditions

- **[P1]**: Database pool must be valid and connected
  - Enforcement: `SqlitePool` parameter validated by sqlx at connection time

- **[P2]**: Migration must be idempotent
  - Enforcement: Uses `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`

### Repository Operation Preconditions

- **[P3]**: Workspace ID must be non-empty and valid format
  - Enforcement: Runtime validation returns `WorkspaceError::InvalidWorkspaceId`

- **[P4]**: Workspace name must be non-empty
  - Enforcement: Runtime validation returns `WorkspaceError::InvalidWorkspaceName`

- **[P5]**: Workspace path must be valid absolute path
  - Enforcement: Runtime validation returns `WorkspaceError::InvalidWorkspacePath`

- **[P6]**: Workspace state must be valid enum value
  - Enforcement: Type system ensures valid enum variants

- **[P7]**: For update/delete operations, workspace must exist
  - Enforcement: Returns `WorkspaceError::WorkspaceNotFound`

## Postconditions

### Schema Migration Postconditions

- **[Q1]**: Table `workspaces` exists after migration completes
  - Type: `Result<(), WorkspaceError>`

- **[Q2]**: All required indexes are created
  - Type: `Result<(), WorkspaceError>`
  - Indexes: idx_workspaces_name, idx_workspaces_state, idx_workspaces_lock_holder

### Save/Insert Operation Postconditions

- **[Q3]**: Returns the saved workspace with persisted data
  - Type: `Result<Workspace, WorkspaceError>`
  - All fields populated including generated ID and timestamps

- **[Q4]**: Saved workspace is queryable immediately after insert
  - Verified by tests querying after save

### Query Operations Postconditions

- **[Q5]**: `get_by_id` returns Some(Workspace) if found, None if not found
  - Type: `Result<Option<Workspace>, WorkspaceError>`

- **[Q6]**: `get_by_name` returns Some(Workspace) if found, None if not found
  - Type: `Result<Option<Workspace>, WorkspaceError>`

- **[Q7]**: `list` returns all workspaces including deleted
  - Type: `Result<Vec<Workspace>, WorkspaceError>`

- **[Q8]**: `list_active` returns only workspaces where state = Active
  - Type: `Result<Vec<Workspace>, WorkspaceError>`

### Delete Operation Postconditions

- **[Q9]**: `delete` marks workspace as Deleted (soft delete)
  - Type: `Result<Workspace, WorkspaceError>`
  - Returns workspace with state = Deleted

- **[Q10]**: Deleted workspace is no longer returned by `list_active`
  - Verified by tests

## Invariants

- **[I1]**: Workspace ID format is always `ws-<uuid>`
  - Enforced: `WorkspaceId::generate()` produces correct format

- **[I2]**: created_at and updated_at are always valid RFC3339 timestamps
  - Enforced: Uses chrono::DateTime::to_rfc3339() for serialization

- **[I3]**: State transitions follow valid state machine rules
  - Enforced: `WorkspaceStateMachine::can_transition()` validates

- **[I4]**: lock_holder is Some only when state = Locked
  - Enforced: Business logic ensures consistency

- **[I5]**: VcsType is always a valid enum variant
  - Enforced: Type system

## Error Taxonomy

| Error Variant | Condition | Example |
|---------------|-----------|---------|
| `WorkspaceError::WorkspaceNotFound` | Workspace ID not found in DB | `get("ws-nonexistent")` |
| `WorkspaceError::WorkspaceExists` | Duplicate name on create | `save(Workspace { name: "existing" })` |
| `WorkspaceError::WorkspaceLocked` | Operation requires unlocked workspace | `lock()` when already locked |
| `WorkspaceError::InvalidStateTransition` | Invalid state transition | `activate()` when already Active |
| `WorkspaceError::InvalidWorkspaceId` | Invalid ID format or empty | `WorkspaceId::parse("")` |
| `WorkspaceError::InvalidWorkspaceName` | Empty name | `WorkspaceName::new("")` |
| `WorkspaceError::InvalidWorkspacePath` | Invalid path | `WorkspacePath::new("relative")` |
| `WorkspaceError::OperationFailed` | Database or I/O error | Connection lost mid-operation |
| `WorkspaceError::RepositoryError` | Repository implementation error | SQL execution failure |

## Contract Signatures

```rust
// Schema migration
pub async fn migrate_workspaces(pool: &SqlitePool) -> Result<(), WorkspaceError>;

// Repository trait
pub trait WorkspaceRepository: Send + Sync {
    fn save(&self, workspace: Workspace) -> Result<Workspace>;
    fn get(&self, id: &WorkspaceId) -> Result<Option<Workspace>>;
    fn get_by_name(&self, name: &str) -> Result<Option<Workspace>>;
    fn list(&self) -> Result<Vec<Workspace>>;
    fn list_active(&self) -> Result<Vec<Workspace>>;
    fn delete(&self, id: &WorkspaceId) -> Result<Workspace>;
}

// SQLite implementation
pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl WorkspaceRepository for SqliteWorkspaceRepository { ... }
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| Database pool valid | Compile-time | `SqlitePool` (validated by sqlx) |
| Workspace ID non-empty | Compile-time | `struct WorkspaceId(String)` with private constructor |
| Workspace name non-empty | Runtime-checked | `WorkspaceName::new() -> Result<WorkspaceName, WorkspaceError>` |
| Workspace path absolute | Runtime-checked | `WorkspacePath::new() -> Result<WorkspacePath, WorkspaceError>` |
| State valid enum | Compile-time | `enum WorkspaceState { ... }` |
| Migration idempotent | Runtime-checked | `CREATE TABLE IF NOT EXISTS` |

## Violation Examples

- **VIOLATES P3**: `repo.get(&WorkspaceId::parse("".into()).unwrap())` -- should produce `Err(WorkspaceError::InvalidWorkspaceId("empty id".into()))`

- **VIOLATES P4**: `Workspace::create(WorkspaceName::new("".into()).unwrap(), path)` -- should produce `Err(WorkspaceError::InvalidWorkspaceName("empty name".into()))`

- **VIOLATES P5**: `Workspace::create(name, WorkspacePath::new("relative/path".into()).unwrap())` -- should produce `Err(WorkspaceError::InvalidWorkspacePath("not absolute".into()))`

- **VIOLATES P7**: `repo.delete(&WorkspaceId::parse("ws-nonexistent".into()).unwrap())` -- should produce `Err(WorkspaceError::WorkspaceNotFound("ws-nonexistent".into()))`

- **VIOLATES Q1**: Running migration on invalid pool -- should produce `Err(WorkspaceError::OperationFailed("connection failed".into()))`

- **VIOLATES Q3**: Saving workspace with duplicate name -- should produce `Err(WorkspaceError::WorkspaceExists(name.into()))`

## Ownership Contracts

- **SqliteWorkspaceRepository**: Takes ownership of `SqlitePool` (shared via Arc internally)
- **save()**: Takes ownership of `Workspace`, returns owned `Workspace`
- **get()**: Takes shared reference `&WorkspaceId`, returns owned `Option<Workspace>` (cloned from DB row)
- **delete()**: Takes shared reference `&WorkspaceId`, returns owned `Workspace` with updated state

## Non-goals

- [ ] Supporting multiple workspaces per agent (future feature)
- [ ] Workspace synchronization logic (separate bead)
- [ ] Workspace backup/restore (separate bead)
- [ ] Migration rollback (handled by separate migration system)

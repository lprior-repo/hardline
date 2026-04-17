# Contract Specification: PostgreSQL Worktree Repository Integration

## Context
- **Feature:** PostgreSQL worktree repository integration for persistent storage
- **Bead ID:** hl-7xr
- **Domain terms:** Worktree, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum, BranchName, AbsolutePath, Metadata
- **Assumptions:**
  - PostgreSQL database is the persistence layer
  - Repository pattern with async_trait
  - UUID-based primary keys (16 bytes BYTEA)
  - JSONB for metadata storage
  - Schema created on repository initialization

## Preconditions

### P1: Repository Initialization
- Database URL must be valid PostgreSQL connection string
- PostgreSQL server must be running and accessible
- Database must exist (or be creatable)
- Connection pool must be configurable (pool_size, max_connections)

### P2: Worktree Creation
- WorktreeName must be non-empty and ≤255 characters
- WorktreeName must be valid (no invalid characters)
- AbsolutePath must start with `/`
- Path must be an absolute filesystem path
- BranchName must be valid git branch name
- WorktreeState must be a valid enum value (0-4)
- WorktreeTypeEnum must be a valid enum value (0-4)

### P3: UUID Validity
- WorktreeId must be 16-byte UUID
- UUID must be version 4 random (new_random())
- UUID must be valid BYTEA for PostgreSQL

### P4: Metadata
- Metadata must be valid JSONB (serde_json serializable)
- Metadata keys must be valid UTF-8 strings
- Metadata values must be valid UTF-8 strings
- Metadata can be empty HashMap (no entries)

## Postconditions

### Q1: Repository Initialization
- `PostgresWorktreeRepository::new()` returns `Ok(PostgresWorktreeRepository)`
- `worktrees` table is created with correct schema
- `worktrees` table has PRIMARY KEY on `id` column
- `worktrees` table has UNIQUE constraint on `name` column
- `worktrees` table has indexes: `idx_worktrees_name`, `idx_worktrees_state`, `idx_worktrees_type`
- Schema creation is idempotent (safe to call multiple times)

### Q2: Save Operation
- `save()` returns `Ok(())` on success
- Worktree is upserted (INSERT if new, UPDATE if exists)
- All fields are persisted: `id`, `name`, `path`, `parent_path`, `state`, `worktree_type`, `branch`, `created_at`, `updated_at`, `metadata`
- `updated_at` is set to current timestamp on update
- UUID is persisted as BYTEA
- Metadata is persisted as JSONB

### Q3: Find by ID
- `find_by_id(id)` returns `Ok(Some(worktree))` if exists
- `find_by_id(id)` returns `Ok(None)` if not found
- All fields are deserialized correctly
- UUID round-trips correctly
- Enums round-trip correctly (state, type)
- Nullable branch round-trips correctly
- Metadata round-trips correctly

### Q4: Find by Name
- `find_by_name(name)` returns `Ok(Some(worktree))` if exists
- `find_by_name(name)` returns `Ok(None)` if not found
- Name matching is case-sensitive
- Name matching is exact (no substring/superstring)

### Q5: List All
- `list_all()` returns `Ok(Vec<Worktree>)`
- Returns empty vector if no worktrees exist
- All worktrees are returned with all fields
- Ordering is deterministic (by created_at ascending)

### Q6: Delete
- `delete(id)` returns `Ok(())` if worktree exists
- `delete(id)` returns `Ok(())` if worktree does not exist (idempotent)
- Worktree is removed from database
- Subsequent `find_by_id()` returns `Ok(None)`

### Q7: Name Exists
- `name_exists(name)` returns `Ok(true)` if name exists
- `name_exists(name)` returns `Ok(false)` if name does not exist
- Name matching is case-sensitive
- Name matching is exact

## Invariants

### I1: Name Uniqueness
- No two worktrees can have the same name in the database
- Enforced by UNIQUE constraint on `name` column

### I2: UUID Uniqueness
- WorktreeId::new_random() generates unique UUID within test run
- UUID round-trips correctly through database persistence

### I3: State Consistency
- WorktreeState::from_u8(WorktreeState::as_u8(s)) == s for all valid states (0-4)
- State is persisted as INTEGER (i32) and round-trips correctly

### I4: Type Consistency
- WorktreeTypeEnum::from_u8(WorktreeTypeEnum::as_u8(t)) == t for all valid types (0-4)
- Type is persisted as INTEGER (i32) and round-trips correctly

### I5: Metadata Integrity
- All metadata key-value pairs are preserved after save/reload
- JSONB serialization/deserialization is lossless

### I6: Timestamp Ordering
- created_at <= updated_at always holds
- Timestamps are stored as INTEGER (i64 Unix timestamp)

### I7: Branch Nullability
- branch can be NULL or contain a valid branch name
- NULL branch round-trips correctly

## Error Taxonomy

### WorktreeDomainError Variants

| Error Variant | When Returned | Description |
|---------------|---------------|-------------|
| `NameAlreadyExists(String)` | save() | Worktree with this name already exists |
| `NotFound(WorktreeId)` | find_by_id(), delete() | Worktree with this ID does not exist |
| `InvalidName(String)` | save(), name_exists() | Worktree name is empty or contains invalid characters |
| `InvalidPath(String)` | save(), repository_new() | Path is not absolute, or database connection fails |
| `InvalidBranch(String)` | save() | Branch name contains invalid characters |
| `CannotRemoveDefaultBranch` | delete() | Attempting to remove worktree for default branch |
| `InvalidStateTransition(WorktreeState, WorktreeState)` | save() | Invalid state transition (e.g., Removing → Creating) |
| `SourcePathNotFound(String)` | save() | Source filesystem path does not exist |
| `InvalidRepository(String)` | save() | Repository path is not a valid git repository |
| `GitError(String)` | save() | Git operation failed |
| `NotInitialized(WorktreeName)` | save() | Worktree is not initialized |
| `AlreadyInitialized(WorktreeName)` | save() | Worktree is already initialized |

## Contract Signatures

### Repository Functions

```rust
/// Initialize repository with database connection
pub fn new(database_url: &str) -> Result<PostgresWorktreeRepository, WorktreeDomainError>

/// Get database pool reference
pub fn pool(&self) -> &PgPool
```

### Repository Trait (WorktreeRepository)

```rust
/// Save a worktree to the repository (upsert)
async fn save(&mut self, worktree: &mut Worktree) -> Result<(), WorktreeDomainError>

/// Find a worktree by ID
async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError>

/// Find a worktree by name
async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError>

/// List all worktrees
async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError>

/// Delete a worktree
async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError>

/// Check if a worktree with given name exists
async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError>
```

### Domain Type Constructors

```rust
/// Create WorktreeName with validation
pub fn new(name: &str) -> Result<Self, WorktreeDomainError>

/// Create AbsolutePath with validation
pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WorktreeDomainError>

/// Create BranchName with validation
pub fn new(name: &str) -> Result<Self, WorktreeDomainError>

/// Create WorktreeId from random bytes
pub fn new_random() -> Self

/// Create WorktreeId from bytes
pub fn from_bytes(bytes: [u8; 16]) -> Self
```

### Worktree Methods

```rust
/// Create uninitialized worktree
pub fn new(
    id: WorktreeId,
    name: WorktreeName,
    path: AbsolutePath,
    parent_path: AbsolutePath,
    state: WorktreeState,
    worktree_type: WorktreeTypeEnum,
    branch: Option<BranchName>,
    created_at: i64,
    updated_at: i64,
) -> Self

/// Create uninitialized worktree with metadata
pub fn uninitialized_with_metadata(...) -> Self

/// Initialize worktree
pub fn initialize(&mut self) -> Result<(), WorktreeDomainError>

/// Suspend worktree
pub fn suspend(&mut self) -> Result<(), WorktreeDomainError>

/// Resume worktree
pub fn resume(&mut self) -> Result<(), WorktreeDomainError>

/// Mark worktree for removal
pub fn mark_for_removal(&mut self) -> Result<(), WorktreeDomainError>

/// Complete worktree removal
pub fn complete_removal(&mut self) -> Result<(), WorktreeDomainError>

/// Get metadata value
pub fn get_metadata(&self, key: &str) -> Option<&str>

/// Get all metadata
pub fn all_metadata(&self) -> &HashMap<String, String>
```

### Enum Conversion Methods

```rust
/// Convert WorktreeState from u8
pub fn from_u8(value: u8) -> Option<Self>

/// Convert WorktreeState to u8
pub fn as_u8(self) -> u8

/// Convert WorktreeTypeEnum from u8
pub fn from_u8(value: u8) -> Option<Self>

/// Convert WorktreeTypeEnum to u8
pub fn as_u8(self) -> u8
```

## Type Encoding

| Field | Database Type | Rust Type | Notes |
|-------|---------------|-----------|-------|
| id | BYTEA(16) | WorktreeId | UUID, primary key |
| name | TEXT(255) | WorktreeName | Unique, not null |
| path | TEXT | AbsolutePath | Absolute filesystem path |
| parent_path | TEXT | AbsolutePath | Absolute filesystem path |
| state | INTEGER | WorktreeState | 0-4 enum, not null |
| worktree_type | INTEGER | WorktreeTypeEnum | 0-4 enum, not null |
| branch | TEXT | Option<BranchName> | Nullable |
| created_at | INTEGER | i64 | Unix timestamp |
| updated_at | INTEGER | i64 | Unix timestamp |
| metadata | JSONB | HashMap<String, String> | JSONB, can be {} |

## Input Validation Rules

### WorktreeName
- Minimum length: 1 character
- Maximum length: 255 characters
- Cannot be whitespace only
- Empty string is invalid
- Unicode characters are allowed

### AbsolutePath
- Must start with `/`
- Cannot be relative path
- Path traversal is allowed (../)

### BranchName
- Must be valid git branch name
- No spaces allowed
- Special characters are allowed (/, -, _, .)

### Metadata
- Keys must be non-empty UTF-8 strings
- Values must be UTF-8 strings
- Empty HashMap is valid
- No size limit (bounded by database)

## Boundary Values

### Name Length
- Min: 1 character (`"a"`)
- Max: 255 characters
- Invalid: 256 characters

### Metadata Entry Count
- Min: 0 entries (empty HashMap)
- Max: No hard limit (database-dependent)
- Tested: 1, 10, 100, 1000, 10000 entries

### Metadata Value Size
- Min: 1 byte
- Max: 1MB (tested)
- Tested: 1B, 10KB, 100KB, 1MB

### UUID Edge Cases
- All zeros: `0x00000000000000000000000000000000`
- All ones: `0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF`
- First half zeros: `0x0000000000000000FFFFFFFFFFFFFFFF`
- Second half zeros: `0xFFFFFFFFFFFFFFFF0000000000000000`
- Standard v4 random: `WorktreeId::new_random()`

### Worktree Count
- Min: 0 worktrees
- Tested: 1, 100, 1000, 10000 worktrees

## Open Questions

1. **Test Database Isolation:** Should each test run in its own transaction or separate database?
2. **Connection Pool Size:** What is optimal pool size for parallel tests?
3. **Schema Cleanup:** Drop schema before each test or rely on transaction rollback?
4. **Fuzz Corpus Storage:** Where to store fuzz corpus seeds?
5. **Kani Integration:** Should Kani run in CI or locally only?
6. **Mutation Testing:** Should `cargo-mutants` run in CI or locally?

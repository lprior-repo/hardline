# ADR-009: DDD Layer Structure - Domain/Application/Infrastructure Separation

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline follows Domain-Driven Design principles with strict layer separation:

1. **Domain layer** - Pure business logic, no I/O, no dependencies
2. **Application layer** - Use cases, orchestration, coordinates domain
3. **Infrastructure layer** - External concerns (DB, VCS, network, filesystem)

The architecture spec requires this structure. This ADR formalizes it with invariants and examples.

---

## Decision

### Layer Structure

```
crates/core/src/
├── domain/                    # PURE - No external dependencies
│   ├── entities/            # Aggregate roots
│   │   ├── workspace.rs
│   │   ├── session.rs
│   │   ├── bead.rs
│   │   └── queue.rs
│   ├── value_objects/       # Immutable domain values
│   │   ├── priority.rs
│   │   ├── workspace_name.rs
│   │   ├── bead_id.rs
│   │   └── mod.rs
│   ├── events/              # Domain events
│   │   ├── workspace_created.rs
│   │   ├── session_completed.rs
│   │   └── mod.rs
│   ├── state/               # State machines
│   │   ├── workspace_state.rs
│   │   ├── session_state.rs
│   │   └── queue_status.rs
│   ├── repository/          # Repository traits (interfaces only)
│   │   ├── workspace_repo.rs
│   │   ├── session_repo.rs
│   │   └── mod.rs
│   └── mod.rs
│
├── application/              # IMPURE - Orchestrates domain
│   ├── workspace_service.rs  # Use cases
│   ├── session_service.rs
│   ├── queue_service.rs
│   ├── coordination.rs      # Multi-step workflows
│   └── mod.rs
│
├── infrastructure/           # IMPURE - External I/O
│   ├── database/            # SQLite persistence
│   │   ├── connection.rs
│   │   ├── migrations.rs
│   │   └── mod.rs
│   ├── vcs/                 # VCS integration
│   │   ├── git_backend.rs
│   │   ├── jj_backend.rs
│   │   └── mod.rs
│   ├── filesystem.rs        # File operations
│   └── mod.rs
│
├── api/                     # API boundaries (CLI, HTTP)
│   ├── cli/
│   │   ├── commands/
│   │   └── mod.rs
│   └── mod.rs
│
└── lib.rs                   # Public exports
```

### Domain Layer Rules

```rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN LAYER - MUST BE PURE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Domain layer has NO external dependencies.
/// This module imports NOTHING from:
/// - tokio
/// - sqlx
/// - std::fs
/// - std::process
///
/// Only allowed imports:
/// - std (without std::fs, std::process)
/// - serde (for serialization traits)
/// - chrono (DateTime for timestamps)
/// - thiserror (Error trait only)
```

### Domain Entity Example

```rust
// crates/core/src/domain/entities/workspace.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::domain::value_objects::{WorkspaceId, WorkspaceName, WorkspaceState};
use crate::domain::state::WorkspaceStateMachine;

/// Workspace aggregate root
///
/// INVARIANTS:
/// - id is globally unique
/// - name is unique within repository
/// - state transitions are valid per state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: PathBuf,           // But PathBuf is in domain - value object!
    pub backend: VcsType,
    pub state: WorkspaceState,
    pub agent_id: Option<AgentId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    /// Create a new workspace (domain factory)
    pub fn new(name: WorkspaceName, path: PathBuf, backend: VcsType) 
        -> Result<Self, WorkspaceError> 
    {
        // Validate invariants
        name.validate()?;  // Domain validation
        
        Ok(Workspace {
            id: WorkspaceId::new(),
            name,
            path,
            backend,
            state: WorkspaceState::Created,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    
    /// Transition to new state (domain behavior)
    pub fn activate(&mut self) -> Result<(), WorkspaceError> {
        self.state.transition_to(WorkspaceState::Active)?;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Claim workspace for agent
    pub fn claim(&mut self, agent_id: AgentId) -> Result<(), WorkspaceError> {
        if self.state != WorkspaceState::Active {
            return Err(WorkspaceError::InvalidState {
                current: self.state,
                operation: "claim",
            });
        }
        if self.agent_id.is_some() {
            return Err(WorkspaceError::AlreadyClaimed {
                by: self.agent_id.unwrap(),
            });
        }
        self.agent_id = Some(agent_id);
        self.updated_at = Utc::now();
        Ok(())
    }
}
```

### Value Object Example

```rust
// crates/core/src/domain/value_objects/priority.rs

use serde::{Deserialize, Serialize};

/// Priority value object
///
/// INVARIANTS:
/// - Value is within 0-4 range
/// - Is immutable (no setters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub const CRITICAL: Priority = Priority(0);
    pub const HIGH: Priority = Priority(1);
    pub const MEDIUM: Priority = Priority(2);
    pub const LOW: Priority = Priority(3);
    pub const BACKLOG: Priority = Priority(4);
    
    pub fn new(value: u8) -> Result<Self, PriorityError> {
        if value > 4 {
            return Err(PriorityError::InvalidValue(value));
        }
        Ok(Priority(value))
    }
    
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}
```

### Repository Trait (Domain Interface)

```rust
// crates/core/src/domain/repository/workspace_repo.rs

use crate::domain::entities::Workspace;
use crate::domain::value_objects::{WorkspaceId, WorkspaceName};

/// Workspace repository trait (domain interface)
///
/// CRITICAL: This is a TRAIT, not an implementation.
/// The domain defines the interface; infrastructure implements it.
pub trait WorkspaceRepository: Send + Sync {
    /// Save workspace to storage
    fn save(&self, workspace: &Workspace) -> Result<(), RepositoryError>;
    
    /// Find workspace by ID
    fn find_by_id(&self, id: &WorkspaceId) -> Result<Option<Workspace>, RepositoryError>;
    
    /// Find workspace by name
    fn find_by_name(&self, name: &WorkspaceName) -> Result<Option<Workspace>, RepositoryError>;
    
    /// List all workspaces
    fn list_all(&self) -> Result<Vec<Workspace>, RepositoryError>;
    
    /// Delete workspace
    fn delete(&self, id: &WorkspaceId) -> Result<(), RepositoryError>;
}
```

### Application Service (Orchestration)

```rust
// crates/core/src/application/workspace_service.rs

use crate::domain::entities::Workspace;
use crate::domain::repository::{WorkspaceRepository, WorkspaceRepoError};
use crate::infrastructure::vcs::VcsBackend;
use crate::infrastructure::filesystem::FilesystemError;

/// Workspace use cases (application layer)
///
/// This layer:
/// - Depends on domain AND infrastructure
/// - Orchestrates multiple domain operations
/// - Handles cross-cutting concerns (transactions, logging)
/// - IS IMPURE (uses async, database, filesystem)
pub struct WorkspaceService<R: WorkspaceRepository, V: VcsBackend> {
    workspace_repo: R,
    vcs_backend: V,
}

impl<R: WorkspaceRepository, V: VcsBackend> WorkspaceService<R, V> {
    /// Create workspace with VCS initialization
    pub async fn create_workspace(
        &self,
        name: WorkspaceName,
        path: PathBuf,
        backend: VcsType,
    ) -> Result<Workspace, WorkspaceServiceError> 
    {
        // 1. Create domain entity
        let workspace = Workspace::new(name.clone(), path.clone(), backend)?;
        
        // 2. Persist to database
        self.workspace_repo
            .save(&workspace)
            .map_err(WorkspaceServiceError::Repository)?;
        
        // 3. Initialize VCS (infrastructure concern)
        match backend {
            VcsType::Git => {
                self.vcs_backend.init(&path)?;
            }
            VcsType::JJ => {
                self.vcs_backend.workspace_create(&path, name.as_str())?;
            }
        }
        
        Ok(workspace)
    }
    
    /// Activate workspace and claim for agent
    pub async fn activate(
        &self,
        workspace_id: WorkspaceId,
        agent_id: AgentId,
    ) -> Result<Workspace, WorkspaceServiceError> {
        // 1. Load from database
        let mut workspace = self.workspace_repo
            .find_by_id(&workspace_id)
            .map_err(WorkspaceServiceError::Repository)?
            .ok_or(WorkspaceServiceError::NotFound(workspace_id))?;
        
        // 2. Domain logic
        workspace.activate()?;
        workspace.claim(agent_id)?;
        
        // 3. Persist
        self.workspace_repo
            .save(&workspace)
            .map_err(WorkspaceServiceError::Repository)?;
        
        Ok(workspace)
    }
}
```

### Infrastructure Implementation

```rust
// crates/core/src/infrastructure/database/workspace_repo.rs

use crate::domain::repository::WorkspaceRepository;
use crate::domain::entities::Workspace;
use crate::domain::value_objects::{WorkspaceId, WorkspaceName};
use sqlx::SqlitePool;

/// SQLite implementation of WorkspaceRepository
///
/// This is infrastructure - depends on sqlx, handles I/O
pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl WorkspaceRepository for SqliteWorkspaceRepository {
    fn save(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        // Convert domain entity to row
        sqlx::query(
            r#"
            INSERT INTO workspaces (id, name, path, backend, state, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                state = excluded.state,
                updated_at = excluded.updated_at
            "#
        )
        .bind(workspace.id.as_str())
        .bind(workspace.name.as_str())
        .bind(workspace.path.to_string_lossy())
        .bind(workspace.backend.as_str())
        .bind(workspace.state.as_str())
        .bind(workspace.created_at.to_rfc3339())
        .bind(workspace.updated_at.to_rfc3339())
        .execute(&self.pool)
        .map_err(RepositoryError::Database)?;
        
        Ok(())
    }
    
    fn find_by_id(&self, id: &WorkspaceId) -> Result<Option<Workspace>, RepositoryError> {
        let row = sqlx::query(
            "SELECT * FROM workspaces WHERE id = ?"
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .map_err(RepositoryError::Database)?;
        
        Ok(row.map(|r| self.row_to_workspace(r)))
    }
    
    // ... other methods
}
```

---

## Variants

### Variant A: No Layer Separation (REJECTED)

```rust
// Everything in one module
// Problems:
mod WorkspaceService {
    async fn create() {
        let db = SqlitePool::connect("...").await?;
        let repo = WorkspaceRepo::new(db);
        let workspace = Workspace::new(...)?;
        repo.save(workspace).await?;
        std::process::Command::new("git").arg("init").arg(&path);
    }
}
```

**Rejected because:**
- Impossible to test domain logic in isolation
- Hidden dependencies everywhere
- No clear boundaries

### Variant B: Full Hexagonal (Ports & Adapters) (DEFERRED)

```rust
// Ports: Repository trait in domain
// Adapters: SqliteWorkspaceAdapter, PostgresWorkspaceAdapter

// Problem: More ceremony than needed for hardline
```

**Deferred because:**
- More complex than necessary
- Hardline uses SQLite, single adapter
- Can evolve to hexagonal later

### Variant C: Domain/Application/Infrastructure (CHOSEN)

**Chosen because:**
- Simple, well-understood
- Clear separation of concerns
- Matches architecture spec exactly
- Testable

---

## Invariants

### Layer Dependency Invariants

```rust
/// INVARIANT: Domain layer depends on nothing outside domain
#[test]
fn domain_has_no_external_dependencies() {
    let domain_files = glob("crates/core/src/domain/**/*.rs");
    
    for file in domain_files {
        let content = read(file);
        
        // Domain must not import:
        assert!(!content.contains("tokio"));
        assert!(!content.contains("sqlx"));
        assert!(!content.contains("std::fs"));
        assert!(!content.contains("std::process"));
    }
}

/// INVARIANT: Application layer can depend on domain AND infrastructure
#[test]
fn application_layer_dependencies() {
    let app_files = glob("crates/core/src/application/**/*.rs");
    
    for file in app_files {
        let content = read(file);
        
        // Application MAY import infrastructure
        assert!(content.contains("crate::domain"));
        // And infrastructure
        assert!(content.contains("crate::infrastructure"));
    }
}

/// INVARIANT: Infrastructure depends only on domain (and external libs)
#[test]
fn infrastructure_layer_dependencies() {
    let infra_files = glob("crates/core/src/infrastructure/**/*.rs");
    
    for file in infra_files {
        let content = read(file);
        
        // Infrastructure may use domain
        assert!(content.contains("crate::domain"));
        // And external libraries
        assert!(content.contains("tokio") || content.contains("sqlx"));
    }
}
```

### Entity Invariants

```rust
/// INVARIANT: Entities have no mutable getters
/// (Enforces immutability where possible)
#[test]
fn entity_immutability() {
    let workspace = Workspace::new(...).unwrap();
    
    // Can't do this - no &mut self methods
    // workspace.state = WorkspaceState::Active;
    
    // Must do this - explicit domain method
    workspace.activate().unwrap();
}

/// INVARIANT: Entities validate on creation
#[test]
fn entity_validates_on_creation() {
    // Invalid name should fail
    let result = WorkspaceName::new("");
    assert!(result.is_err());
    
    // Valid name should succeed
    let result = WorkspaceName::new("valid-name");
    assert!(result.is_ok());
}
```

### State Machine Invariants

```rust
/// INVARIANT: State transitions are valid
#[test]
fn valid_state_transitions() {
    let mut workspace = Workspace::new(...).unwrap();
    
    // Created -> Active is valid
    assert!(workspace.activate().is_ok());
    
    // Active -> Created is NOT valid (no going backwards)
    assert!(workspace.to_created().is_err());
}

/// INVARIANT: Invalid transitions return errors, not panic
#[test]
fn invalid_transition_returns_error() {
    let workspace = Workspace::new(...).unwrap();
    
    let result = workspace.complete(); // Can't complete from Created
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WorkspaceError::InvalidState { .. }));
}
```

### Repository Invariants

```rust
/// INVARIANT: Repository trait has no implementation details
#[test]
fn repository_trait_is_pure_interface() {
    let source = read("crates/core/src/domain/repository/workspace_repo.rs");
    
    // No sqlx imports
    assert!(!source.contains("sqlx"));
    // No tokio imports
    assert!(!source.contains("tokio"));
    // Just trait definition
    assert!(source.contains("pub trait WorkspaceRepository"));
}

/// INVARIANT: Repository operations return domain entities
#[test]
fn repository_returns_domain_entities() {
    // find_by_id should return Workspace, not a database row
    fn signature<R: WorkspaceRepository>(repo: &R) 
        -> Result<Option<Workspace>, RepositoryError> 
    {
        repo.find_by_id(&WorkspaceId::new())
    }
}
```

---

## Consequences

### Positive

1. **Testability** - Domain logic can be tested without DB/VCS
2. **Maintainability** - Clear boundaries, easy to find code
3. **Flexibility** - Can swap infrastructure (e.g., Postgres for SQLite)
4. **Domain purity** - Business logic is clear of infrastructure concerns
5. **DDD alignment** - Matches architecture spec exactly

### Negative

1. **Initial overhead** - More modules, more files
2. **Indirection** - Need to trace through layers
3. **Trait满天飞** - Many traits for interfaces

### File Count Estimate

| Layer | Files |
|-------|-------|
| Domain | ~20 files |
| Application | ~10 files |
| Infrastructure | ~15 files |
| API | ~30 files |
| **Total** | ~75 files |

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/domain/mod.rs` | Module structure |
| `crates/core/src/application/mod.rs` | Application layer |
| `crates/core/src/infrastructure/mod.rs` | Infrastructure layer |
| `crates/core/src/domain/repository/*.rs` | Repository traits |

---

## Related ADRs

- ADR-001: CLI Architecture (API layer)
- ADR-004: VCS Abstraction (infrastructure layer)
- ADR-006: Database Schema (infrastructure layer)

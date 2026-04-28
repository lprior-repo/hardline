//! Repository pattern trait interfaces for DDD persistence abstraction.
//!
//! # Repository Pattern
//!
//! The repository pattern abstracts data access behind interfaces, enabling:
//! - **Dependency injection**: Business logic depends on traits, not concrete implementations
//! - **Testing**: Mock implementations for unit tests without real persistence
//! - **Swappable backends**: Switch between `SQLite`, `PostgreSQL`, in-memory, etc.
//! - **Functional core**: Pure business logic independent of I/O
//!
//! # Architecture
//!
//! This module defines trait interfaces in the **domain layer** (core):
//! - Traits use domain types (`SessionId`, `SessionName`, etc.) not primitives
//! - Methods return `Result`s for proper error handling
//! - Clear documentation of error conditions
//! - No implementation details (`SQLite`, files, etc.) leak through
//!
//! Implementations live in the **infrastructure layer** (shell):
//! - `beads/db.rs` implements `BeadRepository` over `SQLite`
//! - Future: `PostgreSQL`, `Redis`, or in-memory implementations
//!
//! # Design Principles
//!
//! 1. **Domain types in signatures**: Use `SessionId` not `String`, `WorkspaceName` not `&str`
//! 2. **Result returns**: All methods return `Result<T, E>` for error handling
//! 3. **Collection semantics**: List methods return iterators for lazy evaluation
//! 4. **Clear errors**: Each trait documents its error conditions
//! 5. **Testability**: Traits can be mocked for unit testing business logic
//!
//! # Example
//!
//! ```rust,ignore
//! use isolate_core::domain::repository::{SessionRepository, RepositoryError};
//! use isolate_core::domain::SessionName;
//!
//! // Business logic depends on trait (dependency injection)
//! fn get_active_sessions(repo: &dyn SessionRepository) -> Result<Vec<Session>, RepositoryError> {
//!     let all = repo.list_all()?;
//!     Ok(all.into_iter()
//!         .filter(|s| s.is_active())
//!         .collect())
//! }
//!
//! // Test with mock
//! struct MockSessionRepo { sessions: Vec<Session> }
//! impl SessionRepository for MockSessionRepo { /* ... */ }
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::{
    identifiers::{AgentId, BeadId, SessionId, SessionName, WorkspaceName},
    session::BranchState,
};

// ============================================================================
// SHARED ERROR TYPES
// ============================================================================

/// Common errors across all repository operations.
///
/// This error type covers expected failures in repository operations:
/// - **Not found**: Requested entity doesn't exist (informational, not exceptional)
/// - **Conflict**: Operation would violate constraints (duplicate IDs, etc.)
/// - **Invalid input**: Domain validation failed
/// - **Storage failure**: Underlying storage error (corruption, permissions, etc.)
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// Entity not found in repository
    #[error("entity not found: {0}")]
    NotFound(String),

    /// Conflict with existing data (duplicate, constraint violation)
    #[error("conflict: {0}")]
    Conflict(String),

    /// Invalid input for domain operation
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Underlying storage failure
    #[error("storage error: {0}")]
    StorageError(String),

    /// Operation not supported by repository
    #[error("operation not supported: {0}")]
    NotSupported(String),

    /// Concurrent modification conflict
    #[error("concurrent modification: {0}")]
    ConcurrentModification(String),
}

impl RepositoryError {
    /// Create a not found error
    #[must_use]
    pub fn not_found(entity: &str, id: impl std::fmt::Display) -> Self {
        Self::NotFound(format!("{entity} '{id}'"))
    }

    /// Create a conflict error
    #[must_use]
    pub fn conflict(reason: impl Into<String>) -> Self {
        Self::Conflict(reason.into())
    }

    /// Create an invalid input error
    #[must_use]
    pub fn invalid_input(reason: impl Into<String>) -> Self {
        Self::InvalidInput(reason.into())
    }

    /// Create a storage error
    #[must_use]
    pub fn storage_error(reason: impl Into<String>) -> Self {
        Self::StorageError(reason.into())
    }
}

/// Result type alias for repository operations
pub type RepositoryResult<T> = Result<T, RepositoryError>;

// ============================================================================
// SESSION AGGREGATE
// ============================================================================

/// Session aggregate root.
///
/// In DDD, an aggregate is a cluster of domain objects treated as a unit.
/// Session is the aggregate root for session-related data.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Human-readable session name
    pub name: SessionName,
    /// Branch state (detached or on branch)
    pub branch: BranchState,
    /// Absolute path to workspace root
    pub workspace_path: PathBuf,
}

impl Session {
    /// Check if session is active (has a valid branch and workspace)
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.branch.is_detached() && self.workspace_path.exists()
    }
}

// ============================================================================
// SESSION REPOSITORY
// ============================================================================

/// Repository for Session aggregate operations.
///
/// Provides CRUD operations for sessions with domain semantics.
/// Implementations must handle all error conditions documented below.
///
/// # Error Conditions
///
/// - `NotFound`: Session with given ID/name doesn't exist
/// - `Conflict`: Session name already exists (on create), concurrent modification
/// - `InvalidInput`: Invalid session name or ID format
/// - `StorageError`: Database/file corruption, permissions, I/O errors
pub trait SessionRepository: Send + Sync {
    /// Load a session by its unique ID.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if no session with the given ID exists.
    /// Returns `StorageError` on database/file access failure.
    fn load(&self, id: &SessionId) -> RepositoryResult<Session>;

    /// Load a session by its human-readable name.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if no session with the given name exists.
    /// Returns `StorageError` on database/file access failure.
    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session>;

    /// Save a session (create or update).
    ///
    /// If the session ID already exists, updates the session.
    /// If the session ID is new, creates a new session.
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if session name already exists (for new sessions).
    /// Returns `InvalidInput` if session data is invalid.
    /// Returns `StorageError` on database/file write failure.
    fn save(&self, session: &Session) -> RepositoryResult<()>;

    /// Delete a session by ID.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if session doesn't exist.
    /// Returns `StorageError` on database/file deletion failure.
    fn delete(&self, id: &SessionId) -> RepositoryResult<()>;

    /// List all sessions.
    ///
    /// Returns an iterator over all sessions in undefined order.
    /// For sorted results, use `list_sorted_by_name`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on database/file read failure.
    fn list_all(&self) -> RepositoryResult<Vec<Session>>;

    /// List sessions sorted by name.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on database/file read failure.
    fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>> {
        let mut sessions = self.list_all()?;
        sessions.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        Ok(sessions)
    }

    /// Check if a session exists by ID.
    ///
    /// Returns `false` if session doesn't exist (not an error).
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on database/file access failure.
    fn exists(&self, id: &SessionId) -> RepositoryResult<bool> {
        match self.load(id) {
            Ok(_) => Ok(true),
            Err(RepositoryError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get the current (active) session.
    ///
    /// Returns `None` if no session is currently active.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on database/file read failure.
    fn get_current(&self) -> RepositoryResult<Option<Session>>;

    /// Set the current (active) session.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if session doesn't exist.
    /// Returns `StorageError` on state persistence failure.
    fn set_current(&self, id: &SessionId) -> RepositoryResult<()>;

    /// Clear the current session (no active session).
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on state persistence failure.
    fn clear_current(&self) -> RepositoryResult<()>;
}

// ============================================================================
// WORKSPACE AGGREGATE
// ============================================================================

/// Workspace aggregate root.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Workspace name
    pub name: WorkspaceName,
    /// Absolute path to workspace
    pub path: PathBuf,
    /// Current workspace state
    pub state: WorkspaceState,
}

/// Workspace state (from domain/workspace.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,
}

impl WorkspaceState {
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready | Self::Active)
    }

    #[must_use]
    pub const fn is_removed(&self) -> bool {
        matches!(self, Self::Removed)
    }
}

// ============================================================================
// WORKSPACE REPOSITORY
// ============================================================================

/// Repository for Workspace aggregate operations.
///
/// Provides CRUD operations for workspaces with domain semantics.
///
/// # Error Conditions
///
/// - `NotFound`: Workspace with given name doesn't exist
/// - `Conflict`: Workspace name already exists (on create)
/// - `InvalidInput`: Invalid workspace name or path
/// - `StorageError`: Database/file corruption, permissions, I/O errors
pub trait WorkspaceRepository: Send + Sync {
    /// Load a workspace by name.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if workspace doesn't exist.
    /// Returns `StorageError` on access failure.
    fn load(&self, name: &WorkspaceName) -> RepositoryResult<Workspace>;

    /// Save a workspace (create or update).
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if workspace name already exists.
    /// Returns `InvalidInput` if workspace data is invalid.
    /// Returns `StorageError` on write failure.
    fn save(&self, workspace: &Workspace) -> RepositoryResult<()>;

    /// Delete a workspace by name.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if workspace doesn't exist.
    /// Returns `StorageError` on deletion failure.
    fn delete(&self, name: &WorkspaceName) -> RepositoryResult<()>;

    /// List all workspaces.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on read failure.
    fn list_all(&self) -> RepositoryResult<Vec<Workspace>>;

    /// Check if workspace exists.
    ///
    /// Returns `false` if workspace doesn't exist (not an error).
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on access failure.
    fn exists(&self, name: &WorkspaceName) -> RepositoryResult<bool> {
        match self.load(name) {
            Ok(_) => Ok(true),
            Err(RepositoryError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

// ============================================================================
// BEAD AGGREGATE
// ============================================================================

/// Bead aggregate root (issue/task).
///
/// Represents a single unit of work in the beads issue tracker.
/// Uses domain types from the beads module.
#[derive(Debug, Clone)]
pub struct Bead {
    /// Unique bead identifier
    pub id: BeadId,
    /// Bead title
    pub title: String,
    /// Bead description (optional)
    pub description: Option<String>,
    /// Current state
    pub state: BeadState,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modification timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Bead state (from beads/domain.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed {
        closed_at: chrono::DateTime<chrono::Utc>,
    },
}

impl BeadState {
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

// ============================================================================
// BEAD REPOSITORY
// ============================================================================

/// Repository for Bead aggregate operations.
///
/// Provides CRUD operations for beads/issues with domain semantics.
///
/// # Error Conditions
///
/// - `NotFound`: Bead with given ID doesn't exist
/// - `Conflict`: Bead ID already exists (on create)
/// - `InvalidInput`: Invalid bead data (title too long, etc.)
/// - `StorageError`: Database corruption, permissions, I/O errors
pub trait BeadRepository: Send + Sync {
    /// Load a bead by ID.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if bead doesn't exist.
    /// Returns `StorageError` on access failure.
    fn load(&self, id: &BeadId) -> RepositoryResult<Bead>;

    /// Save a bead (create or update).
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if bead ID already exists.
    /// Returns `InvalidInput` if bead data violates constraints.
    /// Returns `StorageError` on write failure.
    fn save(&self, bead: &Bead) -> RepositoryResult<()>;

    /// Delete a bead by ID.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if bead doesn't exist.
    /// Returns `StorageError` on deletion failure.
    fn delete(&self, id: &BeadId) -> RepositoryResult<()>;

    /// List all beads.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on read failure.
    fn list_all(&self) -> RepositoryResult<Vec<Bead>>;

    /// List beads filtered by state.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on read failure.
    fn list_by_state(&self, state: BeadState) -> RepositoryResult<Vec<Bead>> {
        self.list_all()
            .map(|beads| beads.into_iter().filter(|b| b.state == state).collect())
    }

    /// Check if bead exists.
    ///
    /// Returns `false` if bead doesn't exist (not an error).
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on access failure.
    fn exists(&self, id: &BeadId) -> RepositoryResult<bool> {
        match self.load(id) {
            Ok(_) => Ok(true),
            Err(RepositoryError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

// ============================================================================
// AGENT REPOSITORY
// ============================================================================

/// Agent information.
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: AgentId,
    pub state: AgentState,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

/// Agent state (from domain/agent.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Active,
    Idle,
    Offline,
    Error,
}

impl AgentState {
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn is_offline(&self) -> bool {
        matches!(self, Self::Offline)
    }
}

/// Repository for Agent operations.
///
/// Provides CRUD operations for agent registration and heartbeat.
pub trait AgentRepository: Send + Sync {
    /// Load an agent by ID.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if agent doesn't exist.
    /// Returns `StorageError` on access failure.
    fn load(&self, id: &AgentId) -> RepositoryResult<Agent>;

    /// Save agent information.
    ///
    /// # Errors
    ///
    /// Returns `Conflict` if agent ID already exists.
    /// Returns `InvalidInput` if agent data is invalid.
    /// Returns `StorageError` on write failure.
    fn save(&self, agent: &Agent) -> RepositoryResult<()>;

    /// Update agent heartbeat timestamp.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if agent doesn't exist.
    /// Returns `StorageError` on write failure.
    fn heartbeat(&self, id: &AgentId) -> RepositoryResult<()>;

    /// List all agents.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on access failure.
    fn list_all(&self) -> RepositoryResult<Vec<Agent>>;

    /// List active agents.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on access failure.
    fn list_active(&self) -> RepositoryResult<Vec<Agent>> {
        self.list_all()
            .map(|agents| agents.into_iter().filter(|a| a.state.is_active()).collect())
    }
}

// ============================================================================
// RE-EXPORTS
// ============================================================================

// Note: Domain types are already re-exported by the parent module.
// This module only defines the repository traits and aggregates.

// ============================================================================
// MOCK IMPLEMENTATIONS FOR TESTING
// ============================================================================

#[cfg(test)]
mod mock_tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// In-memory session repository for testing.
    struct MockSessionRepo {
        sessions: Arc<Mutex<Vec<Session>>>,
    }

    impl MockSessionRepo {
        fn new() -> Self {
            Self {
                sessions: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl SessionRepository for MockSessionRepo {
        fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
            self.sessions
                .lock()
                .map_err(|e| RepositoryError::StorageError(e.to_string()))?
                .iter()
                .find(|s| &s.id == id)
                .cloned()
                .ok_or_else(|| RepositoryError::not_found("session", id))
        }

        fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session> {
            self.sessions
                .lock()
                .map_err(|e| RepositoryError::StorageError(e.to_string()))?
                .iter()
                .find(|s| &s.name == name)
                .cloned()
                .ok_or_else(|| RepositoryError::not_found("session", name))
        }

        fn save(&self, session: &Session) -> RepositoryResult<()> {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

            if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
                sessions[pos] = session.clone();
            } else {
                sessions.push(session.clone());
            }
            drop(sessions);
            Ok(())
        }

        fn delete(&self, id: &SessionId) -> RepositoryResult<()> {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|e| RepositoryError::StorageError(e.to_string()))?;

            let pos = sessions
                .iter()
                .position(|s| &s.id == id)
                .ok_or_else(|| RepositoryError::not_found("session", id))?;

            sessions.remove(pos);
            drop(sessions);
            Ok(())
        }

        fn list_all(&self) -> RepositoryResult<Vec<Session>> {
            self.sessions
                .lock()
                .map_err(|e| RepositoryError::StorageError(e.to_string()))
                .map(|v| v.clone())
        }

        fn get_current(&self) -> RepositoryResult<Option<Session>> {
            // Simple mock: return first session
            self.list_all().map(|sessions| sessions.first().cloned())
        }

        fn set_current(&self, id: &SessionId) -> RepositoryResult<()> {
            // Verify session exists
            self.load(id)?;
            Ok(())
        }

        fn clear_current(&self) -> RepositoryResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_session_repo() {
        let repo = MockSessionRepo::new();

        let id = SessionId::parse("test-session-1").expect("valid id");
        let name = SessionName::parse("test-session").expect("valid name");
        let session = Session {
            id: id.clone(),
            name,
            branch: BranchState::OnBranch {
                name: "main".to_string(),
            },
            workspace_path: PathBuf::from("/tmp/test"),
        };

        // Save and load
        repo.save(&session).expect("save works");
        let loaded = repo.load(&id).expect("load works");
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.name.as_str(), "test-session");

        // List all
        let all = repo.list_all().expect("list works");
        assert_eq!(all.len(), 1);

        // Delete
        repo.delete(&id).expect("delete works");
        let result = repo.load(&id);
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));

        // Exists check
        let exists = repo.exists(&id).expect("exists works");
        assert!(!exists);
    }
}

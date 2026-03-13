//! Session aggregate and repository trait.
//!
//! Provides the Session aggregate root and repository interface for session persistence.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::{
    identifiers::{SessionId, SessionName},
    session::BranchState,
};

use super::error::{RepositoryError, RepositoryResult};

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

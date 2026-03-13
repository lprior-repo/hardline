//! Workspace aggregate and repository trait.
//!
//! Provides the Workspace aggregate root and repository interface for workspace persistence.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::domain::identifiers::WorkspaceName;

use super::error::{RepositoryError, RepositoryResult};

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

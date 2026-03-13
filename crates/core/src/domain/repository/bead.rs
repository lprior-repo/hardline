//! Bead aggregate and repository trait.
//!
//! Provides the Bead aggregate root and repository interface for bead/issue persistence.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

use crate::domain::identifiers::BeadId;

use super::error::{RepositoryError, RepositoryResult};

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
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

/// Bead state (from beads/domain.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeadState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed { closed_at: DateTime<Utc> },
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

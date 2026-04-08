//! BeadRepository trait for domain layer persistence abstraction.
//!
//! This trait defines the contract for bead persistence operations.
//! Implementations live in the infrastructure layer.

use crate::domain::entities::bead::Bead;
use crate::domain::value_objects::{BeadId, BeadState, Priority};
use crate::error::Result;

/// Repository trait for Bead aggregate persistence.
///
/// Provides CRUD operations with domain semantics:
/// - `insert`: Create new bead (fails if ID exists)
/// - `update`: Modify existing bead (fails if ID not found)
/// - `delete`: Remove bead (fails if ID not found)
/// - `find`: Load bead by ID (returns None if not found)
/// - `find_all`: List all beads
/// - `find_by_state`: Filter beads by state
/// - `exists`: Check bead existence
///
/// # Error Conditions
///
/// - `BeadError::AlreadyExists`: Insert with duplicate ID
/// - `BeadError::NotFound`: Update/delete non-existent bead
pub trait BeadRepository: Send + Sync {
    /// Insert a new bead.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::AlreadyExists` if bead ID already exists.
    fn insert(&self, bead: &Bead) -> Result<()>;

    /// Update an existing bead.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::NotFound` if bead ID doesn't exist.
    fn update(&self, bead: &Bead) -> Result<()>;

    /// Delete a bead by ID.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::NotFound` if bead ID doesn't exist.
    fn delete(&self, id: &BeadId) -> Result<()>;

    /// Find a bead by ID.
    ///
    /// Returns `Ok(Some(bead))` if found, `Ok(None)` if not found.
    fn find(&self, id: &BeadId) -> Result<Option<Bead>>;

    /// List all beads.
    ///
    /// Returns empty vector if no beads exist.
    fn find_all(&self) -> Result<Vec<Bead>>;

    /// Find beads by state.
    ///
    /// Returns empty vector if no beads match the state.
    fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>>;

    /// Find beads by assignee.
    ///
    /// Returns empty vector if no beads match the assignee.
    /// If `assignee` is `None`, returns beads with no assignee.
    fn find_by_assignee(&self, assignee: Option<&str>) -> Result<Vec<Bead>>;

    /// Find beads by priority.
    ///
    /// Returns empty vector if no beads match the priority.
    /// If `priority` is `None`, returns beads with no priority set.
    fn find_by_priority(&self, priority: Option<Priority>) -> Result<Vec<Bead>>;

    /// Check if a bead exists.
    ///
    /// Returns `true` if bead exists, `false` otherwise.
    fn exists(&self, id: &BeadId) -> bool;
}

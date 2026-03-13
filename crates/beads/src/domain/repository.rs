//! BeadRepository trait for domain layer persistence abstraction.
//!
//! This trait defines the contract for bead persistence operations.
//! Implementations live in the infrastructure layer.

use async_trait::async_trait;

use crate::domain::entities::bead::Bead;
use crate::domain::value_objects::{BeadId, BeadState};
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
#[async_trait]
pub trait BeadRepository: Send + Sync {
    /// Insert a new bead.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::AlreadyExists` if bead ID already exists.
    async fn insert(&self, bead: &Bead) -> Result<()>;

    /// Update an existing bead.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::NotFound` if bead ID doesn't exist.
    async fn update(&self, bead: &Bead) -> Result<()>;

    /// Delete a bead by ID.
    ///
    /// # Errors
    ///
    /// Returns `BeadError::NotFound` if bead ID doesn't exist.
    async fn delete(&self, id: &BeadId) -> Result<()>;

    /// Find a bead by ID.
    ///
    /// Returns `Ok(Some(bead))` if found, `Ok(None)` if not found.
    async fn find(&self, id: &BeadId) -> Result<Option<Bead>>;

    /// List all beads.
    ///
    /// Returns empty vector if no beads exist.
    async fn find_all(&self) -> Result<Vec<Bead>>;

    /// Find beads by state.
    ///
    /// Returns empty vector if no beads match the state.
    async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>>;

    /// Check if a bead exists.
    ///
    /// Returns `true` if bead exists, `false` otherwise.
    async fn exists(&self, id: &BeadId) -> bool;
}

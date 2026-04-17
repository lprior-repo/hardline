//! Agent aggregate and repository trait.
//!
//! Provides the Agent aggregate and repository interface for agent persistence.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

use crate::domain::identifiers::AgentId;

use super::error::RepositoryResult;

/// Agent information.
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: AgentId,
    pub state: AgentState,
    pub last_seen: Option<DateTime<Utc>>,
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

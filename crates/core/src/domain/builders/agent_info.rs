// //! Agent info builder
//!
//! Builder for `AgentInfo` with fluent API.

use chrono::{DateTime, Utc};

use crate::domain::agent::{AgentInfo, AgentState as OutputAgentState};

/// Builder for `AgentInfo` with fluent API
///
/// # Required Fields
/// - `id`: Agent ID
/// - `state`: Agent state
///
/// # Optional Fields
/// - `last_seen`: Last seen timestamp
#[derive(Debug, Clone)]
pub struct AgentInfoBuilder {
    // Required fields
    id: Option<crate::domain::AgentId>,
    state: Option<AgentState>,

    // Optional fields
    last_seen: Option<DateTime<Utc>>,
}

/// Agent state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Active,
    Idle,
    Offline,
    Error,
}

impl Default for AgentInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentInfoBuilder {
    /// Create a new builder with no fields set
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            state: None,
            last_seen: None,
        }
    }

    /// Set the agent ID (required)
    #[must_use]
    pub fn id(mut self, id: crate::domain::AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the agent state (required)
    #[must_use]
    pub const fn state(mut self, state: AgentState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the last seen timestamp (optional)
    #[must_use]
    pub const fn last_seen(mut self, last_seen: DateTime<Utc>) -> Self {
        self.last_seen = Some(last_seen);
        self
    }

    /// Build the `AgentInfo`
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRequired` if any required field is not set.
    pub fn build(self) -> Result<AgentInfo, super::errors::BuilderError> {
        let id = self
            .id
            .ok_or(super::errors::BuilderError::MissingRequired { field: "id" })?;
        let state = self
            .state
            .ok_or(super::errors::BuilderError::MissingRequired { field: "state" })?;

        Ok(AgentInfo {
            id,
            state: convert_agent_state(state),
            last_seen: self.last_seen,
        })
    }
}

const fn convert_agent_state(state: AgentState) -> OutputAgentState {
    match state {
        AgentState::Active => OutputAgentState::Active,
        AgentState::Idle => OutputAgentState::Idle,
        AgentState::Offline => OutputAgentState::Offline,
        AgentState::Error => OutputAgentState::Error,
    }
}

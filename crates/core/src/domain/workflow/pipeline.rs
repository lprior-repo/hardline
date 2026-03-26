//! Pipeline orchestration types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::states::PipelineState;

// =============================================================================
// Pipeline ID
// =============================================================================

/// Unique identifier for a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineId(pub String);

impl PipelineId {
    #[must_use]
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        Self(format!("pipeline-{}", timestamp))
    }
}

impl Default for PipelineId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PipelineId({})", self.0)
    }
}

// =============================================================================
// Pipeline Config
// =============================================================================

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub max_iterations: u32,
    pub quality_threshold: u32,
    pub scenarios_path: String,
    pub linter_path: Option<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            quality_threshold: 80,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        }
    }
}

// =============================================================================
// Pipeline
// =============================================================================

/// Pipeline instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub spec_path: String,
    pub state: PipelineState,
    pub iteration: u32,
    pub max_iterations: u32,
    pub quality_threshold: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

impl Pipeline {
    #[must_use]
    pub fn new(spec_path: String) -> Self {
        let now = Utc::now();
        Self {
            id: PipelineId::new(),
            spec_path,
            state: PipelineState::Pending,
            iteration: 0,
            max_iterations: 10,
            quality_threshold: 80,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    #[must_use]
    pub fn with_config(spec_path: String, config: &PipelineConfig) -> Self {
        let now = Utc::now();
        Self {
            id: PipelineId::new(),
            spec_path,
            state: PipelineState::Pending,
            iteration: 0,
            max_iterations: config.max_iterations,
            quality_threshold: config.quality_threshold,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    pub fn transition_to(
        &mut self,
        new_state: PipelineState,
    ) -> Result<(), PipelineTransitionError> {
        self.validate_transition(&new_state)?;

        self.state = new_state;
        self.updated_at = Utc::now();
        Ok(())
    }

    fn validate_transition(
        &self,
        new_state: &PipelineState,
    ) -> Result<(), PipelineTransitionError> {
        if self.is_transition_valid(new_state) {
            Ok(())
        } else if self.state.is_terminal() {
            Err(PipelineTransitionError::AlreadyTerminal {
                current: self.state,
            })
        } else {
            Err(PipelineTransitionError::InvalidTransition {
                from: self.state,
                to: *new_state,
            })
        }
    }

    fn is_transition_valid(&self, new_state: &PipelineState) -> bool {
        self.is_phase_transition(new_state) || self.is_catchall_transition(new_state)
    }

    fn is_phase_transition(&self, new_state: &PipelineState) -> bool {
        matches!(
            (&self.state, new_state),
            (PipelineState::Pending, PipelineState::SpecReview)
                | (PipelineState::SpecReview, PipelineState::UniverseSetup)
                | (PipelineState::SpecReview, PipelineState::Failed)
                | (PipelineState::SpecReview, PipelineState::Escalated)
                | (
                    PipelineState::UniverseSetup,
                    PipelineState::AgentDevelopment
                )
                | (PipelineState::UniverseSetup, PipelineState::Failed)
                | (PipelineState::UniverseSetup, PipelineState::Escalated)
                | (PipelineState::AgentDevelopment, PipelineState::Validation)
                | (
                    PipelineState::AgentDevelopment,
                    PipelineState::AgentDevelopment
                )
                | (PipelineState::AgentDevelopment, PipelineState::Escalated)
                | (PipelineState::Validation, PipelineState::Accepted)
                | (PipelineState::Validation, PipelineState::AgentDevelopment)
                | (PipelineState::Validation, PipelineState::Failed)
                | (PipelineState::Validation, PipelineState::Escalated)
        )
    }

    fn is_catchall_transition(&self, new_state: &PipelineState) -> bool {
        matches!(new_state, PipelineState::Failed | PipelineState::Escalated)
    }

    pub fn increment_iteration(&mut self) -> Result<u32, IterationLimitError> {
        if self.iteration >= self.max_iterations {
            return Err(IterationLimitError {
                current: self.iteration,
                max: self.max_iterations,
            });
        }
        self.iteration += 1;
        self.updated_at = Utc::now();
        Ok(self.iteration)
    }

    #[must_use]
    pub fn can_iterate(&self) -> bool {
        self.iteration < self.max_iterations && self.state.allows_iteration()
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.updated_at = Utc::now();
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
        self.updated_at = Utc::now();
    }
}

// =============================================================================
// Errors
// =============================================================================

/// Error when transitioning pipeline states
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineTransitionError {
    InvalidTransition {
        from: PipelineState,
        to: PipelineState,
    },
    AlreadyTerminal {
        current: PipelineState,
    },
}

impl std::fmt::Display for PipelineTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineTransitionError::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {from:?} to {to:?}")
            }
            PipelineTransitionError::AlreadyTerminal { current } => {
                write!(f, "Pipeline already in terminal state: {current:?}")
            }
        }
    }
}

impl std::error::Error for PipelineTransitionError {}

/// Error during iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IterationLimitError {
    pub current: u32,
    pub max: u32,
}

impl std::fmt::Display for IterationLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Max iterations reached: {} of {}",
            self.current, self.max
        )
    }
}

impl std::error::Error for IterationLimitError {}

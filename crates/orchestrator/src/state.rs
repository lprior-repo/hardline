//! Pipeline state machine types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineId(pub String);

impl PipelineId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
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

/// Pipeline state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    /// Initial state - pipeline created but not started
    Pending,
    /// Running linter on spec
    SpecReview,
    /// Deploying twin/universe
    UniverseSetup,
    /// Agent working (with iteration count)
    AgentDevelopment,
    /// Running scenarios for validation
    Validation,
    /// All scenarios passed - artifact ready for merge
    Accepted,
    /// Human intervention needed
    Escalated,
    /// Validation failed permanently
    Failed,
}

impl PipelineState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineState::Accepted | PipelineState::Escalated | PipelineState::Failed
        )
    }

    #[must_use]
    pub fn allows_iteration(&self) -> bool {
        matches!(self, PipelineState::AgentDevelopment)
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            PipelineState::Pending => "Pending - awaiting start",
            PipelineState::SpecReview => "Spec Review - running linter",
            PipelineState::UniverseSetup => "Universe Setup - deploying twin",
            PipelineState::AgentDevelopment => "Agent Development - working on task",
            PipelineState::Validation => "Validation - running scenarios",
            PipelineState::Accepted => "Accepted - all scenarios passed",
            PipelineState::Escalated => "Escalated - human intervention needed",
            PipelineState::Failed => "Failed - validation failed",
        }
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Maximum number of agent iterations
    pub max_iterations: u32,
    /// Minimum quality threshold for spec (0-100)
    pub quality_threshold: u32,
    /// Path to scenarios directory
    pub scenarios_path: String,
    /// Path to linter binary
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

    pub fn transition_to(&mut self, new_state: PipelineState) -> Result<(), TransitionError> {
        match (&self.state, &new_state) {
            (PipelineState::Pending, PipelineState::SpecReview) => {}
            (PipelineState::SpecReview, PipelineState::UniverseSetup) => {}
            (PipelineState::SpecReview, PipelineState::Failed) => {}
            (PipelineState::SpecReview, PipelineState::Escalated) => {}
            (PipelineState::UniverseSetup, PipelineState::AgentDevelopment) => {}
            (PipelineState::UniverseSetup, PipelineState::Failed) => {}
            (PipelineState::UniverseSetup, PipelineState::Escalated) => {}
            (PipelineState::AgentDevelopment, PipelineState::Validation) => {}
            (PipelineState::AgentDevelopment, PipelineState::AgentDevelopment) => {}
            (PipelineState::AgentDevelopment, PipelineState::Escalated) => {}
            (PipelineState::Validation, PipelineState::Accepted) => {}
            (PipelineState::Validation, PipelineState::AgentDevelopment) => {}
            (PipelineState::Validation, PipelineState::Failed) => {}
            (PipelineState::Validation, PipelineState::Escalated) => {}
            (state, _) if state.is_terminal() => {
                return Err(TransitionError::AlreadyTerminal { current: *state });
            }
            (_, PipelineState::Failed) => {}
            (_, PipelineState::Escalated) => {}
            _ => {
                return Err(TransitionError::InvalidTransition {
                    from: self.state,
                    to: new_state,
                });
            }
        }

        self.state = new_state;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn increment_iteration(&mut self) -> Result<u32, IterationError> {
        if self.iteration >= self.max_iterations {
            return Err(IterationError::MaxIterationsReached {
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

/// Error when transitioning states
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionError {
    InvalidTransition {
        from: PipelineState,
        to: PipelineState,
    },
    AlreadyTerminal {
        current: PipelineState,
    },
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {from:?} to {to:?}")
            }
            TransitionError::AlreadyTerminal { current } => {
                write!(f, "Pipeline already in terminal state: {current:?}")
            }
        }
    }
}

impl std::error::Error for TransitionError {}

/// Error during iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationError {
    MaxIterationsReached { current: u32, max: u32 },
}

impl std::fmt::Display for IterationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IterationError::MaxIterationsReached { current, max } => {
                write!(f, "Max iterations reached: {current} of {max}")
            }
        }
    }
}

impl std::error::Error for IterationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert_eq!(pipeline.state, PipelineState::Pending);
        assert_eq!(pipeline.iteration, 0);
    }

    #[test]
    fn test_valid_transitions() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert!(pipeline.transition_to(PipelineState::SpecReview).is_ok());
        assert!(pipeline.transition_to(PipelineState::UniverseSetup).is_ok());
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_ok());
    }

    #[test]
    fn test_invalid_transition() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        let result = pipeline.transition_to(PipelineState::Validation);
        assert!(result.is_err());
    }

    #[test]
    fn test_iteration_limit() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.transition_to(PipelineState::SpecReview).ok();
        pipeline.transition_to(PipelineState::UniverseSetup).ok();
        pipeline.transition_to(PipelineState::AgentDevelopment).ok();

        for _ in 0..10 {
            assert!(pipeline.increment_iteration().is_ok());
        }

        assert!(pipeline.increment_iteration().is_err());
    }

    #[test]
    fn test_terminal_state_no_transition() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.transition_to(PipelineState::SpecReview).ok();
        pipeline.transition_to(PipelineState::UniverseSetup).ok();
        pipeline.transition_to(PipelineState::AgentDevelopment).ok();
        pipeline.transition_to(PipelineState::Validation).ok();
        pipeline.transition_to(PipelineState::Accepted).ok();

        assert!(pipeline.state.is_terminal());
        let result = pipeline.transition_to(PipelineState::Failed);
        assert!(result.is_err());
    }
}

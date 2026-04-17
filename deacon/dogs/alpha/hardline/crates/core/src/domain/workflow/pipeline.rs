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
            .map_or(0, |d| d.as_nanos());

        Self(format!("pipeline-{timestamp}"))
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

    /// Transition the pipeline to a new state.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is invalid or the pipeline is already
    /// in a terminal state.
    pub fn transition_to(
        &mut self,
        new_state: PipelineState,
    ) -> Result<(), PipelineTransitionError> {
        self.validate_transition(new_state)?;

        self.state = new_state;
        self.updated_at = Utc::now();
        Ok(())
    }

    const fn validate_transition(
        &self,
        new_state: PipelineState,
    ) -> Result<(), PipelineTransitionError> {
        if self.state.is_terminal() {
            return Err(PipelineTransitionError::AlreadyTerminal {
                current: self.state,
            });
        }
        if self.is_transition_valid(new_state) {
            Ok(())
        } else {
            Err(PipelineTransitionError::InvalidTransition {
                from: self.state,
                to: new_state,
            })
        }
    }

    const fn is_transition_valid(&self, new_state: PipelineState) -> bool {
        self.is_phase_transition(new_state) || Self::is_catchall_transition(new_state)
    }

    const fn is_phase_transition(&self, new_state: PipelineState) -> bool {
        matches!(
            (&self.state, new_state),
            (PipelineState::Pending, PipelineState::SpecReview)
                | (
                    PipelineState::SpecReview,
                    PipelineState::UniverseSetup | PipelineState::Failed | PipelineState::Escalated
                )
                | (
                    PipelineState::UniverseSetup
                        | PipelineState::AgentDevelopment
                        | PipelineState::Validation,
                    PipelineState::AgentDevelopment | PipelineState::Escalated
                )
                | (
                    PipelineState::UniverseSetup | PipelineState::Validation,
                    PipelineState::Failed
                )
                | (PipelineState::AgentDevelopment, PipelineState::Validation)
                | (PipelineState::Validation, PipelineState::Accepted)
        )
    }

    const fn is_catchall_transition(new_state: PipelineState) -> bool {
        matches!(new_state, PipelineState::Failed | PipelineState::Escalated)
    }

    /// Increment the iteration counter.
    ///
    /// # Errors
    ///
    /// Returns an error if the iteration limit has been reached.
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
    pub const fn can_iterate(&self) -> bool {
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
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {from:?} to {to:?}")
            }
            Self::AlreadyTerminal { current } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workflow::states::PipelineState;

    // =========================================================================
    // Helper
    // =========================================================================

    fn fresh_pipeline() -> Pipeline {
        Pipeline::new("spec.yaml".to_string())
    }

    /// Force the pipeline into a given state for testing, bypassing validation.
    fn force_state(pipeline: &mut Pipeline, state: PipelineState) {
        pipeline.state = state;
    }

    // =========================================================================
    // PipelineId uniqueness
    // =========================================================================

    #[test]
    fn two_pipeline_ids_differ() {
        let id_a = PipelineId::new();
        let id_b = PipelineId::new();
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn pipeline_id_contains_prefix() {
        let id = PipelineId::new();
        assert!(id.0.starts_with("pipeline-"));
    }

    #[test]
    fn pipeline_id_display_format() {
        let id = PipelineId::new();
        let displayed = id.to_string();
        assert!(displayed.starts_with("PipelineId("));
        assert!(displayed.ends_with(')'));
    }

    #[test]
    fn pipeline_id_default_trait() {
        let id = PipelineId::default();
        assert!(id.0.starts_with("pipeline-"));
    }

    // =========================================================================
    // PipelineConfig defaults
    // =========================================================================

    #[test]
    fn pipeline_config_defaults_are_sensible() {
        let config = PipelineConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.quality_threshold, 80);
        assert_eq!(config.scenarios_path, "scenarios");
        assert!(config.linter_path.is_none());
    }

    #[test]
    fn pipeline_new_uses_default_config_values() {
        let p = fresh_pipeline();
        assert_eq!(p.max_iterations, 10);
        assert_eq!(p.quality_threshold, 80);
        assert_eq!(p.iteration, 0);
        assert!(p.last_error.is_none());
    }

    #[test]
    fn pipeline_with_config_applies_custom_values() {
        let config = PipelineConfig {
            max_iterations: 3,
            quality_threshold: 95,
            scenarios_path: "custom/scenarios".to_string(),
            linter_path: Some("bin/lint".to_string()),
        };
        let p = Pipeline::with_config("spec.yaml".to_string(), &config);
        assert_eq!(p.max_iterations, 3);
        assert_eq!(p.quality_threshold, 95);
        assert_eq!(p.iteration, 0);
        assert!(p.last_error.is_none());
    }

    #[test]
    fn pipeline_new_starts_in_pending() {
        let p = fresh_pipeline();
        assert_eq!(p.state, PipelineState::Pending);
    }

    #[test]
    fn pipeline_timestamps_are_set_on_creation() {
        let before = chrono::Utc::now();
        let p = fresh_pipeline();
        let after = chrono::Utc::now();
        assert!(p.created_at >= before && p.created_at <= after);
        assert!(p.updated_at >= before && p.updated_at <= after);
        assert_eq!(p.created_at, p.updated_at);
    }

    // =========================================================================
    // ALL valid state transitions
    // =========================================================================

    #[test]
    fn pending_to_spec_review_is_valid() {
        let mut p = fresh_pipeline();
        assert!(p.transition_to(PipelineState::SpecReview).is_ok());
        assert_eq!(p.state, PipelineState::SpecReview);
    }

    #[test]
    fn spec_review_to_universe_setup_is_valid() {
        let mut p = fresh_pipeline();
        p.state = PipelineState::SpecReview;
        assert!(p.transition_to(PipelineState::UniverseSetup).is_ok());
        assert_eq!(p.state, PipelineState::UniverseSetup);
    }

    #[test]
    fn spec_review_to_failed_is_valid() {
        let mut p = fresh_pipeline();
        p.state = PipelineState::SpecReview;
        assert!(p.transition_to(PipelineState::Failed).is_ok());
        assert_eq!(p.state, PipelineState::Failed);
    }

    #[test]
    fn spec_review_to_escalated_is_valid() {
        let mut p = fresh_pipeline();
        p.state = PipelineState::SpecReview;
        assert!(p.transition_to(PipelineState::Escalated).is_ok());
        assert_eq!(p.state, PipelineState::Escalated);
    }

    #[test]
    fn universe_setup_to_agent_development_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert_eq!(p.state, PipelineState::AgentDevelopment);
    }

    #[test]
    fn universe_setup_to_escalated_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        assert!(p.transition_to(PipelineState::Escalated).is_ok());
        assert_eq!(p.state, PipelineState::Escalated);
    }

    #[test]
    fn universe_setup_to_failed_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        assert!(p.transition_to(PipelineState::Failed).is_ok());
        assert_eq!(p.state, PipelineState::Failed);
    }

    #[test]
    fn agent_development_to_validation_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(p.transition_to(PipelineState::Validation).is_ok());
        assert_eq!(p.state, PipelineState::Validation);
    }

    #[test]
    fn agent_development_to_agent_development_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert_eq!(p.state, PipelineState::AgentDevelopment);
    }

    #[test]
    fn agent_development_to_escalated_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(p.transition_to(PipelineState::Escalated).is_ok());
        assert_eq!(p.state, PipelineState::Escalated);
    }

    #[test]
    fn agent_development_to_failed_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(p.transition_to(PipelineState::Failed).is_ok());
        assert_eq!(p.state, PipelineState::Failed);
    }

    #[test]
    fn validation_to_accepted_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        assert!(p.transition_to(PipelineState::Accepted).is_ok());
        assert_eq!(p.state, PipelineState::Accepted);
    }

    #[test]
    fn validation_to_agent_development_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert_eq!(p.state, PipelineState::AgentDevelopment);
    }

    #[test]
    fn validation_to_escalated_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        assert!(p.transition_to(PipelineState::Escalated).is_ok());
        assert_eq!(p.state, PipelineState::Escalated);
    }

    #[test]
    fn validation_to_failed_is_valid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        assert!(p.transition_to(PipelineState::Failed).is_ok());
        assert_eq!(p.state, PipelineState::Failed);
    }

    // =========================================================================
    // Catchall transitions: any non-terminal -> Failed, any non-terminal -> Escalated
    // =========================================================================

    #[test]
    fn pending_to_failed_is_valid_via_catchall() {
        let mut p = fresh_pipeline();
        assert!(p.transition_to(PipelineState::Failed).is_ok());
        assert_eq!(p.state, PipelineState::Failed);
    }

    #[test]
    fn pending_to_escalated_is_valid_via_catchall() {
        let mut p = fresh_pipeline();
        assert!(p.transition_to(PipelineState::Escalated).is_ok());
        assert_eq!(p.state, PipelineState::Escalated);
    }

    // =========================================================================
    // ALL invalid state transitions (explicit invalidations)
    // =========================================================================

    #[test]
    fn pending_to_universe_setup_is_invalid() {
        let mut p = fresh_pipeline();
        let result = p.transition_to(PipelineState::UniverseSetup);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn pending_to_agent_development_is_invalid() {
        let mut p = fresh_pipeline();
        let result = p.transition_to(PipelineState::AgentDevelopment);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn pending_to_validation_is_invalid() {
        let mut p = fresh_pipeline();
        let result = p.transition_to(PipelineState::Validation);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn pending_to_accepted_is_invalid() {
        let mut p = fresh_pipeline();
        let result = p.transition_to(PipelineState::Accepted);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn spec_review_to_pending_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::SpecReview);
        let result = p.transition_to(PipelineState::Pending);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn spec_review_to_agent_development_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::SpecReview);
        let result = p.transition_to(PipelineState::AgentDevelopment);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn spec_review_to_validation_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::SpecReview);
        let result = p.transition_to(PipelineState::Validation);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn spec_review_to_accepted_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::SpecReview);
        let result = p.transition_to(PipelineState::Accepted);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn spec_review_to_spec_review_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::SpecReview);
        let result = p.transition_to(PipelineState::SpecReview);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn universe_setup_to_pending_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        let result = p.transition_to(PipelineState::Pending);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn universe_setup_to_spec_review_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        let result = p.transition_to(PipelineState::SpecReview);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn universe_setup_to_validation_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        let result = p.transition_to(PipelineState::Validation);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn universe_setup_to_accepted_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        let result = p.transition_to(PipelineState::Accepted);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn universe_setup_to_universe_setup_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::UniverseSetup);
        let result = p.transition_to(PipelineState::UniverseSetup);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn agent_development_to_pending_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        let result = p.transition_to(PipelineState::Pending);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn agent_development_to_spec_review_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        let result = p.transition_to(PipelineState::SpecReview);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn agent_development_to_universe_setup_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        let result = p.transition_to(PipelineState::UniverseSetup);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn agent_development_to_accepted_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::AgentDevelopment);
        let result = p.transition_to(PipelineState::Accepted);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn validation_to_pending_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        let result = p.transition_to(PipelineState::Pending);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn validation_to_spec_review_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        let result = p.transition_to(PipelineState::SpecReview);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn validation_to_universe_setup_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        let result = p.transition_to(PipelineState::UniverseSetup);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn validation_to_validation_is_invalid() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Validation);
        let result = p.transition_to(PipelineState::Validation);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::InvalidTransition { .. })
        ));
    }

    // =========================================================================
    // Terminal state transitions (AlreadyTerminal errors)
    // =========================================================================

    #[test]
    fn accepted_state_rejects_all_transitions() {
        let terminal_states = [
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        let all_states = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];

        for terminal in terminal_states {
            for target in all_states {
                if terminal == target {
                    continue; // same-state is still blocked but not the main concern
                }
                let mut p = fresh_pipeline();
                force_state(&mut p, terminal);
                let result = p.transition_to(target);
                assert!(
                    matches!(result, Err(PipelineTransitionError::AlreadyTerminal { .. })),
                    "Transition from {terminal:?} to {target:?} should be AlreadyTerminal"
                );
            }
        }
    }

    #[test]
    fn accepted_to_accepted_is_already_terminal() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Accepted);
        let result = p.transition_to(PipelineState::Accepted);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::AlreadyTerminal { .. })
        ));
    }

    #[test]
    fn failed_to_failed_is_already_terminal() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Failed);
        let result = p.transition_to(PipelineState::Failed);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::AlreadyTerminal { .. })
        ));
    }

    #[test]
    fn escalated_to_escalated_is_already_terminal() {
        let mut p = fresh_pipeline();
        force_state(&mut p, PipelineState::Escalated);
        let result = p.transition_to(PipelineState::Escalated);
        assert!(matches!(
            result,
            Err(PipelineTransitionError::AlreadyTerminal { .. })
        ));
    }

    // =========================================================================
    // Transition updates the timestamp
    // =========================================================================

    #[test]
    fn transition_updates_updated_at() {
        let mut p = fresh_pipeline();
        let original_updated = p.updated_at;
        // Tiny sleep to ensure the clock moves, though nanosecond precision
        // usually makes this unnecessary.
        p.transition_to(PipelineState::SpecReview).unwrap();
        assert!(p.updated_at >= original_updated);
    }

    // =========================================================================
    // Happy path: full pipeline lifecycle
    // =========================================================================

    #[test]
    fn full_happy_path() {
        let mut p = fresh_pipeline();
        assert_eq!(p.state, PipelineState::Pending);
        p.transition_to(PipelineState::SpecReview).unwrap();
        p.transition_to(PipelineState::UniverseSetup).unwrap();
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        p.transition_to(PipelineState::Validation).unwrap();
        p.transition_to(PipelineState::Accepted).unwrap();
        assert_eq!(p.state, PipelineState::Accepted);
    }

    #[test]
    fn full_failure_path_from_validation() {
        let mut p = fresh_pipeline();
        p.transition_to(PipelineState::SpecReview).unwrap();
        p.transition_to(PipelineState::UniverseSetup).unwrap();
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        p.transition_to(PipelineState::Validation).unwrap();
        p.transition_to(PipelineState::Failed).unwrap();
        assert_eq!(p.state, PipelineState::Failed);
    }

    #[test]
    fn retry_loop_from_validation_back_to_agent_development() {
        let mut p = fresh_pipeline();
        p.transition_to(PipelineState::SpecReview).unwrap();
        p.transition_to(PipelineState::UniverseSetup).unwrap();
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        p.transition_to(PipelineState::Validation).unwrap();
        // Loop back for another iteration
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        assert_eq!(p.state, PipelineState::AgentDevelopment);
        p.transition_to(PipelineState::Validation).unwrap();
        p.transition_to(PipelineState::Accepted).unwrap();
        assert_eq!(p.state, PipelineState::Accepted);
    }

    #[test]
    fn agent_development_self_loop_represents_reiteration() {
        let mut p = fresh_pipeline();
        p.transition_to(PipelineState::SpecReview).unwrap();
        p.transition_to(PipelineState::UniverseSetup).unwrap();
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        p.transition_to(PipelineState::AgentDevelopment).unwrap();
        assert_eq!(p.state, PipelineState::AgentDevelopment);
    }

    // =========================================================================
    // can_iterate() logic for each state
    // =========================================================================

    #[test]
    fn can_iterate_true_in_agent_development_below_limit() {
        let mut p = fresh_pipeline();
        p.max_iterations = 5;
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(p.can_iterate());
    }

    #[test]
    fn can_iterate_false_in_agent_development_at_limit() {
        let mut p = fresh_pipeline();
        p.max_iterations = 5;
        p.iteration = 5;
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(!p.can_iterate());
    }

    #[test]
    fn can_iterate_false_for_all_non_agent_development_states() {
        let non_ad_states = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for state in non_ad_states {
            let mut p = fresh_pipeline();
            p.max_iterations = 100;
            p.iteration = 0;
            force_state(&mut p, state);
            assert!(
                !p.can_iterate(),
                "can_iterate() should be false in state {state:?}"
            );
        }
    }

    #[test]
    fn can_iterate_false_when_iteration_exceeds_max() {
        let mut p = fresh_pipeline();
        p.max_iterations = 3;
        p.iteration = 10;
        force_state(&mut p, PipelineState::AgentDevelopment);
        assert!(!p.can_iterate());
    }

    // =========================================================================
    // increment_iteration() boundary behavior
    // =========================================================================

    #[test]
    fn increment_iteration_returns_new_value() {
        let mut p = fresh_pipeline();
        p.max_iterations = 5;
        force_state(&mut p, PipelineState::AgentDevelopment);

        assert_eq!(p.increment_iteration().unwrap(), 1);
        assert_eq!(p.increment_iteration().unwrap(), 2);
        assert_eq!(p.increment_iteration().unwrap(), 3);
    }

    #[test]
    fn increment_iteration_fails_at_exact_limit() {
        let mut p = fresh_pipeline();
        p.max_iterations = 3;
        // Pre-set iteration to the max
        p.iteration = 3;
        let result = p.increment_iteration();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.current, 3);
        assert_eq!(err.max, 3);
    }

    #[test]
    fn increment_iteration_fails_one_past_limit() {
        let mut p = fresh_pipeline();
        p.max_iterations = 3;
        p.iteration = 5;
        let result = p.increment_iteration();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.current, 5);
        assert_eq!(err.max, 3);
    }

    #[test]
    fn increment_iteration_updates_timestamp() {
        let mut p = fresh_pipeline();
        p.max_iterations = 10;
        force_state(&mut p, PipelineState::AgentDevelopment);
        let before = p.updated_at;
        p.increment_iteration().unwrap();
        assert!(p.updated_at >= before);
    }

    #[test]
    fn increment_iteration_allows_exactly_max_iterations() {
        let mut p = fresh_pipeline();
        p.max_iterations = 3;
        force_state(&mut p, PipelineState::AgentDevelopment);
        // Should succeed 3 times (iteration goes 0->1, 1->2, 2->3)
        assert_eq!(p.increment_iteration().unwrap(), 1);
        assert_eq!(p.increment_iteration().unwrap(), 2);
        assert_eq!(p.increment_iteration().unwrap(), 3);
        // Fourth call should fail
        assert!(p.increment_iteration().is_err());
    }

    // =========================================================================
    // set_error / clear_error
    // =========================================================================

    #[test]
    fn set_error_stores_message() {
        let mut p = fresh_pipeline();
        p.set_error("something broke".to_string());
        assert_eq!(p.last_error.as_deref(), Some("something broke"));
    }

    #[test]
    fn set_error_overwrites_previous() {
        let mut p = fresh_pipeline();
        p.set_error("first error".to_string());
        p.set_error("second error".to_string());
        assert_eq!(p.last_error.as_deref(), Some("second error"));
    }

    #[test]
    fn clear_error_removes_error() {
        let mut p = fresh_pipeline();
        p.set_error("oops".to_string());
        p.clear_error();
        assert!(p.last_error.is_none());
    }

    #[test]
    fn clear_error_on_clean_pipeline_is_noop() {
        let mut p = fresh_pipeline();
        p.clear_error(); // should not panic
        assert!(p.last_error.is_none());
    }

    #[test]
    fn set_error_updates_timestamp() {
        let mut p = fresh_pipeline();
        let before = p.updated_at;
        p.set_error("err".to_string());
        assert!(p.updated_at >= before);
    }

    #[test]
    fn clear_error_updates_timestamp() {
        let mut p = fresh_pipeline();
        p.set_error("err".to_string());
        let before = p.updated_at;
        p.clear_error();
        assert!(p.updated_at >= before);
    }

    // =========================================================================
    // Error display formatting
    // =========================================================================

    #[test]
    fn invalid_transition_error_displays_states() {
        let err = PipelineTransitionError::InvalidTransition {
            from: PipelineState::Pending,
            to: PipelineState::Accepted,
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid transition"));
        assert!(msg.contains("Pending"));
        assert!(msg.contains("Accepted"));
    }

    #[test]
    fn already_terminal_error_displays_state() {
        let err = PipelineTransitionError::AlreadyTerminal {
            current: PipelineState::Failed,
        };
        let msg = err.to_string();
        assert!(msg.contains("terminal"));
        assert!(msg.contains("Failed"));
    }

    #[test]
    fn iteration_limit_error_displays_values() {
        let err = IterationLimitError {
            current: 10,
            max: 10,
        };
        let msg = err.to_string();
        assert!(msg.contains("10"));
        assert!(msg.contains("Max iterations"));
    }

    // =========================================================================
    // Exhaustive invalid transition matrix
    // =========================================================================

    /// Validates that from a given state, attempting to transition to each
    /// state in `invalid_targets` yields an InvalidTransition error (not
    /// AlreadyTerminal, since the source is non-terminal).
    fn assert_invalid_transitions(from: PipelineState, invalid_targets: &[PipelineState]) {
        for target in invalid_targets {
            let mut p = fresh_pipeline();
            force_state(&mut p, from);
            let result = p.transition_to(*target);
            assert!(
                matches!(
                    result,
                    Err(PipelineTransitionError::InvalidTransition { .. })
                ),
                "Expected InvalidTransition from {from:?} to {target:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn exhaustive_invalid_from_pending() {
        // Pending valid: SpecReview, Failed, Escalated (catchall)
        // Pending invalid: UniverseSetup, AgentDevelopment, Validation, Accepted
        assert_invalid_transitions(
            PipelineState::Pending,
            &[
                PipelineState::Pending,
                PipelineState::UniverseSetup,
                PipelineState::AgentDevelopment,
                PipelineState::Validation,
                PipelineState::Accepted,
            ],
        );
    }

    #[test]
    fn exhaustive_invalid_from_spec_review() {
        // SpecReview valid: UniverseSetup, Failed, Escalated
        // SpecReview invalid: Pending, SpecReview, AgentDevelopment, Validation, Accepted
        assert_invalid_transitions(
            PipelineState::SpecReview,
            &[
                PipelineState::Pending,
                PipelineState::SpecReview,
                PipelineState::AgentDevelopment,
                PipelineState::Validation,
                PipelineState::Accepted,
            ],
        );
    }

    #[test]
    fn exhaustive_invalid_from_universe_setup() {
        // UniverseSetup valid: AgentDevelopment, Escalated, Failed
        // UniverseSetup invalid: Pending, SpecReview, UniverseSetup, Validation, Accepted
        assert_invalid_transitions(
            PipelineState::UniverseSetup,
            &[
                PipelineState::Pending,
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::Validation,
                PipelineState::Accepted,
            ],
        );
    }

    #[test]
    fn exhaustive_invalid_from_agent_development() {
        // AgentDevelopment valid: AgentDevelopment, Validation, Escalated, Failed
        // AgentDevelopment invalid: Pending, SpecReview, UniverseSetup, Accepted
        assert_invalid_transitions(
            PipelineState::AgentDevelopment,
            &[
                PipelineState::Pending,
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::Accepted,
            ],
        );
    }

    #[test]
    fn exhaustive_invalid_from_validation() {
        // Validation valid: AgentDevelopment, Accepted, Escalated, Failed
        // Validation invalid: Pending, SpecReview, UniverseSetup, Validation
        assert_invalid_transitions(
            PipelineState::Validation,
            &[
                PipelineState::Pending,
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::Validation,
            ],
        );
    }

    // =========================================================================
    // Error trait implementations
    // =========================================================================

    #[test]
    fn pipeline_transition_error_implements_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(PipelineTransitionError::InvalidTransition {
                from: PipelineState::Pending,
                to: PipelineState::Accepted,
            });
        let _msg = err.to_string(); // just ensure it compiles and doesn't panic
    }

    #[test]
    fn iteration_limit_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(IterationLimitError { current: 5, max: 5 });
        let _msg = err.to_string();
    }
}

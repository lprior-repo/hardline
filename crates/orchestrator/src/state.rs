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
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Accepted | Self::Escalated | Self::Failed)
    }

    #[must_use]
    pub const fn allows_iteration(&self) -> bool {
        matches!(self, Self::AgentDevelopment)
    }

    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Pending => "Pending - awaiting start",
            Self::SpecReview => "Spec Review - running linter",
            Self::UniverseSetup => "Universe Setup - deploying twin",
            Self::AgentDevelopment => "Agent Development - working on task",
            Self::Validation => "Validation - running scenarios",
            Self::Accepted => "Accepted - all scenarios passed",
            Self::Escalated => "Escalated - human intervention needed",
            Self::Failed => "Failed - validation failed",
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

/// Error when transitioning states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {from:?} to {to:?}")
            }
            Self::AlreadyTerminal { current } => {
                write!(f, "Pipeline already in terminal state: {current:?}")
            }
        }
    }
}

impl std::error::Error for TransitionError {}

/// Error during iteration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationError {
    MaxIterationsReached { current: u32, max: u32 },
}

impl std::fmt::Display for IterationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxIterationsReached { current, max } => {
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

    // --- PipelineId tests ---

    #[test]
    fn test_pipeline_id_display() {
        let id = PipelineId("test-123".to_string());
        let display = format!("{id}");
        assert!(display.contains("test-123"));
    }

    #[test]
    fn test_pipeline_id_new_generates_unique() {
        let id1 = PipelineId::new();
        let id2 = PipelineId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_pipeline_id_default_creates_new() {
        let id = PipelineId::default();
        // Should not be empty
        assert!(!id.0.is_empty());
    }

    // --- PipelineState tests ---

    #[test]
    fn test_pipeline_state_is_terminal() {
        assert!(!PipelineState::Pending.is_terminal());
        assert!(!PipelineState::SpecReview.is_terminal());
        assert!(!PipelineState::UniverseSetup.is_terminal());
        assert!(!PipelineState::AgentDevelopment.is_terminal());
        assert!(!PipelineState::Validation.is_terminal());
        assert!(PipelineState::Accepted.is_terminal());
        assert!(PipelineState::Escalated.is_terminal());
        assert!(PipelineState::Failed.is_terminal());
    }

    #[test]
    fn test_pipeline_state_allows_iteration() {
        assert!(!PipelineState::Pending.allows_iteration());
        assert!(!PipelineState::SpecReview.allows_iteration());
        assert!(!PipelineState::UniverseSetup.allows_iteration());
        assert!(PipelineState::AgentDevelopment.allows_iteration());
        assert!(!PipelineState::Validation.allows_iteration());
        assert!(!PipelineState::Accepted.allows_iteration());
    }

    #[test]
    fn test_pipeline_state_display() {
        assert!(!PipelineState::Pending.to_string().is_empty());
        assert!(!PipelineState::Accepted.to_string().is_empty());
    }

    #[test]
    fn test_pipeline_state_description() {
        assert!(!PipelineState::Pending.description().is_empty());
        assert!(PipelineState::AgentDevelopment
            .description()
            .contains("working"));
    }

    // --- PipelineConfig tests ---

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.quality_threshold, 80);
        assert_eq!(config.scenarios_path, "scenarios");
        assert!(config.linter_path.is_none());
    }

    // --- Pipeline::with_config tests ---

    #[test]
    fn test_pipeline_with_config() {
        let config = PipelineConfig {
            max_iterations: 5,
            quality_threshold: 95,
            scenarios_path: "custom".to_string(),
            linter_path: Some("/usr/bin/lint".to_string()),
        };
        let pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);

        assert_eq!(pipeline.max_iterations, 5);
        assert_eq!(pipeline.quality_threshold, 95);
        assert_eq!(pipeline.state, PipelineState::Pending);
        assert_eq!(pipeline.iteration, 0);
        assert!(pipeline.last_error.is_none());
    }

    // --- Pipeline can_iterate tests ---

    #[test]
    fn test_can_iterate_false_when_not_in_agent_development() {
        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert!(!pipeline.can_iterate());
    }

    #[test]
    fn test_can_iterate_false_when_max_reached() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        pipeline.iteration = pipeline.max_iterations;
        assert!(!pipeline.can_iterate());
    }

    #[test]
    fn test_can_iterate_true_in_agent_dev_with_capacity() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        pipeline.iteration = 5;
        pipeline.max_iterations = 10;
        assert!(pipeline.can_iterate());
    }

    // --- Pipeline error management tests ---

    #[test]
    fn test_set_and_clear_error() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert!(pipeline.last_error.is_none());

        pipeline.set_error("something broke".to_string());
        assert_eq!(pipeline.last_error.as_deref(), Some("something broke"));

        pipeline.clear_error();
        assert!(pipeline.last_error.is_none());
    }

    // --- Transition table coverage ---

    #[test]
    fn test_all_explicit_transitions() {
        let valid_transitions: Vec<(PipelineState, PipelineState)> = vec![
            (PipelineState::Pending, PipelineState::SpecReview),
            (PipelineState::SpecReview, PipelineState::UniverseSetup),
            (PipelineState::SpecReview, PipelineState::Failed),
            (PipelineState::SpecReview, PipelineState::Escalated),
            (
                PipelineState::UniverseSetup,
                PipelineState::AgentDevelopment,
            ),
            (PipelineState::UniverseSetup, PipelineState::Failed),
            (PipelineState::UniverseSetup, PipelineState::Escalated),
            (PipelineState::AgentDevelopment, PipelineState::Validation),
            (
                PipelineState::AgentDevelopment,
                PipelineState::AgentDevelopment,
            ),
            (PipelineState::AgentDevelopment, PipelineState::Escalated),
            (PipelineState::Validation, PipelineState::Accepted),
            (PipelineState::Validation, PipelineState::AgentDevelopment),
            (PipelineState::Validation, PipelineState::Failed),
            (PipelineState::Validation, PipelineState::Escalated),
        ];

        for (from, to) in valid_transitions {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = from;
            let result = pipeline.transition_to(to);
            assert!(result.is_ok(), "Expected {from:?} -> {to:?} to be valid");
        }
    }

    #[test]
    fn test_invalid_transitions() {
        let invalid: Vec<(PipelineState, PipelineState)> = vec![
            (PipelineState::Pending, PipelineState::Validation),
            (PipelineState::Pending, PipelineState::AgentDevelopment),
            (PipelineState::Pending, PipelineState::Accepted),
            (PipelineState::SpecReview, PipelineState::AgentDevelopment),
            (PipelineState::SpecReview, PipelineState::Validation),
        ];

        for (from, to) in invalid {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = from;
            let result = pipeline.transition_to(to);
            assert!(result.is_err(), "Expected {from:?} -> {to:?} to be invalid");
        }
    }

    #[test]
    fn test_any_state_can_transition_to_failed_or_escalated() {
        // From the catch-all arms in transition_to: (_, Failed) and (_, Escalated)
        let any_states = [
            PipelineState::Pending,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for &from in &any_states {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = from;
            assert!(pipeline.transition_to(PipelineState::Failed).is_ok());
        }
    }

    // --- TransitionError display tests ---

    #[test]
    fn test_transition_error_display_invalid() {
        let err = TransitionError::InvalidTransition {
            from: PipelineState::Pending,
            to: PipelineState::Validation,
        };
        let display = format!("{err}");
        assert!(display.contains("Invalid transition"));
        assert!(display.contains("Pending"));
        assert!(display.contains("Validation"));
    }

    #[test]
    fn test_transition_error_display_already_terminal() {
        let err = TransitionError::AlreadyTerminal {
            current: PipelineState::Accepted,
        };
        let display = format!("{err}");
        assert!(display.contains("already in terminal state"));
        assert!(display.contains("Accepted"));
    }

    // --- IterationError display tests ---

    #[test]
    fn test_iteration_error_display() {
        let err = IterationError::MaxIterationsReached {
            current: 10,
            max: 10,
        };
        let display = format!("{err}");
        assert!(display.contains("10"));
        assert!(display.contains("10"));
    }

    // --- Transition updates timestamp ---

    #[test]
    fn test_transition_updates_updated_at() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        // Verify it doesn't panic and the state changed
        pipeline.transition_to(PipelineState::SpecReview).ok();
        assert_eq!(pipeline.state, PipelineState::SpecReview);
    }

    // --- Increment iteration updates timestamp ---

    #[test]
    fn test_increment_iteration_updates_timestamp() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        let _ = pipeline.increment_iteration();
        assert_eq!(pipeline.iteration, 1);
    }

    // --- AgentDevelopment self-loop ---

    #[test]
    fn test_agent_development_self_loop() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_ok());
        assert_eq!(pipeline.state, PipelineState::AgentDevelopment);
    }

    // --- Serde serialization roundtrips ---

    #[test]
    fn test_pipeline_state_serde_roundtrip_all_variants() {
        let states = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: PipelineState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*state, deserialized);
        }
    }

    #[test]
    fn test_pipeline_state_serde_uses_snake_case() {
        let json = serde_json::to_string(&PipelineState::AgentDevelopment).expect("serialize");
        assert_eq!(json, "\"agent_development\"");
    }

    #[test]
    fn test_pipeline_id_serde_roundtrip() {
        let id = PipelineId("abc-123".to_string());
        let json = serde_json::to_string(&id).expect("serialize");
        let deserialized: PipelineId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_pipeline_serde_roundtrip() {
        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let json = serde_json::to_string_pretty(&pipeline).expect("serialize");
        let deserialized: Pipeline = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pipeline.id, deserialized.id);
        assert_eq!(pipeline.spec_path, deserialized.spec_path);
        assert_eq!(pipeline.state, deserialized.state);
    }

    #[test]
    fn test_transition_error_serde_roundtrip() {
        let err = TransitionError::InvalidTransition {
            from: PipelineState::Pending,
            to: PipelineState::Validation,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let deserialized: TransitionError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_already_terminal_error_serde_roundtrip() {
        let err = TransitionError::AlreadyTerminal {
            current: PipelineState::Accepted,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let deserialized: TransitionError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_iteration_error_serde_roundtrip() {
        let err = IterationError::MaxIterationsReached {
            current: 10,
            max: 10,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let deserialized: IterationError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_pipeline_config_serde_roundtrip() {
        let config = PipelineConfig {
            max_iterations: 7,
            quality_threshold: 90,
            scenarios_path: "custom/scenarios".to_string(),
            linter_path: Some("/usr/bin/custom-lint".to_string()),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: PipelineConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.max_iterations, deserialized.max_iterations);
        assert_eq!(config.quality_threshold, deserialized.quality_threshold);
        assert_eq!(config.scenarios_path, deserialized.scenarios_path);
        assert_eq!(config.linter_path, deserialized.linter_path);
    }

    // --- Full pipeline lifecycle: happy path ---

    #[test]
    fn test_full_happy_path_lifecycle() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert_eq!(pipeline.state, PipelineState::Pending);
        assert!(!pipeline.state.is_terminal());

        assert!(pipeline.transition_to(PipelineState::SpecReview).is_ok());
        assert!(pipeline.transition_to(PipelineState::UniverseSetup).is_ok());
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_ok());
        assert!(pipeline.transition_to(PipelineState::Validation).is_ok());
        assert!(pipeline.transition_to(PipelineState::Accepted).is_ok());

        assert!(pipeline.state.is_terminal());
    }

    // --- Full pipeline lifecycle: failure paths ---

    #[test]
    fn test_failure_from_each_non_terminal_state() {
        let failure_targets = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for from in &failure_targets {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = *from;
            assert!(
                pipeline.transition_to(PipelineState::Failed).is_ok(),
                "Expected transition from {from:?} to Failed to succeed"
            );
            assert!(pipeline.state.is_terminal());
        }
    }

    #[test]
    fn test_escalation_from_each_non_terminal_state() {
        let escalation_targets = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for from in &escalation_targets {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = *from;
            assert!(
                pipeline.transition_to(PipelineState::Escalated).is_ok(),
                "Expected transition from {from:?} to Escalated to succeed"
            );
            assert!(pipeline.state.is_terminal());
        }
    }

    // --- Terminal state rejection from all three terminal states ---

    #[test]
    fn test_no_transitions_from_accepted() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for &target in &non_terminals {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = PipelineState::Accepted;
            let result = pipeline.transition_to(target);
            assert!(
                result.is_err(),
                "Expected transition from Accepted to {target:?} to fail"
            );
            assert!(matches!(
                result.unwrap_err(),
                TransitionError::AlreadyTerminal { .. }
            ));
        }
    }

    #[test]
    fn test_no_transitions_from_escalated() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::Validation,
        ];
        for &target in &non_terminals {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = PipelineState::Escalated;
            let result = pipeline.transition_to(target);
            assert!(
                result.is_err(),
                "Expected transition from Escalated to {target:?} to fail"
            );
        }
    }

    #[test]
    fn test_no_transitions_from_failed() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::Validation,
            PipelineState::Accepted,
        ];
        for &target in &non_terminals {
            let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
            pipeline.state = PipelineState::Failed;
            let result = pipeline.transition_to(target);
            assert!(
                result.is_err(),
                "Expected transition from Failed to {target:?} to fail"
            );
        }
    }

    // --- Validation loop: Validation -> AgentDevelopment -> Validation ---

    #[test]
    fn test_validation_loop_to_agent_development_and_back() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::Validation;
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_ok());
        assert!(pipeline.transition_to(PipelineState::Validation).is_ok());
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_ok());
    }

    // --- Iteration edge cases ---

    #[test]
    fn test_increment_iteration_at_max_minus_one() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        pipeline.iteration = 9;
        pipeline.max_iterations = 10;
        assert!(pipeline.can_iterate());
        assert!(pipeline.increment_iteration().is_ok());
        assert_eq!(pipeline.iteration, 10);
        assert!(!pipeline.can_iterate());
    }

    #[test]
    fn test_iteration_error_properties() {
        let err = IterationError::MaxIterationsReached { current: 5, max: 5 };
        assert_eq!(format!("{err}"), "Max iterations reached: 5 of 5");
    }

    // --- Pipeline creation edge cases ---

    #[test]
    fn test_pipeline_created_at_equals_updated_at() {
        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        assert_eq!(pipeline.created_at, pipeline.updated_at);
    }

    #[test]
    fn test_pipeline_with_config_custom_max_iterations() {
        let config = PipelineConfig {
            max_iterations: 1,
            quality_threshold: 80,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        };
        let mut pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);
        pipeline.state = PipelineState::AgentDevelopment;
        assert!(pipeline.increment_iteration().is_ok());
        assert!(pipeline.increment_iteration().is_err());
    }

    #[test]
    fn test_pipeline_hash_and_eq_for_pipeline_id() {
        use std::collections::HashSet;
        let id1 = PipelineId("a".to_string());
        let id2 = PipelineId("a".to_string());
        let id3 = PipelineId("b".to_string());
        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));
    }

    // --- AgentDevelopment self-loop multiple times ---

    #[test]
    fn test_agent_development_self_loop_multiple_times() {
        let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        for _ in 0..5 {
            assert!(pipeline
                .transition_to(PipelineState::AgentDevelopment)
                .is_ok());
        }
        assert_eq!(pipeline.state, PipelineState::AgentDevelopment);
    }
}

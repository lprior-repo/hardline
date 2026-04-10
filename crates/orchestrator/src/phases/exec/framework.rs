//! Phase execution framework
//!
//! Provides a trait-based system for defining and executing pipeline phases.
//! Phases implement the [`Phase`] trait and are registered with a [`PhaseRegistry`]
//! that the [`PipelineExecutor`] uses for phase dispatch.
//!
//! # Architecture
//!
//! ```text
//! PhaseRegistry
//!   ├── SpecReview  (impl Phase)
//!   ├── UniverseSetup (impl Phase)
//!   ├── AgentDevelopment (impl Phase)
//!   └── Validation (impl Phase)
//!
//! PipelineExecutor
//!   └── phase_registry: PhaseRegistry
//!         └── execute_registered_phase(type, ctx) → PhaseResult
//! ```
//!
//! # Extensibility
//!
//! Custom phases can be registered:
//! ```ignore
//! let mut registry = PhaseRegistry::new();
//! registry.register(Box::new(MyCustomPhase));
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use tracing::{debug, info};

use crate::cleanup::PhaseType;
use crate::metrics::{Metrics, PhaseMetrics, ScenarioResult};
use crate::state::{Pipeline, PipelineState};

use super::types::{Decision, PhaseError, PhaseResult};

// ---------------------------------------------------------------------------
// Phase trait
// ---------------------------------------------------------------------------

/// Trait for executable pipeline phases.
///
/// Each phase in the pipeline implements this trait, providing a type
/// identifier and execution logic that transforms pipeline state.
///
/// Phases receive a [`PhaseContext`] with access to the pipeline and
/// resources, and return a [`PhaseResult`] indicating success or failure.
pub trait Phase {
    /// Returns the phase type identifier.
    fn phase_type(&self) -> PhaseType;

    /// Execute the phase logic.
    ///
    /// The implementation is responsible for:
    /// 1. Transitioning the pipeline to the correct state
    /// 2. Performing the phase's work
    /// 3. Recording metrics via the context
    /// 4. Returning a result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns [`PhaseError`] if the phase cannot execute due to invalid
    /// state, failed preconditions, or execution errors.
    fn execute(&self, ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError>;
}

// ---------------------------------------------------------------------------
// PhaseContext
// ---------------------------------------------------------------------------

/// Context provided to phases during execution.
///
/// Bundles all the resources and mutable state a phase needs,
/// including mutable access to the pipeline and metrics.
pub struct PhaseContext<'a> {
    /// The pipeline being executed (mutable for state transitions).
    pub pipeline: &'a mut Pipeline,
    /// Metrics collector for recording phase performance.
    pub metrics: &'a mut Metrics,
    /// Path to the scenarios directory.
    pub scenarios_path: &'a Path,
    /// Path to the linter binary, if configured.
    pub linter_path: &'a Option<PathBuf>,
}

impl<'a> PhaseContext<'a> {
    /// Record phase metrics with timing information.
    pub fn record_phase_metrics(&mut self, phase_name: &str, start: DateTime<Utc>, success: bool) {
        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: self.pipeline.id.0.clone(),
            phase: phase_name.to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success,
        });
    }

    /// Transition the pipeline to a new state.
    ///
    /// Wraps the pipeline's `transition_to` method, converting
    /// transition errors to [`PhaseError`].
    pub fn transition_to(&mut self, state: PipelineState) -> Result<(), PhaseError> {
        self.pipeline
            .transition_to(state)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// PhaseRegistry
// ---------------------------------------------------------------------------

/// Registry of phase implementations.
///
/// Allows phases to be registered and looked up by their [`PhaseType`].
/// The [`PipelineExecutor`](super::PipelineExecutor) uses this to
/// dispatch phase execution.
///
/// # Example
///
/// ```ignore
/// let registry = PhaseRegistry::with_defaults();
/// assert!(registry.contains(&PhaseType::SpecReview));
/// ```
pub struct PhaseRegistry {
    phases: HashMap<PhaseType, Box<dyn Phase>>,
}

impl PhaseRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
        }
    }

    /// Create a registry pre-loaded with the four default phase
    /// implementations.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(DefaultSpecReviewPhase));
        registry.register(Box::new(DefaultUniverseSetupPhase));
        registry.register(Box::new(DefaultAgentDevelopmentPhase));
        registry.register(Box::new(DefaultValidationPhase));
        registry
    }

    /// Register a phase implementation.
    ///
    /// If a phase with the same type is already registered it is replaced.
    pub fn register(&mut self, phase: Box<dyn Phase>) {
        self.phases.insert(phase.phase_type(), phase);
    }

    /// Look up a phase by type.
    #[must_use]
    pub fn get(&self, phase_type: &PhaseType) -> Option<&dyn Phase> {
        self.phases.get(phase_type).map(|p| p.as_ref())
    }

    /// Check whether a phase type is registered.
    #[must_use]
    pub fn contains(&self, phase_type: &PhaseType) -> bool {
        self.phases.contains_key(phase_type)
    }

    /// Return all registered phase types.
    #[must_use]
    pub fn registered_phases(&self) -> Vec<PhaseType> {
        self.phases.keys().copied().collect()
    }
}

impl Default for PhaseRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ---------------------------------------------------------------------------
// Default phase implementations
// ---------------------------------------------------------------------------

/// Default spec-review phase.
///
/// Runs a linter on the spec and checks against the quality threshold.
/// Currently returns a hardcoded quality score of 85.
pub struct DefaultSpecReviewPhase;

impl Phase for DefaultSpecReviewPhase {
    fn phase_type(&self) -> PhaseType {
        PhaseType::SpecReview
    }

    fn execute(&self, ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
        let start = Utc::now();
        info!("Running spec review for: {}", ctx.pipeline.spec_path);

        ctx.transition_to(PipelineState::SpecReview)?;

        let quality_score = run_linter(ctx.linter_path, &ctx.pipeline.spec_path);
        let threshold = ctx.pipeline.quality_threshold;
        let success = quality_score >= threshold;

        ctx.record_phase_metrics("spec_review", start, success);

        if success {
            ctx.transition_to(PipelineState::UniverseSetup)?;
            Ok(PhaseResult {
                success: true,
                message: format!("Spec passed with score {quality_score}"),
                quality_score: Some(quality_score),
                scenario_results: vec![],
            })
        } else {
            ctx.transition_to(PipelineState::Failed)?;
            Ok(PhaseResult {
                success: false,
                message: format!("Spec quality {quality_score} below threshold {threshold}"),
                quality_score: Some(quality_score),
                scenario_results: vec![],
            })
        }
    }
}

/// Default universe-setup phase.
///
/// Prepares the execution environment for agent development.
/// Currently a placeholder that records metrics and transitions state.
pub struct DefaultUniverseSetupPhase;

impl Phase for DefaultUniverseSetupPhase {
    fn phase_type(&self) -> PhaseType {
        PhaseType::UniverseSetup
    }

    fn execute(&self, ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
        let start = Utc::now();
        info!("Setting up universe for pipeline: {}", ctx.pipeline.id.0);

        // UniverseSetup state is already set by the previous phase (SpecReview
        // transitions to UniverseSetup on success). Skip the self-transition.
        ctx.record_phase_metrics("universe_setup", start, true);
        ctx.transition_to(PipelineState::AgentDevelopment)?;

        Ok(PhaseResult {
            success: true,
            message: "Universe setup complete".to_string(),
            quality_score: None,
            scenario_results: vec![],
        })
    }
}

/// Default agent-development phase.
///
/// Represents one agent work iteration. Increments the iteration counter
/// and transitions to validation.
pub struct DefaultAgentDevelopmentPhase;

impl Phase for DefaultAgentDevelopmentPhase {
    fn phase_type(&self) -> PhaseType {
        PhaseType::AgentDevelopment
    }

    fn execute(&self, ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
        let start = Utc::now();
        info!(
            "Agent development iteration {} for pipeline: {}",
            ctx.pipeline.iteration + 1,
            ctx.pipeline.id.0
        );

        ctx.transition_to(PipelineState::AgentDevelopment)?;
        ctx.record_phase_metrics("agent_development", start, true);

        ctx.pipeline
            .increment_iteration()
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        ctx.transition_to(PipelineState::Validation)?;

        Ok(PhaseResult {
            success: true,
            message: format!(
                "Agent development iteration {} complete",
                ctx.pipeline.iteration
            ),
            quality_score: None,
            scenario_results: vec![],
        })
    }
}

/// Default validation phase.
///
/// Runs scenarios and produces a [`Decision`] based on pass rate.
/// Currently returns hardcoded scenario results.
pub struct DefaultValidationPhase;

impl Phase for DefaultValidationPhase {
    fn phase_type(&self) -> PhaseType {
        PhaseType::Validation
    }

    fn execute(&self, ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
        let start = Utc::now();
        info!("Running validation for pipeline: {}", ctx.pipeline.id.0);

        // Validation state is already set by the previous phase (AgentDevelopment
        // transitions to Validation on success). Skip the self-transition.
        let scenario_results = run_scenarios(ctx.scenarios_path);
        ctx.record_phase_metrics("validation", start, !scenario_results.is_empty());

        let decision = make_decision(&scenario_results, ctx.pipeline);

        Ok(PhaseResult {
            success: decision != Decision::Fail,
            message: format!("Validation complete, decision: {decision:?}"),
            quality_score: None,
            scenario_results,
        })
    }
}

// ---------------------------------------------------------------------------
// Free helper functions (shared between framework and legacy code)
// ---------------------------------------------------------------------------

/// Run the spec linter.  Returns a quality score (0–100).
///
/// Currently a stub that returns 85.
pub(crate) fn run_linter(_linter_path: &Option<PathBuf>, _spec_path: &str) -> u32 {
    debug!("Running linter on spec");
    85
}

/// Run scenario tests and collect results.
///
/// Currently returns hardcoded passing results.
pub(crate) fn run_scenarios(_scenarios_path: &Path) -> Vec<ScenarioResult> {
    debug!("Running scenarios");
    vec![
        ScenarioResult {
            name: "happy_path".to_string(),
            passed: true,
            duration_secs: 1.5,
            error: None,
        },
        ScenarioResult {
            name: "edge_case".to_string(),
            passed: true,
            duration_secs: 0.8,
            error: None,
        },
    ]
}

/// Derive a [`Decision`] from scenario results and pipeline state.
pub(crate) fn make_decision(results: &[ScenarioResult], pipeline: &Pipeline) -> Decision {
    let passed_count = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    if total == 0 {
        tracing::warn!("No scenarios ran, defaulting to retry");
        return Decision::Retry;
    }

    let pass_rate = (passed_count * 100) / total;

    if pass_rate >= 100 {
        debug!("All {total} scenarios passed");
        Decision::Accept
    } else if pass_rate >= 50 {
        debug!("{pass_rate}% scenarios passed, allowing retry");
        if pipeline.can_iterate() {
            Decision::Retry
        } else {
            Decision::Escalate
        }
    } else {
        debug!("Only {pass_rate}% scenarios passed, failing");
        Decision::Fail
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Pipeline;

    // helpers ---------------------------------------------------------------

    /// Create a minimal pipeline for testing.
    fn test_pipeline() -> Pipeline {
        Pipeline::new("test_spec.md".to_string())
    }

    /// Create a fresh metrics collector.
    fn test_metrics() -> Metrics {
        Metrics::new()
    }

    fn test_scenarios_path() -> &'static Path {
        Path::new("scenarios")
    }

    fn test_linter_path() -> Option<PathBuf> {
        None
    }

    // --- PhaseContext -------------------------------------------------------

    #[test]
    fn phase_context_transition_to_valid() {
        let mut pipeline = test_pipeline();
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        assert!(ctx.transition_to(PipelineState::SpecReview).is_ok());
        assert_eq!(ctx.pipeline.state, PipelineState::SpecReview);
    }

    #[test]
    fn phase_context_transition_to_invalid_returns_error() {
        let mut pipeline = test_pipeline();
        // Pipeline is Pending — jumping straight to Validation is invalid
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = ctx.transition_to(PipelineState::Validation);
        assert!(result.is_err());
    }

    #[test]
    fn phase_context_record_phase_metrics_records() {
        let mut pipeline = test_pipeline();
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let start = Utc::now();
        ctx.record_phase_metrics("test_phase", start, true);

        let recorded: Vec<_> = metrics.get_for_pipeline(&pipeline.id.0);
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].phase, "test_phase");
        assert!(recorded[0].success);
    }

    // --- PhaseRegistry ------------------------------------------------------

    #[test]
    fn registry_new_is_empty() {
        let reg = PhaseRegistry::new();
        assert!(reg.registered_phases().is_empty());
    }

    #[test]
    fn registry_with_defaults_has_four_phases() {
        let reg = PhaseRegistry::with_defaults();
        let phases = reg.registered_phases();
        assert_eq!(phases.len(), 4);
    }

    #[test]
    fn registry_with_defaults_contains_all_types() {
        let reg = PhaseRegistry::with_defaults();
        assert!(reg.contains(&PhaseType::SpecReview));
        assert!(reg.contains(&PhaseType::UniverseSetup));
        assert!(reg.contains(&PhaseType::AgentDevelopment));
        assert!(reg.contains(&PhaseType::Validation));
    }

    #[test]
    fn registry_default_is_with_defaults() {
        let reg = PhaseRegistry::default();
        assert_eq!(reg.registered_phases().len(), 4);
    }

    #[test]
    fn registry_register_custom_phase() {
        struct CustomPhase;
        impl Phase for CustomPhase {
            fn phase_type(&self) -> PhaseType {
                PhaseType::SpecReview
            }
            fn execute(&self, _ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
                Ok(PhaseResult {
                    success: true,
                    message: "custom".to_string(),
                    quality_score: Some(99),
                    scenario_results: vec![],
                })
            }
        }

        let mut reg = PhaseRegistry::new();
        reg.register(Box::new(CustomPhase));
        assert!(reg.contains(&PhaseType::SpecReview));

        let phase = reg.get(&PhaseType::SpecReview).unwrap();
        let mut pipeline = test_pipeline();
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };
        let result = phase.execute(&mut ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.message, "custom");
        assert_eq!(result.quality_score, Some(99));
    }

    #[test]
    fn registry_register_replaces_existing() {
        struct PhaseA;
        impl Phase for PhaseA {
            fn phase_type(&self) -> PhaseType { PhaseType::SpecReview }
            fn execute(&self, _ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
                Ok(PhaseResult { success: true, message: "a".to_string(), quality_score: None, scenario_results: vec![] })
            }
        }
        struct PhaseB;
        impl Phase for PhaseB {
            fn phase_type(&self) -> PhaseType { PhaseType::SpecReview }
            fn execute(&self, _ctx: &mut PhaseContext<'_>) -> Result<PhaseResult, PhaseError> {
                Ok(PhaseResult { success: true, message: "b".to_string(), quality_score: None, scenario_results: vec![] })
            }
        }

        let mut reg = PhaseRegistry::new();
        reg.register(Box::new(PhaseA));
        reg.register(Box::new(PhaseB));

        // Only one phase per type — the latest wins
        assert_eq!(reg.registered_phases().len(), 1);

        let mut pipeline = test_pipeline();
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };
        let result = reg.get(&PhaseType::SpecReview).unwrap().execute(&mut ctx).unwrap();
        assert_eq!(result.message, "b");
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let reg = PhaseRegistry::new();
        assert!(reg.get(&PhaseType::SpecReview).is_none());
    }

    // --- DefaultSpecReviewPhase ---------------------------------------------

    #[test]
    fn default_spec_review_succeeds_above_threshold() {
        let phase = DefaultSpecReviewPhase;
        let mut pipeline = test_pipeline(); // threshold = 80
        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = phase.execute(&mut ctx).unwrap();
        assert!(result.success);
        assert_eq!(ctx.pipeline.state, PipelineState::UniverseSetup);
        assert_eq!(result.quality_score, Some(85));
    }

    #[test]
    fn default_spec_review_fails_below_threshold() {
        let phase = DefaultSpecReviewPhase;
        let mut pipeline = Pipeline::new("test.md".to_string());
        pipeline.quality_threshold = 100; // impossible to pass with stub score of 85

        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = phase.execute(&mut ctx).unwrap();
        assert!(!result.success);
        assert_eq!(ctx.pipeline.state, PipelineState::Failed);
    }

    // --- DefaultUniverseSetupPhase ------------------------------------------

    #[test]
    fn default_universe_setup_succeeds() {
        let phase = DefaultUniverseSetupPhase;
        let mut pipeline = test_pipeline();
        pipeline.transition_to(PipelineState::SpecReview).unwrap();
        pipeline.transition_to(PipelineState::UniverseSetup).unwrap();

        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = phase.execute(&mut ctx).unwrap();
        assert!(result.success);
        assert_eq!(ctx.pipeline.state, PipelineState::AgentDevelopment);
    }

    // --- DefaultAgentDevelopmentPhase ---------------------------------------

    #[test]
    fn default_agent_dev_succeeds() {
        let phase = DefaultAgentDevelopmentPhase;
        let mut pipeline = test_pipeline();
        pipeline.transition_to(PipelineState::SpecReview).unwrap();
        pipeline.transition_to(PipelineState::UniverseSetup).unwrap();
        pipeline.transition_to(PipelineState::AgentDevelopment).unwrap();

        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = phase.execute(&mut ctx).unwrap();
        assert!(result.success);
        assert_eq!(ctx.pipeline.state, PipelineState::Validation);
        assert_eq!(ctx.pipeline.iteration, 1);
    }

    // --- DefaultValidationPhase ---------------------------------------------

    #[test]
    fn default_validation_accepts_when_all_pass() {
        let phase = DefaultValidationPhase;
        let mut pipeline = test_pipeline();
        pipeline.transition_to(PipelineState::SpecReview).unwrap();
        pipeline.transition_to(PipelineState::UniverseSetup).unwrap();
        pipeline.transition_to(PipelineState::AgentDevelopment).unwrap();
        pipeline.transition_to(PipelineState::Validation).unwrap();

        let mut metrics = test_metrics();
        let mut ctx = PhaseContext {
            pipeline: &mut pipeline,
            metrics: &mut metrics,
            scenarios_path: test_scenarios_path(),
            linter_path: &test_linter_path(),
        };

        let result = phase.execute(&mut ctx).unwrap();
        // Stub scenarios all pass → decision is Accept → success=true
        assert!(result.success);
        assert_eq!(result.scenario_results.len(), 2);
    }

    // --- make_decision ------------------------------------------------------

    #[test]
    fn decision_accept_when_all_pass() {
        let pipeline = test_pipeline();
        let results = vec![
            ScenarioResult { name: "a".into(), passed: true, duration_secs: 1.0, error: None },
            ScenarioResult { name: "b".into(), passed: true, duration_secs: 1.0, error: None },
        ];
        assert_eq!(make_decision(&results, &pipeline), Decision::Accept);
    }

    #[test]
    fn decision_retry_when_half_pass() {
        let mut pipeline = test_pipeline();
        pipeline.state = PipelineState::AgentDevelopment;
        let results = vec![
            ScenarioResult { name: "a".into(), passed: true, duration_secs: 1.0, error: None },
            ScenarioResult { name: "b".into(), passed: false, duration_secs: 1.0, error: Some("err".into()) },
        ];
        assert_eq!(make_decision(&results, &pipeline), Decision::Retry);
    }

    #[test]
    fn decision_fail_when_most_fail() {
        let pipeline = test_pipeline();
        let results = vec![
            ScenarioResult { name: "a".into(), passed: false, duration_secs: 1.0, error: Some("e".into()) },
            ScenarioResult { name: "b".into(), passed: false, duration_secs: 1.0, error: Some("e".into()) },
            ScenarioResult { name: "c".into(), passed: true, duration_secs: 1.0, error: None },
        ];
        assert_eq!(make_decision(&results, &pipeline), Decision::Fail);
    }

    #[test]
    fn decision_retry_when_no_scenarios() {
        let pipeline = test_pipeline();
        assert_eq!(make_decision(&[], &pipeline), Decision::Retry);
    }

    #[test]
    fn decision_escalate_when_max_iterations_reached() {
        let mut pipeline = test_pipeline();
        pipeline.iteration = pipeline.max_iterations; // at max
        let results = vec![
            ScenarioResult { name: "a".into(), passed: true, duration_secs: 1.0, error: None },
            ScenarioResult { name: "b".into(), passed: false, duration_secs: 1.0, error: Some("e".into()) },
        ];
        assert_eq!(make_decision(&results, &pipeline), Decision::Escalate);
    }

    // --- Integration: full pipeline via registry ----------------------------

    #[test]
    fn full_pipeline_via_registry() {
        let reg = PhaseRegistry::with_defaults();

        let mut pipeline = test_pipeline();
        let mut metrics = test_metrics();

        // Spec review
        {
            let mut ctx = PhaseContext {
                pipeline: &mut pipeline,
                metrics: &mut metrics,
                scenarios_path: test_scenarios_path(),
                linter_path: &test_linter_path(),
            };
            let phase = reg.get(&PhaseType::SpecReview).unwrap();
            let result = phase.execute(&mut ctx).unwrap();
            assert!(result.success);
        }

        // Universe setup
        {
            let mut ctx = PhaseContext {
                pipeline: &mut pipeline,
                metrics: &mut metrics,
                scenarios_path: test_scenarios_path(),
                linter_path: &test_linter_path(),
            };
            let phase = reg.get(&PhaseType::UniverseSetup).unwrap();
            let result = phase.execute(&mut ctx).unwrap();
            assert!(result.success);
        }

        // Agent development
        {
            let mut ctx = PhaseContext {
                pipeline: &mut pipeline,
                metrics: &mut metrics,
                scenarios_path: test_scenarios_path(),
                linter_path: &test_linter_path(),
            };
            let phase = reg.get(&PhaseType::AgentDevelopment).unwrap();
            let result = phase.execute(&mut ctx).unwrap();
            assert!(result.success);
        }

        // Validation
        {
            let mut ctx = PhaseContext {
                pipeline: &mut pipeline,
                metrics: &mut metrics,
                scenarios_path: test_scenarios_path(),
                linter_path: &test_linter_path(),
            };
            let phase = reg.get(&PhaseType::Validation).unwrap();
            let result = phase.execute(&mut ctx).unwrap();
            assert!(result.success);
        }

        // Verify metrics were collected for all phases
        assert_eq!(metrics.get_phase_metrics().count(), 4);
    }
}

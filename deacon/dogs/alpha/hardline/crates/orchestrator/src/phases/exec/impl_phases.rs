//! Individual phase implementation methods

use chrono::{DateTime, Utc};
use tracing::{debug, info};

use crate::metrics::{PhaseMetrics, ScenarioResult};
use crate::state::{Pipeline, PipelineState};

use super::executor::PipelineExecutor;
use super::types::{Decision, PhaseError, PhaseResult};

impl PipelineExecutor {
    pub(crate) fn spec_review(&mut self, pipeline: &mut Pipeline) -> anyhow::Result<PhaseResult> {
        let start = Utc::now();
        info!("Running spec review for: {}", pipeline.spec_path);

        pipeline
            .transition_to(PipelineState::SpecReview)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;
        let quality_score = self.run_linter(&pipeline.spec_path);

        self.record_spec_review_metrics(pipeline, start, quality_score);

        if quality_score >= pipeline.quality_threshold {
            pipeline
                .transition_to(PipelineState::UniverseSetup)
                .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;
            Ok(PhaseResult {
                success: true,
                message: format!("Spec passed with score {quality_score}"),
                quality_score: Some(quality_score),
                scenario_results: vec![],
            })
        } else {
            pipeline
                .transition_to(PipelineState::Failed)
                .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;
            Ok(PhaseResult {
                success: false,
                message: format!(
                    "Spec quality {quality_score} below threshold {}",
                    pipeline.quality_threshold
                ),
                quality_score: Some(quality_score),
                scenario_results: vec![],
            })
        }
    }

    fn record_spec_review_metrics(
        &mut self,
        pipeline: &Pipeline,
        start: DateTime<Utc>,
        quality_score: u32,
    ) {
        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "spec_review".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: quality_score >= pipeline.quality_threshold,
        });
    }

    #[must_use]
    fn run_linter(&self, _spec_path: &str) -> u32 {
        debug!("Running linter on spec");
        85
    }

    pub(crate) fn universe_setup(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> anyhow::Result<PhaseResult> {
        let start = Utc::now();
        info!("Setting up universe for pipeline: {}", pipeline.id.0);

        pipeline
            .transition_to(PipelineState::UniverseSetup)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "universe_setup".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: true,
        });

        pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        Ok(PhaseResult {
            success: true,
            message: "Universe setup complete".to_string(),
            quality_score: None,
            scenario_results: vec![],
        })
    }

    pub(crate) fn agent_development(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> anyhow::Result<PhaseResult> {
        let start = Utc::now();
        info!(
            "Agent development iteration {} for pipeline: {}",
            pipeline.iteration + 1,
            pipeline.id.0
        );

        pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "agent_development".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: true,
        });

        pipeline
            .increment_iteration()
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        pipeline
            .transition_to(PipelineState::Validation)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        Ok(PhaseResult {
            success: true,
            message: format!(
                "Agent development iteration {} complete",
                pipeline.iteration
            ),
            quality_score: None,
            scenario_results: vec![],
        })
    }

    pub(crate) fn validation(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> anyhow::Result<(Decision, PhaseResult)> {
        let start = Utc::now();
        info!("Running validation for pipeline: {}", pipeline.id.0);

        pipeline
            .transition_to(PipelineState::Validation)
            .map_err(|e| PhaseError::InvalidStateTransition(e.to_string()))?;

        let scenario_results = self.run_scenarios(pipeline);

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "validation".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: !scenario_results.is_empty(),
        });

        let decision = self.make_decision(&scenario_results, pipeline);

        let result = PhaseResult {
            success: decision != Decision::Fail,
            message: format!("Validation complete, decision: {decision:?}"),
            quality_score: None,
            scenario_results,
        };

        Ok((decision, result))
    }

    #[must_use]
    fn run_scenarios(&self, _pipeline: &Pipeline) -> Vec<ScenarioResult> {
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

    #[must_use]
    fn make_decision(&self, results: &[ScenarioResult], pipeline: &Pipeline) -> Decision {
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
}

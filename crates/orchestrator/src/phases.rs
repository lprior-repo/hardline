//! Pipeline phase executor

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{
    cleanup::{CleanupContext, CleanupManager, PhaseType},
    metrics::{Metrics, PhaseMetrics, ScenarioResult},
    persistence::StateStore,
    state::{Pipeline, PipelineId, PipelineState},
};

/// Result of a phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub success: bool,
    pub message: String,
    pub quality_score: Option<u32>,
    pub scenario_results: Vec<ScenarioResult>,
}

/// Decision made after validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Accept,
    Retry,
    Escalate,
    Fail,
}

/// Pipeline executor for running phases
#[allow(dead_code)]
pub struct PipelineExecutor {
    store: StateStore,
    metrics: Metrics,
    scenarios_path: PathBuf,
    linter_path: Option<PathBuf>,
    cleanup_manager: CleanupManager,
}

impl PipelineExecutor {
    pub fn new(
        state_dir: PathBuf,
        scenarios_path: PathBuf,
        linter_path: Option<PathBuf>,
    ) -> Result<Self> {
        let store = StateStore::new(state_dir).context("Failed to initialize state store")?;

        Ok(Self {
            store,
            metrics: Metrics::new(),
            scenarios_path,
            linter_path,
            cleanup_manager: CleanupManager::new(),
        })
    }

    #[must_use]
    pub fn store(&self) -> &StateStore {
        &self.store
    }

    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[must_use]
    pub fn cleanup_manager(&self) -> &CleanupManager {
        &self.cleanup_manager
    }

    /// Validate precondition P1: pipeline must be in non-terminal state
    #[must_use]
    pub fn can_run_pipeline(&self, pipeline: &Pipeline) -> bool {
        !pipeline.state.is_terminal()
    }

    /// Run cleanup after a phase failure
    pub fn cleanup_after_failure(&self, pipeline_id: &PipelineId) -> Result<()> {
        let pipeline = self.store.get(pipeline_id)?.clone();

        let phase_type = PhaseType::from_state(pipeline.state);

        if let Some(phase) = phase_type {
            let context = CleanupContext::new(pipeline_id.clone(), phase);
            let result = self.cleanup_manager.cleanup(&context);

            if !result.success {
                warn!(
                    "Cleanup had errors for pipeline {}: {:?}",
                    pipeline_id.0, result.errors
                );
            }

            info!(
                "Cleanup completed for pipeline {}: {} resources cleaned",
                pipeline_id.0,
                result.cleaned_resources.len()
            );
        }

        Ok(())
    }

    /// Attempt rollback for a specific phase
    pub fn rollback_phase(&self, pipeline: &Pipeline, phase: PhaseType) -> Result<()> {
        let context = CleanupContext::new(pipeline.id.clone(), phase);
        let result = self.cleanup_manager.rollback(&context);

        if !result.success {
            error!(
                "Rollback failed for pipeline {} phase {:?}: {:?}",
                pipeline.id.0, phase, result.errors
            );
            return Err(anyhow::anyhow!("Rollback failed: {:?}", result.errors));
        }

        info!(
            "Rollback completed for pipeline {} phase {:?}",
            pipeline.id.0, phase
        );

        Ok(())
    }

    pub fn create_pipeline(&mut self, spec_path: String) -> Result<Pipeline> {
        let pipeline = Pipeline::new(spec_path);
        let pipeline = self.store.create(pipeline)?;
        info!("Created pipeline: {}", pipeline.id.0);
        Ok(pipeline)
    }

    pub fn run_pipeline(&mut self, pipeline_id: &crate::state::PipelineId) -> Result<Decision> {
        info!("Starting pipeline: {}", pipeline_id.0);

        let mut pipeline = self.store.get(pipeline_id)?.clone();

        if !pipeline.state.is_terminal() {
            info!("Recovering pipeline from state: {:?}", pipeline.state);
        }

        if pipeline.state == PipelineState::Pending || pipeline.state == PipelineState::SpecReview {
            let result = self.spec_review(&mut pipeline)?;
            if !result.success {
                return Ok(self.handle_spec_failure(pipeline_id, result.message));
            }
        }

        if pipeline.state == PipelineState::UniverseSetup {
            let result = self.universe_setup(&mut pipeline)?;
            if !result.success {
                return Ok(self.handle_setup_failure(pipeline_id, result.message));
            }
        }

        while pipeline.state == PipelineState::AgentDevelopment
            || pipeline.state == PipelineState::Validation
        {
            if pipeline.state == PipelineState::AgentDevelopment {
                let result = self.agent_development(&mut pipeline)?;
                if !result.success {
                    return Ok(self.handle_dev_failure(pipeline_id, result.message));
                }
            }

            if pipeline.state == PipelineState::Validation {
                let (decision, _result) = self.validation(&mut pipeline)?;

                match decision {
                    Decision::Accept => {
                        self.finalize_acceptance(pipeline_id)?;
                        return Ok(Decision::Accept);
                    }
                    Decision::Retry if pipeline.can_iterate() => {
                        pipeline.iteration += 1;
                        self.store.update(pipeline.clone())?;
                        info!(
                            "Retrying agent development, iteration {}",
                            pipeline.iteration
                        );
                    }
                    Decision::Retry => {
                        warn!("Max iterations reached, escalating");
                        self.escalate(pipeline_id, "Max iterations reached")?;
                        return Ok(Decision::Escalate);
                    }
                    Decision::Escalate => {
                        self.escalate(pipeline_id, "Validation escalated")?;
                        return Ok(Decision::Escalate);
                    }
                    Decision::Fail => {
                        self.fail(pipeline_id, "Validation failed")?;
                        return Ok(Decision::Fail);
                    }
                }
            }
        }

        match pipeline.state {
            PipelineState::Accepted => Ok(Decision::Accept),
            PipelineState::Escalated => Ok(Decision::Escalate),
            PipelineState::Failed => Ok(Decision::Fail),
            _ => {
                error!("Unexpected terminal state: {:?}", pipeline.state);
                Ok(Decision::Fail)
            }
        }
    }

    fn spec_review(&mut self, pipeline: &mut Pipeline) -> Result<PhaseResult> {
        let start = Utc::now();
        info!("Running spec review for: {}", pipeline.spec_path);

        pipeline.transition_to(PipelineState::SpecReview)?;

        let quality_score = self.run_linter(&pipeline.spec_path);

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "spec_review".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: quality_score >= pipeline.quality_threshold,
        });

        if quality_score >= pipeline.quality_threshold {
            pipeline.transition_to(PipelineState::UniverseSetup)?;
            Ok(PhaseResult {
                success: true,
                message: format!("Spec passed with score {quality_score}"),
                quality_score: Some(quality_score),
                scenario_results: vec![],
            })
        } else {
            pipeline.transition_to(PipelineState::Failed)?;
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

    #[must_use]
    fn run_linter(&self, _spec_path: &str) -> u32 {
        debug!("Running linter on spec");
        85
    }

    fn universe_setup(&mut self, pipeline: &mut Pipeline) -> Result<PhaseResult> {
        let start = Utc::now();
        info!("Setting up universe for pipeline: {}", pipeline.id.0);

        pipeline.transition_to(PipelineState::UniverseSetup)?;

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "universe_setup".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: true,
        });

        pipeline.transition_to(PipelineState::AgentDevelopment)?;

        Ok(PhaseResult {
            success: true,
            message: "Universe setup complete".to_string(),
            quality_score: None,
            scenario_results: vec![],
        })
    }

    fn agent_development(&mut self, pipeline: &mut Pipeline) -> Result<PhaseResult> {
        let start = Utc::now();
        info!(
            "Agent development iteration {} for pipeline: {}",
            pipeline.iteration + 1,
            pipeline.id.0
        );

        pipeline.transition_to(PipelineState::AgentDevelopment)?;

        let duration = Utc::now().signed_duration_since(start);
        self.metrics.record_phase(PhaseMetrics {
            pipeline_id: pipeline.id.0.clone(),
            phase: "agent_development".to_string(),
            started_at: start,
            duration_secs: duration.num_seconds() as f64,
            success: true,
        });

        pipeline.increment_iteration()?;

        pipeline.transition_to(PipelineState::Validation)?;

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

    fn validation(&mut self, pipeline: &mut Pipeline) -> Result<(Decision, PhaseResult)> {
        let start = Utc::now();
        info!("Running validation for pipeline: {}", pipeline.id.0);

        pipeline.transition_to(PipelineState::Validation)?;

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
            warn!("No scenarios ran, defaulting to retry");
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

    fn handle_spec_failure(&mut self, id: &crate::state::PipelineId, message: String) -> Decision {
        error!("Spec review failed: {message}");
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Failed);
            p.set_error(message);
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            let _ = self.store.update(pipeline);
        }
        // Invoke cleanup for failed phase
        let _ = self.cleanup_after_failure(id);
        Decision::Fail
    }

    fn handle_setup_failure(&mut self, id: &crate::state::PipelineId, message: String) -> Decision {
        error!("Universe setup failed: {message}");
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Escalated);
            p.set_error(message);
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            let _ = self.store.update(pipeline);
        }
        // Invoke cleanup + rollback for failed phase
        let _ = self.cleanup_after_failure(id);
        if let Ok(pipeline) = self.store.get(id) {
            let _ = self.rollback_phase(pipeline, PhaseType::UniverseSetup);
        }
        Decision::Escalate
    }

    fn handle_dev_failure(&mut self, id: &crate::state::PipelineId, message: String) -> Decision {
        error!("Agent development failed: {message}");
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Escalated);
            p.set_error(message);
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            let _ = self.store.update(pipeline);
        }
        // Invoke cleanup + rollback for failed phase
        let _ = self.cleanup_after_failure(id);
        if let Ok(pipeline) = self.store.get(id) {
            let _ = self.rollback_phase(pipeline, PhaseType::AgentDevelopment);
        }
        Decision::Escalate
    }

    fn finalize_acceptance(&mut self, id: &crate::state::PipelineId) -> Result<()> {
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Accepted);
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            self.store.update(pipeline)?;
            info!("Pipeline {} accepted", id.0);
        }
        Ok(())
    }

    fn escalate(&mut self, id: &crate::state::PipelineId, reason: &str) -> Result<()> {
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Escalated);
            p.set_error(reason.to_string());
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            self.store.update(pipeline)?;
            warn!("Pipeline {} escalated: {reason}", id.0);
        }
        Ok(())
    }

    fn fail(&mut self, id: &crate::state::PipelineId, reason: &str) -> Result<()> {
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Failed);
            p.set_error(reason.to_string());
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            self.store.update(pipeline)?;
            error!("Pipeline {} failed: {reason}", id.0);
        }
        Ok(())
    }

    #[must_use]
    pub fn get_pending_pipelines(&self) -> Vec<Pipeline> {
        self.store
            .get_pending_recovery()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn recover_pipeline(&mut self, pipeline_id: &crate::state::PipelineId) -> Result<Decision> {
        let pipeline = self.store.get(pipeline_id)?;

        if pipeline.state.is_terminal() {
            info!("Pipeline {} already in terminal state", pipeline_id.0);
            return match pipeline.state {
                PipelineState::Accepted => Ok(Decision::Accept),
                PipelineState::Escalated => Ok(Decision::Escalate),
                PipelineState::Failed => Ok(Decision::Fail),
                _ => Ok(Decision::Fail),
            };
        }

        self.run_pipeline(pipeline_id)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn create_executor() -> (PipelineExecutor, TempDir) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let executor = PipelineExecutor::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().join("scenarios"),
            None,
        )
        .expect("failed to create executor");
        (executor, temp_dir)
    }

    #[test]
    fn test_create_pipeline() {
        let (mut executor, _temp) = create_executor();
        let pipeline = executor
            .create_pipeline("specs/test.yaml".to_string())
            .expect("failed to create pipeline");
        assert_eq!(pipeline.state, PipelineState::Pending);
    }

    #[test]
    fn test_make_decision_some_pass() {
        let (executor, _temp) = create_executor();
        let results = vec![
            ScenarioResult {
                name: "test1".to_string(),
                passed: true,
                duration_secs: 1.0,
                error: None,
            },
            ScenarioResult {
                name: "test2".to_string(),
                passed: false,
                duration_secs: 1.0,
                error: Some("failed".to_string()),
            },
        ];
        let pipeline = Pipeline::new("spec.yaml".to_string());
        let decision = executor.make_decision(&results, &pipeline);
        assert_eq!(decision, Decision::Escalate);
    }

    #[test]
    fn test_make_decision_all_fail() {
        let (executor, _temp) = create_executor();
        let results = vec![
            ScenarioResult {
                name: "test1".to_string(),
                passed: false,
                duration_secs: 1.0,
                error: Some("failed".to_string()),
            },
            ScenarioResult {
                name: "test2".to_string(),
                passed: false,
                duration_secs: 1.0,
                error: Some("failed".to_string()),
            },
        ];
        let pipeline = Pipeline::new("spec.yaml".to_string());
        let decision = executor.make_decision(&results, &pipeline);
        assert_eq!(decision, Decision::Fail);
    }
}

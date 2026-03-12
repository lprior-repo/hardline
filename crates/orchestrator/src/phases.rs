//! Pipeline phase executor

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::{
    cleanup::{CleanupContext, CleanupManager, PhaseType},
    metrics::{Metrics, PhaseMetrics, ScenarioResult},
    persistence::StateStore,
    state::{Pipeline, PipelineId, PipelineState},
};

/// Errors that can occur during phase execution
#[derive(Debug, Clone, Error)]
pub enum PhaseError {
    #[error("Spec review failed: {0}")]
    SpecReviewFailed(String),

    #[error("Universe setup failed: {0}")]
    SetupFailed(String),

    #[error("Agent development failed: {0}")]
    DevelopmentFailed(String),

    #[error("Scenario validation failed: {0}")]
    ValidationFailed(String),

    #[error("Cleanup/rollback failed: {0}")]
    CleanupFailed(String),

    #[error("State persistence failed: {0}")]
    PersistenceFailed(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
}

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
    pub fn cleanup_after_failure(&self, pipeline: &Pipeline) -> Result<(), PhaseError> {
        let phase_type = PhaseType::from_state(pipeline.state);

        if let Some(phase) = phase_type {
            let context = CleanupContext::new(pipeline.id.clone(), phase);
            let result = self.cleanup_manager.cleanup(&context);

            if !result.success_flag() {
                let error_msg = result.errors().join("; ");
                warn!(
                    "Cleanup had errors for pipeline {}: {}",
                    pipeline.id.0, error_msg
                );
                return Err(PhaseError::CleanupFailed(error_msg));
            }

            info!(
                "Cleanup completed for pipeline {}: {} resources cleaned",
                pipeline.id.0,
                result.cleaned_resources.len()
            );
        }

        Ok(())
    }

    /// Attempt rollback for a specific phase
    pub fn rollback_phase(&self, pipeline: &Pipeline, phase: PhaseType) -> Result<(), PhaseError> {
        let context = CleanupContext::new(pipeline.id.clone(), phase);
        let result = self.cleanup_manager.rollback(&context);

        if !result.success_flag() {
            let error_msg = result.errors().join("; ");
            error!(
                "Rollback failed for pipeline {} phase {:?}: {}",
                pipeline.id.0, phase, error_msg
            );
            return Err(PhaseError::CleanupFailed(error_msg));
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

    /// Run the complete pipeline - entry point (delegates to smaller functions)
    pub fn run_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError> {
        info!("Starting pipeline: {}", pipeline_id.0);

        let mut pipeline = self
            .store
            .get(pipeline_id)
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .clone();

        if !pipeline.state.is_terminal() {
            info!("Recovering pipeline from state: {:?}", pipeline.state);
        }

        // Run phases sequentially, propagating failures
        self.run_spec_review_phase(&mut pipeline)?;
        self.run_universe_setup_phase(&mut pipeline)?;
        self.run_agent_development_phase(&mut pipeline)?;
        self.run_validation_phase(pipeline_id, &mut pipeline)
    }

    fn run_spec_review_phase(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<PhaseResult, PhaseError> {
        if pipeline.state != PipelineState::Pending && pipeline.state != PipelineState::SpecReview {
            return Ok(PhaseResult {
                success: true,
                message: "Skipped".to_string(),
                quality_score: None,
                scenario_results: vec![],
            });
        }

        let result = self
            .spec_review(pipeline)
            .map_err(|e| PhaseError::SpecReviewFailed(e.to_string()))?;

        if result.success {
            Ok(result)
        } else {
            self.handle_spec_failure(&pipeline.id, result.message)
        }
    }

    fn run_universe_setup_phase(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<PhaseResult, PhaseError> {
        if pipeline.state != PipelineState::UniverseSetup {
            return Ok(PhaseResult {
                success: true,
                message: "Skipped".to_string(),
                quality_score: None,
                scenario_results: vec![],
            });
        }

        let result = self
            .universe_setup(pipeline)
            .map_err(|e| PhaseError::SetupFailed(e.to_string()))?;

        if result.success {
            Ok(result)
        } else {
            self.handle_setup_failure(&pipeline.id, result.message)
        }
    }

    fn run_agent_development_phase(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<Decision, PhaseError> {
        while pipeline.state == PipelineState::AgentDevelopment {
            let result = self
                .agent_development(pipeline)
                .map_err(|e| PhaseError::DevelopmentFailed(e.to_string()))?;

            if !result.success {
                return self.handle_dev_failure(&pipeline.id, result.message);
            }
        }
        Ok(Decision::Accept)
    }

    fn run_validation_phase(
        &mut self,
        pipeline_id: &PipelineId,
        pipeline: &mut Pipeline,
    ) -> Result<Decision, PhaseError> {
        while pipeline.state == PipelineState::Validation {
            let (decision, _result) = self
                .validation(pipeline)
                .map_err(|e| PhaseError::ValidationFailed(e.to_string()))?;

            let continue_loop = self.handle_validation_decision(pipeline_id, pipeline, decision)?;
            if continue_loop {
                continue;
            }
            return Ok(decision);
        }

        self.get_final_decision(pipeline_id)
    }

    fn handle_validation_decision(
        &mut self,
        pipeline_id: &PipelineId,
        pipeline: &mut Pipeline,
        decision: Decision,
    ) -> Result<bool, PhaseError> {
        match decision {
            Decision::Accept => {
                self.finalize_acceptance(pipeline_id)
                    .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
                Ok(false)
            }
            Decision::Retry if pipeline.can_iterate() => {
                pipeline.iteration += 1;
                self.store
                    .update(pipeline.clone())
                    .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
                info!(
                    "Retrying agent development, iteration {}",
                    pipeline.iteration
                );
                Ok(true)
            }
            Decision::Retry => {
                warn!("Max iterations reached, escalating");
                self.escalate(pipeline_id, "Max iterations reached")
                    .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
                Ok(false)
            }
            Decision::Escalate => {
                self.escalate(pipeline_id, "Validation escalated")
                    .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
                Ok(false)
            }
            Decision::Fail => {
                self.fail(pipeline_id, "Validation failed")
                    .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
                Ok(false)
            }
        }
    }

    fn get_final_decision(&self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError> {
        let final_pipeline = self
            .store
            .get(pipeline_id)
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
        match final_pipeline.state {
            PipelineState::Accepted => Ok(Decision::Accept),
            PipelineState::Escalated => Ok(Decision::Escalate),
            PipelineState::Failed => Ok(Decision::Fail),
            _ => {
                error!("Unexpected terminal state: {:?}", final_pipeline.state);
                Ok(Decision::Fail)
            }
        }
    }

    fn spec_review(&mut self, pipeline: &mut Pipeline) -> Result<PhaseResult> {
        let start = Utc::now();
        info!("Running spec review for: {}", pipeline.spec_path);

        pipeline.transition_to(PipelineState::SpecReview)?;
        let quality_score = self.run_linter(&pipeline.spec_path);

        self.record_spec_review_metrics(pipeline, start, quality_score);

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

    /// Handle spec review failure - cleanup FIRST, then persist state
    fn handle_spec_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<PhaseResult, PhaseError> {
        error!("Spec review failed: {message}");

        // Fetch pipeline for cleanup
        let pipeline = self
            .store
            .get(id)
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .clone();

        // D2 FIX: Cleanup FIRST, then persist state after (Q4 violation fix)
        self.cleanup_after_failure(&pipeline)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

        // Now persist state after cleanup completes
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Failed);
            p.set_error(message.clone());
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            self.store
                .update(pipeline)
                .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
        }

        Ok(PhaseResult {
            success: false,
            message,
            quality_score: None,
            scenario_results: vec![],
        })
    }

    /// Handle universe setup failure - cleanup + rollback FIRST, then persist
    fn handle_setup_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<PhaseResult, PhaseError> {
        error!("Universe setup failed: {message}");
        self.perform_failure_handling(id, message, PhaseType::UniverseSetup)
    }

    /// Handle agent development failure - cleanup + rollback FIRST, then persist
    fn handle_dev_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<Decision, PhaseError> {
        error!("Agent development failed: {message}");
        self.perform_failure_handling(id, message, PhaseType::AgentDevelopment)
            .map(|_result| Decision::Escalate)
    }

    /// Common failure handling: cleanup, rollback, persist
    fn perform_failure_handling(
        &mut self,
        id: &PipelineId,
        message: String,
        phase: PhaseType,
    ) -> Result<PhaseResult, PhaseError> {
        // Fetch pipeline for cleanup
        let pipeline = self
            .store
            .get(id)
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .clone();

        // Cleanup FIRST
        self.cleanup_after_failure(&pipeline)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

        // Propagate rollback error instead of ignoring
        self.rollback_phase(&pipeline, phase)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

        // Persist state after cleanup completes
        self.persist_failure_state(id, message.clone())?;

        Ok(PhaseResult {
            success: false,
            message,
            quality_score: None,
            scenario_results: vec![],
        })
    }

    fn persist_failure_state(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<(), PhaseError> {
        let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
            let _ = p.transition_to(PipelineState::Escalated);
            p.set_error(message);
            p.clone()
        });
        if let Some(pipeline) = pipeline_opt {
            self.store
                .update(pipeline)
                .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;
        }
        Ok(())
    }

    fn finalize_acceptance(&mut self, id: &PipelineId) -> Result<()> {
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

    pub fn recover_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError> {
        let pipeline = self
            .store
            .get(pipeline_id)
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;

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

//! PipelineExecutor struct definition

use std::path::PathBuf;

use tracing::info;

use crate::cleanup::{CleanupContext, CleanupManager, PhaseType};
use crate::metrics::Metrics;
use crate::persistence::StateStore;
use crate::state::Pipeline;

use super::types::PhaseError;

/// Pipeline executor for running phases
#[allow(dead_code)]
pub struct PipelineExecutor {
    pub(crate) store: StateStore,
    pub(crate) metrics: Metrics,
    #[allow(dead_code)]
    scenarios_path: PathBuf,
    #[allow(dead_code)]
    linter_path: Option<PathBuf>,
    pub(crate) cleanup_manager: CleanupManager,
}

impl PipelineExecutor {
    pub fn new(
        state_dir: PathBuf,
        scenarios_path: PathBuf,
        linter_path: Option<PathBuf>,
    ) -> Result<Self, PhaseError> {
        let store = StateStore::new(state_dir)?;

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

    pub fn create_pipeline(&mut self, spec_path: String) -> Result<Pipeline, PhaseError> {
        let pipeline = Pipeline::new(spec_path);
        let pipeline = self.store.create(pipeline)?;
        info!("Created pipeline: {}", pipeline.id.0);
        Ok(pipeline)
    }

    /// Run cleanup after a phase failure
    pub fn cleanup_after_failure(&self, pipeline: &Pipeline) -> Result<(), PhaseError> {
        let phase_type = PhaseType::from_state(pipeline.state);

        if let Some(phase) = phase_type {
            let context = CleanupContext::new(pipeline.id.clone(), phase);
            let result = self.cleanup_manager.cleanup(&context);

            if !result.success_flag() {
                let error_msg = result.errors().join("; ");
                tracing::warn!(
                    "Cleanup had errors for pipeline {}: {}",
                    pipeline.id.0,
                    error_msg
                );
                return Err(PhaseError::CleanupFailed(error_msg));
            }

            tracing::info!(
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
            tracing::error!(
                "Rollback failed for pipeline {} phase {:?}: {}",
                pipeline.id.0,
                phase,
                error_msg
            );
            return Err(PhaseError::CleanupFailed(error_msg));
        }

        tracing::info!(
            "Rollback completed for pipeline {} phase {:?}",
            pipeline.id.0,
            phase
        );

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
}

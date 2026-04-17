//! Failure handling methods for pipeline phases

use tracing::error;

use crate::cleanup::PhaseType;
use crate::state::{PipelineId, PipelineState};

use super::executor::PipelineExecutor;
use super::types::{PhaseError, PhaseResult};

impl PipelineExecutor {
    pub(crate) fn handle_spec_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<PhaseResult, PhaseError> {
        error!("Spec review failed: {message}");

        let pipeline = self
            .store
            .get(id)
            .map_err(PhaseError::from)?
            .clone();

        self.cleanup_after_failure(&pipeline)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

        self.store
            .mutate_and_persist(id, |pipeline| {
                pipeline
                    .transition_to(PipelineState::Failed)
                    .map_err(|e| format!("invalid state transition for pipeline {id}: {e}"))?;
                pipeline.set_error(message.clone());
                tracing::debug!(pipeline_id = %id.0, new_state = "failed", "pipeline state transitioned");
                Ok(())
            })
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .map_err(PhaseError::InvalidStateTransition)?;

        Ok(PhaseResult {
            success: false,
            message,
            quality_score: None,
            scenario_results: vec![],
        })
    }

    pub(crate) fn handle_setup_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<PhaseResult, PhaseError> {
        error!("Universe setup failed: {message}");
        self.perform_failure_handling(id, message, PhaseType::UniverseSetup)
    }

    pub(crate) fn handle_dev_failure(
        &mut self,
        id: &PipelineId,
        message: String,
    ) -> Result<super::types::Decision, PhaseError> {
        error!("Agent development failed: {message}");
        self.perform_failure_handling(id, message, PhaseType::AgentDevelopment)
            .map(|_result| super::types::Decision::Escalate)
    }

    fn perform_failure_handling(
        &mut self,
        id: &PipelineId,
        message: String,
        phase: PhaseType,
    ) -> Result<PhaseResult, PhaseError> {
        let pipeline = self
            .store
            .get(id)
            .map_err(PhaseError::from)?
            .clone();

        self.cleanup_after_failure(&pipeline)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

        self.rollback_phase(&pipeline, phase)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

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
        self.store
            .mutate_and_persist(id, |pipeline| {
                pipeline
                    .transition_to(PipelineState::Escalated)
                    .map_err(|e| format!("invalid state transition for pipeline {id}: {e}"))?;
                pipeline.set_error(message);
                tracing::debug!(pipeline_id = %id.0, new_state = "escalated", "pipeline state transitioned");
                Ok(())
            })
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .map_err(PhaseError::InvalidStateTransition)?;
        Ok(())
    }
}

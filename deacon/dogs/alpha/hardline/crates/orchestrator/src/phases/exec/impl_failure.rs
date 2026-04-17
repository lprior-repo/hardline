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
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
            .clone();

        self.cleanup_after_failure(&pipeline)
            .map_err(|e| PhaseError::CleanupFailed(e.to_string()))?;

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
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?
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
}

//! State transition methods for pipeline lifecycle

use tracing::{error, info, warn};

use crate::state::{PipelineId, PipelineState, TransitionError};

use super::executor::PipelineExecutor;
use super::types::PhaseError;

impl PipelineExecutor {
    pub fn recover_pipeline(
        &mut self,
        pipeline_id: &PipelineId,
    ) -> Result<super::types::Decision, PhaseError> {
        let pipeline = self
            .store
            .get(pipeline_id)
            .map_err(PhaseError::from)?;

        if pipeline.state.is_terminal() {
            info!("Pipeline {} already in terminal state", pipeline_id.0);
            return match pipeline.state {
                PipelineState::Accepted => Ok(super::types::Decision::Accept),
                PipelineState::Escalated => Ok(super::types::Decision::Escalate),
                PipelineState::Failed => Ok(super::types::Decision::Fail),
                _ => Ok(super::types::Decision::Fail),
            };
        }

        self.run_pipeline(pipeline_id)
    }

    pub(crate) fn finalize_acceptance(&mut self, id: &PipelineId) -> Result<(), PhaseError> {
        self.store
            .mutate_and_persist(id, |pipeline| -> Result<(), TransitionError> {
                pipeline.transition_to(PipelineState::Accepted)?;
                tracing::debug!(pipeline_id = %id.0, new_state = "accepted", "pipeline state transitioned");
                Ok(())
            })
            .map_err(PhaseError::PersistenceFailed)?
            .map_err(PhaseError::InvalidStateTransition)?;
        info!("Pipeline {} accepted", id.0);
        Ok(())
    }

    pub(crate) fn escalate(&mut self, id: &PipelineId, reason: &str) -> Result<(), PhaseError> {
        let reason = reason.to_string();
        self.store
            .mutate_and_persist(id, |pipeline| -> Result<(), TransitionError> {
                pipeline.transition_to(PipelineState::Escalated)?;
                pipeline.set_error(reason.clone());
                tracing::debug!(pipeline_id = %id.0, new_state = "escalated", "pipeline state transitioned");
                Ok(())
            })
            .map_err(PhaseError::PersistenceFailed)?
            .map_err(PhaseError::InvalidStateTransition)?;
        warn!("Pipeline {} escalated: {reason}", id.0);
        Ok(())
    }

    pub(crate) fn fail(&mut self, id: &PipelineId, reason: &str) -> Result<(), PhaseError> {
        let reason = reason.to_string();
        self.store
            .mutate_and_persist(id, |pipeline| -> Result<(), TransitionError> {
                pipeline.transition_to(PipelineState::Failed)?;
                pipeline.set_error(reason.clone());
                tracing::debug!(pipeline_id = %id.0, new_state = "failed", "pipeline state transitioned");
                Ok(())
            })
            .map_err(PhaseError::PersistenceFailed)?
            .map_err(PhaseError::InvalidStateTransition)?;
        error!("Pipeline {} failed: {reason}", id.0);
        Ok(())
    }
}

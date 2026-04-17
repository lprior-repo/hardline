//! State transition methods for pipeline lifecycle

use tracing::{error, info, warn};

use crate::state::{PipelineId, PipelineState};

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
            .map_err(|e| PhaseError::PersistenceFailed(e.to_string()))?;

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

    pub(crate) fn finalize_acceptance(&mut self, id: &PipelineId) -> anyhow::Result<()> {
        let pipeline = self
            .store
            .get_mut(id)
            .map_err(|e| anyhow::anyhow!("failed to get pipeline {id}: {e}"))?;

        pipeline
            .transition_to(PipelineState::Accepted)
            .map_err(|e| anyhow::anyhow!("invalid state transition for pipeline {id}: {e}"))?;
        tracing::debug!(pipeline_id = %id.0, new_state = "accepted", "pipeline state transitioned");

        let pipeline = pipeline.clone();
        self.store.update(pipeline)?;
        info!("Pipeline {} accepted", id.0);
        Ok(())
    }

    pub(crate) fn escalate(&mut self, id: &PipelineId, reason: &str) -> anyhow::Result<()> {
        let pipeline = self
            .store
            .get_mut(id)
            .map_err(|e| anyhow::anyhow!("failed to get pipeline {id}: {e}"))?;

        pipeline
            .transition_to(PipelineState::Escalated)
            .map_err(|e| anyhow::anyhow!("invalid state transition for pipeline {id}: {e}"))?;
        pipeline.set_error(reason.to_string());
        tracing::debug!(pipeline_id = %id.0, new_state = "escalated", "pipeline state transitioned");

        let pipeline = pipeline.clone();
        self.store.update(pipeline)?;
        warn!("Pipeline {} escalated: {reason}", id.0);
        Ok(())
    }

    pub(crate) fn fail(&mut self, id: &PipelineId, reason: &str) -> anyhow::Result<()> {
        let pipeline = self
            .store
            .get_mut(id)
            .map_err(|e| anyhow::anyhow!("failed to get pipeline {id}: {e}"))?;

        pipeline
            .transition_to(PipelineState::Failed)
            .map_err(|e| anyhow::anyhow!("invalid state transition for pipeline {id}: {e}"))?;
        pipeline.set_error(reason.to_string());
        tracing::debug!(pipeline_id = %id.0, new_state = "failed", "pipeline state transitioned");

        let pipeline = pipeline.clone();
        self.store.update(pipeline)?;
        error!("Pipeline {} failed: {reason}", id.0);
        Ok(())
    }
}

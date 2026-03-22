//! State transition methods for pipeline lifecycle

use tracing::{error, info, warn};

use crate::state::{PipelineId, PipelineState};

use super::executor::PipelineExecutor;
use super::types::PhaseError;

impl PipelineExecutor {
    pub fn recover_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<super::types::Decision, PhaseError> {
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

    pub(crate) fn escalate(&mut self, id: &PipelineId, reason: &str) -> anyhow::Result<()> {
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

    pub(crate) fn fail(&mut self, id: &PipelineId, reason: &str) -> anyhow::Result<()> {
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
}

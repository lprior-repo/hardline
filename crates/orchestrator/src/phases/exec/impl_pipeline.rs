//! Pipeline runner methods

use tracing::info;

use crate::state::{Pipeline, PipelineId, PipelineState};

use super::executor::PipelineExecutor;
use super::types::{Decision, PhaseError, PhaseResult};

impl PipelineExecutor {
    /// Run the complete pipeline - entry point
    pub fn run_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError> {
        info!("Starting pipeline: {}", pipeline_id.0);

        let mut pipeline = self.store.get(pipeline_id)?.clone();

        if !pipeline.state.is_terminal() {
            info!("Recovering pipeline from state: {:?}", pipeline.state);
        }

        self.run_spec_review_phase(&mut pipeline)?;
        self.persist_state(&pipeline)?;
        self.run_universe_setup_phase(&mut pipeline)?;
        self.persist_state(&pipeline)?;
        self.run_agent_development_phase(&mut pipeline)?;
        self.persist_state(&pipeline)?;
        self.run_validation_phase(pipeline_id, &mut pipeline)
    }

    fn persist_state(&mut self, pipeline: &Pipeline) -> Result<(), PhaseError> {
        self.store.update(pipeline.clone())?;
        Ok(())
    }

    pub(crate) fn run_spec_review_phase(
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

        let result = self.spec_review(pipeline)?;

        if result.success {
            Ok(result)
        } else {
            self.handle_spec_failure(&pipeline.id, result.message)
        }
    }

    pub(crate) fn run_universe_setup_phase(
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

        let result = self.universe_setup(pipeline)?;

        if result.success {
            Ok(result)
        } else {
            self.handle_setup_failure(&pipeline.id, result.message)
        }
    }

    pub(crate) fn run_agent_development_phase(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<Decision, PhaseError> {
        while pipeline.state == PipelineState::AgentDevelopment {
            let result = self.agent_development(pipeline)?;

            if !result.success {
                return self.handle_dev_failure(&pipeline.id, result.message);
            }
        }
        Ok(Decision::Accept)
    }

    pub(crate) fn run_validation_phase(
        &mut self,
        pipeline_id: &PipelineId,
        pipeline: &mut Pipeline,
    ) -> Result<Decision, PhaseError> {
        while pipeline.state == PipelineState::Validation {
            let (decision, _result) = self.validation(pipeline)?;

            let continue_loop = self.handle_validation_decision(pipeline_id, pipeline, decision)?;
            if continue_loop {
                continue;
            }
            return Ok(decision);
        }

        self.get_final_decision(pipeline_id)
    }

    pub(crate) fn handle_validation_decision(
        &mut self,
        pipeline_id: &PipelineId,
        pipeline: &mut Pipeline,
        decision: Decision,
    ) -> Result<bool, PhaseError> {
        match decision {
            Decision::Accept => {
                self.finalize_acceptance(pipeline_id)?;
                Ok(false)
            }
            Decision::Retry if pipeline.can_iterate() => {
                pipeline
                    .increment_iteration()
                    .map_err(PhaseError::from)?;
                self.store
                    .update(pipeline.clone())
                    .map_err(PhaseError::from)?;
                info!(
                    "Retrying agent development, iteration {}",
                    pipeline.iteration
                );
                Ok(true)
            }
            Decision::Retry => {
                tracing::warn!("Max iterations reached, escalating");
                self.escalate(pipeline_id, "Max iterations reached")?;
                Ok(false)
            }
            Decision::Escalate => {
                self.escalate(pipeline_id, "Validation escalated")?;
                Ok(false)
            }
            Decision::Fail => {
                self.fail(pipeline_id, "Validation failed")?;
                Ok(false)
            }
        }
    }

    fn get_final_decision(&self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError> {
        let final_pipeline = self.store.get(pipeline_id)?;
        match final_pipeline.state {
            PipelineState::Accepted => Ok(Decision::Accept),
            PipelineState::Escalated => Ok(Decision::Escalate),
            PipelineState::Failed => Ok(Decision::Fail),
            _ => {
                tracing::error!("Unexpected terminal state: {:?}", final_pipeline.state);
                Ok(Decision::Fail)
            }
        }
    }
}

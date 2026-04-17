//! Failure handling methods for pipeline phases

use tracing::error;

use crate::cleanup::PhaseType;
use crate::state::{PipelineId, PipelineState, TransitionError};

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
            .mutate_and_persist(id, |pipeline| -> Result<(), TransitionError> {
                pipeline.transition_to(PipelineState::Failed)?;
                pipeline.set_error(message.clone());
                tracing::debug!(pipeline_id = %id.0, new_state = "failed", "pipeline state transitioned");
                Ok(())
            })
            .map_err(PhaseError::PersistenceFailed)?
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
            .mutate_and_persist(id, |pipeline| -> Result<(), TransitionError> {
                pipeline.transition_to(PipelineState::Escalated)?;
                pipeline.set_error(message);
                tracing::debug!(pipeline_id = %id.0, new_state = "escalated", "pipeline state transitioned");
                Ok(())
            })
            .map_err(PhaseError::PersistenceFailed)?
            .map_err(PhaseError::InvalidStateTransition)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::state::{Pipeline, PipelineState};

    use super::super::executor::PipelineExecutor;
    use super::super::types::{Decision, PhaseError};

    /// Helper: create an executor backed by a temp dir
    fn create_executor() -> (PipelineExecutor, TempDir) {
        let temp = TempDir::new().expect("temp dir");
        let exec = PipelineExecutor::new(
            temp.path().to_path_buf(),
            temp.path().join("scenarios"),
            None,
        )
        .expect("executor");
        (exec, temp)
    }

    /// Helper: create a pipeline at a given state in the store.
    /// Walks through the valid transition path to reach states that
    /// can't be reached directly from Pending.
    fn create_pipeline_at(
        exec: &mut PipelineExecutor,
        state: PipelineState,
    ) -> crate::state::PipelineId {
        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();
        exec.store.create(pipeline).expect("create");
        let p = exec.store.get_mut(&id).expect("get_mut");

        // Walk the valid transition path to the target state
        let path: Vec<PipelineState> = match state {
            PipelineState::Pending => vec![],
            PipelineState::SpecReview => vec![PipelineState::SpecReview],
            PipelineState::UniverseSetup => vec![
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
            ],
            PipelineState::AgentDevelopment => vec![
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::AgentDevelopment,
            ],
            PipelineState::Validation => vec![
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::AgentDevelopment,
                PipelineState::Validation,
            ],
            // Terminal states: transition to them via Escalated/Failed from a reachable state
            PipelineState::Accepted | PipelineState::Escalated | PipelineState::Failed => {
                vec![
                    PipelineState::SpecReview,
                    PipelineState::UniverseSetup,
                    PipelineState::AgentDevelopment,
                    PipelineState::Validation,
                    state,
                ]
            }
        };
        for s in path {
            p.transition_to(s)
                .unwrap_or_else(|e| panic!("transition to {s:?} failed: {e}"));
        }
        id
    }

    // --- handle_spec_failure ---

    #[test]
    fn handle_spec_failure_transitions_to_failed() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        let result = exec.handle_spec_failure(&id, "lint error".to_string()).expect("handle");

        assert!(!result.success);
        assert_eq!(result.message, "lint error");
        assert!(result.quality_score.is_none());
        assert!(result.scenario_results.is_empty());

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    #[test]
    fn handle_spec_failure_sets_error_message() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        exec.handle_spec_failure(&id, "type mismatch".to_string()).expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.last_error.as_deref(), Some("type mismatch"));
    }

    #[test]
    fn handle_spec_failure_returns_phase_result_structure() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        let result = exec
            .handle_spec_failure(&id, "error msg".to_string())
            .expect("handle");

        assert!(!result.success);
        assert_eq!(result.message, "error msg");
        assert!(result.quality_score.is_none());
        assert!(result.scenario_results.is_empty());
    }

    #[test]
    fn handle_spec_failure_for_missing_pipeline_returns_persistence_error() {
        let (mut exec, _temp) = create_executor();
        let missing = crate::state::PipelineId("nonexistent".to_string());

        let result = exec.handle_spec_failure(&missing, "error".to_string());
        assert!(result.is_err());
        match result.unwrap_err() {
            PhaseError::PersistenceFailed(msg) => {
                assert!(msg.contains("nonexistent"));
            }
            other => panic!("expected PersistenceFailed, got: {other}"),
        }
    }

    #[test]
    fn handle_spec_failure_from_pending_state() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Pending);

        // Pending has no phase type, so cleanup_after_failure returns Ok (no-op).
        // Then transition to Failed succeeds from any state.
        let result = exec.handle_spec_failure(&id, "early failure".to_string()).expect("handle");

        assert!(!result.success);
        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    // --- handle_setup_failure ---

    #[test]
    fn handle_setup_failure_transitions_to_escalated() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::UniverseSetup);

        exec.handle_setup_failure(&id, "disk full".to_string()).expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Escalated);
    }

    #[test]
    fn handle_setup_failure_sets_error_message() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::UniverseSetup);

        exec.handle_setup_failure(&id, "oom".to_string()).expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.last_error.as_deref(), Some("oom"));
    }

    #[test]
    fn handle_setup_failure_returns_failure_phase_result() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::UniverseSetup);

        let result = exec
            .handle_setup_failure(&id, "setup err".to_string())
            .expect("handle");

        assert!(!result.success);
        assert_eq!(result.message, "setup err");
        assert!(result.quality_score.is_none());
    }

    #[test]
    fn handle_setup_failure_for_missing_pipeline_returns_error() {
        let (mut exec, _temp) = create_executor();
        let missing = crate::state::PipelineId("nope".to_string());

        let result = exec.handle_setup_failure(&missing, "err".to_string());
        assert!(result.is_err());
    }

    // --- handle_dev_failure ---

    #[test]
    fn handle_dev_failure_returns_escalate_decision() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::AgentDevelopment);

        let decision = exec
            .handle_dev_failure(&id, "compile error".to_string())
            .expect("handle");

        assert_eq!(decision, Decision::Escalate);
    }

    #[test]
    fn handle_dev_failure_transitions_to_escalated() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::AgentDevelopment);

        exec.handle_dev_failure(&id, "compile error".to_string())
            .expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Escalated);
    }

    #[test]
    fn handle_dev_failure_sets_error_message() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::AgentDevelopment);

        exec.handle_dev_failure(&id, "agent crash".to_string())
            .expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.last_error.as_deref(), Some("agent crash"));
    }

    #[test]
    fn handle_dev_failure_for_missing_pipeline_returns_error() {
        let (mut exec, _temp) = create_executor();
        let missing = crate::state::PipelineId("ghost".to_string());

        let result = exec.handle_dev_failure(&missing, "err".to_string());
        assert!(result.is_err());
    }

    // --- Cleanup integration: spec failure with cleanup errors ---

    #[test]
    fn handle_spec_failure_from_validation_state() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Validation);

        // Validation phase has a NoopCleanupHandler, so cleanup succeeds.
        let result = exec
            .handle_spec_failure(&id, "validation broke".to_string())
            .expect("handle");

        assert!(!result.success);
        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    // --- persist_failure_state edge cases (tested indirectly) ---

    #[test]
    fn handle_setup_failure_from_pending_no_phase_cleanup() {
        // Pending has no PhaseType (from_state returns None), so cleanup is a no-op.
        // persist_failure_state then transitions to Escalated.
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Pending);

        exec.handle_setup_failure(&id, "early exit".to_string())
            .expect("handle");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Escalated);
    }

    #[test]
    fn handle_dev_failure_from_spec_review() {
        let (mut exec, _temp) = create_executor();
        // Even from SpecReview, handle_dev_failure uses perform_failure_handling
        // which calls rollback_phase(SpecReview) — NoopCleanupHandler, succeeds.
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        let decision = exec
            .handle_dev_failure(&id, "unexpected dev error".to_string())
            .expect("handle");

        assert_eq!(decision, Decision::Escalate);
    }

    // --- PhaseError variant matching ---

    #[test]
    fn handle_spec_failure_error_is_persistence_failed_for_missing() {
        let (mut exec, _temp) = create_executor();
        let missing = crate::state::PipelineId("missing-pipeline".to_string());

        let result = exec.handle_spec_failure(&missing, "err".to_string());
        let err = result.unwrap_err();
        let err_display = format!("{err}");
        assert!(
            err_display.to_lowercase().contains("persistence"),
            "expected PersistenceFailed, got: {err_display}"
        );
    }

    #[test]
    fn handle_setup_failure_error_is_persistence_failed_for_missing() {
        let (mut exec, _temp) = create_executor();
        let missing = crate::state::PipelineId("missing-pipeline".to_string());

        let result = exec.handle_setup_failure(&missing, "err".to_string());
        let err = result.unwrap_err();
        let err_display = format!("{err}");
        assert!(
            err_display.to_lowercase().contains("persistence"),
            "expected PersistenceFailed, got: {err_display}"
        );
    }
}

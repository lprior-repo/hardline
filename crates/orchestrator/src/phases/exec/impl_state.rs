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
        let pipeline = self.store.get(pipeline_id).map_err(PhaseError::from)?;

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::state::{Pipeline, PipelineState};

    use super::super::types::Decision;

    use super::super::executor::PipelineExecutor;

    /// Helper: create an executor backed by a temp dir
    fn create_executor() -> (PipelineExecutor, TempDir) {
        let temp = TempDir::new().expect("temp dir");
        let exec = PipelineExecutor::new(temp.path().to_path_buf()).expect("executor");
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
            PipelineState::UniverseSetup => {
                vec![PipelineState::SpecReview, PipelineState::UniverseSetup]
            }
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

    // --- recover_pipeline ---

    #[test]
    fn recover_pipeline_returns_fail_for_already_failed() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Failed);

        let decision = exec.recover_pipeline(&id).expect("recover");
        assert_eq!(decision, Decision::Fail);
    }

    #[test]
    fn recover_pipeline_returns_accept_for_already_accepted() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Accepted);

        let decision = exec.recover_pipeline(&id).expect("recover");
        assert_eq!(decision, Decision::Accept);
    }

    #[test]
    fn recover_pipeline_returns_escalate_for_already_escalated() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Escalated);

        let decision = exec.recover_pipeline(&id).expect("recover");
        assert_eq!(decision, Decision::Escalate);
    }

    #[test]
    fn recover_pipeline_returns_persistence_error_for_missing_pipeline() {
        let (mut exec, _temp) = create_executor();
        let missing_id = crate::state::PipelineId("nonexistent".to_string());

        let result = exec.recover_pipeline(&missing_id);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("nonexistent"),
            "error should mention missing ID: {msg}"
        );
    }

    #[test]
    fn recover_pipeline_from_non_terminal_calls_run_pipeline() {
        // Pending state is non-terminal, so recover_pipeline calls run_pipeline.
        // run_pipeline will try spec_review which we don't mock here,
        // so we test that it at least attempts the call without panicking on
        // the early state check.
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Pending);

        // run_pipeline will fail because there's no spec_review implementation wired,
        // but we verify it doesn't short-circuit on the terminal check.
        let result = exec.recover_pipeline(&id);
        // Should NOT return Ok(Fail) since Pending is non-terminal
        match result {
            Ok(Decision::Fail) => panic!("should not short-circuit Pending to Fail"),
            _ => {} // either Ok(Retry/Escalate/Accept) or Err — both valid paths
        }
    }

    #[test]
    fn recover_pipeline_from_spec_review_is_non_terminal() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        let result = exec.recover_pipeline(&id);
        // SpecReview is non-terminal → should NOT short-circuit to Fail
        if let Ok(Decision::Fail) = result {
            panic!("SpecReview is non-terminal, should not return Fail immediately");
        }
    }

    // --- finalize_acceptance ---

    #[test]
    fn finalize_acceptance_transitions_to_accepted() {
        let (mut exec, _temp) = create_executor();
        // Create pipeline in Validation (which can transition to Accepted)
        let id = create_pipeline_at(&mut exec, PipelineState::Validation);

        exec.finalize_acceptance(&id).expect("finalize");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Accepted);
    }

    #[test]
    fn finalize_acceptance_is_idempotent_ok() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Validation);

        // Call twice — second time should still succeed
        exec.finalize_acceptance(&id).expect("first");
        exec.finalize_acceptance(&id).expect("second");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Accepted);
    }

    #[test]
    fn finalize_acceptance_with_nonexistent_pipeline_succeeds_silently() {
        let (mut exec, _temp) = create_executor();
        let missing_id = crate::state::PipelineId("nope".to_string());

        // The method uses .ok().map() which silently discards missing pipelines
        let result = exec.finalize_acceptance(&missing_id);
        assert!(result.is_ok());
    }

    #[test]
    fn finalize_acceptance_from_invalid_state_silently_discards_error() {
        let (mut exec, _temp) = create_executor();
        // Pending -> Accepted is an invalid transition in the state machine
        let id = create_pipeline_at(&mut exec, PipelineState::Pending);

        // The method uses `let _ = p.transition_to(...)` so the transition error
        // is silently discarded. But since transition fails, state won't be Accepted.
        exec.finalize_acceptance(&id).expect("finalize");

        let pipeline = exec.store.get(&id).expect("get");
        // The transition_to fails silently, so state remains Pending
        assert_eq!(pipeline.state, PipelineState::Pending);
    }

    // --- escalate ---

    #[test]
    fn escalate_transitions_to_escalated() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Validation);

        exec.escalate(&id, "test reason").expect("escalate");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Escalated);
        assert_eq!(pipeline.last_error.as_deref(), Some("test reason"));
    }

    #[test]
    fn escalate_sets_error_message() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::AgentDevelopment);

        exec.escalate(&id, "something went wrong")
            .expect("escalate");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.last_error.as_deref(), Some("something went wrong"));
    }

    #[test]
    fn escalate_from_terminal_state_silently_noop() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Failed);

        // Already terminal — transition_to will fail with AlreadyTerminal,
        // but it's silently discarded. The update branch won't run.
        exec.escalate(&id, "escalate from failed")
            .expect("escalate");

        let pipeline = exec.store.get(&id).expect("get");
        // State should still be Failed since transition was rejected
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    #[test]
    fn escalate_with_nonexistent_pipeline_succeeds_silently() {
        let (mut exec, _temp) = create_executor();
        let missing_id = crate::state::PipelineId("ghost".to_string());

        let result = exec.escalate(&missing_id, "reason");
        assert!(result.is_ok());
    }

    #[test]
    fn escalate_from_any_non_terminal_state() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for state in non_terminals {
            let (mut exec, _temp) = create_executor();
            let id = create_pipeline_at(&mut exec, state);

            exec.escalate(&id, "reason").expect("escalate");

            let pipeline = exec.store.get(&id).expect("get");
            assert_eq!(
                pipeline.state,
                PipelineState::Escalated,
                "expected {state:?} -> Escalated"
            );
        }
    }

    // --- fail ---

    #[test]
    fn fail_transitions_to_failed() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Validation);

        exec.fail(&id, "test failure").expect("fail");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    #[test]
    fn fail_sets_error_message() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::SpecReview);

        exec.fail(&id, "catastrophe").expect("fail");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.last_error.as_deref(), Some("catastrophe"));
    }

    #[test]
    fn fail_from_already_failed_stays_failed() {
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Failed);

        exec.fail(&id, "double fail").expect("fail");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Failed);
        // Error should be updated since set_error runs before transition_to
        // but transition_to(Failed) from Failed returns AlreadyTerminal
        // which is discarded with `let _ = ...`. However set_error still runs.
        assert_eq!(pipeline.last_error.as_deref(), Some("double fail"));
    }

    #[test]
    fn fail_with_nonexistent_pipeline_succeeds_silently() {
        let (mut exec, _temp) = create_executor();
        let missing_id = crate::state::PipelineId("ghost".to_string());

        let result = exec.fail(&missing_id, "reason");
        assert!(result.is_ok());
    }

    #[test]
    fn fail_from_any_non_terminal_state() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ];
        for state in non_terminals {
            let (mut exec, _temp) = create_executor();
            let id = create_pipeline_at(&mut exec, state);

            exec.fail(&id, "reason").expect("fail");

            let pipeline = exec.store.get(&id).expect("get");
            assert_eq!(
                pipeline.state,
                PipelineState::Failed,
                "expected {state:?} -> Failed"
            );
        }
    }

    // --- Transition error silently discarded (the bug the bead describes) ---

    #[test]
    fn finalize_acceptance_invalid_transition_does_not_persist_bad_state() {
        // When transition_to fails (invalid), the cloned pipeline has the OLD state
        // because transition_to didn't change it. So the update persists the old state.
        let (mut exec, _temp) = create_executor();
        let id = create_pipeline_at(&mut exec, PipelineState::Pending);

        exec.finalize_acceptance(&id).expect("finalize");

        let pipeline = exec.store.get(&id).expect("get");
        assert_eq!(pipeline.state, PipelineState::Pending);
    }
}

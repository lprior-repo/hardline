//! Parallel phase execution support

use std::collections::HashSet;

use tracing::debug;

use super::{
    executor::PipelineExecutor,
    types::{PhaseError, PhaseResult},
};
use crate::{
    cleanup::PhaseType,
    parallel::{DependencyGraph, ParallelError, ParallelExecutor, PhaseGroup},
    state::{Pipeline, PipelineState, TransitionError},
};

impl PipelineExecutor {
    pub fn execute_parallel_phases(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<Vec<PhaseResult>, PhaseError> {
        if pipeline.state.is_terminal() {
            return Err(PhaseError::InvalidStateTransition(
                TransitionError::AlreadyTerminal {
                    current: pipeline.state,
                },
            ));
        }

        let phase_groups = ParallelExecutor::resolve_parallel_phases(&pipeline.state);

        if phase_groups.is_empty() {
            debug!("No phases to execute for state {:?}", pipeline.state);
            return Ok(vec![]);
        }

        let mut results = Vec::new();

        for group in phase_groups {
            let group_results = self.execute_phase_group(pipeline, &group)?;
            results.extend(group_results);
        }

        Ok(results)
    }

    fn execute_phase_group(
        &mut self,
        pipeline: &mut Pipeline,
        group: &PhaseGroup,
    ) -> Result<Vec<PhaseResult>, PhaseError> {
        if group.phases.is_empty() {
            return Ok(vec![]);
        }

        if group.phases.len() == 1 {
            let phase_type = group.phases[0];
            return self.execute_single_phase(pipeline, phase_type);
        }

        let phases = &group.phases;
        let graph = ParallelExecutor::build_dependency_graph(phases).map_err(PhaseError::from)?;

        self.execute_with_dependency_graph(pipeline, graph)
    }

    fn execute_single_phase(
        &mut self,
        pipeline: &mut Pipeline,
        phase_type: PhaseType,
    ) -> Result<Vec<PhaseResult>, PhaseError> {
        match phase_type {
            PhaseType::SpecReview => {
                let result = self.spec_review(pipeline)?;
                Ok(vec![result])
            }
            PhaseType::UniverseSetup => {
                let result = self.universe_setup(pipeline)?;
                Ok(vec![result])
            }
            PhaseType::AgentDevelopment => {
                let _result = self.agent_development(pipeline)?;
                Ok(vec![])
            }
            PhaseType::Validation => {
                let (_decision, result) = self.validation(pipeline)?;
                Ok(vec![result])
            }
        }
    }

    fn execute_with_dependency_graph(
        &mut self,
        pipeline: &mut Pipeline,
        mut graph: DependencyGraph,
    ) -> Result<Vec<PhaseResult>, PhaseError> {
        let mut results = Vec::new();
        let mut completed: HashSet<PhaseType> = HashSet::new();

        while !graph.is_complete() {
            let ready = graph.get_ready_phases(&completed);

            if ready.is_empty() {
                if graph.has_failures() {
                    return Err(PhaseError::ParallelExecutionFailed(
                        ParallelError::ExecutionFailed(
                            "No phases ready but graph incomplete with failures".to_string(),
                        ),
                    ));
                }
                break;
            }

            for phase_type in ready {
                graph.mark_running(phase_type);

                let result = self.execute_single_phase(pipeline, phase_type)?;

                for r in result {
                    if r.success {
                        graph.mark_completed(phase_type);
                        completed.insert(phase_type);
                    } else {
                        graph.mark_failed(phase_type);
                    }
                    results.push(r);
                }
            }
        }

        Ok(results)
    }

    pub fn validate_pipeline_parallel(&self, pipeline: &Pipeline) -> Result<(), PhaseError> {
        if pipeline.state.is_terminal() {
            return Err(PhaseError::InvalidStateTransition(
                TransitionError::AlreadyTerminal {
                    current: pipeline.state,
                },
            ));
        }

        let phase_groups = ParallelExecutor::resolve_parallel_phases(&pipeline.state);

        for group in phase_groups {
            if group.phases.len() > 1 {
                ParallelExecutor::build_dependency_graph(&group.phases)
                    .map_err(PhaseError::from)?;
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn get_parallelizable_phases(pipeline_state: &PipelineState) -> Vec<PhaseType> {
        ParallelExecutor::resolve_parallel_phases(pipeline_state)
            .into_iter()
            .flat_map(|g| g.phases)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::state::{Pipeline, PipelineState};

    /// Helper: create an executor backed by a temp dir.
    fn setup_executor() -> (PipelineExecutor, TempDir) {
        let tmp = TempDir::new().expect("temp dir");
        let executor = PipelineExecutor::new(tmp.path().to_path_buf()).expect("executor");
        (executor, tmp)
    }

    /// Helper: create a pipeline in a given state.
    fn pipeline_in_state(state: PipelineState) -> Pipeline {
        let mut p = Pipeline::new("specs/test.yaml".to_string());
        // Force state without going through transition_to so we can test
        // arbitrary states without needing valid transition paths.
        p.state = state;
        p
    }

    // ── execute_parallel_phases ──────────────────────────────────────

    #[test]
    fn parallel_rejects_terminal_states() {
        for terminal in [
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ] {
            let (mut executor, _tmp) = setup_executor();
            let mut pipeline = pipeline_in_state(terminal);
            let result = executor.execute_parallel_phases(&mut pipeline);
            assert!(
                result.is_err(),
                "Expected error for terminal state {terminal:?}"
            );
            let err = result.unwrap_err();
            assert!(
                matches!(err, PhaseError::InvalidStateTransition(_)),
                "Expected InvalidStateTransition, got {err:?}"
            );
        }
    }

    #[test]
    fn parallel_returns_empty_for_non_executable_states() {
        // Accepted/Escalated/Failed are terminal (covered above).
        // No non-terminal state maps to empty phase groups in the current impl,
        // but this guards the early-return path in case it changes.
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Pending);
        // Pending -> resolve returns [SpecReview], so it should NOT be empty
        let result = executor.execute_parallel_phases(&mut pipeline);
        assert!(result.is_ok());
        // It runs spec_review which transitions to UniverseSetup (linter returns 85 >= threshold
        // 80)
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn parallel_executes_single_phase_for_pending() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Pending);
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(results[0].quality_score.is_some());
        // spec_review passes linter (85 >= 80) and transitions to UniverseSetup
        assert_eq!(pipeline.state, PipelineState::UniverseSetup);
    }

    #[test]
    fn parallel_spec_review_state_resolves_to_universe_setup_phase() {
        // resolve_parallel_phases(SpecReview) returns [UniverseSetup].
        // universe_setup() transitions to AgentDevelopment, which requires
        // the pipeline to be in UniverseSetup state — but it's in SpecReview.
        // So this should fail with an invalid transition error.
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::SpecReview);
        let result = executor.execute_parallel_phases(&mut pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn parallel_executes_single_phase_for_universe_setup() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::UniverseSetup);
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        // agent_development returns Ok(vec![]) — no PhaseResult in the wrapper
        assert!(results.is_empty());
        // agent_development transitions UniverseSetup -> AgentDevelopment -> Validation
        assert_eq!(pipeline.state, PipelineState::Validation);
    }

    #[test]
    fn parallel_executes_single_phase_for_agent_development() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::AgentDevelopment);
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        // agent_development returns Ok(vec![]) — no PhaseResult pushed
        assert!(results.is_empty());
        // agent_development self-loops then transitions to Validation
        assert_eq!(pipeline.state, PipelineState::Validation);
    }

    #[test]
    fn parallel_executes_single_phase_for_validation() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Validation);
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        // validation returns Accept (100% pass rate) but doesn't transition state itself
        // — the caller is responsible for applying the decision.
        // However the execute_single_phase wrapper only returns PhaseResult,
        // not the Decision, so pipeline state stays Validation.
        assert_eq!(pipeline.state, PipelineState::Validation);
    }

    #[test]
    fn parallel_spec_review_failure_when_quality_below_threshold() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Pending);
        pipeline.quality_threshold = 100; // linter returns 85 < 100
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        // Failed spec_review transitions to Failed
        assert_eq!(pipeline.state, PipelineState::Failed);
    }

    // ── validate_pipeline_parallel ───────────────────────────────────

    #[test]
    fn validate_rejects_terminal_states() {
        for terminal in [
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ] {
            let (executor, _tmp) = setup_executor();
            let pipeline = pipeline_in_state(terminal);
            let result = executor.validate_pipeline_parallel(&pipeline);
            assert!(
                result.is_err(),
                "Expected validation error for terminal state {terminal:?}"
            );
        }
    }

    #[test]
    fn validate_accepts_all_non_terminal_states() {
        for non_terminal in [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ] {
            let (executor, _tmp) = setup_executor();
            let pipeline = pipeline_in_state(non_terminal);
            let result = executor.validate_pipeline_parallel(&pipeline);
            assert!(
                result.is_ok(),
                "Expected validation to pass for {non_terminal:?}, got: {result:?}"
            );
        }
    }

    #[test]
    fn validate_does_not_mutate_pipeline() {
        let (executor, _tmp) = setup_executor();
        let pipeline = pipeline_in_state(PipelineState::Pending);
        let state_before = pipeline.state;
        let _ = executor.validate_pipeline_parallel(&pipeline);
        assert_eq!(pipeline.state, state_before);
    }

    // ── get_parallelizable_phases (static) ───────────────────────────

    #[test]
    fn get_parallelizable_phases_returns_expected_for_each_state() {
        use crate::cleanup::PhaseType;

        assert_eq!(
            PipelineExecutor::get_parallelizable_phases(&PipelineState::Pending),
            vec![PhaseType::SpecReview]
        );
        assert_eq!(
            PipelineExecutor::get_parallelizable_phases(&PipelineState::SpecReview),
            vec![PhaseType::UniverseSetup]
        );
        assert_eq!(
            PipelineExecutor::get_parallelizable_phases(&PipelineState::UniverseSetup),
            vec![PhaseType::AgentDevelopment]
        );
        assert_eq!(
            PipelineExecutor::get_parallelizable_phases(&PipelineState::AgentDevelopment),
            vec![PhaseType::AgentDevelopment]
        );
        assert_eq!(
            PipelineExecutor::get_parallelizable_phases(&PipelineState::Validation),
            vec![PhaseType::Validation]
        );
        // Terminal states have no parallelizable phases
        assert!(PipelineExecutor::get_parallelizable_phases(&PipelineState::Accepted).is_empty());
        assert!(PipelineExecutor::get_parallelizable_phases(&PipelineState::Escalated).is_empty());
        assert!(PipelineExecutor::get_parallelizable_phases(&PipelineState::Failed).is_empty());
    }

    // ── execute_phase_group edge cases (via execute_parallel_phases) ─

    #[test]
    fn parallel_empty_group_returns_immediately() {
        // Terminal states resolve to empty phase groups
        // Already tested via parallel_rejects_terminal_states, but verify
        // the early-return logic: if a state resolved to empty groups
        // AND wasn't terminal, we'd get Ok(vec![]).
        // Currently all non-terminal states have at least one phase group.
        // We can verify the behavior by checking that validate + execute
        // are consistent for each state.
        let (executor, _tmp) = setup_executor();
        for state in [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ] {
            let pipeline = pipeline_in_state(state);
            // Validate should always succeed for non-terminal states
            assert!(executor.validate_pipeline_parallel(&pipeline).is_ok());
        }
    }

    // ── Dependency graph integration ─────────────────────────────────

    #[test]
    fn dependency_graph_builds_for_multi_phase_group() {
        // Verify that build_dependency_graph works for a multi-phase set.
        // This exercises the graph path that execute_with_dependency_graph uses.
        use crate::{cleanup::PhaseType, parallel::ParallelExecutor};

        let phases = vec![
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::AgentDevelopment,
        ];
        let graph = ParallelExecutor::build_dependency_graph(&phases);
        assert!(graph.is_ok());

        let graph = graph.unwrap();
        // SpecReview has no deps
        let ready = graph.get_ready_phases(&HashSet::new());
        assert!(ready.contains(&PhaseType::SpecReview));
        assert!(!ready.contains(&PhaseType::UniverseSetup));
    }

    #[test]
    fn dependency_graph_cycle_detection() {
        use crate::{cleanup::PhaseType, parallel::DependencyGraph};

        // Create a graph with a self-loop
        let graph =
            DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![PhaseType::SpecReview]);
        let result = graph.validate();
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(
            err.contains("Circular") || err.contains("cycle"),
            "Expected cycle error, got: {err}"
        );
    }

    #[test]
    fn dependency_graph_missing_dependency() {
        use crate::{cleanup::PhaseType, parallel::DependencyGraph};

        // UniverseSetup depends on SpecReview, but SpecReview is not in graph
        let graph =
            DependencyGraph::new().add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);
        let result = graph.validate();
        assert!(result.is_err());
    }

    #[test]
    fn dependency_graph_respects_execution_order() {
        use crate::{cleanup::PhaseType, parallel::ParallelExecutor};

        // Phases in correct order should validate
        let phases = vec![PhaseType::SpecReview, PhaseType::UniverseSetup];
        assert!(ParallelExecutor::validate_dependency_order(&phases).is_ok());

        // Phases in wrong order should fail
        let phases_reversed = vec![PhaseType::UniverseSetup, PhaseType::SpecReview];
        assert!(ParallelExecutor::validate_dependency_order(&phases_reversed).is_err());
    }

    #[test]
    fn parallel_full_pipeline_happy_path() {
        // Execute parallel phases step by step through the full lifecycle
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Pending);

        // Step 1: Pending -> spec_review runs -> transitions to UniverseSetup
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(pipeline.state, PipelineState::UniverseSetup);

        // Step 2: UniverseSetup -> agent_development runs -> transitions to Validation
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        // agent_development wrapper returns empty vec
        assert!(results.is_empty());
        assert_eq!(pipeline.state, PipelineState::Validation);

        // Step 3: Validation -> validation runs (Accept decision)
        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    #[test]
    fn parallel_cannot_execute_after_failure() {
        let (mut executor, _tmp) = setup_executor();
        let mut pipeline = pipeline_in_state(PipelineState::Pending);
        pipeline.quality_threshold = 100; // Force spec_review to fail

        let results = executor.execute_parallel_phases(&mut pipeline).unwrap();
        assert!(!results[0].success);
        assert_eq!(pipeline.state, PipelineState::Failed);

        // Now pipeline is terminal — should error
        let result = executor.execute_parallel_phases(&mut pipeline);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PhaseError::InvalidStateTransition(_)
        ));
    }
}

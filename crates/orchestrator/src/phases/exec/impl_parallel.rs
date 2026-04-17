//! Parallel phase execution support

use std::collections::HashSet;

use tracing::debug;

use crate::cleanup::PhaseType;
use crate::parallel::{DependencyGraph, ParallelExecutor, PhaseGroup};
use crate::state::{Pipeline, PipelineState};

use super::executor::PipelineExecutor;
use super::types::{PhaseError, PhaseResult};

impl PipelineExecutor {
    pub fn execute_parallel_phases(
        &mut self,
        pipeline: &mut Pipeline,
    ) -> Result<Vec<PhaseResult>, PhaseError> {
        if pipeline.state.is_terminal() {
            return Err(PhaseError::InvalidStateTransition(
                "Cannot execute parallel phases in terminal state".to_string(),
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
        let graph = ParallelExecutor::build_dependency_graph(phases)
            .map_err(|e| PhaseError::ParallelExecutionFailed(e.to_string()))?;

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
                        "No phases ready but graph incomplete with failures".to_string(),
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
                "Pipeline is in terminal state".to_string(),
            ));
        }

        let phase_groups = ParallelExecutor::resolve_parallel_phases(&pipeline.state);

        for group in phase_groups {
            if group.phases.len() > 1 {
                ParallelExecutor::build_dependency_graph(&group.phases)
                    .map_err(|e| PhaseError::ParallelExecutionFailed(e.to_string()))?;
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

//! Parallel phase execution with dependency resolution

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cleanup::PhaseType;

#[derive(Debug, Clone, Error)]
pub enum ParallelError {
    #[error("Dependency not met for phase: {0:?}")]
    DependencyNotMet(PhaseType),

    #[error("Invalid phase configuration: {0}")]
    InvalidPhaseConfiguration(String),

    #[error("Parallel execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PhaseNode {
    pub phase_type: PhaseType,
    pub dependencies: Vec<PhaseType>,
    pub status: PhaseStatus,
}

impl PhaseNode {
    #[must_use]
    pub fn new(phase_type: PhaseType) -> Self {
        Self {
            phase_type,
            dependencies: Vec::new(),
            status: PhaseStatus::Pending,
        }
    }

    #[must_use]
    pub fn with_dependency(mut self, dep: PhaseType) -> Self {
        self.dependencies.push(dep);
        self
    }

    #[must_use]
    pub fn can_execute(&self, completed: &HashSet<PhaseType>) -> bool {
        self.status == PhaseStatus::Pending
            && self.dependencies.iter().all(|d| completed.contains(d))
    }
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    nodes: HashMap<PhaseType, PhaseNode>,
}

impl DependencyGraph {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_phase(mut self, phase: PhaseType, dependencies: Vec<PhaseType>) -> Self {
        let node = PhaseNode {
            phase_type: phase,
            dependencies,
            status: PhaseStatus::Pending,
        };
        self.nodes.insert(phase, node);
        self
    }

    pub fn validate(&self) -> Result<(), ParallelError> {
        for node in self.nodes.values() {
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(ParallelError::InvalidPhaseConfiguration(format!(
                        "Phase {:?} depends on non-existent phase {:?}",
                        node.phase_type, dep
                    )));
                }
            }
        }

        self.detect_cycles()?;
        Ok(())
    }

    fn detect_cycles(&self) -> Result<(), ParallelError> {
        let mut visited: HashSet<PhaseType> = HashSet::new();
        let mut recursion_stack: HashSet<PhaseType> = HashSet::new();

        for phase in self.nodes.keys() {
            if !visited.contains(phase) && self.has_cycle(phase, &mut visited, &mut recursion_stack)
            {
                return Err(ParallelError::InvalidPhaseConfiguration(format!(
                    "Circular dependency detected involving phase {:?}",
                    phase
                )));
            }
        }
        Ok(())
    }

    fn has_cycle(
        &self,
        phase: &PhaseType,
        visited: &mut HashSet<PhaseType>,
        recursion_stack: &mut HashSet<PhaseType>,
    ) -> bool {
        visited.insert(*phase);
        recursion_stack.insert(*phase);

        if let Some(node) = self.nodes.get(phase) {
            for dep in &node.dependencies {
                if !visited.contains(dep) {
                    if self.has_cycle(dep, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(dep) {
                    return true;
                }
            }
        }

        recursion_stack.remove(phase);
        false
    }

    #[must_use]
    pub fn get_ready_phases(&self, completed: &HashSet<PhaseType>) -> Vec<PhaseType> {
        self.nodes
            .values()
            .filter(|node| node.can_execute(completed))
            .map(|node| node.phase_type)
            .collect()
    }

    pub fn mark_running(&mut self, phase: PhaseType) {
        if let Some(node) = self.nodes.get_mut(&phase) {
            node.status = PhaseStatus::Running;
        }
    }

    pub fn mark_completed(&mut self, phase: PhaseType) {
        if let Some(node) = self.nodes.get_mut(&phase) {
            node.status = PhaseStatus::Completed;
        }
    }

    pub fn mark_failed(&mut self, phase: PhaseType) {
        if let Some(node) = self.nodes.get_mut(&phase) {
            node.status = PhaseStatus::Failed;
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.nodes
            .values()
            .all(|n| matches!(n.status, PhaseStatus::Completed | PhaseStatus::Failed))
    }

    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.nodes.values().any(|n| n.status == PhaseStatus::Failed)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PhaseGroup {
    pub phases: Vec<PhaseType>,
    pub max_parallelism: usize,
}

impl PhaseGroup {
    #[must_use]
    pub fn new(phases: Vec<PhaseType>) -> Self {
        let max_parallelism = phases.len();
        Self {
            phases,
            max_parallelism,
        }
    }

    #[must_use]
    pub fn with_max_parallelism(mut self, max: usize) -> Self {
        self.max_parallelism = max;
        self
    }
}

pub struct ParallelExecutor;

impl ParallelExecutor {
    #[must_use]
    pub fn resolve_parallel_phases(
        pipeline_state: &crate::state::PipelineState,
    ) -> Vec<PhaseGroup> {
        match pipeline_state {
            crate::state::PipelineState::Pending => {
                vec![PhaseGroup::new(vec![PhaseType::SpecReview])]
            }
            crate::state::PipelineState::SpecReview => {
                vec![PhaseGroup::new(vec![PhaseType::UniverseSetup])]
            }
            crate::state::PipelineState::UniverseSetup => {
                vec![PhaseGroup::new(vec![PhaseType::AgentDevelopment])]
            }
            crate::state::PipelineState::AgentDevelopment => {
                vec![PhaseGroup::new(vec![PhaseType::AgentDevelopment])]
            }
            crate::state::PipelineState::Validation => {
                vec![PhaseGroup::new(vec![PhaseType::Validation])]
            }
            crate::state::PipelineState::Accepted
            | crate::state::PipelineState::Escalated
            | crate::state::PipelineState::Failed => {
                vec![]
            }
        }
    }

    pub fn build_dependency_graph(phases: &[PhaseType]) -> Result<DependencyGraph, ParallelError> {
        let mut graph = DependencyGraph::new();

        for &phase in phases {
            let deps = Self::get_dependencies(phase);
            graph = graph.add_phase(phase, deps);
        }

        graph.validate()?;
        Ok(graph)
    }

    #[must_use]
    fn get_dependencies(phase: PhaseType) -> Vec<PhaseType> {
        match phase {
            PhaseType::SpecReview => vec![],
            PhaseType::UniverseSetup => vec![PhaseType::SpecReview],
            PhaseType::AgentDevelopment => vec![PhaseType::UniverseSetup],
            PhaseType::Validation => vec![PhaseType::AgentDevelopment],
        }
    }

    pub fn validate_dependency_order(phases: &[PhaseType]) -> Result<(), ParallelError> {
        let graph = Self::build_dependency_graph(phases)?;
        let mut completed: HashSet<PhaseType> = HashSet::new();

        for phase in phases {
            let ready = graph.get_ready_phases(&completed);
            if !ready.contains(phase) {
                return Err(ParallelError::DependencyNotMet(*phase));
            }

            completed.insert(*phase);
        }

        Ok(())
    }
}

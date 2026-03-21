//! Tests for parallel phase execution

use std::collections::HashSet;

use crate::cleanup::PhaseType;
use crate::parallel::{DependencyGraph, ParallelExecutor, PhaseGroup};

#[test]
fn test_dependency_graph_empty() {
    let graph = DependencyGraph::new();
    assert!(graph.is_complete());
}

#[test]
fn test_dependency_graph_single_phase() {
    let graph = DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![]);

    let completed: HashSet<PhaseType> = HashSet::new();
    let ready = graph.get_ready_phases(&completed);

    assert_eq!(ready, vec![PhaseType::SpecReview]);
}

#[test]
fn test_dependency_graph_sequential() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview])
        .add_phase(PhaseType::AgentDevelopment, vec![PhaseType::UniverseSetup])
        .add_phase(PhaseType::Validation, vec![PhaseType::AgentDevelopment]);

    let mut completed: HashSet<PhaseType> = HashSet::new();

    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready, vec![PhaseType::SpecReview]);

    let mut graph = graph;
    graph.mark_completed(PhaseType::SpecReview);
    completed.insert(PhaseType::SpecReview);

    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready, vec![PhaseType::UniverseSetup]);
}

#[test]
fn test_parallel_phases_resolve() {
    let groups = ParallelExecutor::resolve_parallel_phases(&crate::state::PipelineState::Pending);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].phases, vec![PhaseType::SpecReview]);
}

#[test]
fn test_dependency_validation_sequential() {
    let phases = vec![PhaseType::SpecReview, PhaseType::UniverseSetup];

    assert!(ParallelExecutor::validate_dependency_order(&phases).is_ok());
}

#[test]
fn test_dependency_validation_invalid_order() {
    let phases = vec![PhaseType::UniverseSetup, PhaseType::SpecReview];

    assert!(ParallelExecutor::validate_dependency_order(&phases).is_err());
}

#[test]
fn test_resolve_parallel_phases_all_states() {
    use crate::state::PipelineState;

    assert_eq!(
        ParallelExecutor::resolve_parallel_phases(&PipelineState::Pending)[0].phases,
        vec![PhaseType::SpecReview]
    );
    assert_eq!(
        ParallelExecutor::resolve_parallel_phases(&PipelineState::SpecReview)[0].phases,
        vec![PhaseType::UniverseSetup]
    );
    assert_eq!(
        ParallelExecutor::resolve_parallel_phases(&PipelineState::UniverseSetup)[0].phases,
        vec![PhaseType::AgentDevelopment]
    );
    assert_eq!(
        ParallelExecutor::resolve_parallel_phases(&PipelineState::AgentDevelopment)[0].phases,
        vec![PhaseType::AgentDevelopment]
    );
    assert_eq!(
        ParallelExecutor::resolve_parallel_phases(&PipelineState::Validation)[0].phases,
        vec![PhaseType::Validation]
    );
    assert!(ParallelExecutor::resolve_parallel_phases(&PipelineState::Accepted).is_empty());
    assert!(ParallelExecutor::resolve_parallel_phases(&PipelineState::Escalated).is_empty());
    assert!(ParallelExecutor::resolve_parallel_phases(&PipelineState::Failed).is_empty());
}

#[test]
fn test_dependency_graph_is_complete() {
    let graph = DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![]);

    assert!(!graph.is_complete());

    let mut graph = graph;
    graph.mark_completed(PhaseType::SpecReview);

    assert!(graph.is_complete());
}

#[test]
fn test_dependency_graph_has_failures() {
    let mut graph = DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![]);

    assert!(!graph.has_failures());

    graph.mark_failed(PhaseType::SpecReview);

    assert!(graph.has_failures());
}

#[test]
fn test_phase_group_new() {
    let group = PhaseGroup::new(vec![PhaseType::SpecReview, PhaseType::UniverseSetup]);

    assert_eq!(group.phases.len(), 2);
    assert_eq!(group.max_parallelism, 2);
}

#[test]
fn test_phase_group_with_max_parallelism() {
    let group = PhaseGroup::new(vec![PhaseType::SpecReview, PhaseType::UniverseSetup])
        .with_max_parallelism(1);

    assert_eq!(group.max_parallelism, 1);
}

#[test]
fn test_build_dependency_graph_valid() {
    let phases = vec![PhaseType::SpecReview, PhaseType::UniverseSetup];
    let graph = ParallelExecutor::build_dependency_graph(&phases);

    assert!(graph.is_ok());
}

#[test]
fn test_build_dependency_graph_single() {
    let phases = vec![PhaseType::SpecReview];
    let graph = ParallelExecutor::build_dependency_graph(&phases);

    assert!(graph.is_ok());

    let graph = graph.unwrap();
    let completed: HashSet<PhaseType> = HashSet::new();
    let ready = graph.get_ready_phases(&completed);

    assert_eq!(ready, vec![PhaseType::SpecReview]);
}

#[test]
fn test_circular_dependency_detected() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![PhaseType::UniverseSetup])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);

    let result = graph.validate();
    assert!(result.is_err());
}

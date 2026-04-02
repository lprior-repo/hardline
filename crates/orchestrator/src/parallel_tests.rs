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

// --- PhaseNode tests ---

#[test]
fn test_phase_node_new() {
    let node = crate::parallel::PhaseNode::new(PhaseType::SpecReview);
    assert_eq!(node.phase_type, PhaseType::SpecReview);
    assert!(node.dependencies.is_empty());
    assert_eq!(node.status, crate::parallel::PhaseStatus::Pending);
}

#[test]
fn test_phase_node_with_dependency() {
    let node = crate::parallel::PhaseNode::new(PhaseType::UniverseSetup)
        .with_dependency(PhaseType::SpecReview);

    assert_eq!(node.dependencies.len(), 1);
    assert_eq!(node.dependencies[0], PhaseType::SpecReview);
}

#[test]
fn test_phase_node_can_execute() {
    let mut completed: HashSet<PhaseType> = HashSet::new();

    // No dependencies: can execute immediately
    let node = crate::parallel::PhaseNode::new(PhaseType::SpecReview);
    assert!(node.can_execute(&completed));

    // With unmet dependency: cannot execute
    let node = crate::parallel::PhaseNode::new(PhaseType::UniverseSetup)
        .with_dependency(PhaseType::SpecReview);
    assert!(!node.can_execute(&completed));

    // With met dependency: can execute
    completed.insert(PhaseType::SpecReview);
    assert!(node.can_execute(&completed));
}

// --- DependencyGraph status transitions ---

#[test]
fn test_dependency_graph_mark_running() {
    let mut graph = DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![]);
    graph.mark_running(PhaseType::SpecReview);

    // Should not be ready anymore (no longer Pending)
    let completed: HashSet<PhaseType> = HashSet::new();
    let ready = graph.get_ready_phases(&completed);
    assert!(ready.is_empty());
}

#[test]
fn test_dependency_graph_multiple_phases_ready() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::Validation, vec![]);

    let completed: HashSet<PhaseType> = HashSet::new();
    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready.len(), 2);
}

#[test]
fn test_dependency_graph_validate_nonexistent_dependency() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);
    // UniverseSetup depends on SpecReview which was not added
    let result = graph.validate();
    assert!(result.is_err());
}

#[test]
fn test_dependency_graph_validate_valid() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);
    assert!(graph.validate().is_ok());
}

#[test]
fn test_dependency_graph_default() {
    let graph = DependencyGraph::default();
    assert!(graph.is_complete());
    assert!(!graph.has_failures());
}

// --- PhaseStatus ---

#[test]
fn test_phase_status_variants() {
    use crate::parallel::PhaseStatus;

    let statuses = [
        PhaseStatus::Pending,
        PhaseStatus::Running,
        PhaseStatus::Completed,
        PhaseStatus::Failed,
    ];

    for status in &statuses {
        let _ = format!("{status:?}"); // Ensure Debug is available
    }
}

// --- ParallelError display ---

#[test]
fn test_parallel_error_display() {
    use crate::parallel::ParallelError;

    let err = ParallelError::DependencyNotMet(PhaseType::UniverseSetup);
    assert!(format!("{err}").contains("UniverseSetup"));

    let err = ParallelError::InvalidPhaseConfiguration("bad config".to_string());
    assert!(format!("{err}").contains("bad config"));

    let err = ParallelError::ExecutionFailed("oom".to_string());
    assert!(format!("{err}").contains("oom"));
}

// --- Serde roundtrips ---

#[test]
fn test_phase_status_serde_roundtrip_all_variants() {
    use crate::parallel::PhaseStatus;

    let statuses = [
        PhaseStatus::Pending,
        PhaseStatus::Running,
        PhaseStatus::Completed,
        PhaseStatus::Failed,
    ];
    for status in &statuses {
        let json = serde_json::to_string(status).expect("serialize");
        let deserialized: PhaseStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*status, deserialized);
    }
}

#[test]
fn test_phase_type_serde_in_parallel_context() {
    use crate::cleanup::PhaseType;
    let phases = [
        PhaseType::SpecReview,
        PhaseType::UniverseSetup,
        PhaseType::AgentDevelopment,
        PhaseType::Validation,
    ];
    for phase in &phases {
        let json = serde_json::to_string(phase).expect("serialize");
        let deserialized: PhaseType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*phase, deserialized);
    }
}

// --- DependencyGraph edge cases ---

#[test]
fn test_dependency_graph_with_all_phases() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview])
        .add_phase(PhaseType::AgentDevelopment, vec![PhaseType::UniverseSetup])
        .add_phase(PhaseType::Validation, vec![PhaseType::AgentDevelopment]);

    assert!(graph.validate().is_ok());

    let mut completed: HashSet<PhaseType> = HashSet::new();
    let mut graph = graph;

    // Process all phases in order
    let expected_order = [
        PhaseType::SpecReview,
        PhaseType::UniverseSetup,
        PhaseType::AgentDevelopment,
        PhaseType::Validation,
    ];

    for phase in &expected_order {
        let ready = graph.get_ready_phases(&completed);
        assert!(ready.contains(phase), "Expected {:?} to be ready", phase);
        graph.mark_completed(*phase);
        completed.insert(*phase);
    }

    assert!(graph.is_complete());
}

#[test]
fn test_dependency_graph_mark_running_then_completed() {
    let mut graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![]);

    let completed: HashSet<PhaseType> = HashSet::new();

    // Initially ready
    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready.len(), 1);

    // Mark running: no longer ready (not Pending)
    graph.mark_running(PhaseType::SpecReview);
    let ready = graph.get_ready_phases(&completed);
    assert!(ready.is_empty());

    // Still not complete
    assert!(!graph.is_complete());

    // Mark completed: now complete
    graph.mark_completed(PhaseType::SpecReview);
    assert!(graph.is_complete());
}

#[test]
fn test_dependency_graph_self_cycle_detected() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![PhaseType::SpecReview]);

    let result = graph.validate();
    assert!(result.is_err());
}

#[test]
fn test_dependency_graph_three_node_cycle() {
    // A -> B -> C -> A
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![PhaseType::Validation])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview])
        .add_phase(PhaseType::Validation, vec![PhaseType::UniverseSetup]);

    let result = graph.validate();
    assert!(result.is_err());
}

#[test]
fn test_dependency_graph_partial_completion() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);

    let mut completed: HashSet<PhaseType> = HashSet::new();
    let mut graph = graph;

    // Complete only SpecReview
    graph.mark_completed(PhaseType::SpecReview);
    completed.insert(PhaseType::SpecReview);

    // Not complete yet
    assert!(!graph.is_complete());

    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready, vec![PhaseType::UniverseSetup]);
}

#[test]
fn test_dependency_graph_mark_completed_for_nonexistent_phase() {
    let mut graph = DependencyGraph::new();
    // Marking a phase that doesn't exist should not panic
    graph.mark_completed(PhaseType::SpecReview);
    assert!(!graph.has_failures());
}

#[test]
fn test_dependency_graph_mark_failed_for_nonexistent_phase() {
    let mut graph = DependencyGraph::new();
    // Marking a phase that doesn't exist should not panic
    graph.mark_failed(PhaseType::SpecReview);
    assert!(!graph.has_failures()); // No node means no failure
}

// --- PhaseGroup edge cases ---

#[test]
fn test_phase_group_empty() {
    let group = PhaseGroup::new(vec![]);
    assert!(group.phases.is_empty());
    assert_eq!(group.max_parallelism, 0);
}

#[test]
fn test_phase_group_single_phase() {
    let group = PhaseGroup::new(vec![PhaseType::SpecReview]);
    assert_eq!(group.phases.len(), 1);
    assert_eq!(group.max_parallelism, 1);
}

#[test]
fn test_phase_group_with_max_parallelism_zero() {
    let group = PhaseGroup::new(vec![PhaseType::SpecReview, PhaseType::UniverseSetup])
        .with_max_parallelism(0);
    assert_eq!(group.max_parallelism, 0);
}

#[test]
fn test_phase_group_with_max_parallelism_greater_than_phases() {
    let group = PhaseGroup::new(vec![PhaseType::SpecReview])
        .with_max_parallelism(10);
    assert_eq!(group.max_parallelism, 10);
}

// --- ParallelExecutor::build_dependency_graph tests ---

#[test]
fn test_build_dependency_graph_empty_phases() {
    let graph = ParallelExecutor::build_dependency_graph(&[]);
    assert!(graph.is_ok());
    let graph = graph.unwrap();
    assert!(graph.is_complete());
}

#[test]
fn test_build_dependency_graph_all_phases() {
    let phases = vec![
        PhaseType::SpecReview,
        PhaseType::UniverseSetup,
        PhaseType::AgentDevelopment,
        PhaseType::Validation,
    ];
    let graph = ParallelExecutor::build_dependency_graph(&phases);
    assert!(graph.is_ok());
}

#[test]
fn test_build_dependency_graph_reversed_order_fails_validation() {
    let phases = vec![
        PhaseType::Validation,
        PhaseType::AgentDevelopment,
        PhaseType::UniverseSetup,
        PhaseType::SpecReview,
    ];
    // The graph itself is built with correct deps (Validation -> AgentDevelopment, etc.)
    // But the phases are in wrong order for validate_dependency_order
    let result = ParallelExecutor::validate_dependency_order(&phases);
    assert!(result.is_err());
}

#[test]
fn test_validate_dependency_order_empty() {
    let result = ParallelExecutor::validate_dependency_order(&[]);
    assert!(result.is_ok());
}

#[test]
fn test_validate_dependency_order_single_phase() {
    let result = ParallelExecutor::validate_dependency_order(&[PhaseType::SpecReview]);
    assert!(result.is_ok());
}

// --- PhaseNode edge cases ---

#[test]
fn test_phase_node_can_execute_with_multiple_dependencies() {
    let mut completed: HashSet<PhaseType> = HashSet::new();

    let node = crate::parallel::PhaseNode::new(PhaseType::Validation)
        .with_dependency(PhaseType::SpecReview)
        .with_dependency(PhaseType::AgentDevelopment);

    // None met
    assert!(!node.can_execute(&completed));

    // Only one met
    completed.insert(PhaseType::SpecReview);
    assert!(!node.can_execute(&completed));

    // Both met
    completed.insert(PhaseType::AgentDevelopment);
    assert!(node.can_execute(&completed));
}

#[test]
fn test_phase_node_can_execute_non_pending_always_false() {
    let mut node = crate::parallel::PhaseNode::new(PhaseType::SpecReview);
    let completed: HashSet<PhaseType> = HashSet::new();

    assert!(node.can_execute(&completed));

    // If the node is not Pending, can_execute should be false regardless of deps
    node.status = crate::parallel::PhaseStatus::Running;
    assert!(!node.can_execute(&completed));

    node.status = crate::parallel::PhaseStatus::Completed;
    assert!(!node.can_execute(&completed));

    node.status = crate::parallel::PhaseStatus::Failed;
    assert!(!node.can_execute(&completed));
}

// --- DependencyGraph::get_ready_phases filters correctly ---

#[test]
fn test_get_ready_phases_excludes_completed_phases() {
    let graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![]);

    let completed: HashSet<PhaseType> = HashSet::new();
    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready.len(), 2);

    // Even though completed set doesn't contain them, get_ready_phases
    // also checks the node's status (Pending)
    let mut graph = graph;
    graph.mark_completed(PhaseType::SpecReview);

    // Only UniverseSetup is Pending now
    let ready = graph.get_ready_phases(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0], PhaseType::UniverseSetup);
}

// --- is_complete with mixed states ---

#[test]
fn test_is_complete_with_all_completed() {
    let mut graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![]);
    graph.mark_completed(PhaseType::SpecReview);
    graph.mark_completed(PhaseType::UniverseSetup);
    assert!(graph.is_complete());
    assert!(!graph.has_failures());
}

#[test]
fn test_is_complete_with_all_failed() {
    let mut graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![]);
    graph.mark_failed(PhaseType::SpecReview);
    graph.mark_failed(PhaseType::UniverseSetup);
    assert!(graph.is_complete());
    assert!(graph.has_failures());
}

#[test]
fn test_is_complete_with_mixed_completed_and_failed() {
    let mut graph = DependencyGraph::new()
        .add_phase(PhaseType::SpecReview, vec![])
        .add_phase(PhaseType::UniverseSetup, vec![]);
    graph.mark_completed(PhaseType::SpecReview);
    graph.mark_failed(PhaseType::UniverseSetup);
    assert!(graph.is_complete());
    assert!(graph.has_failures());
}

// --- ParallelError Error trait ---

#[test]
fn test_parallel_error_implements_error() {
    use std::error::Error;
    use crate::parallel::ParallelError;
    let err = ParallelError::DependencyNotMet(PhaseType::Validation);
    assert!(err.source().is_none());

    let err = ParallelError::InvalidPhaseConfiguration("bad".to_string());
    assert!(err.source().is_none());

    let err = ParallelError::ExecutionFailed("oom".to_string());
    assert!(err.source().is_none());
}

// --- Proptests for state transition invariants ---

use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

proptest! {
    #[test]
    fn prop_dependency_graph_empty_is_always_complete(
        _seed in 0u32..100u32
    ) {
        let graph = DependencyGraph::new();
        assert!(graph.is_complete());
        assert!(!graph.has_failures());
    }

    #[test]
    fn prop_phase_group_max_parallelism_matches_phases_when_default(
        count in 1u32..10u32,
    ) {
        let phases: Vec<PhaseType> = (0..count).map(|i| match i % 4 {
            0 => PhaseType::SpecReview,
            1 => PhaseType::UniverseSetup,
            2 => PhaseType::AgentDevelopment,
            _ => PhaseType::Validation,
        }).collect();
        let group = PhaseGroup::new(phases.clone());
        prop_assert_eq!(group.max_parallelism, phases.len());
    }

    #[test]
    fn prop_get_ready_phases_never_includes_non_pending(count in 1u32..20u32) {
        let mut graph = DependencyGraph::new();
        let mut phases_added = Vec::new();
        for i in 0..count {
            let phase = match i % 4 {
                0 => PhaseType::SpecReview,
                1 => PhaseType::UniverseSetup,
                2 => PhaseType::AgentDevelopment,
                _ => PhaseType::Validation,
            };
            graph = graph.add_phase(phase, vec![]);
            phases_added.push(phase);
        }

        // Mark all as completed
        for phase in &phases_added {
            graph.mark_completed(*phase);
        }

        let completed: HashSet<PhaseType> = HashSet::new();
        let ready = graph.get_ready_phases(&completed);
        // No phase should be ready since all are Completed
        prop_assert!(ready.is_empty());
    }

    #[test]
    fn prop_is_complete_after_all_marked_completed_or_failed(
        count in 1u32..10u32
    ) {
        let mut graph = DependencyGraph::new();
        let mut phases_added = Vec::new();
        for i in 0..count {
            let phase = match i % 4 {
                0 => PhaseType::SpecReview,
                1 => PhaseType::UniverseSetup,
                2 => PhaseType::AgentDevelopment,
                _ => PhaseType::Validation,
            };
            graph = graph.add_phase(phase, vec![]);
            phases_added.push(phase);
        }

        for (i, phase) in phases_added.iter().enumerate() {
            if i % 2 == 0 {
                graph.mark_completed(*phase);
            } else {
                graph.mark_failed(*phase);
            }
        }

        prop_assert!(graph.is_complete());
    }
}

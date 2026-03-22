//! Tests for workflow module

use chrono::Utc;

use super::{
    CompensationState, JournalState, OperationRecord, OperationState, Pipeline, PipelineConfig,
    PipelineId, PipelineState, RecoveryReport, StepCompletedEvent, StepRecord, StepStatus,
    WorkflowEvent,
};

#[test]
fn test_operation_state_is_terminal() {
    assert!(!OperationState::Started.is_terminal());
    assert!(!OperationState::InProgress.is_terminal());
    assert!(OperationState::Completed.is_terminal());
    assert!(OperationState::Failed.is_terminal());
}

#[test]
fn test_operation_state_serialization() {
    assert_eq!(OperationState::Started.as_str(), "started");
    assert_eq!(OperationState::InProgress.as_str(), "in_progress");
    assert_eq!(OperationState::Completed.as_str(), "completed");
    assert_eq!(OperationState::Failed.as_str(), "failed");

    assert_eq!(
        OperationState::from_str("started"),
        Some(OperationState::Started)
    );
    assert_eq!(
        OperationState::from_str("in_progress"),
        Some(OperationState::InProgress)
    );
    assert_eq!(
        OperationState::from_str("completed"),
        Some(OperationState::Completed)
    );
    assert_eq!(
        OperationState::from_str("failed"),
        Some(OperationState::Failed)
    );
    assert_eq!(OperationState::from_str("invalid"), None);
}

#[test]
fn test_step_status_is_terminal() {
    assert!(!StepStatus::Pending.is_terminal());
    assert!(!StepStatus::Running.is_terminal());
    assert!(StepStatus::Completed.is_terminal());
    assert!(StepStatus::Failed.is_terminal());
    assert!(StepStatus::Skipped.is_terminal());
}

#[test]
fn test_step_record_lifecycle() {
    let mut step = StepRecord::new("op-123".to_string(), 0, "create-db-record".to_string());

    assert_eq!(step.status, StepStatus::Pending);
    assert!(step.started_at.is_none());

    step.start();
    assert_eq!(step.status, StepStatus::Running);
    assert!(step.started_at.is_some());

    step.complete();
    assert_eq!(step.status, StepStatus::Completed);
    assert!(step.completed_at.is_some());
    assert!(step.duration_ms().is_some());
}

#[test]
fn test_step_record_fail() {
    let mut step = StepRecord::new("op-123".to_string(), 0, "create-workspace".to_string());

    step.start();
    step.fail("Directory already exists".to_string());

    assert_eq!(step.status, StepStatus::Failed);
    assert_eq!(
        step.error_message,
        Some("Directory already exists".to_string())
    );
}

#[test]
fn test_operation_record_progress() {
    let mut op = OperationRecord::new(
        "op-123".to_string(),
        "author-1".to_string(),
        "Test operation".to_string(),
        5,
    );

    assert_eq!(op.progress_percentage(), 0.0);

    op.advance_step();
    assert_eq!(op.progress_percentage(), 20.0);

    op.advance_step();
    assert_eq!(op.progress_percentage(), 40.0);

    op.complete();
    assert!(op.completed_at.is_some());
    assert!(op.state.is_terminal());
}

#[test]
fn test_pipeline_valid_transitions() {
    let mut pipeline = Pipeline::new("specs/test.yaml".to_string());

    assert!(pipeline.transition_to(PipelineState::SpecReview).is_ok());
    assert!(pipeline.transition_to(PipelineState::UniverseSetup).is_ok());
    assert!(pipeline
        .transition_to(PipelineState::AgentDevelopment)
        .is_ok());
    assert!(pipeline.transition_to(PipelineState::Validation).is_ok());
    assert!(pipeline.transition_to(PipelineState::Accepted).is_ok());
}

#[test]
fn test_pipeline_invalid_transition() {
    let mut pipeline = Pipeline::new("specs/test.yaml".to_string());

    let result = pipeline.transition_to(PipelineState::Validation);
    assert!(result.is_err());
}

#[test]
fn test_pipeline_terminal_state_no_transition() {
    let mut pipeline = Pipeline::new("specs/test.yaml".to_string());

    pipeline.transition_to(PipelineState::SpecReview).ok();
    pipeline.transition_to(PipelineState::UniverseSetup).ok();
    pipeline.transition_to(PipelineState::AgentDevelopment).ok();
    pipeline.transition_to(PipelineState::Validation).ok();
    pipeline.transition_to(PipelineState::Accepted).ok();

    assert!(pipeline.state.is_terminal());

    let result = pipeline.transition_to(PipelineState::Failed);
    assert!(result.is_err());
}

#[test]
fn test_pipeline_iteration_limit() {
    let mut pipeline = Pipeline::new("specs/test.yaml".to_string());
    pipeline.transition_to(PipelineState::SpecReview).ok();
    pipeline.transition_to(PipelineState::UniverseSetup).ok();
    pipeline.transition_to(PipelineState::AgentDevelopment).ok();

    for _ in 0..10 {
        assert!(pipeline.increment_iteration().is_ok());
    }

    assert!(pipeline.increment_iteration().is_err());
}

#[test]
fn test_pipeline_can_iterate() {
    let mut pipeline = Pipeline::new("specs/test.yaml".to_string());

    assert!(!pipeline.can_iterate());

    pipeline.transition_to(PipelineState::SpecReview).ok();
    pipeline.transition_to(PipelineState::UniverseSetup).ok();
    pipeline.transition_to(PipelineState::AgentDevelopment).ok();

    assert!(pipeline.can_iterate());

    for _ in 0..10 {
        pipeline.increment_iteration().ok();
    }

    assert!(!pipeline.can_iterate());
}

#[test]
fn test_compensation_state_is_terminal() {
    assert!(!CompensationState::CompensationInProgress.is_terminal());
    assert!(CompensationState::NoCompensationNeeded.is_terminal());
    assert!(CompensationState::CompensationCompleted.is_terminal());
    assert!(CompensationState::CompensationFailed.is_terminal());
}

#[test]
fn test_recovery_report() {
    let mut report = RecoveryReport::new();

    report.add_recovered("op-1".to_string());
    report.add_recovered("op-2".to_string());
    report.add_failed("op-3".to_string(), "Database locked".to_string());

    assert_eq!(report.total_incomplete, 3);
    assert_eq!(report.recovered, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(report.recovered_operations.len(), 2);
    assert_eq!(report.failed_operations.len(), 1);
}

#[test]
fn test_journal_state_serialization() {
    assert_eq!(JournalState::PendingExternal.as_str(), "pending_external");
    assert_eq!(JournalState::Compensating.as_str(), "compensating");
    assert_eq!(JournalState::Done.as_str(), "done");
    assert_eq!(
        JournalState::FailedCompensation.as_str(),
        "failed_compensation"
    );

    assert_eq!(
        JournalState::from_str("pending_external"),
        Some(JournalState::PendingExternal)
    );
    assert_eq!(
        JournalState::from_str("compensating"),
        Some(JournalState::Compensating)
    );
    assert_eq!(JournalState::from_str("done"), Some(JournalState::Done));
    assert_eq!(
        JournalState::from_str("failed_compensation"),
        Some(JournalState::FailedCompensation)
    );
}

#[test]
fn test_pipeline_id() {
    let id1 = PipelineId::new();
    let id2 = PipelineId::new();

    assert_ne!(id1, id2);
    assert!(id1.to_string().starts_with("PipelineId("));
}

#[test]
fn test_pipeline_with_config() {
    let config = PipelineConfig {
        max_iterations: 5,
        quality_threshold: 90,
        scenarios_path: "test-scenarios".to_string(),
        linter_path: Some("/usr/bin/linter".to_string()),
    };

    let pipeline = Pipeline::with_config("spec.yaml".to_string(), &config);

    assert_eq!(pipeline.max_iterations, 5);
    assert_eq!(pipeline.quality_threshold, 90);
}

#[test]
fn test_workflow_events() {
    let event = WorkflowEvent::StepCompleted(StepCompletedEvent {
        operation_id: "op-123".to_string(),
        step_index: 2,
        step_name: "create-workspace".to_string(),
        duration_ms: Some(150),
        timestamp: Utc::now(),
    });

    assert!(matches!(event, WorkflowEvent::StepCompleted(_)));
}

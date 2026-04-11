//! Saga pattern executor for durable workflow execution.
//!
//! Implements the orchestration layer that ties together the domain types
//! (states, records, events) into a runnable saga with crash recovery.
//!
//! ## Architecture (Data → Calc → Actions)
//!
//! - **Data**: `SagaDefinition`, `SagaStep`, `SagaResult` — pure domain types
//! - **Calc**: `SagaJournal` — pure journal replay and compensation ordering
//! - **Actions**: `SagaExecutor` — the async runner that executes steps

use serde::{Deserialize, Serialize};

use super::records::{CompensationAction, OperationRecord, RecoveryReport, StepRecord};
use super::states::{CompensationState, OperationState, StepStatus};

// =============================================================================
// Data: Saga Definition
// =============================================================================

/// A single step in a saga definition, including its compensation action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SagaStep {
    /// Human-readable step name
    pub name: String,
    /// Identifier for the compensation function to invoke on rollback
    pub compensate_fn: String,
}

impl SagaStep {
    /// Create a new saga step.
    #[must_use]
    pub fn new(name: impl Into<String>, compensate_fn: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            compensate_fn: compensate_fn.into(),
        }
    }
}

/// Definition of a saga — an ordered list of steps with compensation actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SagaDefinition {
    /// Unique identifier for this saga type
    pub saga_type: String,
    /// Ordered list of steps to execute
    pub steps: Vec<SagaStep>,
}

impl SagaDefinition {
    /// Create a new saga definition with the given type name.
    #[must_use]
    pub fn new(saga_type: impl Into<String>) -> Self {
        Self {
            saga_type: saga_type.into(),
            steps: Vec::new(),
        }
    }

    /// Add a step to the saga definition.
    #[must_use]
    pub fn with_step(mut self, step: SagaStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Number of steps in the saga.
    #[must_use]
    pub fn step_count(&self) -> u32 {
        u32::try_from(self.steps.len()).unwrap_or(u32::MAX)
    }

    /// Get a step by index.
    #[must_use]
    pub fn get_step(&self, index: u32) -> Option<&SagaStep> {
        self.steps.get(usize::try_from(index).unwrap_or(0))
    }
}

// =============================================================================
// Data: Saga Result
// =============================================================================

/// Outcome of a saga execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SagaResult {
    /// The operation ID of this saga run
    pub operation_id: String,
    /// Final state of the operation
    pub state: OperationState,
    /// Final compensation state
    pub compensation_state: CompensationState,
    /// Steps that completed successfully
    pub completed_steps: Vec<StepRecord>,
    /// Steps that were compensated
    pub compensated_steps: Vec<CompensationAction>,
    /// Steps that failed during compensation
    pub failed_compensations: Vec<CompensationAction>,
    /// Error message if the saga failed
    pub error: Option<String>,
}

impl SagaResult {
    /// Create a successful saga result.
    #[must_use]
    pub const fn success(operation_id: String, completed_steps: Vec<StepRecord>) -> Self {
        Self {
            operation_id,
            state: OperationState::Completed,
            compensation_state: CompensationState::NoCompensationNeeded,
            completed_steps,
            compensated_steps: Vec::new(),
            failed_compensations: Vec::new(),
            error: None,
        }
    }

    /// Create a failed saga result (after compensation).
    #[must_use]
    pub const fn failed(
        operation_id: String,
        error: String,
        completed_steps: Vec<StepRecord>,
        compensated_steps: Vec<CompensationAction>,
        failed_compensations: Vec<CompensationAction>,
    ) -> Self {
        let compensation_state = if failed_compensations.is_empty() {
            CompensationState::CompensationCompleted
        } else {
            CompensationState::CompensationFailed
        };

        Self {
            operation_id,
            state: OperationState::Failed,
            compensation_state,
            completed_steps,
            compensated_steps,
            failed_compensations,
            error: Some(error),
        }
    }

    /// Was the saga successful?
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.state == OperationState::Completed
    }

    /// Were all compensations successful?
    #[must_use]
    pub fn compensation_succeeded(&self) -> bool {
        self.compensation_state == CompensationState::CompensationCompleted
            || self.compensation_state == CompensationState::NoCompensationNeeded
    }
}

// =============================================================================
// Calc: Saga Journal (pure computation — no IO)
// =============================================================================

/// The saga journal tracks step completion for crash recovery.
///
/// This is the "source of truth" for which steps have completed.
/// On restart, the journal is replayed to determine where to resume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SagaJournal {
    /// The operation record being tracked
    pub operation: OperationRecord,
    /// Ordered records of each step
    pub step_records: Vec<StepRecord>,
    /// Compensation actions for completed steps
    pub compensation_actions: Vec<CompensationAction>,
}

impl SagaJournal {
    /// Create a new journal for a saga definition.
    #[must_use]
    pub fn new(
        operation_id: String,
        author_id: String,
        description: String,
        definition: &SagaDefinition,
    ) -> Self {
        let total_steps = definition.step_count();
        let operation = OperationRecord::new(operation_id, author_id, description, total_steps);

        let step_records = definition
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                StepRecord::new(
                    operation.operation_id.clone(),
                    u32::try_from(i).unwrap_or(0),
                    step.name.clone(),
                )
            })
            .collect();

        Self {
            operation,
            step_records,
            compensation_actions: Vec::new(),
        }
    }

    /// Restore a journal from persisted state (crash recovery).
    #[must_use]
    pub const fn restore(
        operation: OperationRecord,
        step_records: Vec<StepRecord>,
        compensation_actions: Vec<CompensationAction>,
    ) -> Self {
        Self {
            operation,
            step_records,
            compensation_actions,
        }
    }

    /// Find the next step to execute (first non-terminal step).
    #[must_use]
    pub fn next_step_index(&self) -> Option<u32> {
        self.step_records
            .iter()
            .find(|s| !s.status.is_terminal())
            .map(|s| s.step_index)
    }

    /// Mark a step as started.
    pub fn start_step(&mut self, index: u32) {
        if let Some(step) = self
            .step_records
            .get_mut(usize::try_from(index).unwrap_or(0))
        {
            step.start();
        }
    }

    /// Mark a step as completed and record its compensation action.
    pub fn complete_step(&mut self, index: u32, compensate_fn: String) {
        if let Some(step) = self
            .step_records
            .get_mut(usize::try_from(index).unwrap_or(0))
        {
            step.complete();
            self.operation.advance_step();
        }

        let step_name = self
            .step_records
            .get(usize::try_from(index).unwrap_or(0))
            .map_or_else(|| "unknown".to_string(), |s| s.step_name.clone());

        self.compensation_actions.push(CompensationAction::new(
            index,
            step_name,
            compensate_fn,
        ));
    }

    /// Mark a step as failed.
    pub fn fail_step(&mut self, index: u32, error: String) {
        if let Some(step) = self
            .step_records
            .get_mut(usize::try_from(index).unwrap_or(0))
        {
            step.fail(error);
        }
    }

    /// Get completed compensation actions in reverse order for rollback.
    #[must_use]
    pub fn compensation_actions_reversed(&self) -> Vec<&CompensationAction> {
        self.compensation_actions.iter().rev().collect()
    }

    /// Mark a compensation action as compensated.
    pub fn mark_compensated(&mut self, step_index: u32) {
        if let Some(action) = self
            .compensation_actions
            .iter_mut()
            .find(|a| a.step_index == step_index)
        {
            action.mark_compensated();
        }
    }

    /// Mark a compensation action as failed.
    pub fn mark_compensation_failed(&mut self, step_index: u32, error: String) {
        if let Some(action) = self
            .compensation_actions
            .iter_mut()
            .find(|a| a.step_index == step_index)
        {
            action.mark_failed(error);
        }
    }

    /// Generate a recovery report from the journal state.
    #[must_use]
    pub fn recovery_status(&self) -> RecoveryReport {
        let incomplete: Vec<&StepRecord> = self
            .step_records
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Running))
            .collect();

        let mut report = RecoveryReport::new();
        for step in &incomplete {
            // Running steps at crash time are treated as incomplete
            report.add_recovered(format!(
                "{}:step-{}",
                self.operation.operation_id, step.step_index
            ));
        }
        report
    }
}

// =============================================================================
// Actions: Saga Executor
// =============================================================================

/// Trait for executing a single saga step.
///
/// Implementations provide the actual side-effectful logic for each step.
/// The executor calls these during saga execution and compensation.
pub trait StepExecutor: Send + Sync {
    /// Execute a step by name. Returns Ok(()) on success.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` with a description of what went wrong.
    fn execute_step(&self, step_name: &str) -> Result<(), String>;

    /// Execute a compensation action by function name. Returns Ok(()) on success.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` with a description of what went wrong.
    fn compensate(&self, compensate_fn: &str) -> Result<(), String>;
}

/// The saga executor orchestrates step execution with compensation.
///
/// This is the "Actions" layer — it performs IO via the `StepExecutor` trait.
/// The journal tracks progress for crash recovery.
pub struct SagaExecutor<E: StepExecutor> {
    executor: E,
}

impl<E: StepExecutor> SagaExecutor<E> {
    /// Create a new saga executor with the given step executor.
    #[must_use]
    pub const fn new(executor: E) -> Self {
        Self { executor }
    }

    /// Execute a saga from a definition, creating a fresh journal.
    pub fn execute(
        &self,
        operation_id: String,
        author_id: String,
        description: String,
        definition: &SagaDefinition,
    ) -> SagaResult {
        let mut journal = SagaJournal::new(operation_id, author_id, description, definition);
        journal.operation.state = OperationState::InProgress;

        self.run_remaining(definition, &mut journal)
    }

    /// Resume a saga from an existing journal (crash recovery).
    pub fn resume(&self, definition: &SagaDefinition, journal: &mut SagaJournal) -> SagaResult {
        journal.operation.state = OperationState::InProgress;

        // Mark any steps that were "Running" at crash time as Failed
        for step in &mut journal.step_records {
            if step.status == StepStatus::Running {
                step.fail("Crash during execution".to_string());
            }
        }

        self.run_remaining(definition, journal)
    }

    /// Run all remaining steps in the journal.
    fn run_remaining(&self, definition: &SagaDefinition, journal: &mut SagaJournal) -> SagaResult {
        while let Some(step_index) = journal.next_step_index() {
            let Some(step) = definition.get_step(step_index) else {
                let error = format!("Step index {step_index} out of bounds");
                journal.operation.fail(error.clone());
                return SagaResult::failed(
                    journal.operation.operation_id.clone(),
                    error,
                    journal_completed_steps(journal),
                    Vec::new(),
                    Vec::new(),
                );
            };

            journal.start_step(step_index);

            match self.executor.execute_step(&step.name) {
                Ok(()) => {
                    journal.complete_step(step_index, step.compensate_fn.clone());
                }
                Err(error) => {
                    journal.fail_step(step_index, error.clone());
                    journal.operation.fail(error);

                    // Run compensation for all completed steps
                    return self.compensate(journal);
                }
            }
        }

        // All steps completed successfully
        journal.operation.complete();

        SagaResult::success(
            journal.operation.operation_id.clone(),
            journal_completed_steps(journal),
        )
    }

    /// Run compensation in reverse order for all completed steps.
    fn compensate(&self, journal: &mut SagaJournal) -> SagaResult {
        let actions = journal.compensation_actions_reversed();
        let action_data: Vec<(u32, String)> = actions
            .iter()
            .map(|a| (a.step_index, a.compensate_fn.clone()))
            .collect();

        let mut compensated = Vec::new();
        let mut failed = Vec::new();

        for (step_index, compensate_fn) in action_data {
            match self.executor.compensate(&compensate_fn) {
                Ok(()) => {
                    journal.mark_compensated(step_index);
                    if let Some(action) = journal
                        .compensation_actions
                        .iter()
                        .find(|a| a.step_index == step_index)
                    {
                        compensated.push(action.clone());
                    }
                }
                Err(err) => {
                    journal.mark_compensation_failed(step_index, err);
                    if let Some(action) = journal
                        .compensation_actions
                        .iter()
                        .find(|a| a.step_index == step_index)
                    {
                        failed.push(action.clone());
                    }
                    // Continue compensating remaining steps even after failure
                }
            }
        }

        let original_error = journal
            .operation
            .error_message
            .clone()
            .unwrap_or_default();

        SagaResult::failed(
            journal.operation.operation_id.clone(),
            original_error,
            journal_completed_steps(journal),
            compensated,
            failed,
        )
    }
}

/// Extract completed step records from the journal.
fn journal_completed_steps(journal: &SagaJournal) -> Vec<StepRecord> {
    journal
        .step_records
        .iter()
        .filter(|s| s.status == StepStatus::Completed)
        .cloned()
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SagaStep tests
    // =========================================================================

    #[test]
    fn saga_step_new_creates_step() {
        let step = SagaStep::new("create-workspace", "delete-workspace");
        assert_eq!(step.name, "create-workspace");
        assert_eq!(step.compensate_fn, "delete-workspace");
    }

    // =========================================================================
    // SagaDefinition tests
    // =========================================================================

    #[test]
    fn saga_definition_new_creates_empty() {
        let def = SagaDefinition::new("workspace-setup");
        assert_eq!(def.saga_type, "workspace-setup");
        assert!(def.steps.is_empty());
        assert_eq!(def.step_count(), 0);
    }

    #[test]
    fn saga_definition_with_step_adds_steps() {
        let def = SagaDefinition::new("workspace-setup")
            .with_step(SagaStep::new("create-dir", "remove-dir"))
            .with_step(SagaStep::new("init-git", "remove-git"))
            .with_step(SagaStep::new("create-branch", "delete-branch"));

        assert_eq!(def.step_count(), 3);
        assert_eq!(def.get_step(0).unwrap().name, "create-dir");
        assert_eq!(def.get_step(1).unwrap().name, "init-git");
        assert_eq!(def.get_step(2).unwrap().name, "create-branch");
    }

    #[test]
    fn saga_definition_get_step_out_of_bounds() {
        let def = SagaDefinition::new("test").with_step(SagaStep::new("a", "undo-a"));
        assert!(def.get_step(1).is_none());
    }

    // =========================================================================
    // SagaResult tests
    // =========================================================================

    #[test]
    fn saga_result_success() {
        let result = SagaResult::success("op-1".to_string(), Vec::new());
        assert!(result.is_success());
        assert!(result.compensation_succeeded());
        assert!(result.error.is_none());
    }

    #[test]
    fn saga_result_failed_with_compensation() {
        let result = SagaResult::failed(
            "op-1".to_string(),
            "boom".to_string(),
            Vec::new(),
            vec![CompensationAction::new(0, "a".to_string(), "undo-a".to_string())],
            Vec::new(),
        );
        assert!(!result.is_success());
        assert!(result.compensation_succeeded());
        assert_eq!(result.error, Some("boom".to_string()));
    }

    #[test]
    fn saga_result_failed_with_failed_compensation() {
        let result = SagaResult::failed(
            "op-1".to_string(),
            "boom".to_string(),
            Vec::new(),
            Vec::new(),
            vec![CompensationAction::new(0, "a".to_string(), "undo-a".to_string())],
        );
        assert!(!result.is_success());
        assert!(!result.compensation_succeeded());
    }

    // =========================================================================
    // SagaJournal tests
    // =========================================================================

    #[test]
    fn journal_new_creates_pending_steps() {
        let def = test_definition();
        let journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        assert_eq!(journal.step_records.len(), 3);
        assert_eq!(journal.operation.state, OperationState::Started);
        assert_eq!(journal.operation.current_step, 0);
        for step in &journal.step_records {
            assert_eq!(step.status, StepStatus::Pending);
        }
    }

    #[test]
    fn journal_next_step_returns_first_pending() {
        let def = test_definition();
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        assert_eq!(journal.next_step_index(), Some(0));

        journal.start_step(0);
        journal.complete_step(0, "undo-a".to_string());

        assert_eq!(journal.next_step_index(), Some(1));
    }

    #[test]
    fn journal_next_step_returns_none_when_all_done() {
        let def = test_definition();
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        for i in 0..3 {
            journal.start_step(i);
            journal.complete_step(i, format!("undo-{i}"));
        }

        assert_eq!(journal.next_step_index(), None);
    }

    #[test]
    fn journal_complete_step_records_compensation() {
        let def = test_definition();
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        journal.start_step(0);
        journal.complete_step(0, "undo-a".to_string());

        assert_eq!(journal.compensation_actions.len(), 1);
        assert_eq!(journal.compensation_actions[0].step_index, 0);
        assert_eq!(journal.compensation_actions[0].compensate_fn, "undo-a");
        assert_eq!(journal.operation.current_step, 1);
    }

    #[test]
    fn journal_compensation_actions_reversed() {
        let def = test_definition();
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        for i in 0..3 {
            journal.start_step(i);
            journal.complete_step(i, format!("undo-{i}"));
        }

        let reversed = journal.compensation_actions_reversed();
        assert_eq!(reversed.len(), 3);
        assert_eq!(reversed[0].step_index, 2);
        assert_eq!(reversed[1].step_index, 1);
        assert_eq!(reversed[2].step_index, 0);
    }

    #[test]
    fn journal_recovery_status_reports_running_steps() {
        let def = test_definition();
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        // Step 0 completed, step 1 running (crash point)
        journal.start_step(0);
        journal.complete_step(0, "undo-a".to_string());
        journal.start_step(1);

        let report = journal.recovery_status();
        assert_eq!(report.total_incomplete, 1);
        assert_eq!(report.recovered, 1);
    }

    // =========================================================================
    // Mock StepExecutor for integration tests
    // =========================================================================

    struct MockExecutor {
        results: std::collections::HashMap<String, Result<(), String>>,
        compensate_results: std::collections::HashMap<String, Result<(), String>>,
        executed: std::sync::Mutex<Vec<String>>,
        compensated: std::sync::Mutex<Vec<String>>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                results: std::collections::HashMap::new(),
                compensate_results: std::collections::HashMap::new(),
                executed: std::sync::Mutex::new(Vec::new()),
                compensated: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_step_ok(mut self, name: &str) -> Self {
            self.results.insert(name.to_string(), Ok(()));
            self
        }

        fn with_step_fail(mut self, name: &str, error: &str) -> Self {
            self.results
                .insert(name.to_string(), Err(error.to_string()));
            self
        }

        fn with_compensate_ok(mut self, name: &str) -> Self {
            self.compensate_results
                .insert(name.to_string(), Ok(()));
            self
        }

        fn with_compensate_fail(mut self, name: &str, error: &str) -> Self {
            self.compensate_results
                .insert(name.to_string(), Err(error.to_string()));
            self
        }

        fn executed_steps(&self) -> Vec<String> {
            self.executed.lock().unwrap().clone()
        }

        fn compensated_steps(&self) -> Vec<String> {
            self.compensated.lock().unwrap().clone()
        }
    }

    impl StepExecutor for MockExecutor {
        fn execute_step(&self, step_name: &str) -> Result<(), String> {
            self.executed.lock().unwrap().push(step_name.to_string());
            self.results
                .get(step_name)
                .cloned()
                .unwrap_or(Ok(()))
        }

        fn compensate(&self, compensate_fn: &str) -> Result<(), String> {
            self.compensated
                .lock()
                .unwrap()
                .push(compensate_fn.to_string());
            self.compensate_results
                .get(compensate_fn)
                .cloned()
                .unwrap_or(Ok(()))
        }
    }

    fn test_definition() -> SagaDefinition {
        SagaDefinition::new("test-saga")
            .with_step(SagaStep::new("step-a", "undo-a"))
            .with_step(SagaStep::new("step-b", "undo-b"))
            .with_step(SagaStep::new("step-c", "undo-c"))
    }

    // =========================================================================
    // SagaExecutor tests — happy path
    // =========================================================================

    #[test]
    fn executor_completes_all_steps() {
        let executor = MockExecutor::new()
            .with_step_ok("step-a")
            .with_step_ok("step-b")
            .with_step_ok("step-c");

        let runner = SagaExecutor::new(executor);
        let result = runner.execute(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test saga".to_string(),
            &test_definition(),
        );

        assert!(result.is_success());
        assert_eq!(result.completed_steps.len(), 3);
        assert!(result.compensated_steps.is_empty());
        assert_eq!(runner.executor.executed_steps(), vec!["step-a", "step-b", "step-c"]);
    }

    // =========================================================================
    // SagaExecutor tests — failure with compensation
    // =========================================================================

    #[test]
    fn executor_compensates_on_failure() {
        let executor = MockExecutor::new()
            .with_step_ok("step-a")
            .with_step_ok("step-b")
            .with_step_fail("step-c", "c failed")
            .with_compensate_ok("undo-b")
            .with_compensate_ok("undo-a");

        let runner = SagaExecutor::new(executor);
        let result = runner.execute(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test saga".to_string(),
            &test_definition(),
        );

        assert!(!result.is_success());
        assert!(result.compensation_succeeded());
        assert_eq!(result.completed_steps.len(), 2);
        assert_eq!(result.compensated_steps.len(), 2);
        assert_eq!(result.error, Some("c failed".to_string()));

        // Compensation runs in reverse order
        assert_eq!(
            runner.executor.compensated_steps(),
            vec!["undo-b", "undo-a"]
        );
    }

    // =========================================================================
    // SagaExecutor tests — partial compensation failure
    // =========================================================================

    #[test]
    fn executor_handles_partial_compensation_failure() {
        let executor = MockExecutor::new()
            .with_step_ok("step-a")
            .with_step_ok("step-b")
            .with_step_fail("step-c", "c failed")
            .with_compensate_ok("undo-b")
            .with_compensate_fail("undo-a", "undo-a also failed");

        let runner = SagaExecutor::new(executor);
        let result = runner.execute(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test saga".to_string(),
            &test_definition(),
        );

        assert!(!result.is_success());
        assert!(!result.compensation_succeeded());
        assert_eq!(result.compensated_steps.len(), 1);
        assert_eq!(result.failed_compensations.len(), 1);
        assert_eq!(
            result.failed_compensations[0].compensate_fn,
            "undo-a"
        );
    }

    // =========================================================================
    // SagaExecutor tests — first step failure (no compensation needed)
    // =========================================================================

    #[test]
    fn executor_first_step_failure_no_compensation() {
        let executor = MockExecutor::new().with_step_fail("step-a", "immediate fail");

        let runner = SagaExecutor::new(executor);
        let result = runner.execute(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test saga".to_string(),
            &test_definition(),
        );

        assert!(!result.is_success());
        assert!(result.compensation_succeeded());
        assert!(result.completed_steps.is_empty());
        assert!(result.compensated_steps.is_empty());
        assert_eq!(result.error, Some("immediate fail".to_string()));
    }

    // =========================================================================
    // SagaExecutor tests — resume (crash recovery)
    // =========================================================================

    #[test]
    fn executor_resumes_from_crash_point() {
        let executor = MockExecutor::new()
            .with_step_ok("step-c");

        let def = test_definition();

        // Simulate journal state after crash: step-a completed, step-b was running
        let mut journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test saga".to_string(),
            &def,
        );
        journal.operation.state = OperationState::InProgress;
        journal.start_step(0);
        journal.complete_step(0, "undo-a".to_string());
        journal.start_step(1); // Crash happened here — resume marks it as Failed

        let runner = SagaExecutor::new(executor);
        let result = runner.resume(&def, &mut journal);

        assert!(result.is_success());
        // step-0 was already completed before crash, step-1 was failed by resume,
        // step-2 was executed by resume
        assert_eq!(result.completed_steps.len(), 2);

        // Only step-c was executed (step-b was skipped as failed by crash)
        assert_eq!(
            runner.executor.executed_steps(),
            vec!["step-c"]
        );
    }

    // =========================================================================
    // SagaExecutor tests — empty saga
    // =========================================================================

    #[test]
    fn executor_handles_empty_saga() {
        let executor = MockExecutor::new();
        let runner = SagaExecutor::new(executor);
        let def = SagaDefinition::new("empty");

        let result = runner.execute(
            "op-1".to_string(),
            "agent-1".to_string(),
            "empty saga".to_string(),
            &def,
        );

        assert!(result.is_success());
        assert!(result.completed_steps.is_empty());
    }

    // =========================================================================
    // SagaJournal restore tests
    // =========================================================================

    #[test]
    fn journal_restore_from_persisted_state() {
        let def = test_definition();
        let mut original = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );

        original.operation.state = OperationState::InProgress;
        original.start_step(0);
        original.complete_step(0, "undo-a".to_string());

        let restored = SagaJournal::restore(
            original.operation.clone(),
            original.step_records.clone(),
            original.compensation_actions.clone(),
        );

        assert_eq!(restored.next_step_index(), Some(1));
        assert_eq!(restored.compensation_actions.len(), 1);
    }

    // =========================================================================
    // Serialization round-trip tests
    // =========================================================================

    #[test]
    fn saga_definition_serialization_roundtrip() {
        let def = test_definition();
        let json = serde_json::to_string(&def).unwrap();
        let restored: SagaDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(def, restored);
    }

    #[test]
    fn saga_result_serialization_roundtrip() {
        let result = SagaResult::success(
            "op-1".to_string(),
            vec![StepRecord::new("op-1".to_string(), 0, "step-a".to_string())],
        );
        let json = serde_json::to_string(&result).unwrap();
        let restored: SagaResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, restored);
    }

    #[test]
    fn saga_journal_serialization_roundtrip() {
        let def = test_definition();
        let journal = SagaJournal::new(
            "op-1".to_string(),
            "agent-1".to_string(),
            "test".to_string(),
            &def,
        );
        let json = serde_json::to_string(&journal).unwrap();
        let restored: SagaJournal = serde_json::from_str(&json).unwrap();
        assert_eq!(journal, restored);
    }
}

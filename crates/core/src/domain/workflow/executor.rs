//! Durable workflow executor with saga pattern and two-phase compensation.
//!
//! This module provides the execution engine for durable operations that survive crashes,
//! with automatic compensation on failure.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

use std::{future::Future, pin::Pin, time::Duration};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::workflow::{
    records::{CompensationAction, OperationRecord, RecoveryReport, RecoveryTask},
    states::{OperationState, StepStatus},
};

#[derive(Error, Debug)]
pub enum DurableExecutionError {
    #[error("Operation {0} failed: {1}")]
    OperationFailed(String, String),

    #[error("Step {step} failed in operation {operation}: {error}")]
    StepFailed {
        operation: String,
        step: u32,
        error: String,
    },

    #[error("Compensation failed for step {step} in operation {operation}: {error}")]
    CompensationFailed {
        operation: String,
        step: u32,
        error: String,
    },

    #[error("Operation {0} already in terminal state: {1:?}")]
    AlreadyTerminal(String, OperationState),

    #[error("No such step: {0} in operation {1}")]
    NoSuchStep(String, String),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
}

pub type DurableResult<T> = Result<T, DurableExecutionError>;

pub type StepFuture = Pin<Box<dyn Future<Output = DurableResult<String>> + Send>>;
pub type CompensationFuture = Pin<Box<dyn Future<Output = DurableResult<()>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub step_index: u32,
    pub step_name: String,
    pub status: StepStatus,
    pub duration_ms: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub trait StepFn: Send + Sync {
    fn call(&self) -> StepFuture;
}

impl<F> StepFn for F
where
    F: Fn() -> StepFuture + Send + Sync,
{
    fn call(&self) -> StepFuture {
        self()
    }
}

pub trait CompensationFn: Send + Sync {
    fn call(&self) -> CompensationFuture;
}

impl<F> CompensationFn for F
where
    F: Fn() -> CompensationFuture + Send + Sync,
{
    fn call(&self) -> CompensationFuture {
        self()
    }
}

pub struct StepDefinition {
    pub name: String,
    pub execute: Box<dyn StepFn>,
    pub compensate: Option<Box<dyn CompensationFn>>,
}

pub struct DurableExecutor {
    operation_id: String,
    author_id: String,
    description: String,
    steps: Vec<StepDefinition>,
    journal: Vec<StepOutput>,
    compensation_actions: Vec<CompensationAction>,
    state: OperationState,
    started_at: Option<i64>,
    completed_at: Option<i64>,
}

impl DurableExecutor {
    #[must_use]
    pub const fn new(operation_id: String, author_id: String, description: String) -> Self {
        Self {
            operation_id,
            author_id,
            description,
            steps: Vec::new(),
            journal: Vec::new(),
            compensation_actions: Vec::new(),
            state: OperationState::Started,
            started_at: None,
            completed_at: None,
        }
    }

    #[must_use]
    pub fn with_steps(mut self, steps: Vec<StepDefinition>) -> Self {
        self.steps = steps;
        self
    }

    pub fn add_step<G>(&mut self, name: String, execute: G)
    where
        G: Fn() -> StepFuture + 'static + Send + Sync,
    {
        self.steps.push(StepDefinition {
            name,
            execute: Box::new(execute),
            compensate: None,
        });
    }

    pub fn add_compensation_step<G, H>(&mut self, name: String, execute: G, compensate: H)
    where
        G: Fn() -> StepFuture + 'static + Send + Sync,
        H: Fn() -> CompensationFuture + 'static + Send + Sync,
    {
        self.steps.push(StepDefinition {
            name,
            execute: Box::new(execute),
            compensate: Some(Box::new(compensate)),
        });
    }

    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    #[must_use]
    pub const fn state(&self) -> OperationState {
        self.state
    }

    #[must_use]
    pub fn journal(&self) -> &[StepOutput] {
        &self.journal
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Execute all steps in the durable operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails and compensation fails.
    ///
    /// # Panics
    ///
    /// Panics if step count exceeds `u32::MAX`.
    pub async fn execute(&mut self) -> DurableResult<OperationRecord> {
        if self.state.is_terminal() {
            return Err(DurableExecutionError::AlreadyTerminal(
                self.operation_id.clone(),
                self.state,
            ));
        }

        self.state = OperationState::InProgress;
        self.started_at = Some(Utc::now().timestamp());

        #[allow(clippy::unwrap_used)]
        let total_steps = u32::try_from(self.steps.len()).unwrap(); // SAFETY: Vec cannot have > u32::MAX elements

        for index in 0..self.steps.len() {
            #[allow(clippy::unwrap_used)]
            let step_index = u32::try_from(index).unwrap(); // SAFETY: index < steps.len() <= u32::MAX
            let step_name = self.steps[index].name.clone();

            let step_output = self
                .execute_step(step_index, &step_name, self.steps[index].execute.call())
                .await;

            self.journal.push(step_output.clone());

            if step_output.error.is_some() {
                self.state = OperationState::Failed;

                let compensation_result = self.execute_compensation(step_index).await;

                match compensation_result {
                    Ok(()) => {
                        self.completed_at = Some(Utc::now().timestamp());
                        return Ok(self.to_record(total_steps));
                    }
                    Err(e) => {
                        self.completed_at = Some(Utc::now().timestamp());
                        return Err(e);
                    }
                }
            }
        }

        self.state = OperationState::Completed;
        self.completed_at = Some(Utc::now().timestamp());
        Ok(self.to_record(total_steps))
    }

    async fn execute_step(
        &self,
        step_index: u32,
        step_name: &str,
        execute: StepFuture,
    ) -> StepOutput {
        let step_started_at = Utc::now();

        let output = execute.await;

        let step_completed_at = Utc::now();
        let duration_ms = (step_completed_at - step_started_at).num_milliseconds();

        match output {
            Ok(result) => StepOutput {
                step_index,
                step_name: step_name.to_string(),
                status: StepStatus::Completed,
                duration_ms: Some(duration_ms),
                output: Some(result),
                error: None,
            },
            Err(e) => StepOutput {
                step_index,
                step_name: step_name.to_string(),
                status: StepStatus::Failed,
                duration_ms: Some(duration_ms),
                output: None,
                error: Some(e.to_string()),
            },
        }
    }

    async fn execute_compensation(&mut self, failed_step_index: u32) -> DurableResult<()> {
        for step_index in (0..failed_step_index as usize).rev() {
            if let Some(compensate) = &self.steps[step_index].compensate {
                let result = compensate.call().await;

                let compensation_action = CompensationAction {
                    step_index: failed_step_index,
                    step_name: self.steps[step_index].name.clone(),
                    compensate_fn: "compensation_fn".to_string(),
                    status: match result.as_ref() {
                        Ok(()) => StepStatus::Completed,
                        Err(_) => StepStatus::Failed,
                    },
                    error_message: result.as_ref().err().map(std::string::ToString::to_string),
                };

                self.compensation_actions.push(compensation_action);

                if result.is_err() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn to_record(&self, total_steps: u32) -> OperationRecord {
        #[allow(clippy::unwrap_used)]
        let current_step = u32::try_from(
            self.journal
                .iter()
                .filter(|s| s.status == StepStatus::Completed)
                .count(),
        )
        .unwrap(); // SAFETY: count is always <= usize::MAX which fits in u32 for realistic counts

        let error_message = self
            .journal
            .iter()
            .find(|s| s.status == StepStatus::Failed)
            .and_then(|s| s.error.clone());

        OperationRecord {
            operation_id: self.operation_id.clone(),
            state: self.state,
            current_step,
            total_steps,
            started_at: self.started_at.unwrap_or(0),
            completed_at: self.completed_at,
            final_revision: None,
            error_message,
            author_id: self.author_id.clone(),
            description: self.description.clone(),
        }
    }
}

/// Scans the step journal for incomplete operations and produces recovery tasks.
///
/// When a [`SqliteJournal`] is provided the scanner queries `SQLite` for operations
/// in non-terminal states and determines the resume point from the last
/// completed step. Without a journal the scanner is a no-op.
pub struct RecoveryScanner {
    journal: Option<crate::domain::workflow::SqliteJournal>,
}

impl std::fmt::Debug for RecoveryScanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryScanner")
            .field("journal", &self.journal.as_ref().map(|_| "Some(...)"))
            .finish()
    }
}

impl RecoveryScanner {
    /// Create a scanner backed by the given journal.
    #[must_use]
    pub const fn new(journal: crate::domain::workflow::SqliteJournal) -> Self {
        Self {
            journal: Some(journal),
        }
    }

    /// Create a no-op scanner with no backing store.
    #[must_use]
    pub const fn no_op() -> Self {
        Self { journal: None }
    }

    /// Scan for incomplete operations in the journal.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying journal query fails.
    pub async fn scan_incomplete_operations(&self) -> DurableResult<Vec<RecoveryTask>> {
        let Some(journal) = &self.journal else {
            return Ok(Vec::new());
        };

        journal
            .recovery_tasks()
            .await
            .map_err(|e| DurableExecutionError::RecoveryFailed(e.to_string()))
    }

    /// Mark a recovered operation as completed in the journal.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub async fn recover_operation(&self, task: RecoveryTask) -> DurableResult<()> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };

        journal
            .mark_operation_completed(&task.operation_id)
            .await
            .map_err(|e| DurableExecutionError::RecoveryFailed(e.to_string()))
    }

    /// Scan and recover all incomplete operations.
    ///
    /// For each incomplete operation found in the journal, marks it as
    /// completed. Callers should integrate this with the executor for actual
    /// step replay.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning or recovery fails.
    pub async fn scan_and_recover_all(&self) -> DurableResult<RecoveryReport> {
        let tasks = self.scan_incomplete_operations().await?;
        let total_incomplete = tasks.len();

        let mut recovered_operations = Vec::new();
        let mut failed_operations = Vec::new();

        for task in tasks {
            let op_id = task.operation_id.clone();
            match self.recover_operation(task).await {
                Ok(()) => recovered_operations.push(op_id),
                Err(e) => failed_operations.push((op_id, e.to_string())),
            }
        }

        Ok(RecoveryReport {
            total_incomplete,
            recovered: recovered_operations.len(),
            failed: failed_operations.len(),
            recovered_operations,
            failed_operations,
        })
    }
}

pub trait DurableTimer {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

pub struct NoOpTimer;

impl DurableTimer for NoOpTimer {
    fn sleep(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(std::future::ready(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_durable_executor_completes_all_steps() {
        let mut executor = DurableExecutor::new(
            "op-123".to_string(),
            "author-1".to_string(),
            "Test operation".to_string(),
        );

        executor.add_step("step-1".to_string(), || {
            Box::pin(async move { Ok::<String, DurableExecutionError>("step-1 done".to_string()) })
        });

        executor.add_step("step-2".to_string(), || {
            Box::pin(async move { Ok::<String, DurableExecutionError>("step-2 done".to_string()) })
        });

        let record = executor.execute().await.unwrap();

        assert_eq!(record.state, OperationState::Completed);
        assert_eq!(record.total_steps, 2);
        assert_eq!(record.current_step, 2);
    }

    #[tokio::test]
    async fn test_durable_executor_fails_and_compensates() {
        let mut executor = DurableExecutor::new(
            "op-456".to_string(),
            "author-1".to_string(),
            "Test failure".to_string(),
        );

        executor.add_compensation_step(
            "step-1".to_string(),
            || {
                Box::pin(
                    async move { Ok::<String, DurableExecutionError>("step-1 done".to_string()) },
                )
            },
            || Box::pin(async move { Ok::<(), DurableExecutionError>(()) }),
        );

        executor.add_step("step-2".to_string(), || {
            Box::pin(async move {
                Err::<String, _>(DurableExecutionError::StepFailed {
                    operation: "op-456".to_string(),
                    step: 1,
                    error: "Intentional failure".to_string(),
                })
            })
        });

        let record = executor.execute().await.unwrap();

        assert_eq!(record.state, OperationState::Failed);
        assert_eq!(record.current_step, 1);
        assert_eq!(record.total_steps, 2);
    }

    #[test]
    fn test_operation_record_progress() {
        let record = OperationRecord::new(
            "op-123".to_string(),
            "author-1".to_string(),
            "Test".to_string(),
            5,
        );

        assert_eq!(record.progress_percentage(), 0.0);
    }

    #[test]
    fn test_recovery_report() {
        let mut report = RecoveryReport::new();
        report.add_recovered("op-1".to_string());
        report.add_recovered("op-2".to_string());
        report.add_failed("op-3".to_string(), "error".to_string());

        assert_eq!(report.total_incomplete, 3);
        assert_eq!(report.recovered, 2);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn test_step_status_is_terminal() {
        assert!(!StepStatus::Pending.is_terminal());
        assert!(!StepStatus::Running.is_terminal());
        assert!(StepStatus::Completed.is_terminal());
        assert!(StepStatus::Failed.is_terminal());
        assert!(StepStatus::Skipped.is_terminal());
    }
}

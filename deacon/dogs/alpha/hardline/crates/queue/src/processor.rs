//! Queue merge processor - Drives entries through the merge pipeline
//!
//! The processor coordinates the lifecycle of queue entries:
//! Claim → Rebase → Test → ReadyToMerge → Merge → Done
//!
//! It uses a trait-based VCS abstraction so the domain layer stays pure.
//! Actual Git operations are injected via the `MergeBackend` trait.

use crate::domain::entities::{
    Claimed, FailedRetryable, FailedTerminal, Merged, Merging, Pending, QueueEntry, QueueEntryId,
    QueueStatus, ReadyToMerge, Rebasing, Testing,
};
use crate::domain::ports::QueueRepository;
use crate::error::{QueueError, Result};
use std::fmt;

/// Trait for VCS operations needed during merge processing.
/// Injected into the processor to keep domain logic testable.
pub trait MergeBackend: Send + Sync {
    /// Rebase the entry's branch onto the target branch.
    fn rebase_onto(&self, session_id: &str, target: &str) -> std::result::Result<(), String>;

    /// Run the test suite for the entry's branch.
    fn run_tests(&self, session_id: &str) -> std::result::Result<TestOutcome, String>;

    /// Merge the entry's branch into the target branch.
    fn merge_branch(&self, session_id: &str, target: &str) -> std::result::Result<(), String>;

    /// Push the merged result to the remote.
    fn push(&self, remote: &str, branch: &str) -> std::result::Result<(), String>;
}

/// Outcome of a test run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutcome {
    /// All tests passed.
    Passed,
    /// Tests failed but can be retried.
    Failed,
    /// Tests failed with a terminal error (e.g., build broken).
    Fatal,
}

/// Result of processing a single queue entry through one step.
#[derive(Debug)]
pub enum ProcessingOutcome {
    /// Entry advanced to the next state.
    Advanced {
        entry_id: QueueEntryId,
        new_status: QueueStatus,
    },
    /// Entry reached a terminal state (Merged, FailedTerminal, Cancelled).
    Completed {
        entry_id: QueueEntryId,
        final_status: QueueStatus,
    },
    /// Entry failed but can be retried.
    Retryable {
        entry_id: QueueEntryId,
        error: String,
    },
}

impl fmt::Display for ProcessingOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Advanced { entry_id, new_status } => {
                write!(f, "advanced {} to {new_status:?}", entry_id.as_str())
            }
            Self::Completed {
                entry_id,
                final_status,
            } => {
                write!(f, "completed {} as {final_status:?}", entry_id.as_str())
            }
            Self::Retryable { entry_id, error } => {
                write!(f, "retryable {} error: {error}", entry_id.as_str())
            }
        }
    }
}

/// Configuration for the merge processor.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Remote name for push operations (e.g., "origin").
    pub remote: String,
    /// Target branch for merges (e.g., "main").
    pub target_branch: String,
    /// Maximum number of retry attempts before marking terminal.
    pub max_retries: u32,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            target_branch: "main".to_string(),
            max_retries: 3,
        }
    }
}

/// The queue merge processor.
///
/// Drives queue entries through the merge pipeline. Each call to `process_entry`
/// advances a single entry by one step in the state machine.
pub struct QueueProcessor<R: QueueRepository, B: MergeBackend> {
    repository: R,
    backend: B,
    config: ProcessorConfig,
}

impl<R: QueueRepository, B: MergeBackend> QueueProcessor<R, B> {
    pub fn new(repository: R, backend: B, config: ProcessorConfig) -> Self {
        Self {
            repository,
            backend,
            config,
        }
    }

    /// Process the next pending entry in the queue.
    ///
    /// Claims the next pending entry and begins processing it through
    /// the merge pipeline. Returns the processing outcome.
    pub fn process_next(&self) -> Result<ProcessingOutcome> {
        let entry = self
            .repository
            .dequeue()?
            .ok_or(QueueError::QueueEmpty)?;

        let claimed: QueueEntry<Claimed> = entry
            .claim()
            .map_err(|_| QueueError::InvalidStateTransition {
                from: "Pending".to_string(),
                to: "Claimed".to_string(),
            })?;

        let updated = self.repository.update(claimed)?;
        self.process_entry(updated)
    }

    /// Process a specific entry through its next pipeline step.
    ///
    /// The entry's current status determines which step runs:
    /// - Claimed → rebase
    /// - Rebasing → test
    /// - ReadyToMerge → merge + push
    pub fn process_entry(&self, entry: QueueEntry<Claimed>) -> Result<ProcessingOutcome> {
        let id = entry.id().clone();
        let session = entry.session_id().to_string();
        let target = self.config.target_branch.clone();

        // Step 1: Rebase
        let rebasing: QueueEntry<Rebasing> = entry.start_rebase().map_err(|_| {
            QueueError::InvalidStateTransition {
                from: "Claimed".to_string(),
                to: "Rebasing".to_string(),
            }
        })?;
        self.repository.update(rebasing)?;

        match self.backend.rebase_onto(&session, &target) {
            Ok(()) => {}
            Err(e) => {
                return self.handle_rebase_failure(id, e);
            }
        }

        // Step 2: Test
        let testing: QueueEntry<Testing> = rebasing.start_testing().map_err(|_| {
            QueueError::InvalidStateTransition {
                from: "Rebasing".to_string(),
                to: "Testing".to_string(),
            }
        })?;
        self.repository.update(testing)?;

        let test_outcome = self.backend.run_tests(&session).map_err(|e| {
            QueueError::OperationFailed(format!("test runner error: {e}"))
        })?;

        match test_outcome {
            TestOutcome::Passed => {}
            TestOutcome::Failed => {
                return self.handle_test_failure(id, "tests failed".to_string());
            }
            TestOutcome::Fatal => {
                return self.handle_fatal_test_failure(id, "fatal test failure".to_string());
            }
        }

        // Step 3: Mark ready to merge
        let ready: QueueEntry<ReadyToMerge> = testing
            .mark_ready_to_merge()
            .map_err(|_| QueueError::InvalidStateTransition {
                from: "Testing".to_string(),
                to: "ReadyToMerge".to_string(),
            })?;
        self.repository.update(ready)?;

        // Step 4: Merge
        let merging: QueueEntry<Merging> = ready.start_merging().map_err(|_| {
            QueueError::InvalidStateTransition {
                from: "ReadyToMerge".to_string(),
                to: "Merging".to_string(),
            }
        })?;
        self.repository.update(merging)?;

        self.backend
            .merge_branch(&session, &target)
            .map_err(|e| QueueError::OperationFailed(format!("merge failed: {e}")))?;

        // Step 5: Push
        self.backend
            .push(&self.config.remote, &target)
            .map_err(|e| QueueError::OperationFailed(format!("push failed: {e}")))?;

        // Step 6: Mark merged
        let merged: QueueEntry<Merged> = merging.mark_merged().map_err(|_| {
            QueueError::InvalidStateTransition {
                from: "Merging".to_string(),
                to: "Merged".to_string(),
            }
        })?;
        self.repository.update(merged)?;

        Ok(ProcessingOutcome::Completed {
            entry_id: id,
            final_status: QueueStatus::Merged,
        })
    }

    /// Process all pending entries until the queue is empty or an error occurs.
    ///
    /// Returns all outcomes from processing.
    pub fn process_all(&self) -> Result<Vec<ProcessingOutcome>> {
        let mut outcomes = Vec::new();
        loop {
            match self.process_next() {
                Ok(outcome) => {
                    let is_terminal = matches!(
                        &outcome,
                        ProcessingOutcome::Completed { .. }
                            | ProcessingOutcome::Retryable { .. }
                    );
                    outcomes.push(outcome);
                    if is_terminal {
                        continue;
                    }
                    // Advanced means mid-pipeline, continue
                }
                Err(QueueError::QueueEmpty) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(outcomes)
    }

    fn handle_rebase_failure(
        &self,
        id: QueueEntryId,
        error: String,
    ) -> Result<ProcessingOutcome> {
        // Rebase failures are typically retryable after fixing conflicts
        Ok(ProcessingOutcome::Retryable {
            entry_id: id,
            error: format!("rebase failed: {error}"),
        })
    }

    fn handle_test_failure(
        &self,
        id: QueueEntryId,
        error: String,
    ) -> Result<ProcessingOutcome> {
        // Test failures are retryable (within limits)
        Ok(ProcessingOutcome::Retryable {
            entry_id: id,
            error,
        })
    }

    fn handle_fatal_test_failure(
        &self,
        id: QueueEntryId,
        error: String,
    ) -> Result<ProcessingOutcome> {
        // Fatal test failures are terminal
        Ok(ProcessingOutcome::Completed {
            entry_id: id,
            final_status: QueueStatus::FailedTerminal,
        })
    }
}

/// A no-op merge backend for testing.
pub struct NoopMergeBackend {
    pub rebase_result: std::result::Result<(), String>,
    pub test_result: std::result::Result<TestOutcome, String>,
    pub merge_result: std::result::Result<(), String>,
    pub push_result: std::result::Result<(), String>,
}

impl Default for NoopMergeBackend {
    fn default() -> Self {
        Self {
            rebase_result: Ok(()),
            test_result: Ok(TestOutcome::Passed),
            merge_result: Ok(()),
            push_result: Ok(()),
        }
    }
}

impl NoopMergeBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MergeBackend for NoopMergeBackend {
    fn rebase_onto(&self, _session_id: &str, _target: &str) -> std::result::Result<(), String> {
        self.rebase_result.clone()
    }

    fn run_tests(&self, _session_id: &str) -> std::result::Result<TestOutcome, String> {
        self.test_result.clone()
    }

    fn merge_branch(&self, _session_id: &str, _target: &str) -> std::result::Result<(), String> {
        self.merge_result.clone()
    }

    fn push(&self, _remote: &str, _branch: &str) -> std::result::Result<(), String> {
        self.push_result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::QueueEntry as QEntry;
    use crate::domain::value_objects::Priority;

    fn create_processor() -> QueueProcessor<crate::domain::ports::InMemoryQueueRepository, NoopMergeBackend> {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend::new();
        QueueProcessor::new(repo, backend, ProcessorConfig::default())
    }

    fn enqueue_entry(
        repo: &crate::domain::ports::InMemoryQueueRepository,
        session: &str,
    ) -> QueueEntry<Pending> {
        let entry =
            QueueEntry::<Pending>::enqueue(session.to_string(), None, Priority::default())
                .expect("enqueue should succeed");
        repo.enqueue(entry).expect("repo enqueue should succeed")
    }

    #[test]
    fn processor_process_next_empty_queue_returns_queue_empty() {
        let processor = create_processor();
        let result = processor.process_next();
        assert!(result.is_err());
        let err = result.err().expect("should have error");
        let msg = format!("{err}");
        assert!(msg.contains("empty"), "expected empty, got: {msg}");
    }

    #[test]
    fn processor_process_next_happy_path() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend::new();
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_ok());
        let outcome = result.expect("should succeed");
        match outcome {
            ProcessingOutcome::Completed {
                final_status,
                entry_id,
            } => {
                assert_eq!(final_status, QueueStatus::Merged);
                assert!(entry_id.as_str().starts_with("queue-"));
            }
            other => panic!("expected Completed, got: {other}"),
        }
    }

    #[test]
    fn processor_process_entry_rebase_failure() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            rebase_result: Err("conflict in src/main.rs".to_string()),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_ok());
        match result.expect("should succeed") {
            ProcessingOutcome::Retryable { error, .. } => {
                assert!(error.contains("rebase failed"), "got: {error}");
            }
            other => panic!("expected Retryable, got: {other}"),
        }
    }

    #[test]
    fn processor_process_entry_test_failure() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            test_result: Ok(TestOutcome::Failed),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_ok());
        match result.expect("should succeed") {
            ProcessingOutcome::Retryable { error, .. } => {
                assert!(error.contains("tests failed"), "got: {error}");
            }
            other => panic!("expected Retryable, got: {other}"),
        }
    }

    #[test]
    fn processor_process_entry_fatal_test_failure() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            test_result: Ok(TestOutcome::Fatal),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_ok());
        match result.expect("should succeed") {
            ProcessingOutcome::Completed { final_status, .. } => {
                assert_eq!(final_status, QueueStatus::FailedTerminal);
            }
            other => panic!("expected Completed(FailedTerminal), got: {other}"),
        }
    }

    #[test]
    fn processor_process_all_multiple_entries() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend::new();
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");
        enqueue_entry(&repo, "session-2");
        enqueue_entry(&repo, "session-3");

        let result = processor.process_all();
        assert!(result.is_ok());
        let outcomes = result.expect("should succeed");
        assert_eq!(outcomes.len(), 3);
        for outcome in &outcomes {
            match outcome {
                ProcessingOutcome::Completed { final_status, .. } => {
                    assert_eq!(*final_status, QueueStatus::Merged);
                }
                other => panic!("expected Completed, got: {other}"),
            }
        }
    }

    #[test]
    fn processor_process_all_empty_queue() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend::new();
        let processor = QueueProcessor::new(repo, backend, ProcessorConfig::default());

        let result = processor.process_all();
        assert!(result.is_ok());
        assert!(result.expect("should succeed").is_empty());
    }

    #[test]
    fn processor_config_default() {
        let config = ProcessorConfig::default();
        assert_eq!(config.remote, "origin");
        assert_eq!(config.target_branch, "main");
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn processor_outcome_display_advanced() {
        let id = QueueEntryId::generate();
        let outcome = ProcessingOutcome::Advanced {
            entry_id: id.clone(),
            new_status: QueueStatus::Testing,
        };
        let msg = format!("{outcome}");
        assert!(msg.contains(id.as_str()), "got: {msg}");
        assert!(msg.contains("advanced"), "got: {msg}");
    }

    #[test]
    fn processor_outcome_display_completed() {
        let id = QueueEntryId::generate();
        let outcome = ProcessingOutcome::Completed {
            entry_id: id.clone(),
            final_status: QueueStatus::Merged,
        };
        let msg = format!("{outcome}");
        assert!(msg.contains(id.as_str()), "got: {msg}");
        assert!(msg.contains("completed"), "got: {msg}");
    }

    #[test]
    fn processor_outcome_display_retryable() {
        let id = QueueEntryId::generate();
        let outcome = ProcessingOutcome::Retryable {
            entry_id: id.clone(),
            error: "tests failed".to_string(),
        };
        let msg = format!("{outcome}");
        assert!(msg.contains(id.as_str()), "got: {msg}");
        assert!(msg.contains("retryable"), "got: {msg}");
    }

    #[test]
    fn test_outcome_variants() {
        assert_ne!(TestOutcome::Passed, TestOutcome::Failed);
        assert_ne!(TestOutcome::Failed, TestOutcome::Fatal);
        assert_ne!(TestOutcome::Passed, TestOutcome::Fatal);
    }

    #[test]
    fn noop_backend_default_succeeds() {
        let backend = NoopMergeBackend::default();
        assert!(backend.rebase_onto("s1", "main").is_ok());
        assert!(backend.run_tests("s1").is_ok());
        assert!(backend.merge_branch("s1", "main").is_ok());
        assert!(backend.push("origin", "main").is_ok());
    }

    #[test]
    fn noop_backend_custom_results() {
        let backend = NoopMergeBackend {
            rebase_result: Err("conflict".to_string()),
            test_result: Ok(TestOutcome::Failed),
            merge_result: Err("merge conflict".to_string()),
            push_result: Err("network error".to_string()),
        };
        assert!(backend.rebase_onto("s1", "main").is_err());
        assert_eq!(backend.run_tests("s1").ok(), Some(TestOutcome::Failed));
        assert!(backend.merge_branch("s1", "main").is_err());
        assert!(backend.push("origin", "main").is_err());
    }

    #[test]
    fn processor_process_next_merge_failure() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            merge_result: Err("merge conflict".to_string()),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_err());
        let err = result.err().expect("should have error");
        let msg = format!("{err}");
        assert!(msg.contains("merge failed"), "got: {msg}");
    }

    #[test]
    fn processor_process_next_push_failure() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            push_result: Err("network timeout".to_string()),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");

        let result = processor.process_next();
        assert!(result.is_err());
        let err = result.err().expect("should have error");
        let msg = format!("{err}");
        assert!(msg.contains("push failed"), "got: {msg}");
    }

    #[test]
    fn processor_custom_config() {
        let config = ProcessorConfig {
            remote: "upstream".to_string(),
            target_branch: "develop".to_string(),
            max_retries: 5,
        };
        assert_eq!(config.remote, "upstream");
        assert_eq!(config.target_branch, "develop");
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn processor_process_all_with_mixed_results() {
        let repo = crate::domain::ports::InMemoryQueueRepository::new();
        let backend = NoopMergeBackend {
            test_result: Ok(TestOutcome::Fatal),
            ..NoopMergeBackend::default()
        };
        let processor = QueueProcessor::new(repo.clone(), backend, ProcessorConfig::default());

        enqueue_entry(&repo, "session-1");
        enqueue_entry(&repo, "session-2");

        let result = processor.process_all();
        assert!(result.is_ok());
        let outcomes = result.expect("should succeed");
        assert_eq!(outcomes.len(), 2);
        for outcome in &outcomes {
            match outcome {
                ProcessingOutcome::Completed { final_status, .. } => {
                    assert_eq!(*final_status, QueueStatus::FailedTerminal);
                }
                other => panic!("expected Completed, got: {other}"),
            }
        }
    }
}

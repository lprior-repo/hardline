//! Exactly-once execution guarantee layer
//!
//! The ExactlyOnceExecutor wraps operations with:
//! 1. Journal intent before execution
//! 2. Dedup check (skip if already completed)
//! 3. Execute the operation
//! 4. Record receipt on success
//! 5. Handle crash recovery by replaying incomplete operations

pub mod dedup;
pub mod journal;
pub mod store;
pub mod types;

use dedup::{DedupDecision, DedupError, OperationDeduplicator};
use journal::{InMemoryJournal, Journal, JournalError};
use store::{InMemoryReceiptStore, ReceiptStore, ReceiptStoreError};
use types::{IdempotencyKey, JournalEntry, JournalTransitionError, OperationStatus};

pub use dedup::DedupDecision as ExactlyOnceDedupDecision;
pub use types::{
    IdempotencyKey as ExactlyOnceKey, JournalEntry as ExactlyOnceJournalEntry,
    JournalTransitionError as ExactlyOnceJournalTransitionError,
    OperationStatus as ExactlyOnceOperationStatus, Receipt as ExactlyOnceReceipt,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExactlyOnceError {
    #[error("Dedup check failed: {0}")]
    Dedup(#[from] DedupError),
    #[error("Journal error: {0}")]
    Journal(#[from] JournalError),
    #[error("Journal transition error: {0}")]
    JournalTransition(#[from] JournalTransitionError),
    #[error("Receipt store error: {0}")]
    ReceiptStore(#[from] ReceiptStoreError),
    #[error("Operation already completed: {0}")]
    AlreadyCompleted(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

pub type ExactlyOnceResult<T> = Result<T, ExactlyOnceError>;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub key: IdempotencyKey,
    pub was_deduplicated: bool,
    pub output: Option<serde_json::Value>,
}

pub struct ExactlyOnceExecutor<S: ReceiptStore, J: Journal> {
    deduplicator: OperationDeduplicator<S>,
    journal: J,
}

impl<S: ReceiptStore, J: Journal> ExactlyOnceExecutor<S, J> {
    pub fn new(receipt_store: S, journal: J) -> Self {
        let deduplicator = OperationDeduplicator::new(receipt_store);
        Self {
            deduplicator,
            journal,
        }
    }

    pub async fn execute<F, Fut>(
        &self,
        key: IdempotencyKey,
        payload: serde_json::Value,
        operation: F,
    ) -> ExactlyOnceResult<ExecutionResult>
    where
        F: FnOnce(serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = ExactlyOnceResult<serde_json::Value>>,
    {
        let decision = self.deduplicator.check(&key).await?;
        match decision {
            DedupDecision::AlreadyCompleted { receipt } => {
                return Ok(ExecutionResult {
                    key: receipt.key,
                    was_deduplicated: true,
                    output: None,
                });
            }
            DedupDecision::Execute => {}
            DedupDecision::InProgress => {
                return Err(ExactlyOnceError::AlreadyCompleted(format!(
                    "Operation {key} is already in progress"
                )));
            }
        }

        let entry = JournalEntry::new_intended(key.clone(), payload.clone());
        self.journal.append(entry)?;

        let intended = self
            .journal
            .get(&key)?
            .ok_or_else(|| ExactlyOnceError::Journal(JournalError::NotFound(key.to_string())))?;
        let in_progress = intended.transition_to(OperationStatus::InProgress)?;
        self.journal.update(in_progress)?;

        let result = operation(payload).await;

        match result {
            Ok(output) => {
                let in_progress_entry = self.journal.get(&key)?.ok_or_else(|| {
                    ExactlyOnceError::Journal(JournalError::NotFound(key.to_string()))
                })?;
                let completed = in_progress_entry.transition_to(OperationStatus::Completed)?;
                self.journal.update(completed)?;

                self.deduplicator.record_completion(key.clone()).await?;

                Ok(ExecutionResult {
                    key,
                    was_deduplicated: false,
                    output: Some(output),
                })
            }
            Err(e) => {
                let in_progress_entry = self.journal.get(&key)?.ok_or_else(|| {
                    ExactlyOnceError::Journal(JournalError::NotFound(key.to_string()))
                })?;
                let failed = in_progress_entry
                    .transition_to(OperationStatus::Failed)?
                    .with_error(e.to_string());
                self.journal.update(failed)?;

                Err(ExactlyOnceError::OperationFailed(e.to_string()))
            }
        }
    }

    pub fn recover_incomplete(&self) -> ExactlyOnceResult<Vec<JournalEntry>> {
        let incomplete = self.journal.get_incomplete()?;
        Ok(incomplete)
    }

    pub fn journal_entry(&self, key: &IdempotencyKey) -> ExactlyOnceResult<Option<JournalEntry>> {
        let entry = self.journal.get(key)?;
        Ok(entry)
    }
}

pub struct InMemoryExactlyOnceExecutor {
    inner: ExactlyOnceExecutor<InMemoryReceiptStore, InMemoryJournal>,
}

impl InMemoryExactlyOnceExecutor {
    #[must_use]
    pub fn new() -> Self {
        let receipt_store = InMemoryReceiptStore::new();
        let journal = InMemoryJournal::new();
        Self {
            inner: ExactlyOnceExecutor::new(receipt_store, journal),
        }
    }

    pub async fn execute<F, Fut>(
        &self,
        key: IdempotencyKey,
        payload: serde_json::Value,
        operation: F,
    ) -> ExactlyOnceResult<ExecutionResult>
    where
        F: FnOnce(serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = ExactlyOnceResult<serde_json::Value>>,
    {
        self.inner.execute(key, payload, operation).await
    }

    pub fn recover_incomplete(&self) -> ExactlyOnceResult<Vec<JournalEntry>> {
        self.inner.recover_incomplete()
    }

    pub fn journal_entry(&self, key: &IdempotencyKey) -> ExactlyOnceResult<Option<JournalEntry>> {
        self.inner.journal_entry(key)
    }
}

impl Default for InMemoryExactlyOnceExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn success_op(_payload: serde_json::Value) -> ExactlyOnceResult<serde_json::Value> {
        Ok(serde_json::json!({"status": "ok"}))
    }

    async fn fail_op(_payload: serde_json::Value) -> ExactlyOnceResult<serde_json::Value> {
        Err(ExactlyOnceError::OperationFailed(
            "intentional failure".to_string(),
        ))
    }

    #[tokio::test]
    async fn test_execute_success() {
        let executor = InMemoryExactlyOnceExecutor::new();
        let key = IdempotencyKey::from_static("op-1");

        let result = executor
            .execute(key.clone(), serde_json::json!({"cmd": "merge"}), success_op)
            .await
            .expect("execute");

        assert_eq!(result.key, key);
        assert!(!result.was_deduplicated);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_execute_deduplicates_duplicate_key() {
        let executor = InMemoryExactlyOnceExecutor::new();
        let key = IdempotencyKey::from_static("op-dup");

        let result1 = executor
            .execute(key.clone(), serde_json::json!({}), success_op)
            .await
            .expect("first execute");
        assert!(!result1.was_deduplicated);

        let result2 = executor
            .execute(key.clone(), serde_json::json!({}), success_op)
            .await
            .expect("second execute");
        assert!(result2.was_deduplicated);
        assert!(result2.output.is_none());
    }

    #[tokio::test]
    async fn test_execute_failure_records_in_journal() {
        let executor = InMemoryExactlyOnceExecutor::new();
        let key = IdempotencyKey::from_static("op-fail");

        let result = executor
            .execute(key.clone(), serde_json::json!({}), fail_op)
            .await;
        assert!(result.is_err());

        let entry = executor
            .journal_entry(&key)
            .expect("get entry")
            .expect("some");
        assert_eq!(entry.status, OperationStatus::Failed);
        assert!(entry.error.is_some());
    }

    #[tokio::test]
    async fn test_recover_incomplete_after_success() {
        let executor = InMemoryExactlyOnceExecutor::new();

        executor
            .execute(
                IdempotencyKey::from_static("completed"),
                serde_json::json!({}),
                success_op,
            )
            .await
            .expect("execute");

        let incomplete = executor.recover_incomplete().expect("recover");
        assert!(incomplete.is_empty());
    }

    #[tokio::test]
    async fn test_recover_incomplete_after_failure() {
        let executor = InMemoryExactlyOnceExecutor::new();

        let _ = executor
            .execute(
                IdempotencyKey::from_static("failed"),
                serde_json::json!({}),
                fail_op,
            )
            .await;

        let incomplete = executor.recover_incomplete().expect("recover");
        assert!(incomplete.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_operations_independent() {
        let executor = InMemoryExactlyOnceExecutor::new();

        let r1 = executor
            .execute(
                IdempotencyKey::from_static("op-a"),
                serde_json::json!({}),
                success_op,
            )
            .await
            .expect("op-a");
        let r2 = executor
            .execute(
                IdempotencyKey::from_static("op-b"),
                serde_json::json!({}),
                success_op,
            )
            .await
            .expect("op-b");

        assert!(!r1.was_deduplicated);
        assert!(!r2.was_deduplicated);
        assert_eq!(r1.key, IdempotencyKey::from_static("op-a"));
        assert_eq!(r2.key, IdempotencyKey::from_static("op-b"));
    }

    #[tokio::test]
    async fn test_execution_result_output_contains_value() {
        let executor = InMemoryExactlyOnceExecutor::new();
        let result = executor
            .execute(
                IdempotencyKey::from_static("with-output"),
                serde_json::json!({}),
                success_op,
            )
            .await
            .expect("execute");

        let output = result.output.expect("output");
        assert_eq!(output["status"], "ok");
    }

    #[tokio::test]
    async fn test_default_executor_works() {
        let executor = InMemoryExactlyOnceExecutor::default();
        let result = executor
            .execute(
                IdempotencyKey::from_static("default"),
                serde_json::json!({}),
                success_op,
            )
            .await
            .expect("execute");
        assert!(!result.was_deduplicated);
    }
}

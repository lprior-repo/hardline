//! Operation deduplicator
//!
//! Checks receipts before execution to prevent duplicate operations.
//! Wraps ReceiptStore and Journal to answer "should we execute this?"

use super::store::{ReceiptStore, ReceiptStoreError};
use super::types::{IdempotencyKey, Receipt};

#[derive(Debug, Clone, thiserror::Error)]
pub enum DedupError {
    #[error("Operation already completed: {0}")]
    AlreadyCompleted(String),
    #[error("Operation currently in progress: {0}")]
    InProgress(String),
    #[error("Receipt store error: {0}")]
    Store(#[from] ReceiptStoreError),
}

pub type DedupResult<T> = Result<T, DedupError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupDecision {
    Execute,
    AlreadyCompleted { receipt: Receipt },
    InProgress,
}

pub struct OperationDeduplicator<S: ReceiptStore> {
    receipt_store: S,
}

impl<S: ReceiptStore> OperationDeduplicator<S> {
    pub fn new(receipt_store: S) -> Self {
        Self { receipt_store }
    }

    pub async fn check(&self, key: &IdempotencyKey) -> DedupResult<DedupDecision> {
        let existing = self.receipt_store.get(key).await?;
        match existing {
            Some(receipt) => Ok(DedupDecision::AlreadyCompleted { receipt }),
            None => Ok(DedupDecision::Execute),
        }
    }

    pub async fn record_completion(&self, key: IdempotencyKey) -> DedupResult<()> {
        let receipt = Receipt::new(key);
        self.receipt_store.store(receipt).await?;
        Ok(())
    }

    pub async fn record_completion_with_hash(
        &self,
        key: IdempotencyKey,
        result_hash: String,
    ) -> DedupResult<()> {
        let receipt = Receipt::with_hash(key, result_hash);
        self.receipt_store.store(receipt).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exactly_once::store::InMemoryReceiptStore;

    #[tokio::test]
    async fn test_check_unknown_key_returns_execute() {
        let store = InMemoryReceiptStore::new();
        let dedup = OperationDeduplicator::new(store);
        let key = IdempotencyKey::from_static("new-op");

        let decision = dedup.check(&key).await.expect("check");
        assert_eq!(decision, DedupDecision::Execute);
    }

    #[tokio::test]
    async fn test_check_completed_key_returns_already_completed() {
        let store = InMemoryReceiptStore::new();
        let key = IdempotencyKey::from_static("done-op");
        store.store(Receipt::new(key.clone())).await.expect("store");

        let dedup = OperationDeduplicator::new(store);
        let decision = dedup.check(&key).await.expect("check");
        match decision {
            DedupDecision::AlreadyCompleted { receipt } => {
                assert_eq!(receipt.key, key);
            }
            other => panic!("Expected AlreadyCompleted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_record_completion_then_check_blocks() {
        let store = InMemoryReceiptStore::new();
        let dedup = OperationDeduplicator::new(store);
        let key = IdempotencyKey::from_static("op-1");

        dedup.record_completion(key.clone()).await.expect("record");

        let decision = dedup.check(&key).await.expect("check");
        assert!(
            matches!(decision, DedupDecision::AlreadyCompleted { .. }),
            "Expected AlreadyCompleted"
        );
    }

    #[tokio::test]
    async fn test_record_completion_with_hash() {
        let store = InMemoryReceiptStore::new();
        let dedup = OperationDeduplicator::new(store);
        let key = IdempotencyKey::from_static("hashed-op");

        dedup
            .record_completion_with_hash(key.clone(), "sha256:abc".to_string())
            .await
            .expect("record");

        let decision = dedup.check(&key).await.expect("check");
        match decision {
            DedupDecision::AlreadyCompleted { receipt } => {
                assert_eq!(receipt.result_hash.as_deref(), Some("sha256:abc"));
            }
            other => panic!("Expected AlreadyCompleted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_duplicate_record_rejected() {
        let store = InMemoryReceiptStore::new();
        let dedup = OperationDeduplicator::new(store);
        let key = IdempotencyKey::from_static("dup");

        dedup
            .record_completion(key.clone())
            .await
            .expect("record 1");
        let result = dedup.record_completion(key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_keys_independent() {
        let store = InMemoryReceiptStore::new();
        let dedup = OperationDeduplicator::new(store);

        let key1 = IdempotencyKey::from_static("op-1");
        let _key2 = IdempotencyKey::from_static("op-2");

        dedup.record_completion(key1).await.expect("record 1");

        let decision1 = dedup
            .check(&IdempotencyKey::from_static("op-1"))
            .await
            .expect("check 1");
        let decision2 = dedup
            .check(&IdempotencyKey::from_static("op-2"))
            .await
            .expect("check 2");

        assert!(matches!(decision1, DedupDecision::AlreadyCompleted { .. }));
        assert_eq!(decision2, DedupDecision::Execute);
    }
}

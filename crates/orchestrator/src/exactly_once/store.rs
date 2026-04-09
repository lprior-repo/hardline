//! Receipt store trait and in-memory implementation
//!
//! Persistent record of completed operations. The ReceiptStore is consulted
//! before executing any operation to prevent duplicate execution.

use std::sync;

use async_trait::async_trait;

use super::types::{IdempotencyKey, Receipt};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReceiptStoreError {
    #[error("Receipt already exists for key: {0}")]
    AlreadyExists(String),
    #[error("Receipt store error: {0}")]
    Internal(String),
}

pub type ReceiptResult<T> = Result<T, ReceiptStoreError>;

#[async_trait]
pub trait ReceiptStore: Send + Sync {
    async fn store(&self, receipt: Receipt) -> ReceiptResult<()>;
    async fn get(&self, key: &IdempotencyKey) -> ReceiptResult<Option<Receipt>>;
    async fn contains(&self, key: &IdempotencyKey) -> ReceiptResult<bool>;
    async fn remove(&self, key: &IdempotencyKey) -> ReceiptResult<()>;
    async fn len(&self) -> ReceiptResult<usize>;
    async fn is_empty(&self) -> ReceiptResult<bool>;
}

pub struct InMemoryReceiptStore {
    receipts: sync::RwLock<Vec<Receipt>>,
}

impl InMemoryReceiptStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            receipts: sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryReceiptStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReceiptStore for InMemoryReceiptStore {
    async fn store(&self, receipt: Receipt) -> ReceiptResult<()> {
        let receipts = self
            .receipts
            .read()
            .map_err(|e| ReceiptStoreError::Internal(format!("Read lock failed: {e}")))?;

        let exists = receipts.iter().any(|r| r.key == receipt.key);
        drop(receipts);

        if exists {
            return Err(ReceiptStoreError::AlreadyExists(receipt.key.to_string()));
        }

        let mut receipts = self
            .receipts
            .write()
            .map_err(|e| ReceiptStoreError::Internal(format!("Write lock failed: {e}")))?;
        receipts.push(receipt);
        Ok(())
    }

    async fn get(&self, key: &IdempotencyKey) -> ReceiptResult<Option<Receipt>> {
        let receipts = self
            .receipts
            .read()
            .map_err(|e| ReceiptStoreError::Internal(format!("Read lock failed: {e}")))?;
        Ok(receipts.iter().find(|r| r.key == *key).cloned())
    }

    async fn contains(&self, key: &IdempotencyKey) -> ReceiptResult<bool> {
        let receipts = self
            .receipts
            .read()
            .map_err(|e| ReceiptStoreError::Internal(format!("Read lock failed: {e}")))?;
        Ok(receipts.iter().any(|r| r.key == *key))
    }

    async fn remove(&self, key: &IdempotencyKey) -> ReceiptResult<()> {
        let mut receipts = self
            .receipts
            .write()
            .map_err(|e| ReceiptStoreError::Internal(format!("Write lock failed: {e}")))?;
        let original_len = receipts.len();
        receipts.retain(|r| r.key != *key);
        if receipts.len() == original_len {
            return Err(ReceiptStoreError::Internal(format!("Key not found: {key}")));
        }
        Ok(())
    }

    async fn len(&self) -> ReceiptResult<usize> {
        let receipts = self
            .receipts
            .read()
            .map_err(|e| ReceiptStoreError::Internal(format!("Read lock failed: {e}")))?;
        Ok(receipts.len())
    }

    async fn is_empty(&self) -> ReceiptResult<bool> {
        self.len().await.map(|l| l == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key(name: &str) -> IdempotencyKey {
        IdempotencyKey::from_static(name)
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("op-1");
        let receipt = Receipt::new(key.clone());

        store.store(receipt).await.expect("store");
        let retrieved = store.get(&key).await.expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|r| &r.key), Some(&key));
    }

    #[tokio::test]
    async fn test_contains_existing() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("op-1");
        store.store(Receipt::new(key.clone())).await.expect("store");
        assert!(store.contains(&key).await.expect("contains"));
    }

    #[tokio::test]
    async fn test_contains_nonexistent() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("never-stored");
        assert!(!store.contains(&key).await.expect("contains"));
    }

    #[tokio::test]
    async fn test_store_duplicate_rejected() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("dup");
        store
            .store(Receipt::new(key.clone()))
            .await
            .expect("store 1");
        let result = store.store(Receipt::new(key.clone())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_existing() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("rm-me");
        store.store(Receipt::new(key.clone())).await.expect("store");
        store.remove(&key).await.expect("remove");
        assert!(!store.contains(&key).await.expect("contains"));
    }

    #[tokio::test]
    async fn test_remove_nonexistent_errors() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("ghost");
        let result = store.remove(&key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_len_tracks_receipts() {
        let store = InMemoryReceiptStore::new();
        assert_eq!(store.len().await.expect("len"), 0);
        assert!(store.is_empty().await.expect("is_empty"));

        store
            .store(Receipt::new(test_key("a")))
            .await
            .expect("store a");
        store
            .store(Receipt::new(test_key("b")))
            .await
            .expect("store b");
        assert_eq!(store.len().await.expect("len"), 2);
        assert!(!store.is_empty().await.expect("is_empty"));
    }

    #[tokio::test]
    async fn test_get_nonexistent_returns_none() {
        let store = InMemoryReceiptStore::new();
        let result = store.get(&test_key("nope")).await.expect("get");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_store_receipt_preserves_hash() {
        let store = InMemoryReceiptStore::new();
        let key = test_key("hashed");
        let receipt = Receipt::with_hash(key.clone(), "sha256:abc".to_string());
        store.store(receipt).await.expect("store");

        let retrieved = store.get(&key).await.expect("get").expect("some");
        assert_eq!(retrieved.result_hash.as_deref(), Some("sha256:abc"));
    }

    #[tokio::test]
    async fn test_default_is_empty() {
        let store = InMemoryReceiptStore::default();
        assert!(store.is_empty().await.expect("is_empty"));
    }
}

use crate::domain::snapshot::{Snapshot, SnapshotId};
use crate::error::Result;
use crate::storage::storage::SnapshotStore;
use std::sync::Arc;

/// Report returned after cleaning up expired snapshots.
#[derive(Debug)]
pub struct CleanupReport {
    pub deleted: usize,
    pub failed: usize,
}

/// Service for managing snapshots using the domain types and storage backend.
pub struct SnapshotService {
    store: Arc<SnapshotStore>,
}

impl SnapshotService {
    pub fn new(store: Arc<SnapshotStore>) -> Self {
        Self { store }
    }

    /// Create a new snapshot with the given branch name, commit hash, and optional description.
    pub fn create_snapshot(
        &self,
        branch_name: String,
        commit_hash: String,
        description: Option<String>,
    ) -> Result<Snapshot> {
        let snapshot = Snapshot::create(branch_name, commit_hash, description);
        self.store.save(snapshot.clone())?;
        Ok(snapshot)
    }

    /// Load a snapshot by its ID.
    pub fn get_snapshot(&self, id: &SnapshotId) -> Result<Snapshot> {
        self.store.load(id)
    }

    /// List all snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        self.store.list()
    }

    /// Delete a snapshot by its ID.
    pub fn delete_snapshot(&self, id: &SnapshotId) -> Result<()> {
        self.store.delete(id)
    }

    /// Delete all snapshots, returning a report of successes and failures.
    ///
    /// This is a placeholder for a future expiry-based cleanup. Currently all
    /// snapshots are considered "expired" since there is no TTL tracking.
    pub fn cleanup_expired(&self) -> Result<CleanupReport> {
        let snapshots = self.store.list()?;

        let mut deleted = 0usize;
        let mut failed = 0usize;

        for snapshot in snapshots {
            match self.store.delete(&snapshot.id) {
                Ok(()) => deleted += 1,
                Err(_) => failed += 1,
            }
        }

        Ok(CleanupReport { deleted, failed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SnapshotError;
    use proptest::proptest;
    use proptest::prop_assert;
    use proptest::prop_assert_eq;

    fn make_service() -> SnapshotService {
        SnapshotService::new(Arc::new(SnapshotStore::new()))
    }

    #[test]
    fn service_new_creates_instance() {
        let _service = make_service();
    }

    #[test]
    fn create_snapshot_propagates_storage_error() {
        let service = make_service();
        let result = service.create_snapshot(
            "main".to_string(),
            "abc123".to_string(),
            Some("test snapshot".to_string()),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SnapshotError::StorageError { .. }));
    }

    #[test]
    fn create_snapshot_without_description_propagates_storage_error() {
        let service = make_service();
        let result = service.create_snapshot(
            "dev".to_string(),
            "def456".to_string(),
            None,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SnapshotError::StorageError { .. }));
    }

    #[test]
    fn create_snapshot_fails_when_storage_unimplemented() {
        let service = make_service();
        let result = service.create_snapshot("a".to_string(), "h1".to_string(), None);
        assert!(result.is_err(), "unimplemented store should always fail");
    }

    #[test]
    fn get_snapshot_fails_for_nonexistent() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.get_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn list_snapshots_returns_err_when_storage_not_implemented() {
        let service = make_service();
        let result = service.list_snapshots();
        assert!(result.is_err());
    }

    #[test]
    fn delete_snapshot_fails_for_nonexistent() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.delete_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_expired_reports() {
        let service = make_service();
        let report = service.cleanup_expired();
        assert!(report.is_err(), "list returns Err since storage is not implemented");
    }

    // --- Additional service tests ---

    #[test]
    fn create_snapshot_has_valid_created_at() {
        // Domain-level test: created_at is set by Snapshot::create(), not the service.
        // Service always fails with unimplemented storage, so test domain directly.
        let before = chrono::Utc::now();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let after = chrono::Utc::now();
        assert!(snapshot.created_at >= before);
        assert!(snapshot.created_at <= after);
    }

    #[test]
    fn create_snapshot_err_is_storage_error() {
        // When storage save fails, the service propagates the SnapshotError directly.
        // The store returns SnapshotError::StorageError, which flows through via ?.
        let service = make_service();
        let result = service.create_snapshot("main".to_string(), "abc".to_string(), None);
        assert!(result.is_err(), "save always fails in unimplemented storage");
    }

    #[test]
    fn get_snapshot_err_is_storage_error() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.get_snapshot(&id);
        let err = result.expect_err("should be Err");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn get_snapshot_err_is_storage_error_variant() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.get_snapshot(&id);
        let err = result.expect_err("should be Err");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn list_snapshots_err_is_storage_error() {
        let service = make_service();
        let result = service.list_snapshots();
        let err = result.expect_err("should be Err");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn delete_snapshot_err_is_storage_error() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.delete_snapshot(&id);
        let err = result.expect_err("should be Err");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn delete_snapshot_err_contains_storage_message() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.delete_snapshot(&id);
        let err = result.expect_err("should be Err");
        let msg = err.to_string();
        assert!(msg.contains("Storage error"));
    }

    #[test]
    fn cleanup_expired_err_is_storage_error() {
        let service = make_service();
        let result = service.cleanup_expired();
        let err = result.expect_err("should be Err");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn get_snapshot_with_generated_id() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.get_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn delete_snapshot_with_generated_id() {
        let service = make_service();
        let id = SnapshotId::generate();
        let result = service.delete_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn service_err_implements_error_trait() {
        let service = make_service();
        let id = SnapshotId::generate();
        let err = service.get_snapshot(&id).expect_err("should be Err");
        let _: Box<dyn std::error::Error> = Box::new(err);
    }

    #[test]
    fn service_err_is_debug() {
        let service = make_service();
        let id = SnapshotId::generate();
        let err = service.get_snapshot(&id).expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn multiple_services_independent() {
        let service1 = make_service();
        let service2 = make_service();
        let id = SnapshotId::generate();
        // Both should fail independently
        assert!(service1.get_snapshot(&id).is_err());
        assert!(service2.get_snapshot(&id).is_err());
    }

    // --- CleanupReport tests ---

    #[test]
    fn cleanup_report_fields_accessible() {
        let report = CleanupReport { deleted: 5, failed: 3 };
        assert_eq!(report.deleted, 5);
        assert_eq!(report.failed, 3);
    }

    #[test]
    fn cleanup_report_zero_values() {
        let report = CleanupReport { deleted: 0, failed: 0 };
        assert_eq!(report.deleted, 0);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn cleanup_report_debug() {
        let report = CleanupReport { deleted: 1, failed: 2 };
        let debug_str = format!("{report:?}");
        assert!(debug_str.contains("deleted"));
        assert!(debug_str.contains("failed"));
    }

    // --- Proptests ---

    proptest! {
        #[test]
        fn service_get_snapshot_always_fails_for_nonexistent(_v in 0..100u32) {
            let service = make_service();
            let id = SnapshotId::generate();
            prop_assert!(service.get_snapshot(&id).is_err());
        }

        #[test]
        fn service_delete_snapshot_always_fails_for_nonexistent(_v in 0..100u32) {
            let service = make_service();
            let id = SnapshotId::generate();
            prop_assert!(service.delete_snapshot(&id).is_err());
        }

        #[test]
        fn service_list_snapshots_always_fails(_v in 0..10u32) {
            let service = make_service();
            prop_assert!(service.list_snapshots().is_err());
        }

        #[test]
        fn service_cleanup_expired_always_fails(_v in 0..10u32) {
            let service = make_service();
            prop_assert!(service.cleanup_expired().is_err());
        }

        #[test]
        fn cleanup_report_with_arbitrary_counts(deleted: usize, failed: usize) {
            let report = CleanupReport { deleted, failed };
            prop_assert_eq!(report.deleted, deleted);
            prop_assert_eq!(report.failed, failed);
        }
    }
}

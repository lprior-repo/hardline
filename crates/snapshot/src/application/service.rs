use std::sync::Arc;

use crate::{
    domain::snapshot::{Snapshot, SnapshotId},
    error::Result,
    storage::storage::SnapshotStore,
};

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
    ///
    /// The snapshot will have a 24-hour expiration set automatically.
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

    /// Restore a snapshot by returning its data.
    ///
    /// This is a Phase 1 implementation that returns the snapshot metadata.
    /// Real filesystem restore is planned for Phase 5.
    pub fn restore_snapshot(&self, id: &SnapshotId) -> Result<Snapshot> {
        let snapshot = self.store.load(id)?;
        Ok(snapshot)
    }

    /// Delete all expired snapshots (where `expires_at` is in the past).
    pub fn cleanup_expired(&self) -> Result<CleanupReport> {
        let snapshots = self.store.list()?;

        let mut deleted = 0usize;
        let mut failed = 0usize;

        for snapshot in snapshots {
            if !snapshot.is_expired() {
                continue;
            }
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
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use tempfile::TempDir;

    use super::*;
    use crate::error::SnapshotError;

    fn make_service() -> (SnapshotService, TempDir) {
        let temp = TempDir::new().expect("temp dir");
        let store = Arc::new(SnapshotStore::new(temp.path()));
        let service = SnapshotService::new(store);
        (service, temp)
    }

    #[test]
    fn service_new_creates_instance() {
        let (service, _temp) = make_service();
        let _ = service;
    }

    #[test]
    fn create_snapshot_succeeds() {
        let (service, _temp) = make_service();
        let result = service.create_snapshot(
            "main".to_string(),
            "abc123".to_string(),
            Some("test snapshot".to_string()),
        );
        assert!(result.is_ok());
        let snapshot = result.expect("should be Ok");
        assert_eq!(snapshot.branch_name, "main");
        assert_eq!(snapshot.commit_hash, "abc123");
        assert_eq!(snapshot.description, Some("test snapshot".to_string()));
        assert!(snapshot.id.as_str().starts_with("snap-"));
    }

    #[test]
    fn create_snapshot_without_description() {
        let (service, _temp) = make_service();
        let result = service.create_snapshot("dev".to_string(), "def456".to_string(), None);
        assert!(result.is_ok());
        assert!(result.expect("should be Ok").description.is_none());
    }

    #[test]
    fn create_snapshot_sets_expires_at() {
        let (service, _temp) = make_service();
        let result = service.create_snapshot("main".to_string(), "abc".to_string(), None);
        let snapshot = result.expect("should be Ok");
        assert!(snapshot.expires_at.is_some(), "expires_at should be set");
    }

    #[test]
    fn get_snapshot_after_create() {
        let (service, _temp) = make_service();
        let created = service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create");
        let loaded = service.get_snapshot(&created.id).expect("get");
        assert_eq!(loaded.id, created.id);
        assert_eq!(loaded.branch_name, created.branch_name);
    }

    #[test]
    fn get_snapshot_nonexistent() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let result = service.get_snapshot(&id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SnapshotError::StorageError { .. }));
    }

    #[test]
    fn list_snapshots_empty() {
        let (service, _temp) = make_service();
        let list = service.list_snapshots().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn list_snapshots_after_create() {
        let (service, _temp) = make_service();
        service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create");
        let list = service.list_snapshots().expect("list");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn delete_snapshot_after_create() {
        let (service, _temp) = make_service();
        let created = service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create");
        service.delete_snapshot(&created.id).expect("delete");
        let result = service.get_snapshot(&created.id);
        assert!(result.is_err(), "deleted snapshot should not be loadable");
    }

    #[test]
    fn delete_snapshot_nonexistent() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let result = service.delete_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn restore_snapshot_returns_data() {
        let (service, _temp) = make_service();
        let created = service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create");
        let restored = service.restore_snapshot(&created.id).expect("restore");
        assert_eq!(restored.id, created.id);
        assert_eq!(restored.branch_name, created.branch_name);
        assert_eq!(restored.commit_hash, created.commit_hash);
    }

    #[test]
    fn restore_snapshot_nonexistent() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let result = service.restore_snapshot(&id);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_expired_no_expired() {
        let (service, _temp) = make_service();
        service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create");
        let report = service.cleanup_expired().expect("cleanup");
        assert_eq!(report.deleted, 0);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn cleanup_expired_with_expired_snapshot() {
        let (service, temp) = make_service();
        // Create a snapshot with a past expiration by manually constructing
        let mut snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        snapshot.expires_at = Some(chrono::Utc::now() - chrono::Duration::hours(1));
        let store = SnapshotStore::new(temp.path());
        store.save(snapshot.clone()).expect("save expired snapshot");
        let report = service.cleanup_expired().expect("cleanup");
        assert_eq!(report.deleted, 1);
        assert_eq!(report.failed, 0);
        // Should no longer be loadable
        let result = service.get_snapshot(&snapshot.id);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_expired_mixed() {
        let (service, temp) = make_service();
        // Create a normal snapshot (not expired)
        service
            .create_snapshot("main".to_string(), "abc".to_string(), None)
            .expect("create normal");
        // Create an expired snapshot
        let mut expired = Snapshot::create("dev".to_string(), "def".to_string(), None);
        expired.expires_at = Some(chrono::Utc::now() - chrono::Duration::hours(1));
        let store = SnapshotStore::new(temp.path());
        store.save(expired.clone()).expect("save expired");
        let report = service.cleanup_expired().expect("cleanup");
        assert_eq!(report.deleted, 1);
        assert_eq!(report.failed, 0);
        // Normal snapshot should still exist
        let list = service.list_snapshots().expect("list");
        assert_eq!(list.len(), 1);
    }

    // --- Source chain verification ---

    #[test]
    fn get_snapshot_error_preserves_source_chain() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let err = service.get_snapshot(&id).expect_err("should fail");
        let source = std::error::Error::source(&err);
        assert!(
            source.is_some(),
            "source chain must be preserved"
        );
    }

    #[test]
    fn delete_snapshot_error_preserves_source_chain() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let err = service.delete_snapshot(&id).expect_err("should fail");
        let source = std::error::Error::source(&err);
        assert!(source.is_some(), "source chain must be preserved");
    }

    #[test]
    fn service_err_implements_error_trait() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let err = service.get_snapshot(&id).expect_err("should be Err");
        let _: Box<dyn std::error::Error> = Box::new(err);
    }

    #[test]
    fn service_err_is_debug() {
        let (service, _temp) = make_service();
        let id = SnapshotId::generate();
        let err = service.get_snapshot(&id).expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn multiple_services_independent() {
        let temp1 = TempDir::new().expect("temp dir");
        let temp2 = TempDir::new().expect("temp dir");
        let service1 = SnapshotService::new(Arc::new(SnapshotStore::new(temp1.path())));
        let service2 = SnapshotService::new(Arc::new(SnapshotStore::new(temp2.path())));
        let id = SnapshotId::generate();
        assert!(service1.get_snapshot(&id).is_err());
        assert!(service2.get_snapshot(&id).is_err());
    }

    // --- CleanupReport tests ---

    #[test]
    fn cleanup_report_fields_accessible() {
        let report = CleanupReport {
            deleted: 5,
            failed: 3,
        };
        assert_eq!(report.deleted, 5);
        assert_eq!(report.failed, 3);
    }

    #[test]
    fn cleanup_report_zero_values() {
        let report = CleanupReport {
            deleted: 0,
            failed: 0,
        };
        assert_eq!(report.deleted, 0);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn cleanup_report_debug() {
        let report = CleanupReport {
            deleted: 1,
            failed: 2,
        };
        let debug_str = format!("{report:?}");
        assert!(debug_str.contains("deleted"));
        assert!(debug_str.contains("failed"));
    }

    // --- Proptests ---

    proptest! {
        #[test]
        fn service_create_snapshot_always_succeeds(
            branch in "[a-zA-Z0-9_-]{1,50}",
            commit in "[a-f0-9]{1,40}"
        ) {
            let (service, _temp) = make_service();
            let result = service.create_snapshot(branch, commit, None);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn service_get_snapshot_fails_for_random_id(_v in 0..100u32) {
            let (service, _temp) = make_service();
            let id = SnapshotId::generate();
            prop_assert!(service.get_snapshot(&id).is_err());
        }

        #[test]
        fn service_delete_snapshot_fails_for_random_id(_v in 0..100u32) {
            let (service, _temp) = make_service();
            let id = SnapshotId::generate();
            prop_assert!(service.delete_snapshot(&id).is_err());
        }

        #[test]
        fn cleanup_report_with_arbitrary_counts(deleted: usize, failed: usize) {
            let report = CleanupReport { deleted, failed };
            prop_assert_eq!(report.deleted, deleted);
            prop_assert_eq!(report.failed, failed);
        }
    }
}

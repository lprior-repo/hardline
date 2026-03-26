use crate::domain::snapshot::{
    BranchName, CommitHash, Snapshot, SnapshotFilter, SnapshotId, SnapshotLocation,
    SnapshotMetadata, SnapshotState, SnapshotType, StorageType, WorkspaceId,
};
use crate::error::{RepoResult, Result, SnapshotError, SnapshotRepoError};
use crate::storage::SnapshotStorage;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;

pub struct CleanupReport {
    pub deleted: usize,
    pub failed: usize,
}

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    fn save(&self, snapshot: &Snapshot) -> RepoResult<()>;
    fn find_by_id(&self, id: &SnapshotId) -> RepoResult<Option<Snapshot>>;
    fn list_by_workspace(
        &self,
        workspace_id: &WorkspaceId,
        filter: Option<SnapshotFilter>,
    ) -> RepoResult<Vec<Snapshot>>;
    fn find_expired(&self) -> RepoResult<Vec<Snapshot>>;
    fn delete(&self, id: &SnapshotId) -> RepoResult<()>;
}

pub struct SnapshotService<R: SnapshotRepository, S: SnapshotStorage> {
    snapshot_repo: Arc<R>,
    storage: Arc<S>,
}

impl<R: SnapshotRepository, S: SnapshotStorage> SnapshotService<R, S> {
    pub fn new(snapshot_repo: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            snapshot_repo,
            storage,
        }
    }

    pub async fn create_pre_operation_snapshot(
        &self,
        workspace_id: WorkspaceId,
        operation_name: &str,
        workspace_path: &std::path::Path,
        commit_hash: CommitHash,
        branch: BranchName,
    ) -> Result<Snapshot> {
        let now = Utc::now();
        let snapshot = Snapshot {
            id: SnapshotId::new(),
            workspace_id: workspace_id.clone(),
            snapshot_type: SnapshotType::Checkpoint,
            name: format!("pre-{}-{}", operation_name, now.timestamp()),
            description: Some(format!("Automatic checkpoint before {}", operation_name)),
            state: SnapshotState::Creating,
            created_at: now,
            expires_at: Some(now + Duration::hours(24)),
            metadata: SnapshotMetadata {
                size_bytes: 0,
                commit_hash,
                branch,
                operation_id: None,
                tags: vec![operation_name.to_string()],
            },
            location: SnapshotLocation {
                storage_type: StorageType::Local,
                path: std::path::PathBuf::new(),
            },
        };

        self.snapshot_repo
            .save(&snapshot)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        let result = self
            .storage
            .create_snapshot(&snapshot, workspace_path)
            .await;

        match result {
            Ok(size) => {
                let mut updated_snapshot = snapshot;
                updated_snapshot.state = SnapshotState::Ready;
                updated_snapshot.metadata.size_bytes = size;
                updated_snapshot.location.path = self
                    .storage
                    .get_snapshot_path(&updated_snapshot)
                    .unwrap_or_default();

                self.snapshot_repo
                    .save(&updated_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Ok(updated_snapshot)
            }
            Err(e) => {
                let mut failed_snapshot = snapshot;
                failed_snapshot.state = SnapshotState::Failed;

                self.snapshot_repo
                    .save(&failed_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Err(SnapshotError::CreationFailed(e.to_string()))
            }
        }
    }

    pub async fn create_manual_snapshot(
        &self,
        workspace_id: WorkspaceId,
        name: String,
        description: Option<String>,
        workspace_path: &std::path::Path,
        commit_hash: CommitHash,
        branch: BranchName,
    ) -> Result<Snapshot> {
        let now = Utc::now();
        let snapshot = Snapshot {
            id: SnapshotId::new(),
            workspace_id: workspace_id.clone(),
            snapshot_type: SnapshotType::Manual,
            name,
            description,
            state: SnapshotState::Creating,
            created_at: now,
            expires_at: None,
            metadata: SnapshotMetadata {
                size_bytes: 0,
                commit_hash,
                branch,
                operation_id: None,
                tags: vec![],
            },
            location: SnapshotLocation {
                storage_type: StorageType::Local,
                path: std::path::PathBuf::new(),
            },
        };

        self.snapshot_repo
            .save(&snapshot)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        let result = self
            .storage
            .create_snapshot(&snapshot, workspace_path)
            .await;

        match result {
            Ok(size) => {
                let mut updated_snapshot = snapshot;
                updated_snapshot.state = SnapshotState::Ready;
                updated_snapshot.metadata.size_bytes = size;
                updated_snapshot.location.path = self
                    .storage
                    .get_snapshot_path(&updated_snapshot)
                    .unwrap_or_default();

                self.snapshot_repo
                    .save(&updated_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Ok(updated_snapshot)
            }
            Err(e) => {
                let mut failed_snapshot = snapshot;
                failed_snapshot.state = SnapshotState::Failed;

                self.snapshot_repo
                    .save(&failed_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Err(SnapshotError::CreationFailed(e.to_string()))
            }
        }
    }

    pub async fn restore_snapshot(
        &self,
        snapshot_id: &SnapshotId,
        target_path: &std::path::Path,
    ) -> Result<Snapshot> {
        let snapshot = self
            .snapshot_repo
            .find_by_id(snapshot_id)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?
            .ok_or_else(|| SnapshotError::NotFound(snapshot_id.to_string()))?;

        if snapshot.state != SnapshotState::Ready {
            return Err(SnapshotError::SnapshotNotReady(
                snapshot_id.to_string(),
            ));
        }

        let mut restoring_snapshot = snapshot;
        restoring_snapshot.state = SnapshotState::Restoring;

        self.snapshot_repo
            .save(&restoring_snapshot)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        let result = self.storage.restore_snapshot(&restoring_snapshot, target_path).await;

        match result {
            Ok(()) => {
                restoring_snapshot.state = SnapshotState::Restored;

                self.snapshot_repo
                    .save(&restoring_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Ok(restoring_snapshot)
            }
            Err(e) => {
                restoring_snapshot.state = SnapshotState::Ready;

                self.snapshot_repo
                    .save(&restoring_snapshot)
                    .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

                Err(SnapshotError::RestoreFailed(e.to_string()))
            }
        }
    }

    pub fn list_snapshots(
        &self,
        workspace_id: &WorkspaceId,
        filter: Option<SnapshotFilter>,
    ) -> Result<Vec<Snapshot>> {
        self.snapshot_repo
            .list_by_workspace(workspace_id, filter)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))
    }

    pub fn get_snapshot(&self, snapshot_id: &SnapshotId) -> Result<Option<Snapshot>> {
        self.snapshot_repo
            .find_by_id(snapshot_id)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))
    }

    pub async fn cleanup_expired(&self) -> Result<CleanupReport> {
        let expired = self
            .snapshot_repo
            .find_expired()
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        let mut deleted = 0;
        let mut failed = 0;

        for snapshot in expired {
            match self.storage.delete_snapshot(&snapshot).await {
                Ok(()) => {
                    if self
                        .snapshot_repo
                        .delete(&snapshot.id)
                        .is_ok()
                    {
                        deleted += 1;
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        Ok(CleanupReport { deleted, failed })
    }

    pub async fn delete_snapshot(&self, snapshot_id: &SnapshotId) -> Result<()> {
        let snapshot = self
            .snapshot_repo
            .find_by_id(snapshot_id)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?
            .ok_or_else(|| SnapshotError::NotFound(snapshot_id.to_string()))?;

        self.storage
            .delete_snapshot(&snapshot)
            .await
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        self.snapshot_repo
            .delete(snapshot_id)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::snapshot::{SnapshotId, WorkspaceId};
    use crate::storage::SnapshotStorage;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct InMemorySnapshotRepository {
        snapshots: Mutex<HashMap<String, Snapshot>>,
    }

    impl InMemorySnapshotRepository {
        fn new() -> Self {
            Self {
                snapshots: Mutex::new(HashMap::new()),
            }
        }
    }

    impl Default for InMemorySnapshotRepository {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl SnapshotRepository for InMemorySnapshotRepository {
        fn save(&self, snapshot: &Snapshot) -> RepoResult<()> {
            let mut snapshots = self.snapshots.lock().map_err(|e| SnapshotRepoError::SaveFailed(e.to_string()))?;
            snapshots.insert(snapshot.id.as_str().to_string(), snapshot.clone());
            Ok(())
        }

        fn find_by_id(&self, id: &SnapshotId) -> RepoResult<Option<Snapshot>> {
            let snapshots = self.snapshots.lock().map_err(|e| SnapshotRepoError::NotFound(e.to_string()))?;
            Ok(snapshots.get(id.as_str()).cloned())
        }

        fn list_by_workspace(
            &self,
            workspace_id: &WorkspaceId,
            _filter: Option<SnapshotFilter>,
        ) -> RepoResult<Vec<Snapshot>> {
            let snapshots = self.snapshots.lock().map_err(|e| SnapshotRepoError::NotFound(e.to_string()))?;
            Ok(snapshots
                .values()
                .filter(|s| s.workspace_id.as_str() == workspace_id.as_str())
                .cloned()
                .collect())
        }

        fn find_expired(&self) -> RepoResult<Vec<Snapshot>> {
            let snapshots = self.snapshots.lock().map_err(|e| SnapshotRepoError::NotFound(e.to_string()))?;
            let now = Utc::now();
            Ok(snapshots
                .values()
                .filter(|s| s.expires_at.map(|e| e < now).unwrap_or(false))
                .cloned()
                .collect())
        }

        fn delete(&self, id: &SnapshotId) -> RepoResult<()> {
            let mut snapshots = self.snapshots.lock().map_err(|e| SnapshotRepoError::DeleteFailed(e.to_string()))?;
            snapshots.remove(id.as_str());
            Ok(())
        }
    }

    struct NoOpSnapshotStorage;

    #[async_trait]
    impl SnapshotStorage for NoOpSnapshotStorage {
        async fn create_snapshot(
            &self,
            _snapshot: &Snapshot,
            _source_path: &std::path::Path,
        ) -> crate::error::StorageResult<u64> {
            Ok(0)
        }

        async fn restore_snapshot(
            &self,
            _snapshot: &Snapshot,
            _target_path: &std::path::Path,
        ) -> crate::error::StorageResult<()> {
            Ok(())
        }

        async fn delete_snapshot(&self, _snapshot: &Snapshot) -> crate::error::StorageResult<()> {
            Ok(())
        }

        async fn get_snapshot_size(&self, _snapshot: &Snapshot) -> crate::error::StorageResult<u64> {
            Ok(0)
        }

        fn get_snapshot_path(&self, _snapshot: &Snapshot) -> Option<std::path::PathBuf> {
            Some(std::path::PathBuf::new())
        }
    }

    #[tokio::test]
    async fn test_create_pre_operation_snapshot() {
        let repo = Arc::new(InMemorySnapshotRepository::new());
        let storage = Arc::new(NoOpSnapshotStorage);
        let service = SnapshotService::new(repo.clone(), storage);

        let workspace_id = WorkspaceId::new("test-ws");
        let result = service
            .create_pre_operation_snapshot(
                workspace_id.clone(),
                "rebase",
                std::path::Path::new("/tmp"),
                CommitHash::new("abc123"),
                BranchName::new("main"),
            )
            .await;

        assert!(result.is_ok());
        let snapshot = result.expect("should succeed");
        assert_eq!(snapshot.workspace_id.as_str(), "test-ws");
        assert_eq!(snapshot.snapshot_type, SnapshotType::Checkpoint);
    }
}

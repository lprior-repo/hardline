# ADR-013: Snapshot System - Backup/Restore/Checkpoint

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs a snapshot system for:

1. **Crash recovery** - Restore workspace to known good state
2. **Checkpointing** - Save progress before risky operations
3. **Branching backup** - Backup before restack or rebase
4. **Agent recovery** - Restore agent workspace after crash
5. **Rollback** - Undo failed operations

The architecture spec mentions snapshot but doesn't define the implementation. This ADR formalizes the snapshot system.

---

## Decision

### Snapshot Entity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub workspace_id: WorkspaceId,
    pub snapshot_type: SnapshotType,
    pub name: String,
    pub description: Option<String>,
    pub state: SnapshotState,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: SnapshotMetadata,
    pub location: SnapshotLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    Automatic,   // System-created (pre-operation)
    Manual,      // User-created
    Checkpoint,  // Pre-rebase/pre-merge
    Full,        // Complete workspace copy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotState {
    Creating,   // Snapshot in progress
    Ready,      // Snapshot complete
    Restoring,  // Restore in progress
    Restored,   // Successfully restored
    Expired,    // Past expiration date
    Failed,     // Creation or restore failed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub size_bytes: u64,
    pub commit_hash: CommitHash,
    pub branch: BranchName,
    pub operation_id: Option<OperationId>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotLocation {
    pub storage_type: StorageType,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    Local,       // Local filesystem
    Remote,      // Remote backup storage
}
```

### Snapshot Operations

```rust
pub struct SnapshotService<R: SnapshotRepository, V: VcsBackend> {
    snapshot_repo: R,
    vcs: V,
    storage: Arc<dyn SnapshotStorage>,
}

impl<R: SnapshotRepository, V: VcsBackend> SnapshotService<R, V> {
    /// Create snapshot before risky operation
    pub async fn create_pre_operation_snapshot(
        &self,
        workspace_id: WorkspaceId,
        operation_name: &str,
    ) -> Result<Snapshot, SnapshotError> {
        let workspace = self.workspace_repo.find_by_id(&workspace_id)?
            .ok_or(SnapshotError::WorkspaceNotFound(workspace_id))?;
        
        // Create snapshot metadata
        let snapshot = Snapshot {
            id: SnapshotId::new(),
            workspace_id,
            snapshot_type: SnapshotType::Checkpoint,
            name: format!("pre-{}-{}", operation_name, Utc::now().timestamp()),
            description: Some(format!("Automatic checkpoint before {}", operation_name)),
            state: SnapshotState::Creating,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            metadata: SnapshotMetadata {
                size_bytes: 0,  // Will be updated after creation
                commit_hash: self.vcs.get_current_commit(&workspace.path)?,
                branch: self.vcs.current_branch(&workspace.path)?,
                operation_id: None,
                tags: vec![operation_name.to_string()],
            },
            location: SnapshotLocation {
                storage_type: StorageType::Local,
                path: self.snapshot_dir(&workspace_id)?,
            },
        };
        
        // Save snapshot record
        self.snapshot_repo.save(&snapshot)?;
        
        // Create actual snapshot (filesystem copy)
        let result = self.storage.create_snapshot(&snapshot, &workspace.path).await;
        
        match result {
            Ok(size) => {
                let mut snapshot = snapshot;
                snapshot.state = SnapshotState::Ready;
                snapshot.metadata.size_bytes = size;
                self.snapshot_repo.save(&snapshot)?;
                Ok(snapshot)
            }
            Err(e) => {
                let mut snapshot = snapshot;
                snapshot.state = SnapshotState::Failed;
                self.snapshot_repo.save(&snapshot)?;
                Err(SnapshotError::CreationFailed(e.to_string()))
            }
        }
    }
    
    /// Restore workspace from snapshot
    pub async fn restore_snapshot(
        &self,
        snapshot_id: SnapshotId,
    ) -> Result<Workspace, SnapshotError> {
        let snapshot = self.snapshot_repo.find_by_id(&snapshot_id)?
            .ok_or(SnapshotError::SnapshotNotFound(snapshot_id))?;
        
        if snapshot.state != SnapshotState::Ready {
            return Err(SnapshotError::SnapshotNotReady(snapshot_id));
        }
        
        let workspace = self.workspace_repo.find_by_id(&snapshot.workspace_id)?
            .ok_or(SnapshotError::WorkspaceNotFound(snapshot.workspace_id))?;
        
        // Mark snapshot as restoring
        let mut snapshot = snapshot;
        snapshot.state = SnapshotState::Restoring;
        self.snapshot_repo.save(&snapshot)?;
        
        // Perform restore
        let result = self.storage.restore_snapshot(&snapshot, &workspace.path).await;
        
        match result {
            Ok(()) => {
                // Update snapshot state
                snapshot.state = SnapshotState::Restored;
                self.snapshot_repo.save(&snapshot)?;
                
                // Reload workspace
                let workspace = self.workspace_repo.find_by_id(&snapshot.workspace_id)?
                    .ok_or(SnapshotError::WorkspaceNotFound(snapshot.workspace_id))?;
                
                Ok(workspace)
            }
            Err(e) => {
                snapshot.state = SnapshotState::Ready;  // Revert to Ready
                self.snapshot_repo.save(&snapshot)?;
                Err(SnapshotError::RestoreFailed(e.to_string()))
            }
        }
    }
    
    /// List snapshots for workspace
    pub fn list_snapshots(
        &self,
        workspace_id: WorkspaceId,
        filter: Option<SnapshotFilter>,
    ) -> Result<Vec<Snapshot>, SnapshotError> {
        self.snapshot_repo.list_by_workspace(&workspace_id, filter)
    }
    
    /// Delete old snapshots
    pub async fn cleanup_expired(&self) -> Result<CleanupReport, SnapshotError> {
        let expired = self.snapshot_repo.find_expired()?;
        
        let mut deleted = 0;
        let mut failed = 0;
        
        for snapshot in expired {
            match self.storage.delete_snapshot(&snapshot).await {
                Ok(()) => {
                    self.snapshot_repo.delete(&snapshot.id)?;
                    deleted += 1;
                }
                Err(e) => {
                    failed += 1;
                    // Log but continue
                }
            }
        }
        
        Ok(CleanupReport { deleted, failed })
    }
}
```

### Snapshot Storage Trait

```rust
pub trait SnapshotStorage: Send + Sync {
    /// Create snapshot from source path
    async fn create_snapshot(
        &self,
        snapshot: &Snapshot,
        source_path: &Path,
    ) -> Result<u64, SnapshotStorageError>;
    
    /// Restore snapshot to target path
    async fn restore_snapshot(
        &self,
        snapshot: &Snapshot,
        target_path: &Path,
    ) -> Result<(), SnapshotStorageError>;
    
    /// Delete snapshot
    async fn delete_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> Result<(), SnapshotStorageError>;
    
    /// Get snapshot size
    async fn get_snapshot_size(
        &self,
        snapshot: &Snapshot,
    ) -> Result<u64, SnapshotStorageError>;
}
```

### Local Storage Implementation

```rust
pub struct LocalSnapshotStorage {
    base_path: PathBuf,
}

impl LocalSnapshotStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    fn snapshot_path(&self, snapshot: &Snapshot) -> PathBuf {
        self.base_path
            .join(snapshot.workspace_id.as_str())
            .join(snapshot.id.as_str())
    }
}

impl SnapshotStorage for LocalSnapshotStorage {
    async fn create_snapshot(
        &self,
        snapshot: &Snapshot,
        source_path: &Path,
    ) -> Result<u64, SnapshotStorageError> {
        let dest = self.snapshot_path(snapshot);
        
        // Use reflink/copy-on-write if available, else full copy
        let copy_options = fs_extra::CopyOptions {
            overwrite: false,
            skip_exist: false,
            buffer_size: 64 * 1024,  // 64KB buffer
            ..Default::default()
        };
        
        tokio::task::spawn_blocking({
            let dest = dest.clone();
            let source_path = source_path.to_path_buf();
            move || {
                fs_extra::dir::copy(&source_path, &dest, &copy_options)
                    .map(|count| {
                        // Calculate total size
                        calculate_dir_size(&dest)
                    })
            }
        })
        .await
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))?
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))
    }
    
    async fn restore_snapshot(
        &self,
        snapshot: &Snapshot,
        target_path: &Path,
    ) -> Result<(), SnapshotStorageError> {
        let source = self.snapshot_path(snapshot);
        
        // Remove current content
        if target_path.exists() {
            tokio::fs::remove_dir_all(target_path).await
                .map_err(|e| SnapshotStorageError::Io(e.to_string()))?;
        }
        
        // Copy snapshot to target
        let copy_options = fs_extra::CopyOptions {
            overwrite: false,
            skip_exist: false,
            buffer_size: 64 * 1024,
            ..Default::default()
        };
        
        tokio::task::spawn_blocking({
            let source = source.clone();
            let target_path = target_path.to_path_buf();
            move || {
                fs_extra::dir::copy(&source, &target_path, &copy_options)
                    .map(|_| ())
            }
        })
        .await
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))?
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))
    }
    
    async fn delete_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> Result<(), SnapshotStorageError> {
        let path = self.snapshot_path(snapshot);
        
        tokio::fs::remove_dir_all(&path).await
            .map_err(|e| SnapshotStorageError::Io(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_snapshot_size(
        &self,
        snapshot: &Snapshot,
    ) -> Result<u64, SnapshotStorageError> {
        let path = self.snapshot_path(snapshot);
        
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || calculate_dir_size(&path)
        })
        .await
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))?
        .map_err(|e| SnapshotStorageError::Io(e.to_string()))
    }
}

fn calculate_dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in walkdir(path)? {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }
    Ok(total)
}
```

### Repository Trait

```rust
pub trait SnapshotRepository: Send + Sync {
    fn save(&self, snapshot: &Snapshot) -> Result<(), SnapshotRepoError>;
    fn find_by_id(&self, id: &SnapshotId) -> Result<Option<Snapshot>, SnapshotRepoError>;
    fn list_by_workspace(&self, workspace_id: &WorkspaceId, filter: Option<SnapshotFilter>) 
        -> Result<Vec<Snapshot>, SnapshotRepoError>;
    fn find_expired(&self) -> Result<Vec<Snapshot>, SnapshotRepoError>;
    fn delete(&self, id: &SnapshotId) -> Result<(), SnapshotRepoError>;
}

pub struct SnapshotFilter {
    pub snapshot_type: Option<SnapshotType>,
    pub state: Option<SnapshotState>,
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
}
```

---

## Variants

### Variant A: Full Filesystem Copy (CHOSEN)

```rust
// Copy entire .git + working copy
// Simple, reliable, works with any VCS
```

**Chosen because:**
- Works with Git
- Simple to implement and debug
- No special VCS knowledge needed

### Variant B: Git Ref-based Backup

```rust
// Save refs, restore by recreating from refs
// Problem: Doesn't preserve working copy state
```

**Rejected because:**
- Doesn't capture uncommitted changes
- Complex for non-ref-based workflows

### Variant C: VCS-native Snapshot

```rust
// Use git stash or similar VCS-native mechanism
// Problem: VCS-specific, not portable
```

**Rejected because:**
- VCS-specific implementation details
- Need unified abstraction

---

## Invariants

### Snapshot Lifecycle Invariants

```rust
/// INVARIANT: Creating snapshot cannot be restored
fn assert_creating_not_restorable(snapshot: &Snapshot) {
    if snapshot.state == SnapshotState::Creating {
        assert_ne!(snapshot.state, SnapshotState::Restoring);
    }
}

/// INVARIANT: Only Ready snapshots can be restored
fn assert_only_ready_restorable(snapshot: &Snapshot) {
    if snapshot.state == SnapshotState::Restoring {
        assert_eq!(snapshot.state, SnapshotState::Ready);
    }
}

/// INVARIANT: Restored snapshot has matching state
fn assert_restored_state(snapshot: &Snapshot) {
    if snapshot.state == SnapshotState::Restored {
        assert_eq!(snapshot.state, SnapshotState::Restored);
    }
}
```

### Expiration Invariants

```rust
/// INVARIANT: Expired snapshots have expired timestamp
fn assert_expired_has_past_timestamp(snapshot: &Snapshot) {
    if snapshot.state == SnapshotState::Expired {
        if let Some(expires_at) = snapshot.expires_at {
            assert!(expires_at < Utc::now());
        }
    }
}

/// INVARIANT: Non-expired snapshots have future timestamp
fn assert_non_expired_has_future_timestamp(snapshot: &Snapshot) {
    if snapshot.state != SnapshotState::Expired {
        if let Some(expires_at) = snapshot.expires_at {
            assert!(expires_at > Utc::now());
        }
    }
}
```

### Storage Invariants

```rust
/// INVARIANT: Snapshot path exists for Ready snapshots
fn assert_ready_snapshot_exists(snapshot: &Snapshot, storage: &dyn SnapshotStorage) {
    if snapshot.state == SnapshotState::Ready {
        let size = storage.get_snapshot_size(snapshot);
        assert!(size.is_ok());
        assert!(size.unwrap() > 0);
    }
}

/// INVARIANT: Snapshot size matches metadata
fn assert_size_matches_metadata(snapshot: &Snapshot, actual_size: u64) {
    assert_eq!(snapshot.metadata.size_bytes, actual_size);
}
```

---

## Consequences

### Positive

1. **Crash recovery** - Restore to known good state
2. **Checkpointing** - Safe to try risky operations
3. **Agent recovery** - Restore after crash
4. **Simple implementation** - Filesystem copy works for any VCS

### Negative

1. **Disk usage** - Full copies use significant space
2. **Creation time** - Large workspaces take time to copy
3. **Cleanup burden** - Must manage expired snapshots

### CLI Commands

```bash
hardline snapshot list <workspace-id>           # List snapshots
hardline snapshot create <workspace-id> <name> # Create manual snapshot
hardline snapshot restore <snapshot-id>        # Restore snapshot
hardline snapshot delete <snapshot-id>         # Delete snapshot
hardline snapshot cleanup                     # Delete expired
hardline snapshot prune <workspace-id>        # Keep only N recent
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/snapshot/src/domain/snapshot.rs` | Snapshot entity |
| `crates/snapshot/src/domain/storage.rs` | Storage trait |
| `crates/snapshot/src/infrastructure/local.rs` | Local storage impl |
| `crates/snapshot/src/application/service.rs` | Snapshot operations |

---

## Related ADRs

- ADR-002: Durable Workflow Execution (pre-operation checkpoints)
- ADR-005: Workspace Isolation Model (workspace backup)
- ADR-006: Database Schema (snapshots table)

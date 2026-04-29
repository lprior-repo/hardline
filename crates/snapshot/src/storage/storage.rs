//! Snapshot persistence storage.
//!
//! Provides I/O operations for saving and loading snapshots
//! to/from JSON files in the `.scp/snapshots/` directory.

use std::path::{Path, PathBuf};

use crate::{
    domain::snapshot::{validate_branch_name, Snapshot, SnapshotId},
    error::{Result, SnapshotError},
};

/// File-based storage for snapshots, persisting each as a JSON file.
pub struct SnapshotStore {
    snapshots_dir: PathBuf,
}

impl SnapshotStore {
    /// Create a new store rooted at the given base path.
    ///
    /// Snapshots are stored under `{base_path}/.scp/snapshots/{id}.json`.
    pub fn new(base_path: &Path) -> Self {
        Self {
            snapshots_dir: base_path.join(".scp").join("snapshots"),
        }
    }

    fn snapshot_file_path(&self, id: &SnapshotId) -> PathBuf {
        self.snapshots_dir.join(format!("{}.json", id.as_str()))
    }

    /// Save a snapshot to disk as a JSON file.
    pub fn save(&self, snapshot: Snapshot) -> Result<()> {
        validate_branch_name(&snapshot.branch_name)?;
        std::fs::create_dir_all(&self.snapshots_dir).map_err(|e| {
            SnapshotError::storage_with_source(e, format!(
                "Failed to create snapshots directory {}",
                self.snapshots_dir.display(),
            ))
        })?;
        let path = self.snapshot_file_path(&snapshot.id);
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| {
            SnapshotError::SerializationError(format!(
                "Failed to serialize snapshot {}: {}",
                snapshot.id,
                e,
            ))
        })?;
        std::fs::write(&path, json).map_err(|e| {
            SnapshotError::storage_with_source(e, format!(
                "Failed to write snapshot file {}",
                path.display(),
            ))
        })?;
        Ok(())
    }

    /// Load a snapshot from disk by its ID.
    pub fn load(&self, id: &SnapshotId) -> Result<Snapshot> {
        let path = self.snapshot_file_path(id);
        let json = std::fs::read_to_string(&path).map_err(|e| {
            SnapshotError::storage_with_source(e, format!(
                "Failed to read snapshot file {}",
                path.display(),
            ))
        })?;
        serde_json::from_str(&json).map_err(|e| {
            SnapshotError::DeserializationError(format!(
                "Failed to parse snapshot {}: {}",
                path.display(),
                e,
            ))
        })
    }

    /// List all snapshots in the store.
    ///
    /// Corrupt or unreadable JSON files are skipped with a warning rather than
    /// causing the entire operation to fail.
    pub fn list(&self) -> Result<Vec<Snapshot>> {
        if !self.snapshots_dir.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.snapshots_dir).map_err(|e| {
            SnapshotError::storage_with_source(e, "Failed to read snapshots directory")
        })?;
        let mut snapshots = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Skipping unreadable directory entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let json = match std::fs::read_to_string(&path) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!(
                            "Skipping unreadable snapshot file {}: {}",
                            path.display(),
                            e,
                        );
                        continue;
                    }
                };
                let snapshot: Snapshot = match serde_json::from_str(&json) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(
                            "Skipping corrupt snapshot file {}: {}",
                            path.display(),
                            e,
                        );
                        continue;
                    }
                };
                snapshots.push(snapshot);
            }
        }
        Ok(snapshots)
    }

    /// Delete a snapshot from disk by its ID.
    pub fn delete(&self, id: &SnapshotId) -> Result<()> {
        let path = self.snapshot_file_path(id);
        std::fs::remove_file(&path).map_err(|e| {
            SnapshotError::storage_with_source(e, format!(
                "Failed to delete snapshot file {}",
                path.display(),
            ))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::domain::snapshot::SnapshotType;

    fn make_store(temp_dir: &Path) -> SnapshotStore {
        SnapshotStore::new(temp_dir)
    }

    fn make_temp_dir() -> TempDir {
        TempDir::new().expect("temp dir creation")
    }

    #[test]
    fn save_and_load_roundtrip() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc123".to_string(),
            Some("test snapshot".to_string()),
            SnapshotType::Checkpoint,
        ).expect("valid snapshot");
        let id = snapshot.id.clone();
        store.save(snapshot.clone()).expect("save should succeed");
        let loaded = store.load(&id).expect("load should succeed");
        assert_eq!(loaded.id, snapshot.id);
        assert_eq!(loaded.branch_name, snapshot.branch_name);
        assert_eq!(loaded.commit_hash, snapshot.commit_hash);
        assert_eq!(loaded.description, snapshot.description);
        assert_eq!(loaded.snapshot_type, snapshot.snapshot_type);
        assert_eq!(loaded.expires_at, snapshot.expires_at);
    }

    #[test]
    fn save_creates_directory_structure() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        store.save(snapshot).expect("save should succeed");
        let scp_dir = temp.path().join(".scp");
        assert!(scp_dir.exists(), ".scp directory should be created");
        let snapshots_dir = scp_dir.join("snapshots");
        assert!(snapshots_dir.exists(), "snapshots directory should be created");
    }

    #[test]
    fn load_nonexistent_returns_err() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let id = SnapshotId::generate();
        let result = store.load(&id);
        assert!(result.is_err());
    }

    #[test]
    fn delete_existing_succeeds() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let id = snapshot.id.clone();
        store.save(snapshot).expect("save should succeed");
        store.delete(&id).expect("delete should succeed");
        let result = store.load(&id);
        assert!(result.is_err(), "deleted snapshot should not be loadable");
    }

    #[test]
    fn delete_nonexistent_returns_err() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let id = SnapshotId::generate();
        let result = store.delete(&id);
        assert!(result.is_err());
    }

    #[test]
    fn list_empty_when_no_snapshots() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let list = store.list().expect("list should succeed");
        assert!(list.is_empty());
    }

    #[test]
    fn list_returns_all_saved_snapshots() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let s1 = Snapshot::create("main".to_string(), "aaa".to_string(), None).expect("valid snapshot");
        let s2 = Snapshot::create("dev".to_string(), "bbb".to_string(), None).expect("valid snapshot");
        store.save(s1.clone()).expect("save s1");
        store.save(s2.clone()).expect("save s2");
        let list = store.list().expect("list should succeed");
        assert_eq!(list.len(), 2);
        let ids: Vec<&str> = list.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&s1.id.as_str()));
        assert!(ids.contains(&s2.id.as_str()));
    }

    #[test]
    fn list_ignores_non_json_files() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshots_dir = temp.path().join(".scp").join("snapshots");
        std::fs::create_dir_all(&snapshots_dir).expect("create dir");
        std::fs::write(snapshots_dir.join("readme.txt"), "not json")
            .expect("write non-json file");
        std::fs::write(snapshots_dir.join("corrupt.json"), "not valid json {")
            .expect("write corrupt json");
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        store.save(snapshot.clone()).expect("save snapshot");
        let list = store.list().expect("list should succeed even with corrupt files");
        assert_eq!(list.len(), 1, "corrupt and non-json files should be skipped");
        assert_eq!(list[0].id, snapshot.id);
    }

    #[test]
    fn save_overwrites_existing() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let id = SnapshotId::parse("snap-override-test").expect("valid id");
        let mut s1 = Snapshot::create("main".to_string(), "aaa".to_string(), None).expect("valid snapshot");
        s1.id = id.clone();
        store.save(s1).expect("save s1");
        let mut s2 = Snapshot::create("dev".to_string(), "bbb".to_string(), None).expect("valid snapshot");
        s2.id = id.clone();
        store.save(s2.clone()).expect("save s2 overwrite");
        let loaded = store.load(&id).expect("load should succeed");
        assert_eq!(loaded.branch_name, "dev");
        assert_eq!(loaded.commit_hash, "bbb");
    }

    #[test]
    fn save_preserves_expires_at() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None).expect("valid snapshot");
        let expected_expires = snapshot.expires_at;
        store.save(snapshot.clone()).expect("save");
        let loaded = store.load(&snapshot.id).expect("load");
        assert_eq!(loaded.expires_at, expected_expires);
    }

    #[test]
    fn save_preserves_snapshot_type() {
        let temp = make_temp_dir();
        let store = make_store(temp.path());
        let snapshot = Snapshot::create_with_type(
            "main".to_string(),
            "abc".to_string(),
            None,
            SnapshotType::PreOperation,
        ).expect("valid snapshot");
        store.save(snapshot.clone()).expect("save");
        let loaded = store.load(&snapshot.id).expect("load");
        assert_eq!(loaded.snapshot_type, SnapshotType::PreOperation);
    }
}

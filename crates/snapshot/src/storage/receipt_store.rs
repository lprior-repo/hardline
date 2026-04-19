//! Receipt persistence storage.
//!
//! Provides I/O operations for saving and loading operation receipts
//! to/from JSON files in the `.git/stax/ops/` directory.

use crate::domain::receipt::OpReceipt;
use crate::error::{Result, SnapshotError};
use std::path::{Path, PathBuf};

pub struct ReceiptStore;

impl Default for ReceiptStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptStore {
    pub fn new() -> Self {
        Self
    }

    fn ops_dir(git_dir: &Path) -> PathBuf {
        git_dir.join("stax").join("ops")
    }

    fn receipt_path(git_dir: &Path, op_id: &str) -> PathBuf {
        Self::ops_dir(git_dir).join(format!("{}.json", op_id))
    }

    pub fn save(&self, git_dir: &Path, receipt: &OpReceipt) -> Result<()> {
        let ops_path = Self::ops_dir(git_dir);
        std::fs::create_dir_all(&ops_path).map_err(|e| SnapshotError::StorageError {
            source: Some(e.into()),
            message: format!("Failed to create ops directory {}", ops_path.display()),
        })?;
        let path = Self::receipt_path(git_dir, &receipt.op_id);
        let json = serde_json::to_string_pretty(receipt).map_err(|e| {
            SnapshotError::SerializationError(format!("Failed to serialize receipt: {}", e))
        })?;
        std::fs::write(&path, json).map_err(|e| SnapshotError::StorageError {
            source: Some(e.into()),
            message: format!("Failed to write receipt {}", path.display()),
        })?;
        Ok(())
    }

    pub fn load(&self, git_dir: &Path, op_id: &str) -> Result<OpReceipt> {
        let path = Self::receipt_path(git_dir, op_id);
        let json = std::fs::read_to_string(&path).map_err(|e| SnapshotError::StorageError {
            source: Some(e.into()),
            message: format!("Failed to read receipt {}", path.display()),
        })?;
        serde_json::from_str(&json).map_err(|e| {
            SnapshotError::DeserializationError(format!(
                "Failed to parse receipt {}: {}",
                path.display(),
                e
            ))
        })
    }

    pub fn list_op_ids(&self, git_dir: &Path) -> Result<Vec<String>> {
        let dir = Self::ops_dir(git_dir);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut ops: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|e| SnapshotError::StorageError {
                source: Some(e.into()),
                message: "Failed to read ops directory".to_string(),
            })?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    Some(name.trim_end_matches(".json").to_string())
                } else {
                    None
                }
            })
            .collect();
        ops.sort();
        ops.reverse();
        Ok(ops)
    }

    pub fn latest_op_id(&self, git_dir: &Path) -> Result<Option<String>> {
        let ops = self.list_op_ids(git_dir)?;
        Ok(ops.into_iter().next())
    }

    pub fn load_latest(&self, git_dir: &Path) -> Result<Option<OpReceipt>> {
        match self.latest_op_id(git_dir)? {
            Some(op_id) => self.load(git_dir, &op_id).map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::receipt::OpKind;
    use tempfile::TempDir;

    fn make_store() -> ReceiptStore {
        ReceiptStore::new()
    }

    fn make_temp_git_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn store_new_creates_instance() {
        let _store = make_store();
    }

    #[test]
    fn store_default_creates_instance() {
        let _store = ReceiptStore::default();
    }

    #[test]
    fn save_and_load_receipt_roundtrip() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let mut receipt = OpReceipt::new(
            "20251229T120500Z-abc123".to_string(),
            OpKind::Restack,
            "/tmp/repo".to_string(),
            "main".to_string(),
            "feature/foo".to_string(),
        );
        receipt.add_local_ref("feature/foo", Some("abc123"));
        receipt.update_local_ref_after("feature/foo", "def456");
        receipt.mark_success();

        store.save(&git_dir, &receipt).unwrap();

        let loaded = store.load(&git_dir, "20251229T120500Z-abc123").unwrap();
        assert_eq!(loaded.op_id, receipt.op_id);
        assert_eq!(loaded.status, crate::domain::receipt::OpStatus::Success);
        assert_eq!(loaded.local_refs.len(), 1);
        assert_eq!(loaded.local_refs[0].oid_before, Some("abc123".to_string()));
        assert_eq!(loaded.local_refs[0].oid_after, Some("def456".to_string()));
    }

    #[test]
    fn list_op_ids_empty_when_no_files() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let ops = store.list_op_ids(&git_dir).unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn list_op_ids_returns_sorted_receipts() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        let ops_path = git_dir.join("stax").join("ops");
        std::fs::create_dir_all(&ops_path).unwrap();

        std::fs::write(ops_path.join("20251229T120000Z-aaa111.json"), "{}").unwrap();
        std::fs::write(ops_path.join("20251229T120100Z-bbb222.json"), "{}").unwrap();
        std::fs::write(ops_path.join("20251229T120200Z-ccc333.json"), "{}").unwrap();

        let ops = store.list_op_ids(&git_dir).unwrap();
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], "20251229T120200Z-ccc333");
        assert_eq!(ops[1], "20251229T120100Z-bbb222");
        assert_eq!(ops[2], "20251229T120000Z-aaa111");
    }

    #[test]
    fn latest_op_id_returns_newest() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        let ops_path = git_dir.join("stax").join("ops");
        std::fs::create_dir_all(&ops_path).unwrap();

        std::fs::write(ops_path.join("20251229T120000Z-old.json"), "{}").unwrap();
        std::fs::write(ops_path.join("20251229T120200Z-new.json"), "{}").unwrap();

        let latest = store.latest_op_id(&git_dir).unwrap();
        assert_eq!(latest, Some("20251229T120200Z-new".to_string()));
    }

    #[test]
    fn load_latest_returns_none_when_empty() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let latest = store.load_latest(&git_dir).unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn load_nonexistent_returns_err() {
        let store = make_store();
        let temp = make_temp_git_dir();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let result = store.load(&git_dir, "nonexistent");
        assert!(result.is_err());
    }
}

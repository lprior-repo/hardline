//! Operations infrastructure - persistence, backup refs, and file management.
//!
//! Handles the I/O side of operation tracking:
//! - Generating unique operation IDs
//! - Reading/writing receipt JSON files
//! - Creating/deleting backup refs
//! - Listing operation history

use crate::domain::entities::ops::OpReceipt;
use crate::error::{Result, VcsError};
use std::path::{Path, PathBuf};

/// Generate a unique operation ID: UTC timestamp + hash-based suffix.
///
/// Format: `20251229T120500Z-4f2a9c`
pub fn generate_op_id() -> String {
    let timestamp = format_utc_timestamp();
    let suffix = format_hash_suffix();
    format!("{timestamp}-{suffix}")
}

/// Get the ops directory path: `.git/stax/ops/`
pub fn ops_dir(git_dir: &Path) -> PathBuf {
    git_dir.join("stax").join("ops")
}

/// Ensure the ops directory exists.
pub fn ensure_ops_dir(git_dir: &Path) -> Result<PathBuf> {
    let dir = ops_dir(git_dir);
    std::fs::create_dir_all(&dir).map_err(VcsError::Io)?;
    Ok(dir)
}

/// Get the backup refs prefix for an operation.
pub fn backup_ref_prefix(op_id: &str) -> String {
    format!("refs/stax/backups/{op_id}/")
}

/// Get the full backup ref name for a branch.
pub fn backup_ref_name(op_id: &str, branch: &str) -> String {
    format!("refs/stax/backups/{op_id}/{branch}")
}

/// List all operation IDs (sorted newest first).
pub fn list_op_ids(git_dir: &Path) -> Result<Vec<String>> {
    let dir = ops_dir(git_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut ops: Vec<String> = std::fs::read_dir(&dir)
        .map_err(VcsError::Io)?
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

    // Sort descending (newest first) - timestamp format is lexicographically sortable
    ops.sort();
    ops.reverse();

    Ok(ops)
}

/// Get the latest operation ID.
pub fn latest_op_id(git_dir: &Path) -> Result<Option<String>> {
    list_op_ids(git_dir).map(|ops| ops.into_iter().next())
}

/// Get the receipt file path for an operation ID.
pub fn receipt_path(git_dir: &Path, op_id: &str) -> PathBuf {
    ops_dir(git_dir).join(format!("{op_id}.json"))
}

/// Save a receipt to disk.
pub fn save_receipt(git_dir: &Path, receipt: &OpReceipt) -> Result<()> {
    ensure_ops_dir(git_dir)?;
    let path = receipt_path(git_dir, &receipt.op_id);
    let json =
        serde_json::to_string_pretty(receipt).map_err(|e| VcsError::ParseError(e.to_string()))?;
    std::fs::write(&path, json).map_err(VcsError::Io)?;
    Ok(())
}

/// Load a receipt from disk.
pub fn load_receipt(git_dir: &Path, op_id: &str) -> Result<OpReceipt> {
    let path = receipt_path(git_dir, op_id);
    let json = std::fs::read_to_string(&path).map_err(VcsError::Io)?;
    serde_json::from_str(&json).map_err(|e| VcsError::ParseError(e.to_string()))
}

/// Load the latest receipt.
pub fn load_latest_receipt(git_dir: &Path) -> Result<Option<OpReceipt>> {
    match latest_op_id(git_dir)? {
        Some(op_id) => load_receipt(git_dir, &op_id).map(Some),
        None => Ok(None),
    }
}

// -- Private helpers (pure functions, no I/O) --

fn format_utc_timestamp() -> String {
    let now = chrono::Utc::now();
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

fn format_hash_suffix() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    let hash = hasher.finish() as u32;
    format!("{:06x}", hash & 0xFFFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ops::OpKind;
    use tempfile::TempDir;

    fn now_iso() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn test_git_dir() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).expect("create .git");
        (temp, git_dir)
    }

    // -- generate_op_id --

    #[test]
    fn generate_op_id_format() {
        let id = generate_op_id();
        assert!(id.contains('-'), "should contain dash separator");
        assert!(id.len() > 20, "should be reasonably long");
        assert!(id.contains('Z'), "should contain Z for UTC");
    }

    #[test]
    fn generate_op_id_unique() {
        let id1 = generate_op_id();
        let id2 = generate_op_id();
        assert_ne!(id1, id2, "consecutive calls should produce different IDs");
    }

    // -- Path helpers --

    #[test]
    fn backup_ref_name_format() {
        let name = backup_ref_name("20251229T120500Z-abc123", "feature/foo");
        assert_eq!(
            name,
            "refs/stax/backups/20251229T120500Z-abc123/feature/foo"
        );
    }

    #[test]
    fn backup_ref_prefix_format() {
        let prefix = backup_ref_prefix("20251229T120500Z-abc123");
        assert_eq!(prefix, "refs/stax/backups/20251229T120500Z-abc123/");
    }

    #[test]
    fn ops_dir_path() {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        let dir = ops_dir(&git_dir);
        assert!(dir.to_string_lossy().contains("stax"));
        assert!(dir.to_string_lossy().contains("ops"));
    }

    #[test]
    fn ensure_ops_dir_creates_directory() {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).expect("create .git");

        let dir = ensure_ops_dir(&git_dir).expect("ensure dir");
        assert!(dir.exists());
    }

    // -- list_op_ids --

    #[test]
    fn list_op_ids_empty() {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");

        let ops = list_op_ids(&git_dir).expect("list");
        assert!(ops.is_empty());
    }

    #[test]
    fn list_op_ids_sorted_newest_first() {
        let (_temp, git_dir) = test_git_dir();
        let ops_path = ops_dir(&git_dir);
        std::fs::create_dir_all(&ops_path).expect("create ops dir");

        std::fs::write(ops_path.join("20251229T120000Z-aaa111.json"), "{}").expect("write");
        std::fs::write(ops_path.join("20251229T120100Z-bbb222.json"), "{}").expect("write");
        std::fs::write(ops_path.join("20251229T120200Z-ccc333.json"), "{}").expect("write");
        // Non-json file should be ignored
        std::fs::write(ops_path.join("not-an-op.txt"), "text").expect("write");

        let ops = list_op_ids(&git_dir).expect("list");
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0], "20251229T120200Z-ccc333");
        assert_eq!(ops[1], "20251229T120100Z-bbb222");
        assert_eq!(ops[2], "20251229T120000Z-aaa111");
    }

    // -- latest_op_id --

    #[test]
    fn latest_op_id_empty() {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        let latest = latest_op_id(&git_dir).expect("latest");
        assert!(latest.is_none());
    }

    #[test]
    fn latest_op_id_returns_newest() {
        let (_temp, git_dir) = test_git_dir();
        let ops_path = ops_dir(&git_dir);
        std::fs::create_dir_all(&ops_path).expect("create ops dir");

        std::fs::write(ops_path.join("20251229T120000Z-old.json"), "{}").expect("write");
        std::fs::write(ops_path.join("20251229T120200Z-new.json"), "{}").expect("write");

        let latest = latest_op_id(&git_dir).expect("latest");
        assert_eq!(latest, Some("20251229T120200Z-new".to_string()));
    }

    // -- Receipt persistence --

    #[test]
    fn save_and_load_receipt() {
        let (_temp, git_dir) = test_git_dir();

        let mut receipt = OpReceipt::new(
            "test-op-id".to_string(),
            OpKind::Restack,
            "/tmp/repo".to_string(),
            "main".to_string(),
            "feature".to_string(),
            now_iso(),
        );
        receipt.add_local_ref("feature/foo", Some("abc123"));
        receipt.update_local_ref_after("feature/foo", "def456");
        receipt.mark_success(now_iso());

        save_receipt(&git_dir, &receipt).expect("save");
        let loaded = load_receipt(&git_dir, "test-op-id").expect("load");

        assert_eq!(loaded.op_id, "test-op-id");
        assert_eq!(loaded.status, receipt.status);
        assert_eq!(loaded.local_refs.len(), 1);
        assert_eq!(loaded.local_refs[0].oid_before, Some("abc123".to_string()));
        assert_eq!(loaded.local_refs[0].oid_after, Some("def456".to_string()));
    }

    #[test]
    fn load_receipt_nonexistent() {
        let (_temp, git_dir) = test_git_dir();
        let result = load_receipt(&git_dir, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn load_latest_receipt_empty() {
        let temp = TempDir::new().expect("temp dir");
        let git_dir = temp.path().join(".git");
        let result = load_latest_receipt(&git_dir).expect("load latest");
        assert!(result.is_none());
    }

    #[test]
    fn load_latest_receipt_returns_latest() {
        let (_temp, git_dir) = test_git_dir();

        let old = OpReceipt::new(
            "20251229T120000Z-old".to_string(),
            OpKind::Restack,
            "/tmp".to_string(),
            "main".to_string(),
            "main".to_string(),
            now_iso(),
        );
        let new = OpReceipt::new(
            "20251229T120200Z-new".to_string(),
            OpKind::Submit,
            "/tmp".to_string(),
            "main".to_string(),
            "feature".to_string(),
            now_iso(),
        );

        save_receipt(&git_dir, &old).expect("save old");
        save_receipt(&git_dir, &new).expect("save new");

        let latest = load_latest_receipt(&git_dir)
            .expect("load latest")
            .expect("some");
        assert_eq!(latest.op_id, "20251229T120200Z-new");
    }
}

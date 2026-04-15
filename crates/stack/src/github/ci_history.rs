use crate::domain::value_objects::CiCheckHistory;
use crate::error::{Result, StackError};
use std::path::Path;

const CI_HISTORY_DIR: &str = ".ci_history";

pub struct CiHistoryStore {
    repo_path: std::path::PathBuf,
}

impl CiHistoryStore {
    pub fn open(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let store = Self { repo_path };
        store.ensure_history_dir()?;
        Ok(store)
    }

    fn history_dir(&self) -> std::path::PathBuf {
        self.repo_path.join(CI_HISTORY_DIR)
    }

    fn history_file_path(check_name: &str, branch_name: &str) -> String {
        format!(
            "{}_{}.json",
            check_name.replace('/', "_"),
            branch_name.replace('/', "_")
        )
    }

    fn ensure_history_dir(&self) -> Result<()> {
        let dir = self.history_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                StackError::GitError(format!("Failed to create history dir: {}", e))
            })?;
        }
        Ok(())
    }

    pub fn save(&self, history: &CiCheckHistory) -> Result<()> {
        self.ensure_history_dir()?;
        let file_name = Self::history_file_path(&history.check_name, &history.branch_name);
        let file_path = self.history_dir().join(&file_name);

        let data = serde_json::to_vec_pretty(history)
            .map_err(|e| StackError::GitError(format!("Failed to serialize history: {}", e)))?;

        std::fs::write(&file_path, data)
            .map_err(|e| StackError::GitError(format!("Failed to write history file: {}", e)))?;

        Ok(())
    }

    pub fn load(&self, check_name: &str, branch_name: &str) -> Result<Option<CiCheckHistory>> {
        let file_name = Self::history_file_path(check_name, branch_name);
        let file_path = self.history_dir().join(&file_name);

        if !file_path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&file_path)
            .map_err(|e| StackError::GitError(format!("Failed to read history file: {}", e)))?;

        let history: CiCheckHistory = serde_json::from_slice(&data)
            .map_err(|e| StackError::GitError(format!("Failed to deserialize: {}", e)))?;

        Ok(Some(history))
    }

    pub fn list_checks(&self) -> Result<Vec<(String, String)>> {
        let dir = self.history_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut checks = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| StackError::GitError(format!("Failed to read history dir: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(sep_pos) = stem.find('_') {
                        let check = stem[..sep_pos].replace('_', "/");
                        let branch = stem[sep_pos + 1..].replace('_', "/");
                        checks.push((check, branch));
                    }
                }
            }
        }

        checks.sort();
        checks.dedup();
        Ok(checks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{CiRunRecord, CiStatus};
    use chrono::Utc;
    use tempfile::TempDir;

    fn create_test_store() -> (CiHistoryStore, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = CiHistoryStore::open(temp_dir.path()).expect("open store");
        (store, temp_dir)
    }

    #[test]
    fn test_history_store_open() {
        let (store, _temp_dir) = create_test_store();
        assert!(store.history_dir().exists());
    }

    #[test]
    fn test_save_and_load() {
        let (store, _temp_dir) = create_test_store();
        let mut history = CiCheckHistory::new("test-check", "main");
        history.add_record(CiRunRecord::new(
            "run-1",
            "abc123",
            CiStatus::Success,
            Utc::now(),
            100,
        ));

        store.save(&history).expect("save");

        let loaded = store.load("test-check", "main").expect("load");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.check_name, "test-check");
        assert_eq!(loaded.branch_name, "main");
        assert_eq!(loaded.records.len(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let (store, _temp_dir) = create_test_store();
        let loaded = store.load("nonexistent", "main").expect("load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_list_checks_empty() {
        let (store, _temp_dir) = create_test_store();
        let checks = store.list_checks().expect("list");
        assert!(checks.is_empty());
    }

    #[test]
    fn test_list_checks() {
        let (store, _temp_dir) = create_test_store();
        let mut history1 = CiCheckHistory::new("check-a", "main");
        history1.add_record(CiRunRecord::new(
            "r1",
            "c1",
            CiStatus::Success,
            Utc::now(),
            100,
        ));
        store.save(&history1).expect("save");

        let mut history2 = CiCheckHistory::new("check-b", "feature");
        history2.add_record(CiRunRecord::new(
            "r2",
            "c2",
            CiStatus::Failure,
            Utc::now(),
            50,
        ));
        store.save(&history2).expect("save");

        let checks = store.list_checks().expect("list");
        assert_eq!(checks.len(), 2);
    }

    #[test]
    fn test_history_file_path() {
        let path = CiHistoryStore::history_file_path("check/name", "feature/test");
        assert_eq!(path, "check_name_feature_test.json");
    }
}

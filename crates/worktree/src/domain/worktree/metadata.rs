//! Metadata operations for worktree

use chrono::Utc;

use super::Worktree;

impl Worktree {
    /// Add metadata to the worktree
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
        self.updated_at = Utc::now().timestamp();
    }

    /// Remove metadata from the worktree
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        let removed = self.metadata.remove(key);
        if removed.is_some() {
            self.updated_at = Utc::now().timestamp();
        }
        removed
    }

    /// Get metadata by key
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

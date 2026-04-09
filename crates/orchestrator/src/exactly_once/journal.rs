//! Journal for crash recovery of operations
//!
//! The journal records operation intent before execution. On restart,
//! incomplete operations (Intended, InProgress) are replayed to recover
//! from crashes.

use std::sync;

use super::types::{IdempotencyKey, JournalEntry};

#[derive(Debug, Clone, thiserror::Error)]
pub enum JournalError {
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Journal error: {0}")]
    Internal(String),
    #[error("Transition error: {0}")]
    Transition(#[from] super::types::JournalTransitionError),
}

pub type JournalResult<T> = Result<T, JournalError>;

pub trait Journal: Send + Sync {
    fn append(&self, entry: JournalEntry) -> JournalResult<()>;
    fn update(&self, entry: JournalEntry) -> JournalResult<()>;
    fn get(&self, key: &IdempotencyKey) -> JournalResult<Option<JournalEntry>>;
    fn get_incomplete(&self) -> JournalResult<Vec<JournalEntry>>;
    fn remove(&self, key: &IdempotencyKey) -> JournalResult<()>;
    fn len(&self) -> JournalResult<usize>;
    fn is_empty(&self) -> JournalResult<bool>;
}

pub struct InMemoryJournal {
    entries: sync::RwLock<Vec<JournalEntry>>,
}

impl InMemoryJournal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl Journal for InMemoryJournal {
    fn append(&self, entry: JournalEntry) -> JournalResult<()> {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| JournalError::Internal(format!("Write lock failed: {e}")))?;
        entries.push(entry);
        Ok(())
    }

    fn update(&self, entry: JournalEntry) -> JournalResult<()> {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| JournalError::Internal(format!("Write lock failed: {e}")))?;
        let idx = entries
            .iter()
            .position(|e| e.key == entry.key)
            .ok_or_else(|| JournalError::NotFound(entry.key.to_string()))?;
        entries[idx] = entry;
        Ok(())
    }

    fn get(&self, key: &IdempotencyKey) -> JournalResult<Option<JournalEntry>> {
        let entries = self
            .entries
            .read()
            .map_err(|e| JournalError::Internal(format!("Read lock failed: {e}")))?;
        Ok(entries.iter().find(|e| e.key == *key).cloned())
    }

    fn get_incomplete(&self) -> JournalResult<Vec<JournalEntry>> {
        let entries = self
            .entries
            .read()
            .map_err(|e| JournalError::Internal(format!("Read lock failed: {e}")))?;
        Ok(entries
            .iter()
            .filter(|e| !e.status.is_terminal())
            .cloned()
            .collect())
    }

    fn remove(&self, key: &IdempotencyKey) -> JournalResult<()> {
        let mut entries = self
            .entries
            .write()
            .map_err(|e| JournalError::Internal(format!("Write lock failed: {e}")))?;
        let original_len = entries.len();
        entries.retain(|e| e.key != *key);
        if entries.len() == original_len {
            return Err(JournalError::NotFound(key.to_string()));
        }
        Ok(())
    }

    fn len(&self) -> JournalResult<usize> {
        let entries = self
            .entries
            .read()
            .map_err(|e| JournalError::Internal(format!("Read lock failed: {e}")))?;
        Ok(entries.len())
    }

    fn is_empty(&self) -> JournalResult<bool> {
        self.len().map(|l| l == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::OperationStatus;
    use super::*;

    fn test_entry(name: &str) -> JournalEntry {
        JournalEntry::new_intended(
            IdempotencyKey::from_static(name),
            serde_json::json!({"op": name}),
        )
    }

    #[test]
    fn test_append_and_get() {
        let journal = InMemoryJournal::new();
        let entry = test_entry("op-1");
        let key = entry.key.clone();

        journal.append(entry).expect("append");
        let retrieved = journal.get(&key).expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|e| &e.key), Some(&key));
    }

    #[test]
    fn test_get_nonexistent() {
        let journal = InMemoryJournal::new();
        let key = IdempotencyKey::from_static("ghost");
        let result = journal.get(&key).expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_update_existing() {
        let journal = InMemoryJournal::new();
        let entry = test_entry("op-1");
        let key = entry.key.clone();
        journal.append(entry).expect("append");

        let updated = journal
            .get(&key)
            .expect("get")
            .expect("some")
            .transition_to(OperationStatus::InProgress)
            .expect("transition");
        journal.update(updated).expect("update");

        let retrieved = journal.get(&key).expect("get").expect("some");
        assert_eq!(retrieved.status, OperationStatus::InProgress);
    }

    #[test]
    fn test_update_nonexistent_errors() {
        let journal = InMemoryJournal::new();
        let entry = test_entry("missing");
        let result = journal.update(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_incomplete_filters_terminal() {
        let journal = InMemoryJournal::new();

        let intended = test_entry("intended");
        let key_in_progress = IdempotencyKey::from_static("in-progress");
        let in_progress =
            JournalEntry::new_intended(key_in_progress.clone(), serde_json::json!({}))
                .transition_to(OperationStatus::InProgress)
                .expect("transition");
        let key_completed = IdempotencyKey::from_static("completed");
        let completed = JournalEntry::new_intended(key_completed.clone(), serde_json::json!({}))
            .transition_to(OperationStatus::InProgress)
            .and_then(|e| e.transition_to(OperationStatus::Completed))
            .expect("complete");

        journal.append(intended).expect("append");
        journal.append(in_progress).expect("append");
        journal.append(completed).expect("append");

        let incomplete = journal.get_incomplete().expect("get_incomplete");
        assert_eq!(incomplete.len(), 2);
    }

    #[test]
    fn test_remove_existing() {
        let journal = InMemoryJournal::new();
        let entry = test_entry("rm-me");
        let key = entry.key.clone();
        journal.append(entry).expect("append");

        journal.remove(&key).expect("remove");
        assert!(journal.get(&key).expect("get").is_none());
    }

    #[test]
    fn test_remove_nonexistent_errors() {
        let journal = InMemoryJournal::new();
        let key = IdempotencyKey::from_static("ghost");
        let result = journal.remove(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_len_tracks_entries() {
        let journal = InMemoryJournal::new();
        assert_eq!(journal.len().expect("len"), 0);
        assert!(journal.is_empty().expect("is_empty"));

        journal.append(test_entry("a")).expect("append");
        journal.append(test_entry("b")).expect("append");
        assert_eq!(journal.len().expect("len"), 2);
    }

    #[test]
    fn test_get_incomplete_empty_journal() {
        let journal = InMemoryJournal::new();
        let incomplete = journal.get_incomplete().expect("get_incomplete");
        assert!(incomplete.is_empty());
    }

    #[test]
    fn test_crash_recovery_scenario() {
        let journal = InMemoryJournal::new();

        let entry1 = test_entry("completed-before-crash");
        let key1 = entry1.key.clone();
        let completed = entry1
            .transition_to(OperationStatus::InProgress)
            .and_then(|e| e.transition_to(OperationStatus::Completed))
            .expect("complete");
        journal.append(completed).expect("append");

        let entry2 = test_entry("in-progress-when-crashed");
        let key2 = entry2.key.clone();
        let in_progress = entry2
            .transition_to(OperationStatus::InProgress)
            .expect("start");
        journal.append(in_progress).expect("append");

        let entry3 = test_entry("intended-never-started");
        journal.append(entry3).expect("append");

        let incomplete = journal.get_incomplete().expect("get_incomplete");
        assert_eq!(incomplete.len(), 2);

        let key_incomplete2 = IdempotencyKey::from_static("intended-never-started");
        let incomplete_keys: Vec<&IdempotencyKey> = incomplete.iter().map(|e| &e.key).collect();
        assert!(incomplete_keys.iter().any(|k| **k == key2));
        assert!(incomplete_keys.iter().any(|k| **k == key_incomplete2));
        assert!(!incomplete_keys.iter().any(|k| **k == key1));
    }

    #[test]
    fn test_default_is_empty() {
        let journal = InMemoryJournal::default();
        assert!(journal.is_empty().expect("is_empty"));
    }
}

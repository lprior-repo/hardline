use crate::domain::snapshot::{Snapshot, SnapshotId};
use crate::error::{Result, SnapshotError};

pub struct SnapshotStore;

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self
    }

    pub fn save(&self, _snapshot: Snapshot) -> Result<()> {
        Err(SnapshotError::StorageError(
            "Storage not yet implemented".to_string(),
        ))
    }

    pub fn load(&self, _id: &SnapshotId) -> Result<Snapshot> {
        Err(SnapshotError::StorageError(
            "Storage not yet implemented".to_string(),
        ))
    }

    pub fn list(&self) -> Result<Vec<Snapshot>> {
        Err(SnapshotError::StorageError(
            "Storage not yet implemented".to_string(),
        ))
    }

    pub fn delete(&self, _id: &SnapshotId) -> Result<()> {
        Err(SnapshotError::StorageError(
            "Storage not yet implemented".to_string(),
        ))
    }
}

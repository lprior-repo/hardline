//! State persistence for pipeline recovery

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tracing::{debug, error, info};

use crate::state::{Pipeline, PipelineId};

/// Error types for state store operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Pipeline not found: {0}")]
    NotFound(String),
    #[error("Invalid state file: {0}")]
    InvalidState(String),
}

/// State store for persisting pipeline state
pub struct StateStore {
    state_dir: PathBuf,
    cache: HashMap<String, Pipeline>,
    dirty: bool,
}

impl StateStore {
    pub fn new(state_dir: PathBuf) -> Result<Self, StoreError> {
        fs::create_dir_all(&state_dir)?;

        let mut store = Self {
            state_dir,
            cache: HashMap::new(),
            dirty: false,
        };

        store.load_all()?;

        Ok(store)
    }

    fn state_file_path(&self, id: &PipelineId) -> PathBuf {
        self.state_dir.join(format!("{}.json", id.0))
    }

    fn load_all(&mut self) -> Result<(), StoreError> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match Self::load_single(&path) {
                    Ok(pipeline) => {
                        debug!("Loaded pipeline: {}", pipeline.id.0);
                        self.cache.insert(pipeline.id.0.clone(), pipeline);
                    }
                    Err(e) => {
                        error!("Failed to load pipeline from {:?}: {e}", path);
                    }
                }
            }
        }

        info!("Loaded {} pipelines from state store", self.cache.len());
        Ok(())
    }

    fn load_single(path: &Path) -> Result<Pipeline, StoreError> {
        let content = fs::read_to_string(path)?;
        let pipeline: Pipeline =
            serde_json::from_str(&content).map_err(|e| StoreError::InvalidState(e.to_string()))?;
        Ok(pipeline)
    }

    fn save_single(&self, pipeline: &Pipeline) -> Result<(), StoreError> {
        let path = self.state_file_path(&pipeline.id);
        let content = serde_json::to_string_pretty(pipeline)?;
        fs::write(&path, content)?;
        debug!("Saved pipeline {} to {:?}", pipeline.id.0, path);
        Ok(())
    }

    pub fn create(&mut self, pipeline: Pipeline) -> Result<Pipeline, StoreError> {
        let id = pipeline.id.0.clone();
        self.save_single(&pipeline)?;
        self.cache.insert(id, pipeline.clone());
        self.dirty = true;
        Ok(pipeline)
    }

    pub fn get(&self, id: &PipelineId) -> Result<&Pipeline, StoreError> {
        self.cache
            .get(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))
    }

    pub fn get_mut(&mut self, id: &PipelineId) -> Result<&mut Pipeline, StoreError> {
        self.dirty = true;
        self.cache
            .get_mut(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))
    }

    pub fn update(&mut self, pipeline: Pipeline) -> Result<(), StoreError> {
        self.save_single(&pipeline)?;
        self.cache.insert(pipeline.id.0.clone(), pipeline);
        self.dirty = true;
        Ok(())
    }

    pub fn delete(&mut self, id: &PipelineId) -> Result<(), StoreError> {
        let path = self.state_file_path(id);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.cache
            .remove(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))?;
        self.dirty = true;
        Ok(())
    }

    #[must_use]
    pub fn list(&self) -> Vec<&Pipeline> {
        self.cache.values().collect()
    }

    #[must_use]
    pub fn list_by_state(&self, state: crate::state::PipelineState) -> Vec<&Pipeline> {
        self.cache.values().filter(|p| p.state == state).collect()
    }

    #[must_use]
    pub fn get_pending_recovery(&self) -> Vec<&Pipeline> {
        self.cache
            .values()
            .filter(|p| !p.state.is_terminal())
            .collect()
    }

    #[must_use]
    pub fn exists(&self, id: &PipelineId) -> bool {
        self.cache.contains_key(&id.0)
    }

    pub fn sync(&mut self) -> Result<(), StoreError> {
        if self.dirty {
            for pipeline in self.cache.values() {
                self.save_single(pipeline)?;
            }
            self.dirty = false;
            info!("Synced {} pipelines to disk", self.cache.len());
        }
        Ok(())
    }

    pub fn export_all(&self, path: &Path) -> Result<(), StoreError> {
        let pipelines: Vec<&Pipeline> = self.cache.values().collect();
        let content = serde_json::to_string_pretty(&pipelines)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn import_from(&mut self, path: &Path) -> Result<usize, StoreError> {
        let content = fs::read_to_string(path)?;
        let pipelines: Vec<Pipeline> = serde_json::from_str(&content)?;

        let count = pipelines.len();
        for pipeline in pipelines {
            self.save_single(&pipeline)?;
            self.cache.insert(pipeline.id.0.clone(), pipeline);
        }

        self.dirty = true;
        info!("Imported {} pipelines from {:?}", count, path);
        Ok(count)
    }

    #[cfg(test)]
    pub fn clear(&mut self) -> Result<(), StoreError> {
        for id in self.cache.keys().cloned().collect::<Vec<_>>() {
            let path = self.state_file_path(&PipelineId(id));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        self.cache.clear();
        self.dirty = false;
        Ok(())
    }
}

impl Drop for StateStore {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            error!("Failed to sync state on drop: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::PipelineState;

    fn create_temp_store() -> (StateStore, TempDir) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let store = StateStore::new(temp_dir.path().to_path_buf()).expect("failed to create store");
        (store, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).expect("failed to create pipeline");

        let retrieved = store.get(&id).expect("failed to get pipeline");
        assert_eq!(retrieved.spec_path, "specs/test.yaml");
    }

    #[test]
    fn test_update() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).expect("failed to create pipeline");

        let pipeline = store.get_mut(&id).expect("failed to get mutable pipeline");
        pipeline
            .transition_to(PipelineState::SpecReview)
            .expect("failed to transition");
        let _ = pipeline;

        let retrieved = store.get(&id).expect("failed to get pipeline");
        assert_eq!(retrieved.state, PipelineState::SpecReview);
    }

    #[test]
    fn test_delete() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).expect("failed to create pipeline");
        store.delete(&id).expect("failed to delete pipeline");

        assert!(store.get(&id).is_err());
    }

    #[test]
    fn test_list_by_state() {
        let (mut store, _temp) = create_temp_store();

        let p1 = Pipeline::new("specs/test1.yaml".to_string());
        let p2 = Pipeline::new("specs/test2.yaml".to_string());

        store.create(p1).expect("failed to create pipeline");
        store.create(p2.clone()).expect("failed to create pipeline");

        let p2_id = PipelineId(p2.id.0.clone());
        let pipeline = store.get_mut(&p2_id).expect("failed to get pipeline");
        pipeline
            .transition_to(PipelineState::SpecReview)
            .expect("failed to transition");

        let pending = store.list_by_state(PipelineState::Pending);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_export_import() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        store.create(pipeline).expect("failed to create pipeline");

        let export_path = _temp.path().join("export.json");
        store.export_all(&export_path).expect("failed to export");

        let (mut store2, _temp2) = create_temp_store();
        store2.import_from(&export_path).expect("failed to import");

        assert_eq!(store2.cache.len(), 1);
    }
}

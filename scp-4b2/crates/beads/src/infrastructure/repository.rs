use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::bead::Bead;
use crate::domain::value_objects::{BeadId, BeadState};
use crate::error::{BeadError, Result};

#[async_trait]
pub trait BeadRepository: Send + Sync {
    async fn insert(&self, bead: &Bead) -> Result<()>;
    async fn update(&self, bead: &Bead) -> Result<()>;
    async fn delete(&self, id: &BeadId) -> Result<()>;
    async fn find(&self, id: &BeadId) -> Result<Option<Bead>>;
    async fn find_all(&self) -> Result<Vec<Bead>>;
    async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>>;
    async fn exists(&self, id: &BeadId) -> bool;
}

pub struct InMemoryBeadRepository {
    beads: Arc<tokio::sync::RwLock<HashMap<String, Bead>>>,
}

impl InMemoryBeadRepository {
    pub fn new() -> Self {
        Self {
            beads: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryBeadRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BeadRepository for InMemoryBeadRepository {
    async fn insert(&self, bead: &Bead) -> Result<()> {
        let mut beads = self.beads.write().await;
        let id = bead.id.to_string();
        if beads.contains_key(&id) {
            return Err(BeadError::AlreadyExists(id));
        }
        beads.insert(id, bead.clone());
        Ok(())
    }

    async fn update(&self, bead: &Bead) -> Result<()> {
        let mut beads = self.beads.write().await;
        let id = bead.id.to_string();
        if !beads.contains_key(&id) {
            return Err(BeadError::NotFound(id));
        }
        beads.insert(id, bead.clone());
        Ok(())
    }

    async fn delete(&self, id: &BeadId) -> Result<()> {
        let mut beads = self.beads.write().await;
        let id_str = id.to_string();
        if beads.remove(&id_str).is_none() {
            return Err(BeadError::NotFound(id_str));
        }
        Ok(())
    }

    async fn find(&self, id: &BeadId) -> Result<Option<Bead>> {
        let beads = self.beads.read().await;
        Ok(beads.get(id.as_str()).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Bead>> {
        let beads = self.beads.read().await;
        Ok(beads.values().cloned().collect())
    }

    async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>> {
        let beads = self.beads.read().await;
        Ok(beads
            .values()
            .filter(|b| b.state == state)
            .cloned()
            .collect())
    }

    async fn exists(&self, id: &BeadId) -> bool {
        let beads = self.beads.read().await;
        beads.contains_key(id.as_str())
    }
}

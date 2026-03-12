use chrono::Utc;

use crate::domain::entities::bead::Bead;
use crate::domain::events::BeadEvent;
use crate::domain::value_objects::{BeadId, BeadState, BeadTitle, Priority};
use crate::error::{BeadError, Result};
use crate::infrastructure::repository::BeadRepository;

pub struct BeadService<R: BeadRepository> {
    repository: R,
}

impl<R: BeadRepository> BeadService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_bead(
        &self,
        id: impl TryInto<BeadId>,
        title: impl TryInto<BeadTitle>,
        description: Option<String>,
    ) -> Result<(Bead, BeadEvent)> {
        let id = id.try_into().map_err(|_| {
            BeadError::InvalidId("Failed to convert ID".into())
        })?;
        let title = title.try_into().map_err(|_| {
            BeadError::InvalidTitle("Failed to convert title".into())
        })?;

        if self.repository.exists(&id).await {
            return Err(BeadError::AlreadyExists(id.to_string()));
        }

        let description = match description {
            Some(d) => Some(d.try_into()?),
            None => None,
        };

        let bead = Bead::create(id.clone(), title.clone(), description);
        let event = BeadEvent::Created {
            id: id.clone(),
            title,
            created_at: Utc::now(),
        };

        self.repository.insert(&bead).await?;

        Ok((bead, event))
    }

    pub async fn get_bead(&self, id: &BeadId) -> Result<Bead> {
        self.repository
            .find(id)
            .await?
            .ok_or_else(|| BeadError::NotFound(id.to_string()))
    }

    pub async fn update_bead_state(
        &self,
        id: &BeadId,
        new_state: BeadState,
    ) -> Result<(Bead, BeadEvent)> {
        let bead = self.get_bead(id).await?;
        let old_state = bead.state.clone();

        if !bead.can_transition_to(&new_state) {
            return Err(BeadError::InvalidStateTransition {
                from: format!("{:?}", old_state),
                to: format!("{:?}", new_state),
            });
        }

        let updated = bead.transition(new_state)?;
        self.repository.update(&updated).await?;

        let event = BeadEvent::StateChanged {
            id: id.clone(),
            old_state,
            new_state: updated.state.clone(),
            changed_at: Utc::now(),
        };

        Ok((updated, event))
    }

    pub async fn set_priority(
        &self,
        id: &BeadId,
        priority: Priority,
    ) -> Result<(Bead, BeadEvent)> {
        let mut bead = self.get_bead(id).await?;
        bead.priority = Some(priority);
        self.repository.update(&bead).await?;

        let event = BeadEvent::PrioritySet {
            id: id.clone(),
            priority,
            changed_at: Utc::now(),
        };

        Ok((bead, event))
    }

    pub async fn assign_bead(
        &self,
        id: &BeadId,
        assignee: Option<String>,
    ) -> Result<(Bead, BeadEvent)> {
        let mut bead = self.get_bead(id).await?;
        bead.assignee = assignee.clone();
        self.repository.update(&bead).await?;

        let event = BeadEvent::AssigneeSet {
            id: id.clone(),
            assignee,
            changed_at: Utc::now(),
        };

        Ok((bead, event))
    }

    pub async fn add_dependency(
        &self,
        id: &BeadId,
        depends_on: BeadId,
    ) -> Result<(Bead, BeadEvent)> {
        let bead = self.get_bead(id).await?;

        if !self.repository.exists(&depends_on).await {
            return Err(BeadError::InvalidDependency(format!(
                "Bead {} does not exist",
                depends_on
            )));
        }

        if depends_on == *id {
            return Err(BeadError::DependencyCycle(
                "Bead cannot depend on itself".into(),
            ));
        }

        let mut updated = bead;
        updated.depends_on.push(depends_on.clone());
        self.repository.update(&updated).await?;

        let event = BeadEvent::DependencyAdded {
            id: id.clone(),
            depends_on,
            changed_at: Utc::now(),
        };

        Ok((updated, event))
    }

    pub async fn list_beads(&self) -> Result<Vec<Bead>> {
        self.repository.find_all().await
    }

    pub async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>> {
        self.repository.find_by_state(state).await
    }

    pub async fn delete_bead(&self, id: &BeadId) -> Result<BeadEvent> {
        let _bead = self.get_bead(id).await?;
        self.repository.delete(id).await?;

        Ok(BeadEvent::Deleted {
            id: id.clone(),
            deleted_at: Utc::now(),
        })
    }
}

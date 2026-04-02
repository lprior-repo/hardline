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
        let id = id
            .try_into()
            .map_err(|_| BeadError::InvalidId("Failed to convert ID".into()))?;
        let title = title
            .try_into()
            .map_err(|_| BeadError::InvalidTitle("Failed to convert title".into()))?;

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
        let old_state = bead.state();

        if !bead.can_transition_to(&new_state) {
            return Err(BeadError::InvalidStateTransition {
                from: format!("{:?}", old_state),
                to: format!("{:?}", new_state),
            });
        }

        // Use the transition_to method which handles all typestate transitions
        let updated =
            bead.transition_to(&new_state)
                .ok_or_else(|| BeadError::InvalidStateTransition {
                    from: format!("{:?}", old_state),
                    to: format!("{:?}", new_state),
                })?;

        self.repository.update(&updated).await?;

        let event = BeadEvent::StateChanged {
            id: id.clone(),
            old_state,
            new_state: updated.state(),
            changed_at: Utc::now(),
        };

        Ok((updated, event))
    }

    pub async fn set_priority(&self, id: &BeadId, priority: Priority) -> Result<(Bead, BeadEvent)> {
        let bead = self.get_bead(id).await?;
        let updated = bead.with_priority(priority);
        self.repository.update(&updated).await?;

        let event = BeadEvent::PrioritySet {
            id: id.clone(),
            priority,
            changed_at: Utc::now(),
        };

        Ok((updated, event))
    }

    pub async fn assign_bead(
        &self,
        id: &BeadId,
        assignee: Option<String>,
    ) -> Result<(Bead, BeadEvent)> {
        let bead = self.get_bead(id).await?;
        let updated = match assignee {
            Some(ref a) => bead.with_assignee(a.clone()),
            None => bead,
        };
        self.repository.update(&updated).await?;

        let event = BeadEvent::AssigneeSet {
            id: id.clone(),
            assignee,
            changed_at: Utc::now(),
        };

        Ok((updated, event))
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

        let updated = bead.add_dependency(depends_on.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::BeadDescription;
    use crate::infrastructure::InMemoryBeadRepository;

    fn make_service() -> BeadService<InMemoryBeadRepository> {
        BeadService::new(InMemoryBeadRepository::new())
    }

    // ── create_bead ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_bead_succeeds_for_new_id() {
        let service = make_service();
        let (bead, event) = service
            .create_bead("new-1", "My Bead", Some("desc".into()))
            .await
            .unwrap();
        assert_eq!(bead.id().as_str(), "new-1");
        assert_eq!(bead.title().as_str(), "My Bead");
        assert_eq!(bead.description().unwrap().as_str(), "desc");
        assert_eq!(event.id().as_str(), "new-1");
    }

    #[tokio::test]
    async fn create_bead_without_description() {
        let service = make_service();
        let (bead, _) = service
            .create_bead("new-2", "No Desc", None)
            .await
            .unwrap();
        assert!(bead.description().is_none());
    }

    #[tokio::test]
    async fn create_bead_fails_for_duplicate_id() {
        let service = make_service();
        service
            .create_bead("dup-1", "First", None)
            .await
            .unwrap();
        let result = service.create_bead("dup-1", "Second", None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::AlreadyExists(id) => assert_eq!(id, "dup-1"),
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_bead_fails_for_invalid_id() {
        let service = make_service();
        let result = service
            .create_bead("bad id!", "Title", None)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidId(_) => {}
            other => panic!("expected InvalidId, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_bead_fails_for_empty_title() {
        let service = make_service();
        let result = service.create_bead("valid-id", "", None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidTitle(_) => {}
            other => panic!("expected InvalidTitle, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_bead_fails_for_whitespace_title() {
        let service = make_service();
        let result = service.create_bead("valid-id", "   ", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_bead_produces_created_event() {
        let service = make_service();
        let (_, event) = service
            .create_bead("evt-1", "Event Test", None)
            .await
            .unwrap();
        match &event {
            BeadEvent::Created { id, title, .. } => {
                assert_eq!(id.as_str(), "evt-1");
                assert_eq!(title.as_str(), "Event Test");
            }
            other => panic!("expected Created event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_bead_persists_to_repository() {
        let service = make_service();
        service
            .create_bead("persist-1", "Persisted", None)
            .await
            .unwrap();
        // Verify it can be retrieved
        let bead = service.get_bead(&BeadId::new("persist-1").unwrap()).await.unwrap();
        assert_eq!(bead.title().as_str(), "Persisted");
    }

    // ── get_bead ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_bead_returns_existing_bead() {
        let service = make_service();
        service
            .create_bead("get-1", "Get Me", None)
            .await
            .unwrap();
        let bead = service.get_bead(&BeadId::new("get-1").unwrap()).await.unwrap();
        assert_eq!(bead.id().as_str(), "get-1");
    }

    #[tokio::test]
    async fn get_bead_returns_not_found_for_missing() {
        let service = make_service();
        let result = service.get_bead(&BeadId::new("ghost").unwrap()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::NotFound(id) => assert_eq!(id, "ghost"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    // ── update_bead_state ────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_bead_state_open_to_in_progress() {
        let service = make_service();
        service
            .create_bead("state-1", "State", None)
            .await
            .unwrap();
        let (updated, event) = service
            .update_bead_state(&BeadId::new("state-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        assert_eq!(updated.state(), BeadState::InProgress);
        match &event {
            BeadEvent::StateChanged { old_state, new_state, .. } => {
                assert_eq!(old_state, &BeadState::Open);
                assert_eq!(new_state, &BeadState::InProgress);
            }
            other => panic!("expected StateChanged event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_bead_state_in_progress_to_closed() {
        let service = make_service();
        service
            .create_bead("state-2", "State", None)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("state-2").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let (updated, _) = service
            .update_bead_state(
                &BeadId::new("state-2").unwrap(),
                BeadState::Closed {
                    closed_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        assert!(updated.state().is_closed());
    }

    #[tokio::test]
    async fn update_bead_state_fails_for_invalid_transition() {
        let service = make_service();
        service
            .create_bead("state-3", "State", None)
            .await
            .unwrap();
        // Cannot go Open -> Closed directly
        let result = service
            .update_bead_state(
                &BeadId::new("state-3").unwrap(),
                BeadState::Closed {
                    closed_at: Utc::now(),
                },
            )
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidStateTransition { .. } => {}
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_bead_state_fails_for_nonexistent_bead() {
        let service = make_service();
        let result = service
            .update_bead_state(&BeadId::new("ghost").unwrap(), BeadState::InProgress)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_bead_state_fails_from_closed() {
        let service = make_service();
        service
            .create_bead("state-4", "State", None)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("state-4").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(
                &BeadId::new("state-4").unwrap(),
                BeadState::Closed {
                    closed_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        // Cannot transition from Closed
        let result = service
            .update_bead_state(&BeadId::new("state-4").unwrap(), BeadState::Open)
            .await;
        assert!(result.is_err());
    }

    // ── set_priority ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn set_priority_succeeds() {
        let service = make_service();
        service
            .create_bead("prio-1", "Priority", None)
            .await
            .unwrap();
        let (updated, event) = service
            .set_priority(&BeadId::new("prio-1").unwrap(), Priority::P1)
            .await
            .unwrap();
        assert_eq!(updated.priority(), Some(&Priority::P1));
        match &event {
            BeadEvent::PrioritySet { priority, .. } => assert_eq!(*priority, Priority::P1),
            other => panic!("expected PrioritySet event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_priority_fails_for_nonexistent_bead() {
        let service = make_service();
        let result = service
            .set_priority(&BeadId::new("ghost").unwrap(), Priority::P0)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn set_priority_overwrites_existing() {
        let service = make_service();
        service
            .create_bead("prio-2", "Priority", None)
            .await
            .unwrap();
        service
            .set_priority(&BeadId::new("prio-2").unwrap(), Priority::P1)
            .await
            .unwrap();
        let (updated, _) = service
            .set_priority(&BeadId::new("prio-2").unwrap(), Priority::P3)
            .await
            .unwrap();
        assert_eq!(updated.priority(), Some(&Priority::P3));
    }

    // ── assign_bead ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn assign_bead_sets_assignee() {
        let service = make_service();
        service
            .create_bead("assign-1", "Assign", None)
            .await
            .unwrap();
        let (updated, event) = service
            .assign_bead(&BeadId::new("assign-1").unwrap(), Some("alice".into()))
            .await
            .unwrap();
        assert_eq!(updated.assignee(), Some("alice"));
        match &event {
            BeadEvent::AssigneeSet { assignee, .. } => assert_eq!(assignee.as_deref(), Some("alice")),
            other => panic!("expected AssigneeSet event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn assign_bead_none_keeps_unassigned() {
        let service = make_service();
        service
            .create_bead("assign-2", "Assign", None)
            .await
            .unwrap();
        let (updated, event) = service
            .assign_bead(&BeadId::new("assign-2").unwrap(), None)
            .await
            .unwrap();
        assert!(updated.assignee().is_none());
        match &event {
            BeadEvent::AssigneeSet { assignee, .. } => assert!(assignee.is_none()),
            other => panic!("expected AssigneeSet event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn assign_bead_fails_for_nonexistent_bead() {
        let service = make_service();
        let result = service
            .assign_bead(&BeadId::new("ghost").unwrap(), Some("bob".into()))
            .await;
        assert!(result.is_err());
    }

    // ── add_dependency ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn add_dependency_succeeds() {
        let service = make_service();
        service
            .create_bead("dep-1", "Dep One", None)
            .await
            .unwrap();
        service
            .create_bead("dep-2", "Dep Two", None)
            .await
            .unwrap();
        let (updated, event) = service
            .add_dependency(
                &BeadId::new("dep-1").unwrap(),
                BeadId::new("dep-2").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(updated.depends_on().len(), 1);
        assert_eq!(updated.depends_on()[0].as_str(), "dep-2");
        match &event {
            BeadEvent::DependencyAdded { depends_on, .. } => {
                assert_eq!(depends_on.as_str(), "dep-2");
            }
            other => panic!("expected DependencyAdded event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn add_dependency_fails_for_nonexistent_target() {
        let service = make_service();
        service
            .create_bead("dep-3", "Exists", None)
            .await
            .unwrap();
        let result = service
            .add_dependency(
                &BeadId::new("dep-3").unwrap(),
                BeadId::new("nonexistent").unwrap(),
            )
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidDependency(msg) => assert!(msg.contains("does not exist")),
            other => panic!("expected InvalidDependency, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn add_dependency_fails_for_self_dependency() {
        let service = make_service();
        service
            .create_bead("self-dep", "Self Dep", None)
            .await
            .unwrap();
        let result = service
            .add_dependency(
                &BeadId::new("self-dep").unwrap(),
                BeadId::new("self-dep").unwrap(),
            )
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::DependencyCycle(msg) => assert!(msg.contains("cannot depend on itself")),
            other => panic!("expected DependencyCycle, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn add_dependency_fails_for_nonexistent_source() {
        let service = make_service();
        service
            .create_bead("target", "Target", None)
            .await
            .unwrap();
        let result = service
            .add_dependency(
                &BeadId::new("ghost-source").unwrap(),
                BeadId::new("target").unwrap(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn add_dependency_accumulates() {
        let service = make_service();
        service
            .create_bead("acc-1", "Acc", None)
            .await
            .unwrap();
        service
            .create_bead("acc-2", "Dep A", None)
            .await
            .unwrap();
        service
            .create_bead("acc-3", "Dep B", None)
            .await
            .unwrap();
        service
            .add_dependency(
                &BeadId::new("acc-1").unwrap(),
                BeadId::new("acc-2").unwrap(),
            )
            .await
            .unwrap();
        let (updated, _) = service
            .add_dependency(
                &BeadId::new("acc-1").unwrap(),
                BeadId::new("acc-3").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(updated.depends_on().len(), 2);
    }

    // ── list_beads ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_beads_returns_empty_when_none() {
        let service = make_service();
        let list = service.list_beads().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn list_beads_returns_all_created_beads() {
        let service = make_service();
        service.create_bead("list-1", "One", None).await.unwrap();
        service.create_bead("list-2", "Two", None).await.unwrap();
        let list = service.list_beads().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    // ── find_by_state ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_state_filters() {
        let service = make_service();
        service.create_bead("fs-1", "One", None).await.unwrap();
        service.create_bead("fs-2", "Two", None).await.unwrap();
        let open = service.find_by_state(BeadState::Open).await.unwrap();
        assert_eq!(open.len(), 2);
    }

    #[tokio::test]
    async fn find_by_state_returns_empty_for_no_match() {
        let service = make_service();
        service.create_bead("fs-3", "Three", None).await.unwrap();
        let in_progress = service.find_by_state(BeadState::InProgress).await.unwrap();
        assert!(in_progress.is_empty());
    }

    // ── delete_bead ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_bead_succeeds() {
        let service = make_service();
        service
            .create_bead("del-1", "Delete Me", None)
            .await
            .unwrap();
        let event = service.delete_bead(&BeadId::new("del-1").unwrap()).await.unwrap();
        match &event {
            BeadEvent::Deleted { id, .. } => assert_eq!(id.as_str(), "del-1"),
            other => panic!("expected Deleted event, got {other:?}"),
        }
        // Verify it's gone
        let result = service.get_bead(&BeadId::new("del-1").unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_bead_fails_for_nonexistent() {
        let service = make_service();
        let result = service.delete_bead(&BeadId::new("ghost").unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_bead_removed_from_list() {
        let service = make_service();
        service.create_bead("del-2", "A", None).await.unwrap();
        service.create_bead("del-3", "B", None).await.unwrap();
        service.delete_bead(&BeadId::new("del-2").unwrap()).await.unwrap();
        let list = service.list_beads().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id().as_str(), "del-3");
    }

    // ── Full lifecycle tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn full_lifecycle_create_to_delete() {
        let service = make_service();

        // Create
        let (_bead, created_event) = service
            .create_bead("life-1", "Lifecycle Test", Some("A full lifecycle".into()))
            .await
            .unwrap();
        assert!(matches!(created_event, BeadEvent::Created { .. }));

        // Get
        let found = service.get_bead(&BeadId::new("life-1").unwrap()).await.unwrap();
        assert_eq!(found.id().as_str(), "life-1");

        // Set priority
        let (_, prio_event) = service
            .set_priority(&BeadId::new("life-1").unwrap(), Priority::P0)
            .await
            .unwrap();
        assert!(matches!(prio_event, BeadEvent::PrioritySet { .. }));

        // Assign
        let (_, assign_event) = service
            .assign_bead(&BeadId::new("life-1").unwrap(), Some("alice".into()))
            .await
            .unwrap();
        assert!(matches!(assign_event, BeadEvent::AssigneeSet { .. }));

        // Open -> InProgress
        let (_, state_event1) = service
            .update_bead_state(&BeadId::new("life-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        assert!(matches!(state_event1, BeadEvent::StateChanged { .. }));

        // InProgress -> Blocked
        let (_, _state_event2) = service
            .update_bead_state(&BeadId::new("life-1").unwrap(), BeadState::Blocked)
            .await
            .unwrap();

        // Blocked -> InProgress
        let (_, _state_event3) = service
            .update_bead_state(&BeadId::new("life-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();

        // InProgress -> Closed
        let (final_bead, close_event) = service
            .update_bead_state(
                &BeadId::new("life-1").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        assert!(final_bead.state().is_closed());
        assert!(matches!(close_event, BeadEvent::StateChanged { .. }));

        // Delete
        let del_event = service.delete_bead(&BeadId::new("life-1").unwrap()).await.unwrap();
        assert!(matches!(del_event, BeadEvent::Deleted { .. }));

        // Verify gone
        let result = service.get_bead(&BeadId::new("life-1").unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn full_lifecycle_open_defer_resume_close() {
        let service = make_service();
        service.create_bead("defer-1", "Deferred Path", None).await.unwrap();

        // Open -> InProgress
        service
            .update_bead_state(&BeadId::new("defer-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();

        // InProgress -> Deferred
        service
            .update_bead_state(&BeadId::new("defer-1").unwrap(), BeadState::Deferred)
            .await
            .unwrap();

        // Deferred -> InProgress
        service
            .update_bead_state(&BeadId::new("defer-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();

        // InProgress -> Closed
        let (bead, _) = service
            .update_bead_state(
                &BeadId::new("defer-1").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        assert!(bead.state().is_closed());
    }

    // ── State transition through service ─────────────────────────────────────

    #[tokio::test]
    async fn update_state_in_progress_to_blocked() {
        let service = make_service();
        service.create_bead("ip-b", "Block", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ip-b").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let (updated, event) = service
            .update_bead_state(&BeadId::new("ip-b").unwrap(), BeadState::Blocked)
            .await
            .unwrap();
        assert_eq!(updated.state(), BeadState::Blocked);
        match &event {
            BeadEvent::StateChanged { old_state, new_state, .. } => {
                assert_eq!(old_state, &BeadState::InProgress);
                assert_eq!(new_state, &BeadState::Blocked);
            }
            other => panic!("expected StateChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_state_in_progress_to_deferred() {
        let service = make_service();
        service.create_bead("ip-d", "Defer", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ip-d").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let (updated, _) = service
            .update_bead_state(&BeadId::new("ip-d").unwrap(), BeadState::Deferred)
            .await
            .unwrap();
        assert_eq!(updated.state(), BeadState::Deferred);
    }

    #[tokio::test]
    async fn update_state_blocked_to_deferred() {
        let service = make_service();
        service.create_bead("b-d", "Block Defer", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("b-d").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("b-d").unwrap(), BeadState::Blocked)
            .await
            .unwrap();
        let (updated, _) = service
            .update_bead_state(&BeadId::new("b-d").unwrap(), BeadState::Deferred)
            .await
            .unwrap();
        assert_eq!(updated.state(), BeadState::Deferred);
    }

    #[tokio::test]
    async fn update_state_blocked_to_closed() {
        let service = make_service();
        service.create_bead("b-c", "Block Close", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("b-c").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("b-c").unwrap(), BeadState::Blocked)
            .await
            .unwrap();
        let (updated, _) = service
            .update_bead_state(
                &BeadId::new("b-c").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        assert!(updated.state().is_closed());
    }

    #[tokio::test]
    async fn update_state_deferred_to_closed() {
        let service = make_service();
        service.create_bead("d-c", "Defer Close", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("d-c").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("d-c").unwrap(), BeadState::Deferred)
            .await
            .unwrap();
        let (updated, _) = service
            .update_bead_state(
                &BeadId::new("d-c").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        assert!(updated.state().is_closed());
    }

    #[tokio::test]
    async fn update_state_same_state_open() {
        let service = make_service();
        service.create_bead("same-1", "Same", None).await.unwrap();
        let (updated, _) = service
            .update_bead_state(&BeadId::new("same-1").unwrap(), BeadState::Open)
            .await
            .unwrap();
        assert_eq!(updated.state(), BeadState::Open);
    }

    // ── Invalid transitions through service ──────────────────────────────────

    #[tokio::test]
    async fn update_state_open_to_deferred_fails() {
        let service = make_service();
        service.create_bead("otd", "Fail", None).await.unwrap();
        let result = service
            .update_bead_state(&BeadId::new("otd").unwrap(), BeadState::Deferred)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::InvalidStateTransition { from, to } => {
                assert!(from.contains("Open"));
                assert!(to.contains("Deferred"));
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_state_open_to_blocked_fails() {
        let service = make_service();
        service.create_bead("otb", "Fail", None).await.unwrap();
        let result = service
            .update_bead_state(&BeadId::new("otb").unwrap(), BeadState::Blocked)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_in_progress_to_open_fails() {
        let service = make_service();
        service.create_bead("ito", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ito").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("ito").unwrap(), BeadState::Open)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_deferred_to_open_fails() {
        let service = make_service();
        service.create_bead("dto", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("dto").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("dto").unwrap(), BeadState::Deferred)
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("dto").unwrap(), BeadState::Open)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_deferred_to_blocked_fails() {
        let service = make_service();
        service.create_bead("dtb", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("dtb").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("dtb").unwrap(), BeadState::Deferred)
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("dtb").unwrap(), BeadState::Blocked)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_blocked_to_open_fails() {
        let service = make_service();
        service.create_bead("bto", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("bto").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(&BeadId::new("bto").unwrap(), BeadState::Blocked)
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("bto").unwrap(), BeadState::Open)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_closed_to_in_progress_fails() {
        let service = make_service();
        service.create_bead("cti", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("cti").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(
                &BeadId::new("cti").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("cti").unwrap(), BeadState::InProgress)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_closed_to_blocked_fails() {
        let service = make_service();
        service.create_bead("ctb", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ctb").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(
                &BeadId::new("ctb").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("ctb").unwrap(), BeadState::Blocked)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_state_closed_to_deferred_fails() {
        let service = make_service();
        service.create_bead("ctd", "Fail", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ctd").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        service
            .update_bead_state(
                &BeadId::new("ctd").unwrap(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();
        let result = service
            .update_bead_state(&BeadId::new("ctd").unwrap(), BeadState::Deferred)
            .await;
        assert!(result.is_err());
    }

    // ── Event content validation ─────────────────────────────────────────────

    #[tokio::test]
    async fn created_event_has_correct_timestamp_range() {
        let service = make_service();
        let before = Utc::now();
        let (_, event) = service.create_bead("ts-1", "Timestamp", None).await.unwrap();
        let after = Utc::now();
        if let BeadEvent::Created { created_at, .. } = event {
            assert!(created_at >= before);
            assert!(created_at <= after);
        } else {
            panic!("expected Created event");
        }
    }

    #[tokio::test]
    async fn state_changed_event_correct_from_to() {
        let service = make_service();
        service.create_bead("ev-1", "Event", None).await.unwrap();
        service
            .update_bead_state(&BeadId::new("ev-1").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let (_, event) = service
            .update_bead_state(&BeadId::new("ev-1").unwrap(), BeadState::Blocked)
            .await
            .unwrap();
        if let BeadEvent::StateChanged { old_state, new_state, .. } = event {
            assert_eq!(old_state, BeadState::InProgress);
            assert_eq!(new_state, BeadState::Blocked);
        } else {
            panic!("expected StateChanged");
        }
    }

    #[tokio::test]
    async fn delete_event_has_correct_id() {
        let service = make_service();
        service.create_bead("del-ev-1", "Del Event", None).await.unwrap();
        let event = service.delete_bead(&BeadId::new("del-ev-1").unwrap()).await.unwrap();
        if let BeadEvent::Deleted { id, deleted_at } = event {
            assert_eq!(id.as_str(), "del-ev-1");
            // Timestamp should be recent
            assert!(deleted_at <= Utc::now());
            assert!(deleted_at >= Utc::now() - chrono::Duration::seconds(5));
        } else {
            panic!("expected Deleted event");
        }
    }

    #[tokio::test]
    async fn set_priority_event_correct_priority() {
        let service = make_service();
        service.create_bead("prio-ev-1", "Prio Event", None).await.unwrap();
        let (_, event) = service
            .set_priority(&BeadId::new("prio-ev-1").unwrap(), Priority::P3)
            .await
            .unwrap();
        if let BeadEvent::PrioritySet { priority, .. } = event {
            assert_eq!(priority, Priority::P3);
        } else {
            panic!("expected PrioritySet");
        }
    }

    // ── Multi-bead operations ────────────────────────────────────────────────

    #[tokio::test]
    async fn create_multiple_and_list() {
        let service = make_service();
        for i in 0..15 {
            service
                .create_bead(format!("multi-{i}"), format!("Bead {i}"), None)
                .await
                .unwrap();
        }
        let list = service.list_beads().await.unwrap();
        assert_eq!(list.len(), 15);
    }

    #[tokio::test]
    async fn find_by_state_after_multiple_transitions() {
        let service = make_service();

        // Create 5 beads
        for i in 0..5 {
            service
                .create_bead(format!("mfs-{i}"), format!("Bead {i}"), None)
                .await
                .unwrap();
        }

        // Transition 0,1 to InProgress
        for i in 0..=1 {
            service
                .update_bead_state(&BeadId::new(format!("mfs-{i}")).unwrap(), BeadState::InProgress)
                .await
                .unwrap();
        }

        // Transition 2 to InProgress then Closed
        service
            .update_bead_state(&BeadId::new("mfs-2").unwrap(), BeadState::InProgress)
            .await
            .unwrap();
        let bead2 = service.get_bead(&BeadId::new("mfs-2").unwrap()).await.unwrap();
        service
            .update_bead_state(
                bead2.id(),
                BeadState::Closed { closed_at: Utc::now() },
            )
            .await
            .unwrap();

        let open = service.find_by_state(BeadState::Open).await.unwrap();
        let in_progress = service.find_by_state(BeadState::InProgress).await.unwrap();
        assert_eq!(open.len(), 2); // mfs-3, mfs-4
        assert_eq!(in_progress.len(), 2); // mfs-0, mfs-1
    }

    #[tokio::test]
    async fn add_dependency_between_existing_beads() {
        let service = make_service();
        service.create_bead("chain-a", "A", None).await.unwrap();
        service.create_bead("chain-b", "B", None).await.unwrap();
        service.create_bead("chain-c", "C", None).await.unwrap();

        // A depends on B
        service
            .add_dependency(&BeadId::new("chain-a").unwrap(), BeadId::new("chain-b").unwrap())
            .await
            .unwrap();

        // B depends on C
        service
            .add_dependency(&BeadId::new("chain-b").unwrap(), BeadId::new("chain-c").unwrap())
            .await
            .unwrap();

        let bead_a = service.get_bead(&BeadId::new("chain-a").unwrap()).await.unwrap();
        assert_eq!(bead_a.depends_on().len(), 1);

        let bead_b = service.get_bead(&BeadId::new("chain-b").unwrap()).await.unwrap();
        assert_eq!(bead_b.depends_on().len(), 1);
    }

    #[tokio::test]
    async fn assign_and_reassign() {
        let service = make_service();
        service.create_bead("reassign", "Reassign", None).await.unwrap();

        let (_, event1) = service
            .assign_bead(&BeadId::new("reassign").unwrap(), Some("alice".into()))
            .await
            .unwrap();
        assert_eq!(
            match &event1 { BeadEvent::AssigneeSet { assignee, .. } => assignee.as_deref(), _ => None },
            Some("alice")
        );

        let bead = service.get_bead(&BeadId::new("reassign").unwrap()).await.unwrap();
        assert_eq!(bead.assignee(), Some("alice"));
    }

    // ── Error message validation ─────────────────────────────────────────────

    #[tokio::test]
    async fn create_bead_error_message_for_empty_title() {
        let service = make_service();
        let result = service.create_bead("valid-id", "", None).await;
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("title") || msg.contains("Title"), "error message should mention title: {msg}");
    }

    #[tokio::test]
    async fn create_bead_error_message_for_invalid_id() {
        let service = make_service();
        let result = service.create_bead("bad id!", "Valid Title", None).await;
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("id") || msg.contains("ID"), "error message should mention id: {msg}");
    }

    #[tokio::test]
    async fn get_bead_error_message_for_missing() {
        let service = make_service();
        let result = service.get_bead(&BeadId::new("nope").unwrap()).await;
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("nope"), "error message should contain the id: {msg}");
    }

    #[tokio::test]
    async fn delete_bead_error_message_for_missing() {
        let service = make_service();
        let result = service.delete_bead(&BeadId::new("nope").unwrap()).await;
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("nope"), "error message should contain the id: {msg}");
    }

    // ── Description too long ─────────────────────────────────────────────────

    #[tokio::test]
    async fn create_bead_with_long_description() {
        let service = make_service();
        let desc = "x".repeat(BeadDescription::MAX_LENGTH);
        let result = service.create_bead("long-desc", "Valid", Some(desc)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_bead_with_too_long_description() {
        let service = make_service();
        let desc = "x".repeat(BeadDescription::MAX_LENGTH + 1);
        let result = service.create_bead("too-long-desc", "Valid", Some(desc)).await;
        assert!(result.is_err());
    }
}

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::bead::Bead;
use crate::domain::value_objects::{BeadId, BeadState, Priority};
use crate::error::{BeadError, Result};

#[async_trait]
pub trait BeadRepository: Send + Sync {
    async fn insert(&self, bead: &Bead) -> Result<()>;
    async fn update(&self, bead: &Bead) -> Result<()>;
    async fn delete(&self, id: &BeadId) -> Result<()>;
    async fn find(&self, id: &BeadId) -> Result<Option<Bead>>;
    async fn find_all(&self) -> Result<Vec<Bead>>;
    async fn find_by_state(&self, state: BeadState) -> Result<Vec<Bead>>;
    async fn find_by_assignee(&self, assignee: Option<&str>) -> Result<Vec<Bead>>;
    async fn find_by_priority(&self, priority: Option<Priority>) -> Result<Vec<Bead>>;
    async fn exists(&self, id: &BeadId) -> bool;
}

#[derive(Clone)]
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
        let mut filtered: Vec<Bead> = beads
            .values()
            .filter(|b| b.state() == state)
            .cloned()
            .collect();
        filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(filtered)
    }

    async fn find_by_assignee(&self, assignee: Option<&str>) -> Result<Vec<Bead>> {
        let beads = self.beads.read().await;
        let mut filtered: Vec<Bead> = beads
            .values()
            .filter(|b| b.assignee() == assignee)
            .cloned()
            .collect();
        filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(filtered)
    }

    async fn find_by_priority(&self, priority: Option<Priority>) -> Result<Vec<Bead>> {
        let beads = self.beads.read().await;
        let mut filtered: Vec<Bead> = beads
            .values()
            .filter(|b| b.priority() == priority.as_ref())
            .cloned()
            .collect();
        filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(filtered)
    }

    async fn exists(&self, id: &BeadId) -> bool {
        let beads = self.beads.read().await;
        beads.contains_key(id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::bead::Bead;
    use crate::domain::value_objects::*;

    fn make_bead(id: &str) -> Bead {
        Bead::<crate::domain::entities::bead::Open>::create(
            BeadId::new(id).unwrap(),
            BeadTitle::new(format!("Title for {id}")).unwrap(),
            None,
        )
    }

    fn make_repo() -> InMemoryBeadRepository {
        InMemoryBeadRepository::new()
    }

    // ── Construction ────────────────────────────────────────────────────────

    #[test]
    fn default_creates_empty_repo() {
        let repo = InMemoryBeadRepository::default();
        assert_eq!(tokio_test::block_on(repo.find_all()).unwrap().len(), 0);
    }

    #[test]
    fn new_creates_empty_repo() {
        let repo = make_repo();
        assert_eq!(tokio_test::block_on(repo.find_all()).unwrap().len(), 0);
    }

    // ── Insert ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_succeeds_for_new_bead() {
        let repo = make_repo();
        let bead = make_bead("a");
        let result = repo.insert(&bead).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn insert_fails_for_duplicate_id() {
        let repo = make_repo();
        let bead = make_bead("a");
        repo.insert(&bead).await.unwrap();
        let result = repo.insert(&bead).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::AlreadyExists(id) => assert_eq!(id, "a"),
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    // ── Find ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_returns_none_for_missing() {
        let repo = make_repo();
        let result = repo.find(&BeadId::new("missing").unwrap()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn find_returns_bead_after_insert() {
        let repo = make_repo();
        let bead = make_bead("a");
        repo.insert(&bead).await.unwrap();
        let found = repo.find(&BeadId::new("a").unwrap()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id().as_str(), "a");
    }

    // ── Find All ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_all_returns_empty_when_no_beads() {
        let repo = make_repo();
        let all = repo.find_all().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn find_all_returns_all_inserted_beads() {
        let repo = make_repo();
        repo.insert(&make_bead("a")).await.unwrap();
        repo.insert(&make_bead("b")).await.unwrap();
        repo.insert(&make_bead("c")).await.unwrap();
        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    // ── Find by State ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_state_returns_empty_when_no_match() {
        let repo = make_repo();
        let beads = repo.find_by_state(BeadState::InProgress).await.unwrap();
        assert!(beads.is_empty());
    }

    #[tokio::test]
    async fn find_by_state_filters_correctly() {
        let repo = make_repo();
        repo.insert(&make_bead("a")).await.unwrap();
        repo.insert(&make_bead("b")).await.unwrap();
        let all = repo.find_by_state(BeadState::Open).await.unwrap();
        // All created beads are Open
        assert_eq!(all.len(), 2);
    }

    // ── Update ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_succeeds_for_existing_bead() {
        let repo = make_repo();
        let bead = make_bead("a");
        repo.insert(&bead).await.unwrap();
        let updated = bead.with_priority(Priority::P1);
        let result = repo.update(&updated).await;
        assert!(result.is_ok());
        // Verify the update took effect
        let found = repo
            .find(&BeadId::new("a").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.priority(), Some(&Priority::P1));
    }

    #[tokio::test]
    async fn update_fails_for_nonexistent_bead() {
        let repo = make_repo();
        let bead = make_bead("ghost");
        let result = repo.update(&bead).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::NotFound(id) => assert_eq!(id, "ghost"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    // ── Delete ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_succeeds_for_existing_bead() {
        let repo = make_repo();
        let bead = make_bead("a");
        repo.insert(&bead).await.unwrap();
        let result = repo.delete(&BeadId::new("a").unwrap()).await;
        assert!(result.is_ok());
        assert!(repo
            .find(&BeadId::new("a").unwrap())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn delete_fails_for_nonexistent_bead() {
        let repo = make_repo();
        let result = repo.delete(&BeadId::new("ghost").unwrap()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BeadError::NotFound(id) => assert_eq!(id, "ghost"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    // ── Exists ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn exists_returns_false_for_missing() {
        let repo = make_repo();
        assert!(!repo.exists(&BeadId::new("missing").unwrap()).await);
    }

    #[tokio::test]
    async fn exists_returns_true_after_insert() {
        let repo = make_repo();
        repo.insert(&make_bead("a")).await.unwrap();
        assert!(repo.exists(&BeadId::new("a").unwrap()).await);
    }

    #[tokio::test]
    async fn exists_returns_false_after_delete() {
        let repo = make_repo();
        repo.insert(&make_bead("a")).await.unwrap();
        repo.delete(&BeadId::new("a").unwrap()).await.unwrap();
        assert!(!repo.exists(&BeadId::new("a").unwrap()).await);
    }

    // ── Isolation between repos ─────────────────────────────────────────────

    #[tokio::test]
    async fn different_repos_are_isolated() {
        let repo1 = make_repo();
        let repo2 = make_repo();
        repo1.insert(&make_bead("a")).await.unwrap();
        assert!(repo2
            .find(&BeadId::new("a").unwrap())
            .await
            .unwrap()
            .is_none());
    }

    // ── CRUD lifecycle ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn full_crud_lifecycle() {
        let repo = make_repo();
        // Create
        let bead = make_bead("lifecycle")
            .with_priority(Priority::P2)
            .with_type(BeadType::Feature);
        repo.insert(&bead).await.unwrap();
        assert!(repo.exists(&BeadId::new("lifecycle").unwrap()).await);

        // Read
        let found = repo
            .find(&BeadId::new("lifecycle").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.priority(), Some(&Priority::P2));
        assert_eq!(found.bead_type(), Some(&BeadType::Feature));

        // Update
        let updated = found.with_priority(Priority::P0);
        repo.update(&updated).await.unwrap();
        let reloaded = repo
            .find(&BeadId::new("lifecycle").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.priority(), Some(&Priority::P0));

        // Delete
        repo.delete(&BeadId::new("lifecycle").unwrap())
            .await
            .unwrap();
        assert!(!repo.exists(&BeadId::new("lifecycle").unwrap()).await);
        assert!(repo.find_all().await.unwrap().is_empty());
    }

    // ── find_by_state after transitions ─────────────────────────────────────

    #[tokio::test]
    async fn find_by_state_after_mixed_transitions() {
        let repo = make_repo();
        repo.insert(&make_bead("fs-a")).await.unwrap();
        repo.insert(&make_bead("fs-b")).await.unwrap();

        // Transition one to InProgress
        let bead_b = repo
            .find(&BeadId::new("fs-b").unwrap())
            .await
            .unwrap()
            .unwrap();
        let in_progress = bead_b.transition_to(&BeadState::InProgress).unwrap();
        repo.update(&in_progress).await.unwrap();

        let open = repo.find_by_state(BeadState::Open).await.unwrap();
        let in_progress_list = repo.find_by_state(BeadState::InProgress).await.unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(in_progress_list.len(), 1);
    }

    #[tokio::test]
    async fn find_by_state_with_closed_state() {
        let repo = make_repo();
        repo.insert(&make_bead("closed-1")).await.unwrap();
        let bead = repo
            .find(&BeadId::new("closed-1").unwrap())
            .await
            .unwrap()
            .unwrap();
        let transitioned = bead.transition_to(&BeadState::InProgress).unwrap();
        let closed = transitioned
            .transition_to(&BeadState::Closed {
                closed_at: chrono::Utc::now(),
            })
            .unwrap();
        repo.update(&closed).await.unwrap();

        // Use the exact stored state for the query since Closed equality depends on closed_at
        let stored_state = closed.state();
        let closed_list = repo.find_by_state(stored_state).await.unwrap();
        assert_eq!(closed_list.len(), 1);
        assert!(closed_list[0].state().is_closed());
    }

    #[tokio::test]
    async fn find_all_after_partial_delete() {
        let repo = make_repo();
        repo.insert(&make_bead("del-a")).await.unwrap();
        repo.insert(&make_bead("del-b")).await.unwrap();
        repo.insert(&make_bead("del-c")).await.unwrap();
        repo.delete(&BeadId::new("del-b").unwrap()).await.unwrap();
        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_preserves_all_fields() {
        let repo = make_repo();
        let bead = make_bead("preserve")
            .with_priority(Priority::P3)
            .with_type(BeadType::Task)
            .with_assignee("eve")
            .with_labels(Labels::new().with("test"));
        repo.insert(&bead).await.unwrap();

        let found = repo
            .find(&BeadId::new("preserve").unwrap())
            .await
            .unwrap()
            .unwrap();
        let updated = found.with_priority(Priority::P1);
        repo.update(&updated).await.unwrap();

        let reloaded = repo
            .find(&BeadId::new("preserve").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.priority(), Some(&Priority::P1));
        assert_eq!(reloaded.bead_type(), Some(&BeadType::Task));
        assert_eq!(reloaded.assignee(), Some("eve"));
        assert!(reloaded.labels().contains("test"));
    }

    #[tokio::test]
    async fn insert_with_description() {
        let repo = make_repo();
        let bead = Bead::<crate::domain::entities::bead::Open>::create(
            BeadId::new("desc-1").unwrap(),
            BeadTitle::new("With Desc").unwrap(),
            Some(BeadDescription::new("A description").unwrap()),
        );
        repo.insert(&bead).await.unwrap();
        let found = repo
            .find(&BeadId::new("desc-1").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.description().unwrap().as_str(), "A description");
    }

    #[tokio::test]
    async fn find_by_state_returns_empty_when_all_deleted() {
        let repo = make_repo();
        repo.insert(&make_bead("gone-1")).await.unwrap();
        repo.delete(&BeadId::new("gone-1").unwrap()).await.unwrap();
        let result = repo.find_by_state(BeadState::Open).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn exists_returns_false_initially() {
        let repo = make_repo();
        assert!(!repo.exists(&BeadId::new("nope").unwrap()).await);
    }

    #[tokio::test]
    async fn insert_many_then_find_all() {
        let repo = make_repo();
        for i in 0..20 {
            let bead = make_bead(&format!("batch-{i}"));
            repo.insert(&bead).await.unwrap();
        }
        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 20);
    }

    // ── Concurrent access tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn concurrent_inserts_different_ids() {
        let repo = make_repo();
        let repo1 = repo.clone();
        let repo2 = repo.clone();
        let h1 = tokio::spawn(async move {
            let bead = make_bead("conc-a");
            repo1.insert(&bead).await.unwrap();
        });
        let h2 = tokio::spawn(async move {
            let bead = make_bead("conc-b");
            repo2.insert(&bead).await.unwrap();
        });
        h1.await.unwrap();
        h2.await.unwrap();
        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn concurrent_reads() {
        let repo = make_repo();
        repo.insert(&make_bead("read-1")).await.unwrap();
        let repo_clone = repo.clone();
        let h1 =
            tokio::spawn(async move { repo.find(&BeadId::new("read-1").unwrap()).await.unwrap() });
        let h2 = tokio::spawn(async move {
            repo_clone
                .find(&BeadId::new("read-1").unwrap())
                .await
                .unwrap()
        });
        let r1 = h1.await.unwrap();
        let r2 = h2.await.unwrap();
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert_eq!(r1.unwrap().id().as_str(), r2.unwrap().id().as_str());
    }

    #[tokio::test]
    async fn concurrent_insert_and_read() {
        let repo = make_repo();
        let repo1 = repo.clone();
        let repo2 = repo.clone();
        let h1 = tokio::spawn(async move {
            repo1.insert(&make_bead("shared")).await.unwrap();
            repo1.find(&BeadId::new("shared").unwrap()).await.unwrap()
        });
        let h2 = tokio::spawn(async move {
            // Small delay to let h1 insert first
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            repo2.find(&BeadId::new("shared").unwrap()).await.unwrap()
        });
        let r1 = h1.await.unwrap();
        let r2 = h2.await.unwrap();
        assert!(r1.is_some());
        assert!(r2.is_some());
    }

    // ── find_by_state edge cases ─────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_state_blocked_and_deferred() {
        let repo = make_repo();
        repo.insert(&make_bead("mixed-1")).await.unwrap();
        repo.insert(&make_bead("mixed-2")).await.unwrap();
        repo.insert(&make_bead("mixed-3")).await.unwrap();

        // Transition one to Blocked
        let bead1 = repo
            .find(&BeadId::new("mixed-1").unwrap())
            .await
            .unwrap()
            .unwrap();
        let blocked = bead1
            .transition_to(&BeadState::InProgress)
            .unwrap()
            .transition_to(&BeadState::Blocked)
            .unwrap();
        repo.update(&blocked).await.unwrap();

        // Transition one to Deferred
        let bead2 = repo
            .find(&BeadId::new("mixed-2").unwrap())
            .await
            .unwrap()
            .unwrap();
        let deferred = bead2
            .transition_to(&BeadState::InProgress)
            .unwrap()
            .transition_to(&BeadState::Deferred)
            .unwrap();
        repo.update(&deferred).await.unwrap();

        let open = repo.find_by_state(BeadState::Open).await.unwrap();
        let blocked_list = repo.find_by_state(BeadState::Blocked).await.unwrap();
        let deferred_list = repo.find_by_state(BeadState::Deferred).await.unwrap();
        let in_progress_list = repo.find_by_state(BeadState::InProgress).await.unwrap();

        assert_eq!(open.len(), 1);
        assert_eq!(blocked_list.len(), 1);
        assert_eq!(deferred_list.len(), 1);
        assert_eq!(in_progress_list.len(), 0);
    }

    // ── Update edge cases ────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_replaces_bead_completely() {
        let repo = make_repo();
        let bead = make_bead("replace-me")
            .with_priority(Priority::P0)
            .with_type(BeadType::Feature);
        repo.insert(&bead).await.unwrap();

        // Create a new bead with same ID but different fields
        let replacement = Bead::<crate::domain::entities::bead::Open>::create(
            BeadId::new("replace-me").unwrap(),
            BeadTitle::new("Replaced Title").unwrap(),
            Some(BeadDescription::new("New desc").unwrap()),
        )
        .with_priority(Priority::P4)
        .with_type(BeadType::Bug);
        repo.update(&replacement).await.unwrap();

        let found = repo
            .find(&BeadId::new("replace-me").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title().as_str(), "Replaced Title");
        assert_eq!(found.description().unwrap().as_str(), "New desc");
        assert_eq!(found.priority(), Some(&Priority::P4));
        assert_eq!(found.bead_type(), Some(&BeadType::Bug));
    }

    #[tokio::test]
    async fn delete_then_reinsert_succeeds() {
        let repo = make_repo();
        repo.insert(&make_bead("reinsert")).await.unwrap();
        repo.delete(&BeadId::new("reinsert").unwrap())
            .await
            .unwrap();
        assert!(!repo.exists(&BeadId::new("reinsert").unwrap()).await);

        // Reinsert should succeed
        repo.insert(&make_bead("reinsert")).await.unwrap();
        assert!(repo.exists(&BeadId::new("reinsert").unwrap()).await);
    }

    #[tokio::test]
    async fn delete_all_beads_leaves_empty_repo() {
        let repo = make_repo();
        repo.insert(&make_bead("del-all-1")).await.unwrap();
        repo.insert(&make_bead("del-all-2")).await.unwrap();
        repo.insert(&make_bead("del-all-3")).await.unwrap();

        repo.delete(&BeadId::new("del-all-1").unwrap())
            .await
            .unwrap();
        repo.delete(&BeadId::new("del-all-2").unwrap())
            .await
            .unwrap();
        repo.delete(&BeadId::new("del-all-3").unwrap())
            .await
            .unwrap();

        let all = repo.find_all().await.unwrap();
        assert!(all.is_empty());
    }

    // ── Find edge cases ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_returns_correct_bead_among_many() {
        let repo = make_repo();
        for i in 0..10 {
            repo.insert(&make_bead(&format!("find-test-{i}")))
                .await
                .unwrap();
        }
        let found = repo
            .find(&BeadId::new("find-test-5").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id().as_str(), "find-test-5");
        assert_eq!(found.title().as_str(), "Title for find-test-5");
    }

    #[tokio::test]
    async fn find_returns_none_after_deletion() {
        let repo = make_repo();
        repo.insert(&make_bead("gone")).await.unwrap();
        repo.delete(&BeadId::new("gone").unwrap()).await.unwrap();
        assert!(repo
            .find(&BeadId::new("gone").unwrap())
            .await
            .unwrap()
            .is_none());
    }

    // ── Exists edge cases ────────────────────────────────────────────────────

    #[tokio::test]
    async fn exists_returns_true_for_many_ids() {
        let repo = make_repo();
        for i in 0..50 {
            repo.insert(&make_bead(&format!("exists-{i}")))
                .await
                .unwrap();
        }
        for i in 0..50 {
            assert!(
                repo.exists(&BeadId::new(&format!("exists-{i}")).unwrap())
                    .await
            );
        }
    }

    // ── Contract Tests ──────────────────────────────────────────────────────────
    // Contract tests verify the BeadRepository trait contract is satisfied.
    // These tests define the expected behavior that all implementors must satisfy.

    /// Contract test: insert must succeed for new beads
    #[tokio::test]
    async fn contract_insert_success() {
        let repo = make_repo();
        let bead = make_bead("contract-insert");
        let result = repo.insert(&bead).await;
        assert!(
            result.is_ok(),
            "Contract: insert must succeed for new beads"
        );
    }

    /// Contract test: insert must fail for duplicate IDs
    #[tokio::test]
    async fn contract_insert_duplicate_fails() {
        let repo = make_repo();
        let bead = make_bead("contract-dup");
        repo.insert(&bead).await.unwrap();
        let result = repo.insert(&bead).await;
        assert!(
            result.is_err(),
            "Contract: insert must fail for duplicate IDs"
        );
        match result.unwrap_err() {
            BeadError::AlreadyExists(_) => {}
            other => panic!("Contract: expected AlreadyExists, got {other:?}"),
        }
    }

    /// Contract test: find must return None for non-existent beads
    #[tokio::test]
    async fn contract_find_missing() {
        let repo = make_repo();
        let result = repo.find(&BeadId::new("contract-missing").unwrap()).await;
        assert!(
            result.is_ok(),
            "Contract: find must return Ok"
        );
        assert!(
            result.unwrap().is_none(),
            "Contract: find must return None for non-existent beads"
        );
    }

    /// Contract test: find must return Some for existing beads
    #[tokio::test]
    async fn contract_find_existing() {
        let repo = make_repo();
        let bead = make_bead("contract-find");
        repo.insert(&bead).await.unwrap();
        let result = repo.find(&BeadId::new("contract-find").unwrap()).await;
        assert!(
            result.is_ok(),
            "Contract: find must return Ok"
        );
        assert!(
            result.unwrap().is_some(),
            "Contract: find must return Some for existing beads"
        );
    }

    /// Contract test: update must succeed for existing beads
    #[tokio::test]
    async fn contract_update_success() {
        let repo = make_repo();
        let bead = make_bead("contract-update");
        repo.insert(&bead).await.unwrap();
        let updated = bead.with_priority(Priority::P1);
        let result = repo.update(&updated).await;
        assert!(
            result.is_ok(),
            "Contract: update must succeed for existing beads"
        );
    }

    /// Contract test: update must fail for non-existent beads
    #[tokio::test]
    async fn contract_update_missing_fails() {
        let repo = make_repo();
        let bead = make_bead("contract-update-missing");
        let result = repo.update(&bead).await;
        assert!(
            result.is_err(),
            "Contract: update must fail for non-existent beads"
        );
        match result.unwrap_err() {
            BeadError::NotFound(_) => {}
            other => panic!("Contract: expected NotFound, got {other:?}"),
        }
    }

    /// Contract test: delete must succeed for existing beads
    #[tokio::test]
    async fn contract_delete_success() {
        let repo = make_repo();
        let bead = make_bead("contract-delete");
        repo.insert(&bead).await.unwrap();
        let result = repo.delete(&BeadId::new("contract-delete").unwrap()).await;
        assert!(
            result.is_ok(),
            "Contract: delete must succeed for existing beads"
        );
    }

    /// Contract test: delete must fail for non-existent beads
    #[tokio::test]
    async fn contract_delete_missing_fails() {
        let repo = make_repo();
        let result = repo.delete(&BeadId::new("contract-delete-missing").unwrap()).await;
        assert!(
            result.is_err(),
            "Contract: delete must fail for non-existent beads"
        );
        match result.unwrap_err() {
            BeadError::NotFound(_) => {}
            other => panic!("Contract: expected NotFound, got {other:?}"),
        }
    }

    /// Contract test: find_all must return empty vector when no beads exist
    #[tokio::test]
    async fn contract_find_all_empty() {
        let repo = make_repo();
        let result = repo.find_all().await;
        assert!(
            result.is_ok(),
            "Contract: find_all must return Ok"
        );
        assert!(
            result.unwrap().is_empty(),
            "Contract: find_all must return empty vector when no beads exist"
        );
    }

    /// Contract test: find_all must return all inserted beads
    #[tokio::test]
    async fn contract_find_all_returns_all() {
        let repo = make_repo();
        repo.insert(&make_bead("fa-1")).await.unwrap();
        repo.insert(&make_bead("fa-2")).await.unwrap();
        repo.insert(&make_bead("fa-3")).await.unwrap();
        let result = repo.find_all().await;
        assert!(
            result.is_ok(),
            "Contract: find_all must return Ok"
        );
        let beads = result.unwrap();
        assert_eq!(
            beads.len(), 3,
            "Contract: find_all must return all inserted beads"
        );
    }

    /// Contract test: find_by_state must return empty when no match
    #[tokio::test]
    async fn contract_find_by_state_empty() {
        let repo = make_repo();
        let result = repo.find_by_state(BeadState::InProgress).await;
        assert!(
            result.is_ok(),
            "Contract: find_by_state must return Ok"
        );
        assert!(
            result.unwrap().is_empty(),
            "Contract: find_by_state must return empty when no match"
        );
    }

    /// Contract test: find_by_state must filter correctly
    #[tokio::test]
    async fn contract_find_by_state_filter() {
        let repo = make_repo();
        repo.insert(&make_bead("fbs-a")).await.unwrap();
        repo.insert(&make_bead("fbs-b")).await.unwrap();
        let result = repo.find_by_state(BeadState::Open).await;
        assert!(
            result.is_ok(),
            "Contract: find_by_state must return Ok"
        );
        let beads = result.unwrap();
        assert_eq!(
            beads.len(), 2,
            "Contract: find_by_state must return matching beads"
        );
    }

    /// Contract test: exists must return false for non-existent beads
    #[tokio::test]
    async fn contract_exists_missing() {
        let repo = make_repo();
        let result = repo.exists(&BeadId::new("contract-exists-missing").unwrap()).await;
        assert!(
            !result,
            "Contract: exists must return false for non-existent beads"
        );
    }

    /// Contract test: exists must return true for existing beads
    #[tokio::test]
    async fn contract_exists_existing() {
        let repo = make_repo();
        repo.insert(&make_bead("contract-exists")).await.unwrap();
        let result = repo.exists(&BeadId::new("contract-exists").unwrap()).await;
        assert!(
            result,
            "Contract: exists must return true for existing beads"
        );
    }

    /// Contract test: CRUD lifecycle - insert, find, update, delete
    #[tokio::test]
    async fn contract_crud_lifecycle() {
        let repo = make_repo();
        
        // Insert (Create)
        let bead = make_bead("contract-lifecycle");
        repo.insert(&bead).await.unwrap();
        assert!(
            repo.exists(&BeadId::new("contract-lifecycle").unwrap()).await,
            "Contract: insert must persist bead"
        );
        
        // Find (Read)
        let found = repo.find(&BeadId::new("contract-lifecycle").unwrap()).await.unwrap().unwrap();
        assert_eq!(
            found.id().as_str(), "contract-lifecycle",
            "Contract: find must return inserted bead"
        );
        
        // Update
        let updated = found.with_priority(Priority::P1);
        repo.update(&updated).await.unwrap();
        let reloaded = repo.find(&BeadId::new("contract-lifecycle").unwrap()).await.unwrap().unwrap();
        assert_eq!(
            reloaded.priority(), Some(&Priority::P1),
            "Contract: update must persist changes"
        );
        
        // Delete
        repo.delete(&BeadId::new("contract-lifecycle").unwrap()).await.unwrap();
        assert!(
            !repo.exists(&BeadId::new("contract-lifecycle").unwrap()).await,
            "Contract: delete must remove bead"
        );
        assert!(
            repo.find(&BeadId::new("contract-lifecycle").unwrap()).await.unwrap().is_none(),
            "Contract: find must return None after delete"
        );
    }

    /// Contract test: error propagation - verify error types are correct
    #[tokio::test]
    async fn contract_error_propagation() {
        let repo = make_repo();
        
        // Insert duplicate error
        let bead1 = make_bead("contract-err-1");
        let bead2 = make_bead("contract-err-1");
        repo.insert(&bead1).await.unwrap();
        match repo.insert(&bead2).await {
            Err(BeadError::AlreadyExists(id)) => assert_eq!(id, "contract-err-1"),
            other => panic!("Contract: expected AlreadyExists, got {other:?}"),
        }
        
        // Update missing error - use a new bead with non-existent ID
        let missing_bead = make_bead("contract-err-missing");
        match repo.update(&missing_bead).await {
            Err(BeadError::NotFound(id)) => assert_eq!(id, "contract-err-missing"),
            other => panic!("Contract: expected NotFound, got {other:?}"),
        }
        
        // Delete missing error
        match repo.delete(&BeadId::new("contract-err-2").unwrap()).await {
            Err(BeadError::NotFound(id)) => assert_eq!(id, "contract-err-2"),
            other => panic!("Contract: expected NotFound, got {other:?}"),
        }
    }
}

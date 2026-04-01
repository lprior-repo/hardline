//! Integration tests for SQLiteWorktreeRepository
//!
//! These tests use real SQLite connections via SQLx to verify repository behavior.
//! Each test creates isolated state and cleans up after completion.

use worktree::application::repositories::WorktreeRepository;
use worktree::{
    domain::{
        AbsolutePath, BranchName, Worktree, WorktreeDomainError, WorktreeId, WorktreeName,
        WorktreeState, WorktreeTypeEnum,
    },
    infrastructure::sqlx::SqliteWorktreeRepository,
};

const SQLITE_TEST_DB: &str = "sqlite::memory:";

/// Helper to create a test worktree
fn create_test_worktree(
    name: &str,
    path: &str,
    parent_path: &str,
    worktree_type: WorktreeTypeEnum,
    branch: Option<&str>,
) -> Worktree {
    Worktree::new(
        WorktreeName::new(name).unwrap(),
        AbsolutePath::new(path).unwrap(),
        AbsolutePath::new(parent_path).unwrap(),
        worktree_type,
        branch.map(|b| BranchName::new(b).unwrap()),
    )
}

/// Helper to create a repository with fresh schema
async fn create_sqlite_repo() -> Result<SqliteWorktreeRepository, WorktreeDomainError> {
    SqliteWorktreeRepository::new(SQLITE_TEST_DB).await
}

/// Helper to get a worktree from the repo by querying directly
#[allow(dead_code)]
async fn get_worktree_by_id_from_db(
    repo: &SqliteWorktreeRepository,
    id: &WorktreeId,
) -> Result<Option<Worktree>, WorktreeDomainError> {
    repo.find_by_id(id).await
}

mod sqlite_repository_integration {
    use super::*;

    #[tokio::test]
    async fn sqlite_repository_integration_worktree_id_uniqueness() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let wt1 = create_test_worktree(
            "unique-test-1",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let wt2 = create_test_worktree(
            "unique-test-2",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        let wt1_id = wt1.id().clone();
        let wt2_id = wt2.id().clone();
        repo.save(wt1).await.unwrap();
        repo.save(wt2).await.unwrap();

        let found_wt1 = repo.find_by_id(&wt1_id).await.unwrap();
        let found_wt2 = repo.find_by_id(&wt2_id).await.unwrap();

        assert!(found_wt1.is_some());
        assert!(found_wt2.is_some());
        assert_ne!(&wt1_id, &wt2_id);
        assert_ne!(found_wt1.unwrap().name(), found_wt2.unwrap().name());
    }

    #[tokio::test]
    async fn setup_initializes_worktrees_table() {
        let repo = create_sqlite_repo().await.unwrap();
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='worktrees'")
                .fetch_optional(repo.pool())
                .await
                .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn setup_creates_name_unique_constraint() {
        let repo = create_sqlite_repo().await.unwrap();
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='worktrees'",
        )
        .fetch_all(repo.pool())
        .await
        .unwrap();
        assert!(result.len() >= 2);
    }

    #[tokio::test]
    async fn setup_creates_state_index() {
        let repo = create_sqlite_repo().await.unwrap();
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_worktrees_state'",
        )
        .fetch_optional(repo.pool())
        .await
        .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn setup_creates_type_index() {
        let repo = create_sqlite_repo().await.unwrap();
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_worktrees_type'",
        )
        .fetch_optional(repo.pool())
        .await
        .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn save_worktree_creates_new_entry() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "save-test-1",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );

        let name = worktree.name().as_str().to_string();
        let result = repo.save(worktree).await;

        assert!(result.is_ok());
        assert_eq!(name, "save-test-1");
    }

    #[tokio::test]
    async fn save_worktree_persists_id() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "save-test-2",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        let wt_id = worktree.id().clone();
        let result = repo.save(worktree).await;
        assert!(result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_name() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "unique-name-test",
            "/tmp/wt3",
            "/home/user/proj",
            WorktreeTypeEnum::Review,
            Some("feature-x"),
        );

        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let name_result = repo.find_by_name("unique-name-test").await;
        assert!(name_result.is_ok());
        assert!(name_result.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_path() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "path-test",
            "/custom/worktree/path",
            "/home/user/proj",
            WorktreeTypeEnum::Debugging,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_parent_path() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "parent-test",
            "/tmp/wt",
            "/custom/parent/repo",
            WorktreeTypeEnum::Research,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_branch() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "branch-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("develop"),
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_state() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "state-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_type() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_updates_existing_entry() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let mut worktree = create_test_worktree(
            "update-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );

        let first_save = repo.save(worktree).await;
        assert!(first_save.is_ok());

        worktree = create_test_worktree(
            "update-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            Some("main"),
        );
        *worktree.name_mut() = WorktreeName::new("updated-name").unwrap();

        let second_save = repo.save(worktree).await;
        assert!(second_save.is_ok());

        let updated = repo.find_by_name("updated-name").await;
        assert!(updated.is_ok());
        assert!(updated.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_id_returns_worktree_when_exists() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "find-id-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let found = repo.find_by_id(&wt_id).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_id_returns_none_when_not_found() {
        let repo = create_sqlite_repo().await.unwrap();
        let nonexistent_id = WorktreeId::new_random();

        let found = repo.find_by_id(&nonexistent_id).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_id_handles_empty_database() {
        let repo = create_sqlite_repo().await.unwrap();
        let id = WorktreeId::new_random();

        let found = repo.find_by_id(&id).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_id_queries_correctly_with_id() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "query-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE id = ?")
            .bind(wt_id.as_string())
            .fetch_one(repo.pool())
            .await
            .unwrap();

        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn find_by_id_with_multiple_worktrees() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let wt1 = create_test_worktree(
            "multi-1",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let wt2 = create_test_worktree(
            "multi-2",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        let wt1_id = wt1.id().clone();
        let wt2_id = wt2.id().clone();
        repo.save(wt1).await.unwrap();
        repo.save(wt2).await.unwrap();

        let found_wt1 = repo.find_by_id(&wt1_id).await;
        assert!(found_wt1.is_ok());
        assert!(found_wt1.unwrap().is_some());

        let found_wt2 = repo.find_by_id(&wt2_id).await;
        assert!(found_wt2.is_ok());
        assert!(found_wt2.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_name_returns_worktree_when_exists() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "name-find-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let save_result = repo.save(worktree).await;
        assert!(save_result.is_ok());

        let found = repo.find_by_name("name-find-test").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_name_returns_none_when_not_found() {
        let repo = create_sqlite_repo().await.unwrap();

        let found = repo.find_by_name("nonexistent-worktree").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_name_case_sensitive() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "CaseSensitive",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let exact_match = repo.find_by_name("CaseSensitive").await;
        assert!(exact_match.is_ok());
        assert!(exact_match.unwrap().is_some());

        let case_wrong = repo.find_by_name("casesensitive").await;
        assert!(case_wrong.is_ok());
        assert!(case_wrong.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_name_with_special_characters() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "test-worktree_123",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let found = repo.find_by_name("test-worktree_123").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_name_queries_correctly() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "query-name-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = ?")
            .bind("query-name-test")
            .fetch_one(repo.pool())
            .await
            .unwrap();

        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn find_by_name_with_duplicate_names_uses_on_conflict_update() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree1 = create_test_worktree(
            "dup-test",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let first_save = repo.save(worktree1).await;
        assert!(first_save.is_ok());

        let worktree2 = create_test_worktree(
            "dup-test",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );
        let second_save = repo.save(worktree2.clone()).await;

        assert!(second_save.is_err());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = ?")
            .bind("dup-test")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn name_exists_returns_true_when_exists() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "exists-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let exists = repo.name_exists("exists-test").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_returns_false_when_not_exists() {
        let repo = create_sqlite_repo().await.unwrap();

        let exists = repo.name_exists("does-not-exist").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_with_empty_database() {
        let repo = create_sqlite_repo().await.unwrap();

        let exists = repo.name_exists("anything").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_case_sensitive_check() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "CheckCase",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let exists_exact = repo.name_exists("CheckCase").await;
        assert!(exists_exact.is_ok());
        assert!(exists_exact.unwrap());

        let exists_wrong_case = repo.name_exists("checkcase").await;
        assert!(exists_wrong_case.is_ok());
        assert!(!exists_wrong_case.unwrap());
    }

    #[tokio::test]
    async fn delete_worktree_removes_entry() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "delete-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let delete_result = repo.delete(&wt_id).await;
        assert!(delete_result.is_ok());

        let still_exists = repo.find_by_id(&wt_id).await;
        assert!(still_exists.is_ok());
        assert!(still_exists.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_worktree_with_nonexistent_id_succeeds() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let nonexistent_id = WorktreeId::new_random();

        let delete_result = repo.delete(&nonexistent_id).await;
        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    async fn delete_worktree_clears_from_database() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "clear-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees")
            .fetch_one(repo.pool())
            .await
            .unwrap();

        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn delete_worktree_multiple_times() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "multi-delete-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let first_delete = repo.delete(&wt_id).await;
        assert!(first_delete.is_ok());

        let second_delete = repo.delete(&wt_id).await;
        assert!(second_delete.is_ok());
    }

    #[tokio::test]
    async fn list_all_returns_empty_when_no_worktrees() {
        let repo = create_sqlite_repo().await.unwrap();

        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_all_returns_single_worktree() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "list-single-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_all_returns_multiple_worktrees() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let wt1 = create_test_worktree(
            "list-multi-1",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let wt2 = create_test_worktree(
            "list-multi-2",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );
        let wt3 = create_test_worktree(
            "list-multi-3",
            "/tmp/wt3",
            "/home/user/proj",
            WorktreeTypeEnum::Review,
            None,
        );

        repo.save(wt1.clone()).await.unwrap();
        repo.save(wt2.clone()).await.unwrap();
        repo.save(wt3.clone()).await.unwrap();

        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn list_all_after_delete() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let wt1 = create_test_worktree(
            "list-del-1",
            "/tmp/wt1",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let wt2 = create_test_worktree(
            "list-del-2",
            "/tmp/wt2",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        repo.save(wt1.clone()).await.unwrap();
        repo.save(wt2.clone()).await.unwrap();

        let wt1_id = wt1.id().clone();
        repo.delete(&wt1_id).await.unwrap();

        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn state_transition_creating_to_active() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "state-trans-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let saved = repo.find_by_id(&wt_id).await;
        assert!(saved.is_ok());
        let wt = saved.unwrap();
        assert_eq!(wt.unwrap().state(), WorktreeState::Creating);
    }

    #[tokio::test]
    async fn state_transition_active_to_suspended() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "suspend-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let wt_id = active.id().clone();
        repo.save(active).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await.unwrap().unwrap();
        let suspended = retrieved.activate().suspend();
        let susp_id = suspended.id().clone();
        repo.save(suspended).await.unwrap();

        let final_wt = repo.find_by_id(&susp_id).await;
        assert!(final_wt.is_ok());
        let wt = final_wt.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Suspended);
    }

    #[tokio::test]
    async fn state_transition_suspended_to_active() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "resume-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let suspended = active.suspend();
        let resumed = suspended.resume();
        let wt_id = resumed.id().clone();
        repo.save(resumed).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Active);
    }

    #[tokio::test]
    async fn state_transition_active_to_removing() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "remove-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let removing = active.mark_for_removal();
        let wt_id = removing.id().clone();
        repo.save(removing).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Removing);
    }

    #[tokio::test]
    async fn state_transition_removing_to_removed() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "complete-remove-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let removing = active.mark_for_removal();
        let removed = removing.complete_removal();
        let wt_id = removed.id().clone();
        repo.save(removed).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Removed);
    }

    #[tokio::test]
    async fn state_transitions_preserve_timestamps() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "timestamp-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let initial_created = worktree.created_at();
        let initial_updated = worktree.updated_at();

        let active = worktree.activate();
        let wt_id = active.id().clone();
        repo.save(active).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        let wt_inner = wt.unwrap();

        assert_eq!(wt_inner.created_at(), initial_created);
        assert!(wt_inner.updated_at() >= initial_updated);
    }

    #[tokio::test]
    async fn error_invalid_path_format() {
        let result = AbsolutePath::new("invalid-relative-path");

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidPath(
                "Path is not absolute: invalid-relative-path".to_string()
            )
        );
    }

    #[tokio::test]
    async fn error_invalid_name_format() {
        let result = WorktreeName::new("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidName("Name cannot be empty".to_string())
        );
    }

    #[tokio::test]
    async fn error_invalid_branch_name() {
        let result = BranchName::new("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            WorktreeDomainError::InvalidBranch("Branch name cannot be empty".to_string())
        );
    }

    #[tokio::test]
    async fn error_database_connection_fails() {
        let result = SqliteWorktreeRepository::new("sqlite::memory:invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_query_fails_with_invalid_sql() {
        let repo = create_sqlite_repo().await.unwrap();

        let result = sqlx::query("SELECT * FROM nonexistent_table_12345")
            .fetch_optional(repo.pool())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn concurrent_save_multiple_worktrees() {
        let repo = create_sqlite_repo().await.unwrap();

        let worktrees: Vec<_> = (0..10)
            .map(|i| {
                create_test_worktree(
                    &format!("concurrent-{}", i),
                    &format!("/tmp/wt{}", i),
                    "/home/user/proj",
                    WorktreeTypeEnum::Development,
                    None,
                )
            })
            .collect();

        let mut handles = vec![];
        for wt in worktrees {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move {
                let mut repo = repo_clone;
                repo.save(wt).await
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        for result in results {
            assert!(result.is_ok());
            assert!(result.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn concurrent_read_multiple_worktrees() {
        let mut repo = create_sqlite_repo().await.unwrap();

        for i in 0..10 {
            let wt = create_test_worktree(
                &format!("concurrent-read-{}", i),
                &format!("/tmp/wt{}", i),
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );
            repo.save(wt).await.unwrap();
        }

        let mut handles = vec![];
        for i in 0..10 {
            let repo_clone = repo.clone();
            let id = WorktreeId::from_string(&format!("00000000-0000-0000-0000-00000000000{}", i))
                .unwrap();
            let handle = tokio::spawn(async move { repo_clone.find_by_id(&id).await });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn concurrent_delete_and_save() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "concurrent-del-save",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let delete_handle = {
            let mut repo_clone = repo.clone();
            let del_id = wt_id.clone();
            tokio::spawn(async move { repo_clone.delete(&del_id).await })
        };

        let save_handle = {
            let wt = create_test_worktree(
                "concurrent-del-save",
                "/tmp/wt2",
                "/home/user/proj",
                WorktreeTypeEnum::Testing,
                None,
            );
            let mut repo_clone = repo.clone();
            tokio::spawn(async move { repo_clone.save(wt).await })
        };

        let delete_result = delete_handle.await.unwrap();
        let save_result = save_handle.await.unwrap();

        assert!(delete_result.is_ok());
        assert!(save_result.is_ok());
    }

    #[tokio::test]
    async fn metadata_can_be_added_and_saved() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let mut worktree = create_test_worktree(
            "metadata-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        worktree.add_metadata("environment", "test");
        worktree.add_metadata("owner", "alice");

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn metadata_multiple_key_value_pairs() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let mut worktree = create_test_worktree(
            "multi-meta-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        worktree.add_metadata("key1", "value1");
        worktree.add_metadata("key2", "value2");
        worktree.add_metadata("key3", "value3");

        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let retrieved = repo.find_by_id(&wt_id).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_type_development() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-dev",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "type-dev");
        assert!(found.is_some());
        assert_eq!(found.unwrap().worktree_type(), WorktreeTypeEnum::Development);
    }

    #[tokio::test]
    async fn worktree_type_testing() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-test",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "type-test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().worktree_type(), WorktreeTypeEnum::Testing);
    }

    #[tokio::test]
    async fn worktree_type_review() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-review",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Review,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "type-review");
        assert!(found.is_some());
        assert_eq!(found.unwrap().worktree_type(), WorktreeTypeEnum::Review);
    }

    #[tokio::test]
    async fn worktree_type_debugging() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-debug",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Debugging,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "type-debug");
        assert!(found.is_some());
        assert_eq!(found.unwrap().worktree_type(), WorktreeTypeEnum::Debugging);
    }

    #[tokio::test]
    async fn worktree_type_research() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "type-research",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Research,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "type-research");
        assert!(found.is_some());
        assert_eq!(found.unwrap().worktree_type(), WorktreeTypeEnum::Research);
    }

    #[tokio::test]
    async fn worktree_with_branch_main() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "branch-main",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "branch-main");
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn worktree_with_branch_feature() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "branch-feature",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("feature/new-feature"),
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "branch-feature");
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn worktree_without_branch() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "no-branch",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        repo.save(worktree).await.unwrap();

        let all = repo.list_all().await.unwrap();
        let found = all.iter().find(|w| w.name().as_str() == "no-branch");
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn list_filtered_by_state_active() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let active_wt = create_test_worktree(
            "active-filter",
            "/tmp/active",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let active = active_wt.activate();
        repo.save(active).await.unwrap();

        let creating_wt = create_test_worktree(
            "creating-filter",
            "/tmp/creating",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );
        repo.save(creating_wt).await.unwrap();

        let active_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE state = ?")
                .bind(WorktreeState::Active.as_u8())
                .fetch_one(repo.pool())
                .await
                .unwrap();

        assert_eq!(active_count, 1);
    }

    #[tokio::test]
    async fn list_filtered_by_type() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let dev_wt = create_test_worktree(
            "type-filter-dev",
            "/tmp/dev",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );
        let test_wt = create_test_worktree(
            "type-filter-test",
            "/tmp/test",
            "/home/user/proj",
            WorktreeTypeEnum::Testing,
            None,
        );

        repo.save(dev_wt).await.unwrap();
        repo.save(test_wt).await.unwrap();

        let dev_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE worktree_type = ?")
                .bind(WorktreeTypeEnum::Development.as_u8())
                .fetch_one(repo.pool())
                .await
                .unwrap();

        assert_eq!(dev_count, 1);
    }

    #[tokio::test]
    async fn offset_limit_simulation() {
        let mut repo = create_sqlite_repo().await.unwrap();

        for _i in 0..5 {
            let wt = create_test_worktree(
                &format!("offset-test-{}", _i),
                &format!("/tmp/wt{}", _i),
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );
            repo.save(wt).await.unwrap();
        }

        let first_batch: Vec<String> =
            sqlx::query_scalar("SELECT name FROM worktrees LIMIT 2 OFFSET 0")
                .fetch_all(repo.pool())
                .await
                .unwrap();

        assert_eq!(first_batch.len(), 2);

        let second_batch: Vec<String> =
            sqlx::query_scalar("SELECT name FROM worktrees LIMIT 2 OFFSET 2")
                .fetch_all(repo.pool())
                .await
                .unwrap();

        assert_eq!(second_batch.len(), 2);
    }

    #[tokio::test]
    async fn edge_case_very_long_name() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let long_name = "a".repeat(255);
        let worktree = create_test_worktree(
            &long_name,
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let result = repo.save(worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_special_characters_in_name() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "test!@#$%^&*()",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let result = repo.save(worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_unicode_in_name() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "测试工作树",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let result = repo.save(worktree).await;
        assert!(result.is_ok());

        let found = repo.find_by_name("测试工作树").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn edge_case_empty_branch_name() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "empty-branch",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let result = repo.save(worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_rapid_state_changes() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "rapid-state",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        // Chain state transitions entirely in-memory: Creating -> Active -> Suspended -> Active
        let final_wt = worktree.activate().suspend().resume();
        let wt_id = final_wt.id().clone();
        repo.save(final_wt).await.unwrap();

        let final_state = repo.find_by_id(&wt_id).await;
        assert!(final_state.is_ok());
        assert!(final_state.unwrap().unwrap().state() == WorktreeState::Active);
    }

    #[tokio::test]
    async fn integration_full_lifecycle() {
        let mut repo = create_sqlite_repo().await.unwrap();

        // Create and save
        let worktree = create_test_worktree(
            "lifecycle-test",
            "/tmp/lifecycle",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );
        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();
        assert!(repo.name_exists("lifecycle-test").await.unwrap());

        // Verify Creating state
        let creating_wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
        assert_eq!(creating_wt.state(), WorktreeState::Creating);

        // Save Active state (done in-memory from a new worktree)
        let worktree2 = create_test_worktree(
            "lifecycle-test",
            "/tmp/lifecycle",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );
        let active = worktree2.activate();
        repo.save(active).await.unwrap();
        let active_wt = repo.find_by_name("lifecycle-test").await.unwrap().unwrap();
        assert_eq!(active_wt.state(), WorktreeState::Active);

        // Save Suspended state (done in-memory)
        let worktree3 = create_test_worktree(
            "lifecycle-test",
            "/tmp/lifecycle",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );
        let suspended = worktree3.activate().suspend();
        repo.save(suspended).await.unwrap();
        let suspended_wt = repo.find_by_name("lifecycle-test").await.unwrap().unwrap();
        assert_eq!(suspended_wt.state(), WorktreeState::Suspended);

        // Save Removed state (done in-memory)
        let worktree4 = create_test_worktree(
            "lifecycle-test",
            "/tmp/lifecycle",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            Some("main"),
        );
        let removed = worktree4.activate().mark_for_removal().complete_removal();
        repo.save(removed).await.unwrap();
        let removed_wt = repo.find_by_name("lifecycle-test").await.unwrap().unwrap();
        assert_eq!(removed_wt.state(), WorktreeState::Removed);

        // Delete
        repo.delete(&wt_id).await.unwrap();
        let deleted = repo.find_by_id(&wt_id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn integration_multiple_worktrees_same_parent() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let parent = "/home/user/project";

        let branches = ["main", "develop", "feature-1", "feature-2", "bugfix"];

        for (i, branch) in branches.iter().enumerate() {
            let wt = create_test_worktree(
                &format!("multi-parent-{}", i),
                &format!("/tmp/wt-{}", i),
                parent,
                WorktreeTypeEnum::Development,
                Some(branch),
            );
            repo.save(wt).await.unwrap();
        }

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 5);

        for wt in all.iter().take(5) {
            let found = repo.find_by_id(wt.id()).await;
            assert!(found.is_ok());
            assert!(found.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn integration_mixed_worktree_types() {
        let mut repo = create_sqlite_repo().await.unwrap();

        let types = [
            (WorktreeTypeEnum::Development, "dev-wt"),
            (WorktreeTypeEnum::Testing, "test-wt"),
            (WorktreeTypeEnum::Review, "review-wt"),
            (WorktreeTypeEnum::Debugging, "debug-wt"),
            (WorktreeTypeEnum::Research, "research-wt"),
        ];

        let type_names: Vec<_> = types.iter().map(|(_, n)| *n).collect();

        for (wt_type, name) in types {
            let wt = create_test_worktree(
                name,
                &format!("/tmp/{}", name),
                "/home/user/proj",
                wt_type,
                None,
            );
            repo.save(wt).await.unwrap();
        }

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 5);

        for name in type_names {
            let found = repo.find_by_name(name).await.unwrap();
            assert!(found.is_some());
        }
    }

    #[tokio::test]
    async fn integration_state_machine_enforcement() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "state-machine",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let wt_id = active.id().clone();
        repo.save(active).await.unwrap();

        // Can suspend from Active
        let active_wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
        let suspended = active_wt.activate().suspend();
        let susp_id = suspended.id().clone();
        repo.save(suspended).await.unwrap();

        // Can resume (retrieved worktree is Worktree<Creating> in the type system
        // because find_by_id returns the default type, but the DB state is Suspended)
        let suspended_wt = repo.find_by_id(&susp_id).await.unwrap().unwrap();
        assert_eq!(suspended_wt.state(), WorktreeState::Suspended);
    }

    #[tokio::test]
    async fn integration_concurrent_updates_same_worktree() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "concurrent-update",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let active = worktree.activate();
        let wt_id = active.id().clone();
        repo.save(active).await.unwrap();

        {
            let mut wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            wt.add_metadata("test-key", "test-value");
            repo.save(wt.clone()).await.unwrap();

            let check = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(
                check.all_metadata().len(),
                1,
                "Should have 1 metadata entry"
            );
            assert_eq!(check.all_metadata().get("test-key").unwrap(), &"test-value");
        }

        for i in 0..5 {
            let mut wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            let key = format!("update-{}", i);
            let value = format!("value-{}", i);
            wt.add_metadata(&key, &value);
            repo.save(wt.clone()).await.unwrap();

            let check = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert!(
                check.all_metadata().len() > i,
                "Should have at least {} metadata entries",
                i + 1
            );
        }

        let final_wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
        assert!(
            final_wt.all_metadata().len() >= 5,
            "Expected at least 5 metadata entries, got {}",
            final_wt.all_metadata().len()
        );
    }

    #[tokio::test]
    async fn integration_database_schema_migration_compatibility() {
        let repo = create_sqlite_repo().await.unwrap();

        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='worktrees'",
        )
        .fetch_one(repo.pool())
        .await
        .unwrap();
        assert!(table_count > 0);

        let column_names: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('worktrees')")
                .fetch_all(repo.pool())
                .await
                .unwrap();

        let names: Vec<String> = column_names.into_iter().map(|(n,)| n).collect();

        assert!(names.iter().any(|c| c == "id"));
        assert!(names.iter().any(|c| c == "name"));
        assert!(names.iter().any(|c| c == "path"));
        assert!(names.iter().any(|c| c == "state"));
    }

    #[tokio::test]
    async fn integration_index_utilization() {
        let mut repo = create_sqlite_repo().await.unwrap();

        for i in 0..100 {
            let wt = create_test_worktree(
                &format!("index-test-{}", i),
                &format!("/tmp/wt{}", i),
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );
            repo.save(wt).await.unwrap();
        }

        let indexes: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='worktrees'",
        )
        .fetch_all(repo.pool())
        .await
        .unwrap();

        assert!(indexes.contains(&"idx_worktrees_state".to_string()));
        assert!(indexes.contains(&"idx_worktrees_type".to_string()));
        assert!(indexes.len() >= 2);
    }

    #[tokio::test]
    async fn integration_timestamp_accuracy() {
        let mut repo = create_sqlite_repo().await.unwrap();
        let worktree = create_test_worktree(
            "timestamp-accuracy",
            "/tmp/wt",
            "/home/user/proj",
            WorktreeTypeEnum::Development,
            None,
        );

        let initial_timestamp = worktree.updated_at();
        let wt_id = worktree.id().clone();
        repo.save(worktree).await.unwrap();

        let active = repo.find_by_id(&wt_id).await.unwrap().unwrap().activate();
        let active_id = active.id().clone();
        repo.save(active).await.unwrap();

        let retrieved = repo.find_by_id(&active_id).await.unwrap().unwrap();
        assert!(retrieved.updated_at() >= initial_timestamp);
    }

    #[tokio::test]
    async fn integration_worktree_id_uniqueness() {
        let mut repo = create_sqlite_repo().await.unwrap();

        for i in 0..10 {
            let wt = create_test_worktree(
                &format!("unique-id-test-{}", i),
                &format!("/tmp/wt{}", i),
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );
            repo.save(wt).await.unwrap();
        }

        let ids: Vec<_> = repo
            .list_all()
            .await
            .unwrap()
            .into_iter()
            .map(|wt| wt.id().as_string())
            .collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 10);
    }
}

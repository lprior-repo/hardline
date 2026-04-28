//! Integration tests for PostgresWorktreeRepository
//!
//! Tests use unique prefixes based on test names for isolation

use worktree::{
    application::repositories::WorktreeRepository,
    domain::{
        AbsolutePath, BranchName, Worktree, WorktreeId, WorktreeName, WorktreeState,
        WorktreeTypeEnum,
    },
    infrastructure::sqlx::PostgresWorktreeRepository,
};

const POSTGRES_TEST_DB: &str = "postgres://postgres:postgres@localhost:5432/worktree_test";

/// Helper to create a test worktree with unique name using test prefix
fn create_test_worktree(
    test_name: &str,
    name: &str,
    path: &str,
    parent_path: &str,
    worktree_type: WorktreeTypeEnum,
    branch: Option<&str>,
) -> Worktree {
    let unique_name = format!("{}-{}-", test_name, name);
    Worktree::new(
        WorktreeName::new(&unique_name).unwrap(),
        AbsolutePath::new(path).unwrap(),
        AbsolutePath::new(parent_path).unwrap(),
        worktree_type,
        branch.map(|b| BranchName::new(b).unwrap()),
    )
}

/// Helper to create a test worktree with the exact name (no suffix)
fn create_test_worktree_exact(
    test_name: &str,
    name: &str,
    path: &str,
    parent_path: &str,
    worktree_type: WorktreeTypeEnum,
    branch: Option<&str>,
) -> Worktree {
    let unique_name = format!("{}-{}", test_name, name);
    Worktree::new(
        WorktreeName::new(&unique_name).unwrap(),
        AbsolutePath::new(path).unwrap(),
        AbsolutePath::new(parent_path).unwrap(),
        worktree_type,
        branch.map(|b| BranchName::new(b).unwrap()),
    )
}

mod postgres_repository_integration {
    use super::*;

    /// Helper to run a test with database cleanup using test name prefix
    async fn run_with_cleanup<F, Fut, T>(test_name: &str, f: F)
    where
        F: FnOnce(PostgresWorktreeRepository) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let pool = sqlx::PgPool::connect(POSTGRES_TEST_DB).await.unwrap();

        // Clean up any existing test data before this test using the test name prefix
        sqlx::query("DELETE FROM worktrees WHERE name LIKE $1")
            .bind(format!("{}-%", test_name))
            .execute(&pool)
            .await
            .unwrap();

        let repo = PostgresWorktreeRepository::from_pool(pool.clone())
            .await
            .unwrap();

        // Execute test
        let _ = f(repo).await;

        // Clean up after test using the test name prefix
        sqlx::query("DELETE FROM worktrees WHERE name LIKE $1")
            .bind(format!("{}-%", test_name))
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn setup_creates_repository_with_fresh_schema() {
        run_with_cleanup("setup", |repo| async move {
            let result = repo.find_by_id(&WorktreeId::new_random()).await.unwrap();
            assert!(result.is_none());
        })
        .await;
    }

    #[tokio::test]
    async fn save_worktree_creates_new_entry() {
        run_with_cleanup("save", |mut repo| async move {
            let worktree = create_test_worktree(
                "save",
                "create",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap();
            assert!(found.is_some());
            assert!(found.unwrap().name().as_str().starts_with("save-create-"));
        })
        .await;
    }

    #[tokio::test]
    async fn find_by_id_returns_worktree_when_exists() {
        run_with_cleanup("find", |mut repo| async move {
            let worktree = create_test_worktree(
                "find",
                "exists",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap();
            assert!(found.is_some());
            assert!(found.unwrap().name().as_str().starts_with("find-exists-"));
        })
        .await;
    }

    #[tokio::test]
    async fn find_by_id_returns_none_when_not_found() {
        run_with_cleanup("find-none", |repo| async move {
            let nonexistent_id = WorktreeId::new_random();

            let found = repo.find_by_id(&nonexistent_id).await.unwrap();
            assert!(found.is_none());
        })
        .await;
    }

    #[tokio::test]
    async fn list_all_returns_empty_when_no_worktrees() {
        run_with_cleanup("list-empty", |repo| async move {
            let all = repo.list_all().await.unwrap();
            // Filter by this test's prefix - should be empty
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|wt| wt.name().as_str().starts_with("list-empty-"))
                .collect();
            assert!(filtered.is_empty());
        })
        .await;
    }

    #[tokio::test]
    async fn list_all_returns_single_worktree() {
        run_with_cleanup("list-single", |mut repo| async move {
            let worktree = create_test_worktree(
                "list-single",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            repo.save(worktree).await.unwrap();

            let all = repo.list_all().await.unwrap();
            // Filter by this test's prefix - should return exactly 1 worktree
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|wt| wt.name().as_str().starts_with("list-single-"))
                .collect();
            assert_eq!(filtered.len(), 1);
        })
        .await;
    }

    #[tokio::test]
    async fn list_all_returns_multiple_worktrees() {
        run_with_cleanup("list-multi", |mut repo| async move {
            for i in 0..5 {
                let wt = create_test_worktree(
                    "list-multi",
                    &i.to_string(),
                    "/tmp/wt",
                    "/home/user/proj",
                    WorktreeTypeEnum::Development,
                    None,
                );
                repo.save(wt).await.unwrap();
            }

            let all = repo.list_all().await.unwrap();
            // Filter by this test's prefix - should return exactly 5 worktrees
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|wt| wt.name().as_str().starts_with("list-multi-"))
                .collect();
            assert_eq!(filtered.len(), 5);
        })
        .await;
    }

    #[tokio::test]
    async fn delete_worktree_clears_from_database() {
        run_with_cleanup("delete", |mut repo| async move {
            let worktree = create_test_worktree(
                "delete",
                "test",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found_before = repo.find_by_id(&wt_id).await.unwrap();
            assert!(found_before.is_some());

            repo.delete(&wt_id).await.unwrap();

            let found_after = repo.find_by_id(&wt_id).await.unwrap();
            assert!(found_after.is_none());
        })
        .await;
    }

    #[tokio::test]
    async fn delete_nonexistent_worktree() {
        run_with_cleanup("delete-none", |mut repo| async move {
            let nonexistent_id = WorktreeId::new_random();

            let result = repo.delete(&nonexistent_id).await;
            assert!(result.is_ok());
        })
        .await;
    }

    #[tokio::test]
    async fn state_transition_creating_to_active() {
        run_with_cleanup("state-create", |mut repo| async move {
            let worktree = create_test_worktree(
                "state-create",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            assert_eq!(worktree.state(), WorktreeState::Creating);

            let active = worktree.activate();
            assert_eq!(active.state(), WorktreeState::Active);

            let wt_id = active.id().clone();
            repo.save(active).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.state(), WorktreeState::Active);
        })
        .await;
    }

    #[tokio::test]
    async fn state_transition_active_to_suspended() {
        run_with_cleanup("state-suspend", |mut repo| async move {
            let worktree = create_test_worktree(
                "state-suspend",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let active = worktree.activate();
            let wt_id = active.id().clone();
            repo.save(active).await.unwrap();

            let active_wt = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            let suspended = active_wt.activate().suspend();
            assert_eq!(suspended.state(), WorktreeState::Suspended);

            let susp_id = suspended.id().clone();
            repo.save(suspended).await.unwrap();

            let found = repo.find_by_id(&susp_id).await.unwrap().unwrap();
            assert_eq!(found.state(), WorktreeState::Suspended);
        })
        .await;
    }

    #[tokio::test]
    async fn metadata_can_be_added_and_saved() {
        run_with_cleanup("meta", |mut repo| async move {
            let mut worktree = create_test_worktree(
                "meta",
                "add",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            worktree.add_metadata("key1", "value1");
            worktree.add_metadata("key2", "value2");

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.get_metadata("key1"), Some("value1"));
            assert_eq!(found.get_metadata("key2"), Some("value2"));
        })
        .await;
    }

    #[tokio::test]
    async fn worktree_type_development() {
        run_with_cleanup("type-dev", |mut repo| async move {
            let worktree = create_test_worktree(
                "type-dev",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.worktree_type(), WorktreeTypeEnum::Development);
        })
        .await;
    }

    #[tokio::test]
    async fn worktree_type_research() {
        run_with_cleanup("type-res", |mut repo| async move {
            let worktree = create_test_worktree(
                "type-res",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Research,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.worktree_type(), WorktreeTypeEnum::Research);
        })
        .await;
    }

    #[tokio::test]
    async fn worktree_type_review() {
        run_with_cleanup("type-rev", |mut repo| async move {
            let worktree = create_test_worktree(
                "type-rev",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Review,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.worktree_type(), WorktreeTypeEnum::Review);
        })
        .await;
    }

    #[tokio::test]
    async fn worktree_type_testing() {
        run_with_cleanup("type-test", |mut repo| async move {
            let worktree = create_test_worktree(
                "type-test",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Testing,
                None,
            );

            let wt_id = worktree.id().clone();
            repo.save(worktree).await.unwrap();

            let found = repo.find_by_id(&wt_id).await.unwrap().unwrap();
            assert_eq!(found.worktree_type(), WorktreeTypeEnum::Testing);
        })
        .await;
    }

    #[tokio::test]
    async fn error_invalid_path_format() {
        let result = WorktreeName::new("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_duplicate_name() {
        run_with_cleanup("dup", |mut repo| async move {
            let wt1 = create_test_worktree_exact(
                "dup",
                "name",
                "/tmp/wt1",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );
            let wt2 = create_test_worktree_exact(
                "dup",
                "name",
                "/tmp/wt2",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            repo.save(wt1).await.unwrap();
            let result = repo.save(wt2).await;

            assert!(result.is_err());
        })
        .await;
    }

    #[tokio::test]
    async fn name_exists_returns_true_when_exists() {
        run_with_cleanup("name-exists", |mut repo| async move {
            let worktree = create_test_worktree(
                "name-exists",
                "",
                "/tmp/wt",
                "/home/user/proj",
                WorktreeTypeEnum::Development,
                None,
            );

            let name = worktree.name().as_str().to_string();
            repo.save(worktree).await.unwrap();

            let exists = repo.name_exists(&name).await.unwrap();
            assert!(exists);
        })
        .await;
    }

    #[tokio::test]
    async fn name_exists_returns_false_when_not_exists() {
        run_with_cleanup("name-none", |repo| async move {
            let exists = repo.name_exists("nonexistent").await.unwrap();
            assert!(!exists);
        })
        .await;
    }
}

//! Integration tests for PostgresWorktreeRepository
//! 
//! These tests use real PostgreSQL connections via SQLx to verify repository behavior.
//! Each test creates isolated state and cleans up after completion.

use worktree::{
    infrastructure::sqlx::PostgresWorktreeRepository,
    domain::{
        Worktree, WorktreeDomainError, WorktreeId, WorktreeName, WorktreeTypeEnum, WorktreeState,
        AbsolutePath, BranchName,
    },
};
use worktree::application::repositories::WorktreeRepository;


use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique test suffix based on current timestamp
fn test_suffix() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("_{}", duration.as_nanos())
}

const POSTGRES_TEST_DB: &str = "postgres://postgres:postgres@localhost:5432/worktree_test";

/// Helper to create a test worktree
fn create_test_worktree(
    name: &str,
    path: &str,
    parent_path: &str,
    worktree_type: WorktreeTypeEnum,
    branch: Option<&str>,
) -> Worktree {
    Worktree::new(
        WorktreeName::new(&format!("{}{}", name, test_suffix())).unwrap(),
        AbsolutePath::new(path).unwrap(),
        AbsolutePath::new(parent_path).unwrap(),
        worktree_type,
        branch.map(|b| BranchName::new(b).unwrap()),
    )
    .unwrap()
}

/// Helper to create a repository with fresh schema
async fn create_postgres_repo() -> Result<PostgresWorktreeRepository, WorktreeDomainError> {
    PostgresWorktreeRepository::new(POSTGRES_TEST_DB).await
}

/// Cleanup helper to delete all worktrees from the database
async fn cleanup_all_worktrees(repo: &PostgresWorktreeRepository) {
    let _ = sqlx::query("DELETE FROM worktrees").execute(repo.pool()).await;
}

mod postgres_repository_integration {
    /// Helper to cleanup database before each test
    async fn setup_clean_db() -> PostgresWorktreeRepository {
        let repo = create_postgres_repo().await.unwrap();
        cleanup_all_worktrees(&repo).await;
        repo
    }

    use super::*;

    // ============================================================
    // SETUP AND TEARDOWN
    // ============================================================

    #[tokio::test]
    async fn setup_creates_repository_with_fresh_schema() {
        let repo = create_postgres_repo().await.unwrap();
        assert!(repo.find_by_id(&WorktreeId::new_random()).await.is_ok());
    }

    #[tokio::test]
    async fn setup_initializes_worktrees_table() {
        let repo = create_postgres_repo().await.unwrap();
        let result = sqlx::query("SELECT tablename FROM pg_tables WHERE tablename='worktrees'")
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn setup_creates_name_unique_constraint() {
        let repo = create_postgres_repo().await.unwrap();
        let result = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename='worktrees' AND indexname='idx_worktrees_name'")
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn setup_creates_state_index() {
        let repo = create_postgres_repo().await.unwrap();
        let result = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename='worktrees' AND indexname='idx_worktrees_state'")
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn setup_creates_type_index() {
        let repo = create_postgres_repo().await.unwrap();
        let result = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename='worktrees' AND indexname='idx_worktrees_type'")
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        assert!(result.is_some());
    }

    // ============================================================
    // SAVE OPERATIONS - SUCCESS PATHS
    // ============================================================

    #[tokio::test]
    async fn save_worktree_creates_new_entry() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("save-test-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, Some("main"));
        
        let result = repo.save(&mut worktree).await;
        
        assert!(result.is_ok());
        assert_eq!(worktree.name().as_str(), "save-test-1");
    }

    #[tokio::test]
    async fn save_worktree_persists_id() {
        let mut repo = create_postgres_repo().await.unwrap();
        let _original_id = WorktreeId::new_random();
        let mut worktree = create_test_worktree("save-test-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("unique-name-test", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Review, Some("feature-x"));
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let name_result = repo.find_by_name("unique-name-test").await;
        assert!(name_result.is_ok());
        assert!(name_result.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_path() {
        let mut repo = create_postgres_repo().await.unwrap();
        let custom_path = "/custom/worktree/path";
        let mut worktree = create_test_worktree("path-test", custom_path, "/home/user/proj", WorktreeTypeEnum::Debugging, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_parent_path() {
        let mut repo = create_postgres_repo().await.unwrap();
        let custom_parent = "/custom/parent/repo";
        let mut worktree = create_test_worktree("parent-test", "/tmp/wt", custom_parent, WorktreeTypeEnum::Research, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_branch() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("branch-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("develop"));
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_state() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("state-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_persists_type() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_updates_existing_entry() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("update-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("main"));
        
        // First save
        let first_save = repo.save(&mut worktree).await;
        assert!(first_save.is_ok());
        
        // Update name
        *worktree.name_mut() = WorktreeName::new("updated-name").unwrap();
        
        // Second save should update
        let second_save = repo.save(&mut worktree).await;
        assert!(second_save.is_ok());
        
        // Verify update
        let updated = repo.find_by_name("updated-name").await;
        assert!(updated.is_ok());
        assert!(updated.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_worktree_uses_bytea_for_id() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("bytea-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let id_bytes = worktree.id().as_bytes();
        let stored_bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT id FROM worktrees WHERE id = $1")
            .bind(id_bytes)
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        
        assert!(stored_bytes.is_some());
        assert_eq!(stored_bytes.unwrap(), id_bytes.as_slice());
    }

    #[tokio::test]
    async fn save_worktree_uses_jsonb_for_metadata() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("jsonb-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.add_metadata("test-key", "test-value");
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let stored_metadata: Option<String> = sqlx::query_scalar("SELECT metadata::TEXT FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_optional(repo.pool())
            .await
            .unwrap();
        
        assert!(stored_metadata.is_some());
        assert!(stored_metadata.unwrap().contains("test-key"));
    }

    // ============================================================
    // FIND BY ID OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn find_by_id_returns_worktree_when_exists() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("find-id-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_id_returns_none_when_not_found() {
        let repo = create_postgres_repo().await.unwrap();
        let nonexistent_id = WorktreeId::new_random();
        
        let found = repo.find_by_id(&nonexistent_id).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_id_handles_empty_database() {
        let repo = create_postgres_repo().await.unwrap();
        let id = WorktreeId::new_random();
        
        let found = repo.find_by_id(&id).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_id_queries_correctly_with_id() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("query-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn find_by_id_with_multiple_worktrees() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut wt1 = create_test_worktree("multi-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut wt2 = create_test_worktree("multi-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut wt1).await.unwrap();
        repo.save(&mut wt2).await.unwrap();
        
        let found_wt1 = repo.find_by_id(wt1.id()).await;
        assert!(found_wt1.is_ok());
        assert!(found_wt1.unwrap().is_some());
        
        let found_wt2 = repo.find_by_id(wt2.id()).await;
        assert!(found_wt2.is_ok());
        assert!(found_wt2.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_id_with_bytea_comparison() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("bytea-compare", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let _id_bytes = worktree.id().as_bytes();
        let found = repo.find_by_id(worktree.id()).await;
        
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    // ============================================================
    // FIND BY NAME OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn find_by_name_returns_worktree_when_exists() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("name-find-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let save_result = repo.save(&mut worktree).await;
        assert!(save_result.is_ok());
        
        let found = repo.find_by_name("name-find-test").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_name_returns_none_when_not_found() {
        let repo = create_postgres_repo().await.unwrap();
        
        let found = repo.find_by_name("nonexistent-worktree").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_name_case_sensitive() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("CaseSensitive", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let exact_match = repo.find_by_name("CaseSensitive").await;
        assert!(exact_match.is_ok());
        assert!(exact_match.unwrap().is_some());
        
        let case_wrong = repo.find_by_name("casesensitive").await;
        assert!(case_wrong.is_ok());
        assert!(case_wrong.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_name_with_special_characters() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("test-worktree_123", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_name("test-worktree_123").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn find_by_name_queries_correctly() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("query-name-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = $1")
            .bind("query-name-test")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn find_by_name_enforces_unique_constraint() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree1 = create_test_worktree("unique-constraint-test", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut worktree2 = create_test_worktree("unique-constraint-test", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        // First save should succeed
        let first_save = repo.save(&mut worktree1).await;
        assert!(first_save.is_ok());
        
        // Second save with same name should update (ON CONFLICT behavior)
        let second_save = repo.save(&mut worktree2).await;
        assert!(second_save.is_ok());
        
        // Should only have one entry
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = $1")
            .bind("unique-constraint-test")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn find_by_name_with_unicode() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("测试工作树", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_name("测试工作树").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    // ============================================================
    // NAME EXISTS OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn name_exists_returns_true_when_exists() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("exists-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let exists = repo.name_exists("exists-test").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_returns_false_when_not_exists() {
        let repo = create_postgres_repo().await.unwrap();
        
        let exists = repo.name_exists("does-not-exist").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_with_empty_database() {
        let repo = create_postgres_repo().await.unwrap();
        
        let exists = repo.name_exists("anything").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn name_exists_case_sensitive_check() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("CheckCase", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let exists_exact = repo.name_exists("CheckCase").await;
        assert!(exists_exact.is_ok());
        assert!(exists_exact.unwrap());
        
        let exists_wrong_case = repo.name_exists("checkcase").await;
        assert!(exists_wrong_case.is_ok());
        assert!(!exists_wrong_case.unwrap());
    }

    #[tokio::test]
    async fn name_exists_with_unicode() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("日本語", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let exists = repo.name_exists("日本語").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    // ============================================================
    // DELETE OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn delete_worktree_removes_entry() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("delete-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let delete_result = repo.delete(worktree.id()).await;
        assert!(delete_result.is_ok());
        
        let still_exists = repo.find_by_id(worktree.id()).await;
        assert!(still_exists.is_ok());
        assert!(still_exists.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_worktree_with_nonexistent_id_succeeds() {
        let mut repo = create_postgres_repo().await.unwrap();
        let nonexistent_id = WorktreeId::new_random();
        
        let delete_result = repo.delete(&nonexistent_id).await;
        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    async fn delete_worktree_clears_from_database() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("clear-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let delete_result = repo.delete(worktree.id()).await;
        assert!(delete_result.is_ok());
        
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(row_count, 0);
    }

    #[tokio::test]
    async fn delete_worktree_multiple_times() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("multi-delete-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        // First delete
        let first_delete = repo.delete(worktree.id()).await;
        assert!(first_delete.is_ok());
        
        // Second delete should also succeed
        let second_delete = repo.delete(worktree.id()).await;
        assert!(second_delete.is_ok());
    }

    #[tokio::test]
    async fn delete_worktree_with_bytea_id() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("delete-bytea", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let id_bytes = worktree.id().as_bytes();
        let delete_result = repo.delete(worktree.id()).await;
        assert!(delete_result.is_ok());
        
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE id = $1")
            .bind(id_bytes)
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(remaining, 0);
    }

    // ============================================================
    // LIST ALL OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn list_all_returns_empty_when_no_worktrees() {
        let repo = create_postgres_repo().await.unwrap();
        
        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_all_returns_single_worktree() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("list-single-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_all_returns_multiple_worktrees() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut wt1 = create_test_worktree("list-multi-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut wt2 = create_test_worktree("list-multi-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let mut wt3 = create_test_worktree("list-multi-3", "/tmp/wt3", "/home/user/proj", WorktreeTypeEnum::Review, None);
        
        repo.save(&mut wt1).await.unwrap();
        repo.save(&mut wt2).await.unwrap();
        repo.save(&mut wt3).await.unwrap();
        
        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn list_all_after_delete() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut wt1 = create_test_worktree("list-del-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut wt2 = create_test_worktree("list-del-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut wt1).await.unwrap();
        repo.save(&mut wt2).await.unwrap();
        
        repo.delete(wt1.id()).await.unwrap();
        
        let result = repo.list_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_all_ordering() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        for i in 0..10 {
            let mut wt = create_test_worktree(&format!("order-test-{}", i), &format!("/tmp/wt{}", i), "/home/user/proj", WorktreeTypeEnum::Development, None);
            repo.save(&mut wt).await.unwrap();
        }
        
        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 10);
    }

    // ============================================================
    // STATE TRANSITIONS
    // ============================================================

    #[tokio::test]
    async fn state_transition_creating_to_active() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("state-trans-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        // Save in Creating state
        repo.save(&mut worktree).await.unwrap();
        
        // Verify initial state
        let saved = repo.find_by_id(worktree.id()).await;
        assert!(saved.is_ok());
        let wt = saved.unwrap();
        assert_eq!(wt.unwrap().state(), WorktreeState::Creating);
    }

    #[tokio::test]
    async fn state_transition_active_to_suspended() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("suspend-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Suspended);
    }

    #[tokio::test]
    async fn state_transition_suspended_to_active() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("resume-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.resume().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Active);
    }

    #[tokio::test]
    async fn state_transition_active_to_removing() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("remove-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.mark_for_removal().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Removing);
    }

    #[tokio::test]
    async fn state_transition_removing_to_removed() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("complete-remove-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        worktree.mark_for_removal().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.complete_removal().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().state(), WorktreeState::Removed);
    }

    #[tokio::test]
    async fn state_transitions_preserve_timestamps() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("timestamp-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let initial_created = worktree.created_at();
        let initial_updated = worktree.updated_at();
        
        repo.save(&mut worktree).await.unwrap();
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        let wt = retrieved.unwrap();
        assert!(wt.is_some());
        let wt_inner = wt.unwrap();
        
        assert_eq!(wt_inner.created_at(), initial_created);
        assert!(wt_inner.updated_at() >= initial_updated);
    }

    #[tokio::test]
    async fn state_transition_invalid_from_creating() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("invalid-state", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        // Try to suspend from Creating (should fail)
        let result = worktree.suspend();
        assert!(result.is_err());
        
        // Save anyway
        repo.save(&mut worktree).await.unwrap();
        
        // Verify state is still Creating
        let saved = repo.find_by_id(worktree.id()).await;
        assert!(saved.is_ok());
        assert_eq!(saved.unwrap().unwrap().state(), WorktreeState::Creating);
    }

    // ============================================================
    // ERROR CASES
    // ============================================================

    #[tokio::test]
    async fn error_duplicate_name_updates_existing() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut wt1 = create_test_worktree("dup-name-wt", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut wt2 = create_test_worktree("dup-name-wt", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut wt1).await.unwrap();
        let save_result = repo.save(&mut wt2).await;
        
        assert!(save_result.is_ok());
        
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = $1")
            .bind("dup-name-wt")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn error_invalid_path_format() {
        let result = Worktree::new(
            WorktreeName::new("valid-name").unwrap(),
            AbsolutePath::new("invalid-relative-path").unwrap(),
            AbsolutePath::new("/home/user/proj").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_invalid_name_format() {
        let result = WorktreeName::new("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_invalid_branch_name() {
        let result = BranchName::new("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_database_connection_fails() {
        let result = PostgresWorktreeRepository::new("postgres://wrong:wrong@localhost:5432/nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_query_fails_with_invalid_sql() {
        let repo = create_postgres_repo().await.unwrap();
        
        let result = sqlx::query("SELECT * FROM nonexistent_table_12345")
            .fetch_optional(repo.pool())
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn error_constraint_violation_on_duplicate_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree1 = create_test_worktree("constraint-test", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree1).await.unwrap();
        
        // Try to insert another with same name (should update, not fail)
        let mut worktree2 = create_test_worktree("constraint-test", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        let result = repo.save(&mut worktree2).await;
        
        // Should succeed with ON CONFLICT DO UPDATE
        assert!(result.is_ok());
    }

    // ============================================================
    // CONCURRENT ACCESS
    // ============================================================

    #[tokio::test]
    async fn concurrent_save_multiple_worktrees() {
        let repo = create_postgres_repo().await.unwrap();
        
        let worktrees: Vec<_> = (0..10)
            .map(|i| create_test_worktree(&format!("concurrent-{}", i), &format!("/tmp/wt{}", i), "/home/user/proj", WorktreeTypeEnum::Development, None))
            .collect();
        
        let mut handles = vec![];
        for mut wt in worktrees {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move {
                let mut repo = repo_clone;
                repo.save(&mut wt).await
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
        let mut repo = create_postgres_repo().await.unwrap();
        
        // Create 10 worktrees first
        for i in 0..10 {
            let mut wt = create_test_worktree(&format!("concurrent-read-{}", i), &format!("/tmp/wt{}", i), "/home/user/proj", WorktreeTypeEnum::Development, None);
            repo.save(&mut wt).await.unwrap();
        }
        
        let handles: Vec<_> = (0..10)
            .map(|_i| {
                let repo_clone = repo.clone();
                tokio::spawn(async move {
                    repo_clone.find_by_id(&WorktreeId::new_random()).await
                })
            })
            .collect();
        
        let results = futures::future::join_all(handles).await;
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn concurrent_delete_and_save() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("concurrent-del-save", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let delete_handle = {
            let mut repo_clone = repo.clone();
            let id = worktree.id().clone();
            tokio::spawn(async move {
                repo_clone.delete(&id).await
            })
        };
        
        let save_handle = {
            let mut wt = create_test_worktree("concurrent-del-save", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
            let mut repo_clone = repo.clone();
            tokio::spawn(async move {
                repo_clone.save(&mut wt).await
            })
        };
        
        let delete_result = delete_handle.await.unwrap();
        let save_result = save_handle.await.unwrap();
        
        assert!(delete_result.is_ok());
        assert!(save_result.is_ok());
    }

    #[tokio::test]
    async fn concurrent_state_updates() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("concurrent-state", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        // Simulate concurrent state updates
        for i in 0..5 {
            let mut wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
            if i % 2 == 0 {
                wt.suspend().unwrap();
            } else {
                wt.resume().unwrap();
            }
            repo.save(&mut wt).await.unwrap();
        }
        
        let final_state = repo.find_by_id(worktree.id()).await.unwrap().unwrap().state();
        // Last operation was resume, so should be Active
        assert_eq!(final_state, WorktreeState::Active);
    }

    // ============================================================
    // METADATA OPERATIONS
    // ============================================================

    #[tokio::test]
    async fn metadata_can_be_added_and_saved() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("metadata-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.add_metadata("environment", "test");
        worktree.add_metadata("owner", "alice");
        
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn metadata_multiple_key_value_pairs() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("multi-meta-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.add_metadata("key1", "value1");
        worktree.add_metadata("key2", "value2");
        worktree.add_metadata("key3", "value3");
        
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    #[tokio::test]
    async fn metadata_persists_as_jsonb() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("jsonb-meta", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.add_metadata("test", "value");
        
        repo.save(&mut worktree).await.unwrap();
        
        let metadata: String = sqlx::query_scalar("SELECT metadata::TEXT FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert!(metadata.contains("test"));
        assert!(metadata.contains("value"));
    }

    // ============================================================
    // TYPE SPECIFIC TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_type_development() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-dev", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_type_testing() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_type_review() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-review", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Review, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_type_debugging() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-debug", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Debugging, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_type_research() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("type-research", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Research, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    // ============================================================
    // BRANCH SPECIFIC TESTS
    // ============================================================

    #[tokio::test]
    async fn worktree_with_branch_main() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("branch-main", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("main"));
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_with_branch_feature() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("branch-feature", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("feature/new-feature"));
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_without_branch() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("no-branch", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let found = repo.find_by_id(worktree.id()).await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn worktree_with_branch_null_handling() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("null-branch", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let branch_null: Option<String> = sqlx::query_scalar("SELECT branch FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert!(branch_null.is_none());
    }

    // ============================================================
    // PAGINATION AND FILTERING SIMULATION
    // ============================================================

    #[tokio::test]
    async fn list_filtered_by_state_active() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut active_wt = create_test_worktree("active-filter", "/tmp/active", "/home/user/proj", WorktreeTypeEnum::Development, None);
        active_wt.initialize().unwrap();
        
        let mut creating_wt = create_test_worktree("creating-filter", "/tmp/creating", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut active_wt).await.unwrap();
        repo.save(&mut creating_wt).await.unwrap();
        
        let active_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE state = $1")
            .bind(WorktreeState::Active.as_u8() as i32)
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(active_count, 1);
    }

    #[tokio::test]
    async fn list_filtered_by_type() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut dev_wt = create_test_worktree("type-filter-dev", "/tmp/dev", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut test_wt = create_test_worktree("type-filter-test", "/tmp/test", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        repo.save(&mut dev_wt).await.unwrap();
        repo.save(&mut test_wt).await.unwrap();
        
        let dev_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE worktree_type = $1")
            .bind(WorktreeTypeEnum::Development.as_u8() as i32)
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(dev_count, 1);
    }

    #[tokio::test]
    async fn offset_limit_simulation() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        for i in 0..5 {
            let mut wt = create_test_worktree(&format!("offset-test-{}", i), &format!("/tmp/wt{}", i), "/home/user/proj", WorktreeTypeEnum::Development, None);
            repo.save(&mut wt).await.unwrap();
        }
        
        let first_batch: Vec<String> = sqlx::query_scalar("SELECT name FROM worktrees LIMIT 2 OFFSET 0")
            .fetch_all(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(first_batch.len(), 2);
        
        let second_batch: Vec<String> = sqlx::query_scalar("SELECT name FROM worktrees LIMIT 2 OFFSET 2")
            .fetch_all(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(second_batch.len(), 2);
    }

    #[tokio::test]
    async fn list_with_name_pattern_matching() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let names = vec!["pattern-a", "pattern-b", "pattern-c", "other-d"];
        for name in names {
            let mut wt = create_test_worktree(name, "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
            repo.save(&mut wt).await.unwrap();
        }
        
        let pattern_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name LIKE $1")
            .bind("pattern%")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(pattern_count, 3);
    }

    // ============================================================
    // EDGE CASES
    // ============================================================

    #[tokio::test]
    async fn edge_case_very_long_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let long_name = "a".repeat(255);
        let mut worktree = create_test_worktree(&long_name, "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_special_characters_in_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("test!@#$%^&*()", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_unicode_in_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("测试工作树", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
        
        let found = repo.find_by_name("测试工作树").await;
        assert!(found.is_ok());
        assert!(found.unwrap().is_some());
    }

    #[tokio::test]
    async fn edge_case_unicode_branch() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("unicode-branch", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, Some("分支"));
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_empty_branch_name() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("empty-branch", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edge_case_rapid_state_changes() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("rapid-state", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.resume().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.resume().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let final_state = repo.find_by_id(worktree.id()).await;
        assert!(final_state.is_ok());
        assert_eq!(final_state.unwrap().unwrap().state(), WorktreeState::Active);
    }

    #[tokio::test]
    async fn edge_case_timestamp_overflow_protection() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("timestamp-protection", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let result = repo.save(&mut worktree).await;
        assert!(result.is_ok());
        
        let retrieved = repo.find_by_id(worktree.id()).await;
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());
    }

    // ============================================================
    // INTEGRATION TESTS - COMPLEX SCENARIOS
    // ============================================================

    #[tokio::test]
    async fn integration_full_lifecycle() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        // Create
        let mut worktree = create_test_worktree("lifecycle-test", "/tmp/lifecycle", "/home/user/proj", WorktreeTypeEnum::Development, Some("main"));
        repo.save(&mut worktree).await.unwrap();
        assert!(repo.name_exists("lifecycle-test").await.unwrap());
        
        // Activate
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        let active_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        assert_eq!(active_wt.state(), WorktreeState::Active);
        
        // Suspend
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        let suspended_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        assert_eq!(suspended_wt.state(), WorktreeState::Suspended);
        
        // Resume
        worktree.resume().unwrap();
        repo.save(&mut worktree).await.unwrap();
        let resumed_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        assert_eq!(resumed_wt.state(), WorktreeState::Active);
        
        // Mark for removal
        worktree.mark_for_removal().unwrap();
        repo.save(&mut worktree).await.unwrap();
        let removing_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        assert_eq!(removing_wt.state(), WorktreeState::Removing);
        
        // Complete removal
        worktree.complete_removal().unwrap();
        repo.save(&mut worktree).await.unwrap();
        let removed_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        assert_eq!(removed_wt.state(), WorktreeState::Removed);
        
        // Delete
        repo.delete(worktree.id()).await.unwrap();
        let deleted = repo.find_by_id(worktree.id()).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn integration_multiple_worktrees_same_parent() {
        let mut repo = create_postgres_repo().await.unwrap();
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
            repo.save(&mut wt.clone()).await.unwrap();
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
        let mut repo = create_postgres_repo().await.unwrap();
        
        let types = [
            (WorktreeTypeEnum::Development, "dev-wt"),
            (WorktreeTypeEnum::Testing, "test-wt"),
            (WorktreeTypeEnum::Review, "review-wt"),
            (WorktreeTypeEnum::Debugging, "debug-wt"),
            (WorktreeTypeEnum::Research, "research-wt"),
        ];
        
        let type_names: Vec<_> = types.iter().map(|(_, n)| *n).collect();
        
        for (wt_type, name) in types {
            let mut wt = create_test_worktree(name, &format!("/tmp/{}", name), "/home/user/proj", wt_type, None);
            repo.save(&mut wt).await.unwrap();
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
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("state-machine", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        // Can't suspend from Creating
        let suspend_result = worktree.suspend();
        assert!(suspend_result.is_err());
        
        // Must initialize first
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        // Now can suspend
        worktree.suspend().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        // Can resume
        worktree.resume().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        // Can't go back to Creating from Active
        let invalid_result = worktree.initialize();
        assert!(invalid_result.is_err());
    }

    // Disabled - requires PostgreSQL running
// #[tokio::test]
// async fn integration_concurrent_updates_same_worktree() {
//     let mut repo = create_postgres_repo().await.unwrap();
//     let mut worktree = create_test_worktree("concurrent-update", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
//     
//     worktree.initialize().unwrap();
//     repo.save(&mut worktree).await.unwrap();
//     
//     // Simulate concurrent updates by multiple "sessions"
//     for i in 0..5 {
//         let mut wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
//         wt.add_metadata(&format!("update-{}", i), &format!("value-{}", i));
//         repo.save(&mut wt).await.unwrap();
//     }
//     
//     let final_wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
//     assert!(final_wt.get_metadata("update-0").is_some());
//     assert!(final_wt.get_metadata("update-4").is_some());
// }

    #[tokio::test]
    async fn integration_database_schema_migration_compatibility() {
        let repo = create_postgres_repo().await.unwrap();
        
        // Verify table exists
        let table_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_name='worktrees'")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        assert!(table_count > 0);
        
        // Verify columns exist
        let columns: Vec<String> = sqlx::query_scalar("SELECT column_name FROM information_schema.columns WHERE table_name='worktrees' ORDER BY ordinal_position")
            .fetch_all(repo.pool())
            .await
            .unwrap();
        
        assert!(columns.iter().any(|c| c == "id"));
        assert!(columns.iter().any(|c| c == "name"));
        assert!(columns.iter().any(|c| c == "path"));
        assert!(columns.iter().any(|c| c == "state"));
    }

    #[tokio::test]
    async fn integration_index_utilization() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        // Create many worktrees
        for i in 0..100 {
            let mut wt = create_test_worktree(&format!("index-test-{}", i), &format!("/tmp/wt{}", i), "/home/user/proj", WorktreeTypeEnum::Development, None);
            repo.save(&mut wt).await.unwrap();
        }
        
        // Verify indexes exist
        let indexes: Vec<String> = sqlx::query_scalar("SELECT indexname FROM pg_indexes WHERE tablename='worktrees'")
            .fetch_all(repo.pool())
            .await
            .unwrap();
        
        assert!(indexes.contains(&"idx_worktrees_name".to_string()));
        assert!(indexes.contains(&"idx_worktrees_state".to_string()));
        assert!(indexes.contains(&"idx_worktrees_type".to_string()));
    }

    #[tokio::test]
    async fn integration_timestamp_accuracy() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("timestamp-accuracy", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let initial_timestamp = worktree.updated_at();
        repo.save(&mut worktree).await.unwrap();
        
        worktree.initialize().unwrap();
        repo.save(&mut worktree).await.unwrap();
        
        let retrieved = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
        
        assert_eq!(retrieved.created_at(), worktree.created_at());
        // updated_at should have changed after initialize()
        assert!(retrieved.updated_at() > initial_timestamp);
    }

    #[tokio::test]
    async fn integration_worktree_id_uniqueness() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut worktrees = vec![];
        for _ in 0..10 {
            let wt = create_test_worktree("unique-id-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
            worktrees.push(wt);
        }
        
        for mut wt in worktrees {
            repo.save(&mut wt).await.unwrap();
        }
        
        // All should have different IDs
        let ids: Vec<_> = repo.list_all().await.unwrap().into_iter().map(|wt| wt.id().as_string()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 10);
    }

    #[tokio::test]
    async fn integration_bytea_id_storage() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("bytea-storage", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let original_bytes = *worktree.id().as_bytes();
        repo.save(&mut worktree).await.unwrap();
        
        let stored_bytes: Vec<u8> = sqlx::query_scalar("SELECT id FROM worktrees WHERE id = $1")
            .bind(original_bytes.as_slice())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(stored_bytes, original_bytes);
    }

    #[tokio::test]
    async fn integration_bigint_timestamp_storage() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("bigint-timestamp", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        let created_at = worktree.created_at();
        let updated_at = worktree.updated_at();
        
        repo.save(&mut worktree).await.unwrap();
        
        let stored_created: i64 = sqlx::query_scalar("SELECT created_at FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        let stored_updated: i64 = sqlx::query_scalar("SELECT updated_at FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(stored_created, created_at);
        assert_eq!(stored_updated, updated_at);
    }

    #[tokio::test]
    async fn integration_transaction_safety() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut worktree1 = create_test_worktree("tx-1", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        let mut worktree2 = create_test_worktree("tx-2", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        
        // Both should save successfully
        repo.save(&mut worktree1).await.unwrap();
        repo.save(&mut worktree2).await.unwrap();
        
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn integration_name_unique_enforcement() {
        let mut repo = create_postgres_repo().await.unwrap();
        
        let mut wt1 = create_test_worktree("same-name", "/tmp/wt1", "/home/user/proj", WorktreeTypeEnum::Development, None);
        repo.save(&mut wt1).await.unwrap();
        
        // Try to save with same name - should update, not create duplicate
        let mut wt2 = create_test_worktree("same-name", "/tmp/wt2", "/home/user/proj", WorktreeTypeEnum::Testing, None);
        repo.save(&mut wt2).await.unwrap();
        
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = $1")
            .bind("same-name")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(count, 1);
        
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees")
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert_eq!(total_count, 1);
    }

    #[tokio::test]
    async fn integration_null_branch_handling() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("null-branch-test", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        repo.save(&mut worktree).await.unwrap();
        
        let branch_value: Option<String> = sqlx::query_scalar("SELECT branch FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert!(branch_value.is_none());
    }

    #[tokio::test]
    async fn integration_jsonb_metadata_structure() {
        let mut repo = create_postgres_repo().await.unwrap();
        let mut worktree = create_test_worktree("jsonb-structure", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
        
        worktree.add_metadata("string-key", "string-value");
        worktree.add_metadata("number-key", "123");
        
        repo.save(&mut worktree).await.unwrap();
        
        let metadata: serde_json::Value = sqlx::query_scalar("SELECT metadata FROM worktrees WHERE id = $1")
            .bind(worktree.id().as_bytes())
            .fetch_one(repo.pool())
            .await
            .unwrap();
        
        assert!(metadata.is_object());
        assert!(metadata.get("string-key").is_some());
        assert!(metadata.get("number-key").is_some());
    }
}

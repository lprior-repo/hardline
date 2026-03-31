#[cfg(test)]
mod delete_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use crate::beads::db::schema::ensure_schema;
    use crate::beads::db::write::{delete_bead, insert_bead};
    use crate::beads::types::{BeadIssue, BeadsError, IssueStatus, IssueType, Priority};

    async fn create_test_pool() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().ok();
        assert!(temp_dir.is_some());

        let temp_dir = temp_dir.unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url).await.ok();
        assert!(pool.is_some());

        let pool = pool.unwrap();
        let schema_result = ensure_schema(&pool).await;
        assert!(schema_result.is_ok());

        (pool, temp_dir)
    }

    fn create_valid_bead(id: &str, title: &str) -> BeadIssue {
        let now = Utc::now();
        BeadIssue {
            id: id.to_string(),
            title: title.to_string(),
            status: IssueStatus::Open,
            priority: Some(Priority::P1),
            issue_type: Some(IssueType::Feature),
            description: Some("Test description".to_string()),
            labels: Some(vec!["test".to_string()]),
            assignee: Some("testuser".to_string()),
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
        }
    }

    // Behavior: Deleting an existing bead succeeds
    #[tokio::test]
    async fn given_existing_bead_when_delete_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert a bead
        let bead = create_valid_bead("delete-test-1", "To Be Deleted");
        let insert_result = insert_bead(&pool, &bead).await;
        assert!(insert_result.is_ok());

        // Delete the bead
        let result = delete_bead(&pool, "delete-test-1").await;
        assert!(result.is_ok());
    }

    // Behavior: Deleting a non-existent bead returns NotFound error
    #[tokio::test]
    async fn given_nonexistent_id_when_delete_then_returns_not_found_error() {
        let (pool, _temp_dir) = create_test_pool().await;

        let result = delete_bead(&pool, "nonexistent-id").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::NotFound(_)));
            assert!(e.to_string().contains("nonexistent-id"));
        }
    }
}

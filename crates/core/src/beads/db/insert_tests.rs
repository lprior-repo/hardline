#[cfg(test)]
mod insert_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use crate::beads::db::schema::ensure_schema;
    use crate::beads::db::write::insert_bead;
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

    // Behavior: Insert a valid bead succeeds
    #[tokio::test]
    async fn given_valid_bead_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let bead = create_valid_bead("test-1", "Test Issue");

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_ok());
    }

    // Behavior: Inserting a bead with duplicate ID fails
    #[tokio::test]
    async fn given_duplicate_id_when_insert_then_returns_duplicate_error() {
        let (pool, _temp_dir) = create_test_pool().await;
        let bead = create_valid_bead("duplicate-id", "First Issue");

        // First insert should succeed
        let first_result = insert_bead(&pool, &bead).await;
        assert!(first_result.is_ok());

        // Second insert with same ID should fail
        let second_bead = create_valid_bead("duplicate-id", "Second Issue");
        let second_result = insert_bead(&pool, &second_bead).await;
        assert!(second_result.is_err());

        if let Err(e) = second_result {
            assert!(matches!(e, BeadsError::DuplicateId(_)));
            assert!(e.to_string().contains("duplicate-id"));
        }
    }

    // Behavior: Inserting a bead with empty ID fails validation
    #[tokio::test]
    async fn given_empty_id_when_insert_then_returns_validation_error() {
        let (pool, _temp_dir) = create_test_pool().await;
        let bead = create_valid_bead("", "Test Issue");

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::ValidationFailed(_)));
            assert!(e.to_string().contains("ID"));
        }
    }

    // Behavior: Inserting a bead with empty title fails validation
    #[tokio::test]
    async fn given_empty_title_when_insert_then_returns_validation_error() {
        let (pool, _temp_dir) = create_test_pool().await;
        let bead = create_valid_bead("test-id", "");

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::ValidationFailed(_)));
            assert!(e.to_string().contains("Title"));
        }
    }

    // Behavior: Insert bead with all optional fields as None succeeds
    #[tokio::test]
    async fn given_minimal_bead_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = Utc::now();
        let bead = BeadIssue {
            id: "minimal-1".to_string(),
            title: "Minimal Issue".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
        };

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_ok());
    }

    // Behavior: Insert bead with all fields populated succeeds
    #[tokio::test]
    async fn given_complete_bead_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = Utc::now();
        let bead = BeadIssue {
            id: "complete-1".to_string(),
            title: "Complete Issue".to_string(),
            status: IssueStatus::InProgress,
            priority: Some(Priority::P0),
            issue_type: Some(IssueType::Bug),
            description: Some("A complete description".to_string()),
            labels: Some(vec!["bug".to_string(), "critical".to_string()]),
            assignee: Some("developer".to_string()),
            parent: Some("parent-1".to_string()),
            depends_on: Some(vec!["dep-1".to_string(), "dep-2".to_string()]),
            blocked_by: Some(vec!["blocker-1".to_string()]),
            created_at: now,
            updated_at: now,
            closed_at: Some(now),
        };

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_ok());
    }

    // Behavior: Insert bead with different statuses succeeds (with proper closed_at)
    #[tokio::test]
    async fn given_various_statuses_when_insert_then_all_succeed() {
        let (pool, _temp_dir) = create_test_pool().await;

        let non_closed_statuses = [
            IssueStatus::Open,
            IssueStatus::InProgress,
            IssueStatus::Blocked,
            IssueStatus::Deferred,
        ];

        for (i, status) in non_closed_statuses.iter().enumerate() {
            let now = Utc::now();
            let bead = BeadIssue {
                id: format!("status-{i}"),
                title: format!("Status Test {i}"),
                status: *status,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: now,
                updated_at: now,
                closed_at: None,
            };

            let result = insert_bead(&pool, &bead).await;
            assert!(result.is_ok(), "Failed to insert bead with status {status}");
        }
    }

    // Behavior: Inserting closed status without closed_at fails validation
    #[tokio::test]
    async fn given_closed_status_without_closed_at_when_insert_then_returns_validation_error() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = Utc::now();
        let bead = BeadIssue {
            id: "closed-no-date".to_string(),
            title: "Closed without date".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: now,
            updated_at: now,
            closed_at: None, // This violates the invariant!
        };

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::ValidationFailed(_)));
            assert!(e.to_string().contains("closed_at"));
        }
    }

    // Behavior: Inserting closed status with closed_at succeeds
    #[tokio::test]
    async fn given_closed_status_with_closed_at_when_insert_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;
        let now = Utc::now();
        let bead = BeadIssue {
            id: "closed-with-date".to_string(),
            title: "Closed with date".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: now,
            updated_at: now,
            closed_at: Some(now), // Proper closed_at set
        };

        let result = insert_bead(&pool, &bead).await;
        assert!(result.is_ok());
    }
}

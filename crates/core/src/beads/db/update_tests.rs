#[cfg(test)]
mod update_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use crate::beads::types::{BeadIssue, BeadsError, IssueStatus, IssueType, Priority};
    use crate::beads::db::schema::ensure_schema;
    use crate::beads::db::write::{insert_bead, update_bead};

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

    // Behavior: Updating an existing bead succeeds
    #[tokio::test]
    async fn given_existing_bead_when_update_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert a bead
        let original = create_valid_bead("update-test-1", "Original Title");
        let insert_result = insert_bead(&pool, &original).await;
        assert!(insert_result.is_ok());

        // Update the bead
        let updated = BeadIssue {
            id: "update-test-1".to_string(),
            title: "Updated Title".to_string(),
            status: IssueStatus::InProgress,
            priority: Some(Priority::P0),
            issue_type: Some(IssueType::Bug),
            description: Some("Updated description".to_string()),
            labels: Some(vec!["bug".to_string(), "critical".to_string()]),
            assignee: Some("developer".to_string()),
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: original.created_at,
            updated_at: Utc::now(),
            closed_at: None,
        };

        let result = update_bead(&pool, "update-test-1", &updated).await;
        assert!(result.is_ok());

        let returned = result.unwrap();
        assert_eq!(returned.title, "Updated Title");
        assert_eq!(returned.status, IssueStatus::InProgress);
        assert_eq!(returned.priority, Some(Priority::P0));
    }

    // Behavior: Updating a non-existent bead returns NotFound error
    #[tokio::test]
    async fn given_nonexistent_id_when_update_then_returns_not_found_error() {
        let (pool, _temp_dir) = create_test_pool().await;

        let updated = create_valid_bead("nonexistent-id", "Updated Title");
        let result = update_bead(&pool, "nonexistent-id", &updated).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, BeadsError::NotFound(_)));
            assert!(e.to_string().contains("nonexistent-id"));
        }
    }

    // Behavior: Updating a bead with empty title returns validation error
    #[tokio::test]
    async fn given_empty_title_when_update_then_returns_validation_error() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert a bead
        let original = create_valid_bead("empty-title-test", "Original Title");
        let insert_result = insert_bead(&pool, &original).await;
        assert!(insert_result.is_ok());

        // Try to update with empty title
        let updated = BeadIssue {
            title: String::new(),
            ..original.clone()
        };

        let result = update_bead(&pool, "empty-title-test", &updated).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::ValidationFailed(_)));
            assert!(e.to_string().contains("Title"));
        }
    }

    // Behavior: Updating status to closed sets closed_at
    #[tokio::test]
    async fn given_open_bead_when_closing_then_can_set_closed_at() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert an open bead
        let original = create_valid_bead("close-test", "To Be Closed");
        let insert_result = insert_bead(&pool, &original).await;
        assert!(insert_result.is_ok());

        // Close the bead with closed_at timestamp
        let closed_time = Utc::now();
        let closed = BeadIssue {
            id: "close-test".to_string(),
            title: "To Be Closed".to_string(),
            status: IssueStatus::Closed,
            priority: original.priority,
            issue_type: original.issue_type,
            description: original.description.clone(),
            labels: original.labels.clone(),
            assignee: original.assignee.clone(),
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: original.created_at,
            updated_at: closed_time,
            closed_at: Some(closed_time),
        };

        let result = update_bead(&pool, "close-test", &closed).await;
        assert!(result.is_ok());

        let returned = result.unwrap();
        assert_eq!(returned.status, IssueStatus::Closed);
        assert!(returned.closed_at.is_some());
    }

    // Behavior: Updating all fields succeeds
    #[tokio::test]
    async fn given_existing_bead_when_updating_all_fields_then_succeeds() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert a bead
        let original = create_valid_bead("full-update-test", "Original");
        let insert_result = insert_bead(&pool, &original).await;
        assert!(insert_result.is_ok());

        // Update all fields
        let now = Utc::now();
        let updated = BeadIssue {
            id: "full-update-test".to_string(),
            title: "Fully Updated".to_string(),
            status: IssueStatus::Closed,
            priority: Some(Priority::P2),
            issue_type: Some(IssueType::Task),
            description: Some("New description".to_string()),
            labels: Some(vec!["new-label".to_string()]),
            assignee: Some("new-assignee".to_string()),
            parent: Some("parent-123".to_string()),
            depends_on: Some(vec!["dep-1".to_string()]),
            blocked_by: Some(vec!["blocker-1".to_string()]),
            created_at: original.created_at,
            updated_at: now,
            closed_at: Some(now),
        };

        let result = update_bead(&pool, "full-update-test", &updated).await;
        assert!(result.is_ok());

        let returned = result.unwrap();
        assert_eq!(returned.title, "Fully Updated");
        assert_eq!(returned.status, IssueStatus::Closed);
        assert_eq!(returned.priority, Some(Priority::P2));
        assert_eq!(returned.issue_type, Some(IssueType::Task));
        assert_eq!(returned.description, Some("New description".to_string()));
        assert_eq!(returned.labels, Some(vec!["new-label".to_string()]));
        assert_eq!(returned.assignee, Some("new-assignee".to_string()));
        assert_eq!(returned.parent, Some("parent-123".to_string()));
        assert_eq!(returned.depends_on, Some(vec!["dep-1".to_string()]));
        assert_eq!(returned.blocked_by, Some(vec!["blocker-1".to_string()]));
        assert!(returned.closed_at.is_some());
    }

    // Behavior: Updating to closed status without closed_at fails validation
    // This tests the invariant: status='closed' => closed_at IS NOT NULL
    #[tokio::test]
    async fn given_open_bead_when_updating_to_closed_without_closed_at_then_returns_validation_error() {
        let (pool, _temp_dir) = create_test_pool().await;

        // First insert an open bead
        let original = create_valid_bead("invariant-test", "Invariant Test");
        let insert_result = insert_bead(&pool, &original).await;
        assert!(insert_result.is_ok());

        // Try to close without setting closed_at (violates invariant!)
        let closed = BeadIssue {
            id: "invariant-test".to_string(),
            title: "Invariant Test".to_string(),
            status: IssueStatus::Closed,
            priority: original.priority,
            issue_type: original.issue_type,
            description: original.description.clone(),
            labels: original.labels.clone(),
            assignee: original.assignee.clone(),
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: original.created_at,
            updated_at: Utc::now(),
            closed_at: None, // Missing closed_at with Closed status!
        };

        let result = update_bead(&pool, "invariant-test", &closed).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, BeadsError::ValidationFailed(_)));
            assert!(e.to_string().contains("closed_at"));
        }
    }
}

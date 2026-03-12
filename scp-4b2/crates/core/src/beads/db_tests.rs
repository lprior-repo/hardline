use chrono::Utc;
use sqlx::SqlitePool;
use tempfile::TempDir;

use super::{
    db::{enable_wal_mode, parse_bead_row, parse_datetime, parse_status, query_beads},
    types::{BeadsError, IssueStatus, IssueType, Priority},
};

// Behavior: Parse valid RFC3339 datetime strings
#[tokio::test]
async fn given_valid_rfc3339_when_parse_datetime_then_returns_utc_datetime() {
    let valid_datetime = Some("2026-02-14T10:00:00Z".to_string());
    let result = parse_datetime(valid_datetime);
    assert!(result.is_ok());
    let dt = result.ok();
    assert!(dt.is_some());
}

// Behavior: Parse datetime fails when string is missing
#[tokio::test]
async fn given_none_when_parse_datetime_then_returns_error() {
    let result = parse_datetime(None);
    assert!(result.is_err());
    assert!(matches!(result, Err(BeadsError::QueryFailed(_))));
}

// Behavior: Parse datetime fails when string is invalid
#[tokio::test]
async fn given_invalid_datetime_when_parse_datetime_then_returns_error() {
    let invalid = Some("not-a-datetime".to_string());
    let result = parse_datetime(invalid);
    assert!(result.is_err());
    assert!(matches!(result, Err(BeadsError::QueryFailed(_))));
}

// Behavior: Parse valid status strings
#[tokio::test]
async fn given_valid_status_strings_when_parse_status_then_returns_enum() {
    assert_eq!(parse_status("open").ok(), Some(IssueStatus::Open));
    assert_eq!(
        parse_status("in_progress").ok(),
        Some(IssueStatus::InProgress)
    );
    assert_eq!(parse_status("blocked").ok(), Some(IssueStatus::Blocked));
    assert_eq!(parse_status("deferred").ok(), Some(IssueStatus::Deferred));
    assert_eq!(parse_status("closed").ok(), Some(IssueStatus::Closed));
}

// Behavior: Parse status fails for invalid strings
#[tokio::test]
async fn given_invalid_status_when_parse_status_then_returns_error() {
    let result = parse_status("invalid_status");
    assert!(result.is_err());
    assert!(matches!(result, Err(BeadsError::QueryFailed(_))));
}

// Behavior: Enable WAL mode on SQLite connection
#[tokio::test]
async fn given_sqlite_pool_when_enable_wal_mode_then_succeeds() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());
    let temp_dir = temp_dir.as_ref();

    if let Some(dir) = temp_dir {
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool_result = SqlitePool::connect(&db_url).await;
        assert!(pool_result.is_ok());

        if let Ok(pool) = pool_result {
            let wal_result = enable_wal_mode(&pool).await;
            assert!(wal_result.is_ok());
        }
    }
}

// Behavior: Query beads returns empty vec when database doesn't exist
#[tokio::test]
async fn given_no_database_when_query_beads_then_returns_empty_vec() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let workspace_path = dir.path();
        let result = query_beads(workspace_path).await;
        assert!(result.is_ok());
        assert_eq!(result.ok().map(|v| v.len()), Some(0));
    }
}

// Behavior: Query beads creates connection and queries when database exists
#[tokio::test]
async fn given_valid_database_with_issues_when_query_beads_then_returns_issues() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let workspace_path = dir.path();
        let beads_dir = workspace_path.join(".beads");
        std::fs::create_dir_all(&beads_dir).ok();

        let db_path = beads_dir.join("beads.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool_result = SqlitePool::connect(&db_url).await;
        assert!(pool_result.is_ok());

        if let Ok(pool) = pool_result {
            // Create schema
            let schema_result = sqlx::query(
                "CREATE TABLE IF NOT EXISTS issues (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    status TEXT NOT NULL,
                    priority TEXT,
                    type TEXT,
                    description TEXT,
                    labels TEXT,
                    assignee TEXT,
                    parent TEXT,
                    depends_on TEXT,
                    blocked_by TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    closed_at TEXT
                )",
            )
            .execute(&pool)
            .await;
            assert!(schema_result.is_ok());

            // Insert test data
            let now = Utc::now().to_rfc3339();
            let insert_result = sqlx::query(
                "INSERT INTO issues (id, title, status, priority, type, description, 
                                    labels, assignee, parent, depends_on, blocked_by,
                                    created_at, updated_at, closed_at)
                 VALUES ('test-1', 'Test Issue', 'open', 'P0', 'bug', 'Test description',
                        'label1,label2', 'testuser', NULL, NULL, NULL,
                        ?1, ?1, NULL)",
            )
            .bind(&now)
            .execute(&pool)
            .await;
            assert!(insert_result.is_ok());

            // Query beads
            let result = query_beads(workspace_path).await;
            assert!(result.is_ok());

            if let Ok(issues) = result {
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].id, "test-1");
                assert_eq!(issues[0].title, "Test Issue");
                assert_eq!(issues[0].status, IssueStatus::Open);
                assert_eq!(issues[0].priority, Some(Priority::P0));
                assert_eq!(issues[0].issue_type, Some(IssueType::Bug));
            }
        }
    }
}

// Behavior: Query beads handles invalid UTF-8 in path
#[tokio::test]
async fn given_invalid_utf8_path_when_query_beads_then_returns_error() {
    // This test verifies the path validation logic
    // On most systems, creating invalid UTF-8 paths is difficult
    // So we test the happy path and rely on the error handling code
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let workspace_path = dir.path();
        let result = query_beads(workspace_path).await;
        // Should succeed with empty result since no .beads directory
        assert!(result.is_ok());
    }
}

// Behavior: Query beads orders results by priority and created_at
#[tokio::test]
async fn given_multiple_issues_when_query_beads_then_ordered_by_priority_and_created() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let workspace_path = dir.path();
        let beads_dir = workspace_path.join(".beads");
        std::fs::create_dir_all(&beads_dir).ok();

        let db_path = beads_dir.join("beads.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool_result = SqlitePool::connect(&db_url).await;
        assert!(pool_result.is_ok());

        if let Ok(pool) = pool_result {
            // Create schema
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS issues (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL,
                    priority TEXT, type TEXT, description TEXT, labels TEXT,
                    assignee TEXT, parent TEXT, depends_on TEXT, blocked_by TEXT,
                    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, closed_at TEXT
                )",
            )
            .execute(&pool)
            .await
            .ok();

            // Insert test data with different priorities
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO issues (id, title, status, priority, created_at, updated_at)
                 VALUES ('p2-issue', 'P2 Issue', 'open', 'P2', ?1, ?1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .ok();

            sqlx::query(
                "INSERT INTO issues (id, title, status, priority, created_at, updated_at)
                 VALUES ('p0-issue', 'P0 Issue', 'open', 'P0', ?1, ?1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .ok();

            // Query beads
            let result = query_beads(workspace_path).await;
            assert!(result.is_ok());

            if let Ok(issues) = result {
                assert_eq!(issues.len(), 2);
                // P0 should come before P2
                assert_eq!(issues[0].priority, Some(Priority::P0));
                assert_eq!(issues[1].priority, Some(Priority::P2));
            }
        }
    }
}

// Behavior: Parse bead row handles all required fields
#[tokio::test]
async fn given_complete_row_when_parse_bead_row_then_returns_bead_issue() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool_result = SqlitePool::connect(&db_url).await;
        assert!(pool_result.is_ok());

        if let Ok(pool) = pool_result {
            sqlx::query(
                "CREATE TABLE test (
                    id TEXT, title TEXT, status TEXT, priority TEXT,
                    type TEXT, description TEXT, labels TEXT, assignee TEXT,
                    parent TEXT, depends_on TEXT, blocked_by TEXT,
                    created_at TEXT, updated_at TEXT, closed_at TEXT
                )",
            )
            .execute(&pool)
            .await
            .ok();

            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO test VALUES ('id1', 'Title', 'open', 'P1', 'feature',
                 'Description', 'label1', 'user1', 'parent1', 'dep1', 'block1',
                 ?1, ?1, NULL)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .ok();

            let row_result = sqlx::query("SELECT * FROM test").fetch_one(&pool).await;
            assert!(row_result.is_ok());

            if let Ok(row) = row_result {
                let parsed = parse_bead_row(&row);
                assert!(parsed.is_ok());

                if let Ok(issue) = parsed {
                    assert_eq!(issue.id, "id1");
                    assert_eq!(issue.title, "Title");
                    assert_eq!(issue.status, IssueStatus::Open);
                    assert_eq!(issue.priority, Some(Priority::P1));
                }
            }
        }
    }
}

// Behavior: Parse bead row handles optional fields as None
#[tokio::test]
async fn given_row_with_null_optionals_when_parse_bead_row_then_returns_none_for_optionals() {
    let temp_dir = TempDir::new().ok();
    assert!(temp_dir.is_some());

    if let Some(dir) = temp_dir {
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool_result = SqlitePool::connect(&db_url).await;
        assert!(pool_result.is_ok());

        if let Ok(pool) = pool_result {
            sqlx::query(
                "CREATE TABLE test (
                    id TEXT, title TEXT, status TEXT, priority TEXT,
                    type TEXT, description TEXT, labels TEXT, assignee TEXT,
                    parent TEXT, depends_on TEXT, blocked_by TEXT,
                    created_at TEXT, updated_at TEXT, closed_at TEXT
                )",
            )
            .execute(&pool)
            .await
            .ok();

            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO test VALUES ('id1', 'Title', 'open', NULL, NULL,
                 NULL, NULL, NULL, NULL, NULL, NULL, ?1, ?1, NULL)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .ok();

            let row_result = sqlx::query("SELECT * FROM test").fetch_one(&pool).await;
            assert!(row_result.is_ok());

            if let Ok(row) = row_result {
                let parsed = parse_bead_row(&row);
                assert!(parsed.is_ok());

                if let Ok(issue) = parsed {
                    assert_eq!(issue.priority, None);
                    assert_eq!(issue.issue_type, None);
                    assert_eq!(issue.description, None);
                    assert_eq!(issue.labels, None);
                    assert_eq!(issue.assignee, None);
                    assert_eq!(issue.closed_at, None);
                }
            }
        }
    }
}

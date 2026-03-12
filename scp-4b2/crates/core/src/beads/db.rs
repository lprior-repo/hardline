#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::types::{BeadIssue, BeadsError, IssueStatus, Priority};

/// Parse a datetime string from RFC3339 format.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the string is missing or invalid.
pub(crate) fn parse_datetime(datetime_str: Option<String>) -> Result<DateTime<Utc>, BeadsError> {
    datetime_str
        .ok_or_else(|| BeadsError::QueryFailed("Missing required datetime field".to_string()))
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid datetime format '{s}': {e}")))
        })
}

/// Parse a status string into `IssueStatus`.
///
/// # Errors
///
/// Returns `BeadsError::QueryFailed` if the status string is invalid.
pub(crate) fn parse_status(status_str: &str) -> Result<IssueStatus, BeadsError> {
    status_str.parse().map_err(|_| {
        BeadsError::QueryFailed(format!("Invalid status value '{status_str}'. Expected one of: open, in_progress, done, cancelled"))
    })
}

/// Enable `WAL` mode on the `SQLite` connection for better crash recovery.
///
/// # Errors
///
/// Returns `BeadsError` if the `PRAGMA` statement fails.
pub(crate) async fn enable_wal_mode(pool: &SqlitePool) -> std::result::Result<(), BeadsError> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to enable WAL mode: {e}")))?;
    Ok(())
}

/// Query all issues from the beads database.
///
/// Parse a single row from the beads database into a `BeadIssue`
///
/// # Errors
///
/// Returns `BeadsError` if any required field is missing or malformed
pub(crate) fn parse_bead_row(
    row: &sqlx::sqlite::SqliteRow,
) -> std::result::Result<BeadIssue, BeadsError> {
    let status_str: String = row
        .try_get("status")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'status' error: {e}")))?;
    let status = parse_status(&status_str)?;

    let priority_str: Option<String> = row.try_get("priority").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'priority' error: {e}"))
    })?;
    let priority = priority_str
        .and_then(|p: String| p.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()))
        .and_then(Priority::from_u32);

    let issue_type_str: Option<String> = row
        .try_get("type")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'type' error: {e}")))?;
    let issue_type = issue_type_str.and_then(|s: String| s.parse().ok());

    let labels_str: Option<String> = row
        .try_get("labels")
        .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'labels' error: {e}")))?;
    let labels =
        labels_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let depends_on_str: Option<String> = row.try_get("depends_on").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'depends_on' error: {e}"))
    })?;
    let depends_on =
        depends_on_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    let blocked_by_str: Option<String> = row.try_get("blocked_by").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'blocked_by' error: {e}"))
    })?;
    let blocked_by =
        blocked_by_str.map(|s: String| s.split(',').map(String::from).collect::<Vec<String>>());

    // Required datetime fields - fail if missing or invalid
    let created_at_str: Option<String> = row.try_get("created_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'created_at' error: {e}"))
    })?;
    let created_at = parse_datetime(created_at_str)?;

    let updated_at_str: Option<String> = row.try_get("updated_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'updated_at' error: {e}"))
    })?;
    let updated_at = parse_datetime(updated_at_str)?;

    // Optional datetime field
    let closed_at_str: Option<String> = row.try_get("closed_at").map_err(|e: sqlx::Error| {
        BeadsError::QueryFailed(format!("Field 'closed_at' error: {e}"))
    })?;
    let closed_at = closed_at_str
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| BeadsError::QueryFailed(format!("Invalid closed_at datetime: {e}")))
        })
        .transpose()?;

    Ok(BeadIssue {
        id: row
            .try_get("id")
            .map_err(|e: sqlx::Error| BeadsError::QueryFailed(format!("Field 'id' error: {e}")))?,
        title: row.try_get("title").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'title' error: {e}"))
        })?,
        status,
        priority,
        issue_type,
        description: row.try_get("description").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'description' error: {e}"))
        })?,
        labels,
        assignee: row.try_get("assignee").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'assignee' error: {e}"))
        })?,
        parent: row.try_get("parent").map_err(|e: sqlx::Error| {
            BeadsError::QueryFailed(format!("Field 'parent' error: {e}"))
        })?,
        depends_on,
        blocked_by,
        created_at,
        updated_at,
        closed_at,
    })
}

/// # Errors
///
/// Returns `BeadsError` if:
/// - The database cannot be opened or queried
/// - Any required field is missing or malformed
/// - Status or datetime values are invalid
pub async fn query_beads(workspace_path: &Path) -> std::result::Result<Vec<BeadIssue>, BeadsError> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        tracing::warn!(
            "Beads database not found at {}. It will be created when needed.",
            beads_db.display()
        );
        return Ok(Vec::new());
    }

    let path_str = beads_db.to_str().ok_or_else(|| {
        BeadsError::DatabaseError("Beads database path contains invalid UTF-8".to_string())
    })?;

    let db_url = format!("sqlite://{path_str}?mode=rw");
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to connect to beads.db: {e}")))?;

    // Enable WAL mode for better crash recovery
    enable_wal_mode(&pool).await?;

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, title, status, priority, type, description, labels, assignee,
                parent, depends_on, blocked_by, created_at, updated_at, closed_at
         FROM issues ORDER BY priority, created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.iter()
        .map(parse_bead_row)
        .collect::<std::result::Result<Vec<_>, BeadsError>>()
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to parse bead issues: {e}")))
}

/// Validate a bead issue for insertion.
///
/// # Errors
///
/// Returns `BeadsError::ValidationFailed` if:
/// - ID is empty
/// - Title is empty
fn validate_bead_for_insert(issue: &BeadIssue) -> std::result::Result<(), BeadsError> {
    if issue.id.is_empty() {
        return Err(BeadsError::ValidationFailed(
            "ID cannot be empty".to_string(),
        ));
    }
    if issue.title.is_empty() {
        return Err(BeadsError::ValidationFailed(
            "Title cannot be empty".to_string(),
        ));
    }
    // Enforce invariant: status='closed' => closed_at IS NOT NULL
    // This matches the CHECK constraint in the database schema
    if issue.status == IssueStatus::Closed && issue.closed_at.is_none() {
        return Err(BeadsError::ValidationFailed(
            "closed_at must be set when status is 'closed'".to_string(),
        ));
    }
    Ok(())
}

/// Serialize optional vector as comma-separated string.
fn serialize_optional_vec(v: Option<&Vec<String>>) -> Option<String> {
    v.and_then(|items| {
        if items.is_empty() {
            None
        } else {
            Some(items.join(","))
        }
    })
}

/// Insert a bead issue into the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - Validation fails (empty ID or title)
/// - The insert operation fails
/// - A bead with the same ID already exists (`DuplicateId`)
pub async fn insert_bead(
    pool: &SqlitePool,
    issue: &BeadIssue,
) -> std::result::Result<(), BeadsError> {
    // Validate input
    validate_bead_for_insert(issue)?;

    // Serialize optional fields
    let priority_str = issue
        .priority
        .map(|priority| format!("P{}", priority.to_u32()));
    let issue_type_str = issue
        .issue_type
        .as_ref()
        .map(std::string::ToString::to_string);
    let labels_str = serialize_optional_vec(issue.labels.as_ref());
    let depends_on_str = serialize_optional_vec(issue.depends_on.as_ref());
    let blocked_by_str = serialize_optional_vec(issue.blocked_by.as_ref());
    let created_at_str = issue.created_at.to_rfc3339();
    let updated_at_str = issue.updated_at.to_rfc3339();
    let closed_at_str = issue.closed_at.map(|dt| dt.to_rfc3339());

    // Execute insert
    let result = sqlx::query(
        "INSERT INTO issues (id, title, status, priority, type, description, labels,
                             assignee, parent, depends_on, blocked_by,
                             created_at, updated_at, closed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )
    .bind(&issue.id)
    .bind(&issue.title)
    .bind(issue.status.to_string())
    .bind(priority_str)
    .bind(issue_type_str)
    .bind(&issue.description)
    .bind(labels_str)
    .bind(&issue.assignee)
    .bind(&issue.parent)
    .bind(depends_on_str)
    .bind(blocked_by_str)
    .bind(&created_at_str)
    .bind(&updated_at_str)
    .bind(closed_at_str)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") || error_msg.contains("PRIMARY KEY") {
                Err(BeadsError::DuplicateId(issue.id.clone()))
            } else {
                Err(BeadsError::InsertFailed(format!(
                    "Failed to insert bead '{}': {e}",
                    issue.id
                )))
            }
        }
    }
}

/// Create the issues table schema if it does not exist.
///
/// # Errors
///
/// Returns `BeadsError` if the schema creation fails.
pub async fn ensure_schema(pool: &SqlitePool) -> std::result::Result<(), BeadsError> {
    sqlx::query(
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
    .execute(pool)
    .await
    .map_err(|e| BeadsError::DatabaseError(format!("Failed to create issues schema: {e}")))?;
    Ok(())
}

/// Delete a bead issue from the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - The bead with the given ID does not exist (`NotFound`)
/// - The delete operation fails (`DatabaseError`)
pub async fn delete_bead(pool: &SqlitePool, id: &str) -> std::result::Result<(), BeadsError> {
    let result = sqlx::query("DELETE FROM issues WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to delete bead '{id}': {e}")))?;

    if result.rows_affected() == 0 {
        return Err(BeadsError::NotFound(id.to_string()));
    }

    Ok(())
}

/// Update an existing bead issue in the database.
///
/// # Errors
///
/// Returns `BeadsError` if:
/// - Validation fails (empty title)
/// - The bead with the given ID does not exist (`NotFound`)
/// - The update operation fails
pub async fn update_bead(
    pool: &SqlitePool,
    id: &str,
    issue: &BeadIssue,
) -> std::result::Result<BeadIssue, BeadsError> {
    // Validate input
    if issue.title.is_empty() {
        return Err(BeadsError::ValidationFailed(
            "Title cannot be empty".to_string(),
        ));
    }

    // Enforce invariant: status='closed' => closed_at IS NOT NULL
    // This matches the CHECK constraint in the database schema
    if issue.status == IssueStatus::Closed && issue.closed_at.is_none() {
        return Err(BeadsError::ValidationFailed(
            "closed_at must be set when status is 'closed'".to_string(),
        ));
    }

    // Serialize optional fields
    let priority_str = issue
        .priority
        .map(|priority| format!("P{}", priority.to_u32()));
    let issue_type_str = issue
        .issue_type
        .as_ref()
        .map(std::string::ToString::to_string);
    let labels_str = serialize_optional_vec(issue.labels.as_ref());
    let depends_on_str = serialize_optional_vec(issue.depends_on.as_ref());
    let blocked_by_str = serialize_optional_vec(issue.blocked_by.as_ref());
    let updated_at_str = issue.updated_at.to_rfc3339();
    let closed_at_str = issue.closed_at.map(|dt| dt.to_rfc3339());

    // Execute update
    let result = sqlx::query(
        "UPDATE issues SET
            title = ?1,
            status = ?2,
            priority = ?3,
            type = ?4,
            description = ?5,
            labels = ?6,
            assignee = ?7,
            parent = ?8,
            depends_on = ?9,
            blocked_by = ?10,
            updated_at = ?11,
            closed_at = ?12
         WHERE id = ?13",
    )
    .bind(&issue.title)
    .bind(issue.status.to_string())
    .bind(priority_str)
    .bind(issue_type_str)
    .bind(&issue.description)
    .bind(labels_str)
    .bind(&issue.assignee)
    .bind(&issue.parent)
    .bind(depends_on_str)
    .bind(blocked_by_str)
    .bind(&updated_at_str)
    .bind(closed_at_str)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| BeadsError::DatabaseError(format!("Failed to update bead '{id}': {e}")))?;

    // Check if row was updated
    if result.rows_affected() == 0 {
        return Err(BeadsError::NotFound(id.to_string()));
    }

    // Return the updated issue
    Ok(issue.clone())
}

#[cfg(test)]
mod insert_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use super::{
        super::types::{BeadIssue, BeadsError, IssueStatus, IssueType, Priority},
        ensure_schema, insert_bead,
    };

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

#[cfg(test)]
mod update_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use super::{
        super::types::{BeadIssue, BeadsError, IssueStatus, IssueType, Priority},
        ensure_schema, insert_bead, update_bead,
    };

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
    async fn given_open_bead_when_updating_to_closed_without_closed_at_then_returns_validation_error(
    ) {
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

#[cfg(test)]
mod delete_tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use super::{
        super::types::{BeadIssue, BeadsError, IssueStatus, IssueType, Priority},
        delete_bead, ensure_schema, insert_bead,
    };

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

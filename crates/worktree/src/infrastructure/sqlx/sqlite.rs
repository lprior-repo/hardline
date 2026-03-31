use crate::application::repositories::WorktreeRepository;
use crate::domain::{
    AbsolutePath, BranchName, Worktree, WorktreeDomainError, WorktreeId, WorktreeName,
    WorktreeState, WorktreeTypeEnum,
};
use serde::Deserialize;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::FromRow;
use sqlx::SqlitePool;

#[derive(Deserialize, FromRow)]
struct SqliteWorktreeRow {
    id: String,
    name: String,
    path: String,
    parent_path: String,
    state: u8,
    worktree_type: u8,
    branch: Option<String>,
    created_at: i64,
    updated_at: i64,
    metadata: String,
}

impl SqliteWorktreeRow {
    fn to_worktree(&self) -> Worktree {
        let id = WorktreeId::from_string(&self.id).unwrap_or_else(|_| WorktreeId::new_random());
        let name =
            WorktreeName::new(&self.name).unwrap_or_else(|_| WorktreeName::new("unknown").unwrap());
        let path =
            AbsolutePath::new(&self.path).unwrap_or_else(|_| AbsolutePath::new("/tmp").unwrap());
        let parent_path = AbsolutePath::new(&self.parent_path)
            .unwrap_or_else(|_| AbsolutePath::new("/tmp").unwrap());
        let state = WorktreeState::from_u8(self.state).unwrap_or(WorktreeState::Creating);
        let worktree_type =
            WorktreeTypeEnum::from_u8(self.worktree_type).unwrap_or(WorktreeTypeEnum::Development);
        let branch = self.branch.as_ref().and_then(|b| BranchName::new(b).ok());

        // Deserialize metadata from JSON
        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_str(&self.metadata)
                .unwrap_or_else(|_| std::collections::HashMap::new());

        Worktree::uninitialized_with_metadata(
            id,
            name,
            path,
            parent_path,
            worktree_type,
            branch,
            state,
            self.created_at,
            self.updated_at,
            metadata,
        )
    }
}

/// Repository implementation using SQLite and SQLx
#[derive(Clone)]
pub struct SqliteWorktreeRepository {
    pool: SqlitePool,
}

#[async_trait::async_trait]
impl WorktreeRepository for SqliteWorktreeRepository {
    async fn save<S: Send>(&mut self, worktree: Worktree<S>) -> Result<(), WorktreeDomainError> {
        let query = r#"
            INSERT INTO worktrees (id, name, path, parent_path, state, worktree_type, branch, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                parent_path = excluded.parent_path,
                state = excluded.state,
                worktree_type = excluded.worktree_type,
                branch = excluded.branch,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata
        "#;

        let id_str = worktree.id().to_string();
        let name_str = worktree.name().as_str().to_string();
        let path_str = worktree.path().as_str().to_string();
        let parent_path_str = worktree.parent_path().as_str().to_string();
        let state_u8 = worktree.state().as_u8();
        let type_u8 = worktree.worktree_type().as_u8();
        let branch_opt = worktree.branch().map(|b| b.as_str().to_string());
        let created_at = worktree.created_at();
        let updated_at = worktree.updated_at();
        let metadata_json =
            serde_json::to_string(worktree.all_metadata()).unwrap_or("{}".to_string());

        sqlx::query(query)
            .bind(&id_str)
            .bind(&name_str)
            .bind(&path_str)
            .bind(&parent_path_str)
            .bind(state_u8)
            .bind(type_u8)
            .bind(&branch_opt)
            .bind(created_at)
            .bind(updated_at)
            .bind(&metadata_json)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to save worktree: {}", e))
            })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError> {
        let row = sqlx::query_as::<_, SqliteWorktreeRow>("SELECT * FROM worktrees WHERE id = ?")
            .bind(id.as_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to query worktree: {}", e))
            })?;

        Ok(row.map(|row| row.to_worktree()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        let row = sqlx::query_as::<_, SqliteWorktreeRow>("SELECT * FROM worktrees WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to query worktree: {}", e))
            })?;

        Ok(row.map(|row| row.to_worktree()))
    }

    async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError> {
        let rows = sqlx::query_as::<_, SqliteWorktreeRow>("SELECT * FROM worktrees")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to query worktrees: {}", e))
            })?;

        Ok(rows.into_iter().map(|row| row.to_worktree()).collect())
    }

    async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError> {
        sqlx::query("DELETE FROM worktrees WHERE id = ?")
            .bind(id.as_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to delete worktree: {}", e))
            })?;

        Ok(())
    }

    async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to check name: {}", e))
            })?;

        Ok(count > 0)
    }
}

impl SqliteWorktreeRepository {
    /// Create a new SQLite repository
    pub async fn new(database_url: &str) -> Result<Self, WorktreeDomainError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to connect to database: {}", e))
            })?;

        // Initialize schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS worktrees (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                state INTEGER NOT NULL DEFAULT 0,
                worktree_type INTEGER NOT NULL DEFAULT 0,
                branch TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS idx_worktrees_state ON worktrees(state);
            CREATE INDEX IF NOT EXISTS idx_worktrees_type ON worktrees(worktree_type);
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| {
            WorktreeDomainError::InvalidPath(format!("Failed to initialize schema: {}", e))
        })?;

        Ok(Self { pool })
    }

    /// Get the pool for testing (internal access)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

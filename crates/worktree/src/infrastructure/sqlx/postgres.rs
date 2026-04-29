use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};

use crate::{
    application::repositories::WorktreeRepository,
    domain::{
        AbsolutePath, BranchName, Worktree, WorktreeDomainError, WorktreeId, WorktreeName,
        WorktreeState, WorktreeTypeEnum,
    },
};

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Deserialize, FromRow)]
struct PostgresWorktreeRow {
    id: [u8; 16],
    name: String,
    path: String,
    parent_path: String,
    state: i32,
    worktree_type: i32,
    branch: Option<String>,
    created_at: i64,
    updated_at: i64,
    metadata: String,
}

impl PostgresWorktreeRow {
    fn to_worktree(&self) -> Worktree {
        let id = WorktreeId::from_bytes(self.id);
        let name = WorktreeName::new(&self.name).unwrap_or_else(|err| {
            eprintln!("Invalid worktree name '{}': {}", self.name, err);
            // "unknown" is a valid fallback since it passes validation
            WorktreeName::new_unchecked("unknown".to_string())
        });
        let path = AbsolutePath::new(&self.path).unwrap_or_else(|err| {
            eprintln!("Invalid path '{}': {}", self.path, err);
            // Use new_unchecked since /tmp is guaranteed to be valid
            // SAFETY: /tmp is always an absolute path without traversal
            unsafe { AbsolutePath::new_unchecked("/tmp") }
        });
        let parent_path = AbsolutePath::new(&self.parent_path).unwrap_or_else(|err| {
            eprintln!("Invalid parent path '{}': {}", self.parent_path, err);
            // Use new_unchecked since /tmp is guaranteed to be valid
            // SAFETY: /tmp is always an absolute path without traversal
            unsafe { AbsolutePath::new_unchecked("/tmp") }
        });
        let state = WorktreeState::from_u8(u8::try_from(self.state).unwrap_or_default()).unwrap_or_else(|| {
            eprintln!(
                "Invalid state value {}: {}",
                self.state,
                WorktreeDomainError::InvalidPath("Unknown state code".to_string())
            );
            WorktreeState::Creating
        });
        let worktree_type =
            WorktreeTypeEnum::from_u8(u8::try_from(self.worktree_type).unwrap_or_default()).unwrap_or_else(|| {
                eprintln!(
                    "Invalid type value {}: {}",
                    self.worktree_type,
                    WorktreeDomainError::InvalidPath("Unknown type code".to_string())
                );
                WorktreeTypeEnum::Development
            });
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

/// Repository implementation using `PostgreSQL` and `SQLx`
#[derive(Clone)]
pub struct PostgresWorktreeRepository {
    pool: PgPool,
}

#[async_trait::async_trait]
impl WorktreeRepository for PostgresWorktreeRepository {
    async fn save<S: Send>(&mut self, worktree: Worktree<S>) -> Result<(), WorktreeDomainError> {
        let id_bytes = worktree.id().as_bytes();

        let query = r"
            INSERT INTO worktrees (id, name, path, parent_path, state, worktree_type, branch, created_at, updated_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)
            ON CONFLICT (id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                parent_path = excluded.parent_path,
                state = excluded.state,
                worktree_type = excluded.worktree_type,
                branch = excluded.branch,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata
        ";

        let name_str = worktree.name().as_str().to_string();
        let path_str = worktree.path().as_str().to_string();
        let parent_path_str = worktree.parent_path().as_str().to_string();
        let state_i32 = i32::from(worktree.state().as_u8());
        let type_i32 = i32::from(worktree.worktree_type().as_u8());
        let branch_opt = worktree.branch().map(|b| b.as_str().to_string());
        let created_at = worktree.created_at();
        let updated_at = worktree.updated_at();
        let metadata_json = match serde_json::to_string(worktree.all_metadata()) {
            Ok(json) => json,
            Err(e) => {
                return Err(WorktreeDomainError::InvalidPath(format!(
                    "Failed to serialize metadata: {e}"
                )));
            }
        };

        sqlx::query(query)
            .bind(id_bytes)
            .bind(name_str)
            .bind(path_str)
            .bind(parent_path_str)
            .bind(state_i32)
            .bind(type_i32)
            .bind(branch_opt)
            .bind(created_at)
            .bind(updated_at)
            .bind(metadata_json)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to save worktree: {e}"))
            })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &WorktreeId) -> Result<Option<Worktree>, WorktreeDomainError> {
        let row = sqlx::query_as::<_, PostgresWorktreeRow>("SELECT id, name, path, parent_path, state, worktree_type, branch, created_at, updated_at, metadata::TEXT as metadata FROM worktrees WHERE id = $1")
            .bind(id.as_bytes())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| WorktreeDomainError::InvalidPath(format!("Failed to query worktree: {e}")))?;

        Ok(row.map(|row| row.to_worktree()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        let row = sqlx::query_as::<_, PostgresWorktreeRow>("SELECT id, name, path, parent_path, state, worktree_type, branch, created_at, updated_at, metadata::TEXT as metadata FROM worktrees WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| WorktreeDomainError::InvalidPath(format!("Failed to query worktree: {e}")))?;

        Ok(row.map(|row| row.to_worktree()))
    }

    async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError> {
        let rows = sqlx::query_as::<_, PostgresWorktreeRow>("SELECT id, name, path, parent_path, state, worktree_type, branch, created_at, updated_at, metadata::TEXT as metadata FROM worktrees")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WorktreeDomainError::InvalidPath(format!("Failed to query worktrees: {e}")))?;

        Ok(rows.into_iter().map(|row| row.to_worktree()).collect())
    }

    async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError> {
        let id_bytes = id.as_bytes();

        sqlx::query("DELETE FROM worktrees WHERE id = $1")
            .bind(id_bytes)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to delete worktree: {e}"))
            })?;

        Ok(())
    }

    async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE name = $1")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to check name: {e}"))
            })?;

        Ok(count > 0)
    }
}

impl PostgresWorktreeRepository {
    /// Create a new `PostgreSQL` repository
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or schema initialization fails.
    pub async fn new(database_url: &str) -> Result<Self, WorktreeDomainError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                WorktreeDomainError::InvalidPath(format!("Failed to connect to database: {e}"))
            })?;

        // Initialize schema - execute each statement separately since SQLx doesn't support multiple
        // statements Create table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS worktrees (
                id BYTEA PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                state INTEGER NOT NULL DEFAULT 0,
                worktree_type INTEGER NOT NULL DEFAULT 0,
                branch TEXT,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                metadata JSONB DEFAULT '{}',
                UNIQUE(name)
            )
            ",
        )
        .execute(&pool)
        .await
        .map_err(|e| {
            WorktreeDomainError::InvalidPath(format!("Failed to create worktrees table: {e}"))
        })?;

        // Create indexes - ignore errors if they already exist
        let _ = sqlx::query(r"CREATE INDEX IF NOT EXISTS idx_worktrees_name ON worktrees(name);")
            .execute(&pool)
            .await;
        let _ =
            sqlx::query(r"CREATE INDEX IF NOT EXISTS idx_worktrees_state ON worktrees(state);")
                .execute(&pool)
                .await;
        let _ = sqlx::query(
            r"CREATE INDEX IF NOT EXISTS idx_worktrees_type ON worktrees(worktree_type);",
        )
        .execute(&pool)
        .await;

        Ok(Self { pool })
    }

    /// Create a repository from an existing pool (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn from_pool(pool: PgPool) -> Result<Self, WorktreeDomainError> {
        // Initialize schema - execute each statement separately since SQLx doesn't support multiple
        // statements Create table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS worktrees (
                id BYTEA PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                state INTEGER NOT NULL DEFAULT 0,
                worktree_type INTEGER NOT NULL DEFAULT 0,
                branch TEXT,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                metadata JSONB DEFAULT '{}',
                UNIQUE(name)
            )
            ",
        )
        .execute(&pool)
        .await
        .map_err(|e| {
            WorktreeDomainError::InvalidPath(format!("Failed to create worktrees table: {e}"))
        })?;

        // Create indexes - ignore errors if they already exist
        let _ = sqlx::query(r"CREATE INDEX IF NOT EXISTS idx_worktrees_name ON worktrees(name);")
            .execute(&pool)
            .await;
        let _ =
            sqlx::query(r"CREATE INDEX IF NOT EXISTS idx_worktrees_state ON worktrees(state);")
                .execute(&pool)
                .await;
        let _ = sqlx::query(
            r"CREATE INDEX IF NOT EXISTS idx_worktrees_type ON worktrees(worktree_type);",
        )
        .execute(&pool)
        .await;

        Ok(Self { pool })
    }

    /// Get the pool for testing (internal access)
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

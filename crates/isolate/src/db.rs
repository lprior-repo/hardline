//! Database operations for session persistence using `SQLx`
//!
//! This module provides async SQLite-based persistence with:
//! - Connection pooling (no Arc<Mutex<>>)
//! - Zero unwraps, zero panics
//! - Simple embedded schema (no migration files)
//! - Pure functional patterns with Railway-Oriented Programming

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::{future::Future, path::Path, str::FromStr, time::SystemTime};

/// Functional I/O helper module - wraps mutable buffer operations in pure functions
mod io {
    use std::path::Path;

    use tokio::io::AsyncReadExt;

    /// Monadic wrapper for reading exact bytes from a file
    ///
    /// Encapsulates buffer mutation in a pure function that returns Result.
    /// This makes calling code fully functional with no exposed mutation.
    ///
    /// # Errors
    /// Returns IO error if file cannot be opened or read.
    pub async fn read_exact_bytes<const N: usize>(path: &Path) -> std::io::Result<[u8; N]> {
        // Mutation is encapsulated within this function scope
        let mut buffer = [0u8; N];
        let mut file = tokio::fs::File::open(path).await?;
        file.read_exact(&mut buffer).await?;
        Ok(buffer) // Return immutable buffer
    }
}

use isolate_core::{
    config::RecoveryConfig, log_recovery, should_log_recovery, Error, RecoveryPolicy, Result,
    WorkspaceState,
};
use num_traits::cast::ToPrimitive;
use sqlx::{Row, SqlitePool};

use crate::session::{validate_session_name, Session, SessionStatus, SessionUpdate};

const CURRENT_SCHEMA_VERSION: i64 = 1;
const SQLITE_BUSY_TIMEOUT_MS: i64 = 5000;
const SQLITE_WAL_AUTOCHECKPOINT_PAGES: i64 = 1000;
const SQLITE_BUSY_RETRY_ATTEMPTS: u32 = 8;
const SQLITE_BUSY_RETRY_BASE_MS: u64 = 25;
const REQUEST_FINGERPRINT_VERSION: &str = "v1";

/// Database schema as SQL string - executed once on init
const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY CHECK(version = 1)
);

CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
    state TEXT NOT NULL DEFAULT 'created' CHECK(state IN ('created', 'working', 'ready', 'merged', 'abandoned', 'conflict')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,
    metadata TEXT,
    removal_status TEXT DEFAULT NULL CHECK(removal_status IS NULL OR removal_status IN ('pending', 'failed', 'orphaned')),
    removal_error TEXT DEFAULT NULL,
    removal_attempted_at INTEGER DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS state_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    reason TEXT NOT NULL,
    agent_id TEXT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS processed_commands (
    command_id TEXT PRIMARY KEY,
    request_fingerprint TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS add_operation_journal (
    operation_id TEXT PRIMARY KEY,
    session_name TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    command_id TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending_external', 'compensating', 'done', 'failed_compensation')),
    last_error TEXT,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    registered_at TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    current_session TEXT,
    current_command TEXT,
    actions_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS broadcasts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    sent_to TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_add_operation_state ON add_operation_journal(state);

CREATE INDEX IF NOT EXISTS idx_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_state ON sessions(state);
CREATE INDEX IF NOT EXISTS idx_name ON sessions(name);
CREATE INDEX IF NOT EXISTS idx_transitions_session ON state_transitions(session_id);
CREATE INDEX IF NOT EXISTS idx_transitions_timestamp ON state_transitions(timestamp);
CREATE INDEX IF NOT EXISTS idx_agents_last_seen ON agents(last_seen);
CREATE INDEX IF NOT EXISTS idx_agents_session ON agents(current_session);
CREATE INDEX IF NOT EXISTS idx_broadcasts_timestamp ON broadcasts(timestamp);

CREATE TRIGGER IF NOT EXISTS update_timestamp
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;
";

/// Database wrapper for session storage with connection pooling
#[derive(Clone, Debug)]
pub struct SessionDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddOperationRecord {
    pub operation_id: String,
    pub session_name: String,
    pub workspace_path: String,
    pub command_id: Option<String>,
    pub state: String,
    pub last_error: Option<String>,
}

impl SessionDb {
    /// Get a reference to the underlying connection pool
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Open or create a session database at the given path
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if:
    /// - Database file cannot be opened
    /// - Schema initialization fails
    /// - Database is corrupted
    pub async fn open(path: &Path) -> Result<Self> {
        Self::open_internal(path, false).await
    }

    /// Create or open database (for init command only)
    pub async fn create_or_open(path: &Path) -> Result<Self> {
        Self::open_internal(path, true).await
    }

    async fn open_internal(path: &Path, allow_create: bool) -> Result<Self> {
        // Ensure parent directory exists for create operations
        if let Some(parent) = path.parent() {
            if !parent.exists() && allow_create {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    Error::IoError(format!("Failed to create parent directory: {e}"))
                })?;
            }
        }

        // Convert path to string for use in connection string
        let path_str = path.to_str().ok_or_else(|| {
            Error::DatabaseError("Database path contains invalid UTF-8".to_string())
        })?;

        // Check recovery policy for connection mode
        let policy = get_recovery_policy();
        let mode = if policy == RecoveryPolicy::FailFast {
            "rw" // No auto-create, fail on missing/corrupt
        } else {
            "rwc" // Auto-create and recover (existing behavior)
        };

        // Pre-flight check: Detect permission issues BEFORE SQLite tries to open
        // This prevents silent recovery from permission-denied scenarios
        if tokio::fs::try_exists(path).await.is_ok_and(|v| v) {
            use tokio::fs::File;
            if let Err(e) = File::open(path).await {
                return Err(Error::DatabaseError(format!(
                    "Database file is not accessible: {e}\n\n\
                     Run 'isolate doctor' to diagnose, or fix permissions manually:\n\
                     chmod 644 {p}",
                    p = path.display()
                )));
            }
        }

        // Load config to get recovery policy
        // We use a default config if loading fails to ensure we can still try to open DB
        let recovery_config = match isolate_core::config::load_config().await {
            Ok(config) => config.recovery,
            Err(_) => isolate_core::config::RecoveryConfig::default(),
        };

        // SQLx connection string with mode parameter
        let db_url = if path.is_absolute() {
            format!("sqlite:///{path_str}?mode={mode}")
        } else {
            format!("sqlite:{path_str}?mode={mode}")
        };

        // Combine pre-flight checks (functional AND composition)
        let preflight_result = async {
            check_wal_integrity(path, &recovery_config).await?;
            check_database_integrity(path, &recovery_config).await
        }
        .await;

        // Try to open database (with recovery on preflight or connection failure)
        let pool = match preflight_result {
            Ok(()) => {
                // Pre-flight checks passed, try normal connection
                match create_connection_pool(&db_url).await {
                    Ok(p) => p,
                    Err(e) => {
                        // Connection failed despite passing checks - try recovery
                        attempt_database_recovery(path, allow_create, &db_url, e, &recovery_config)
                            .await?
                    }
                }
            }
            Err(preflight_err) => {
                // Pre-flight checks failed (corruption detected)
                // Attempt recovery (Railway pattern: error track → recovery → success track)
                attempt_database_recovery(
                    path,
                    allow_create,
                    &db_url,
                    preflight_err,
                    &recovery_config,
                )
                .await?
            }
        };

        // Configure WAL mode once for this process/pool.
        enable_wal_mode(&pool).await?;

        // Try to initialize schema, with recovery for corrupted databases
        match init_schema(&pool).await {
            Ok(()) => {
                check_schema_version(&pool).await?;
                // Initialize lock manager tables (CRIT-001 fix)
                let lock_mgr = isolate_core::coordination::locks::LockManager::new(pool.clone());
                lock_mgr.init().await?;
                Ok(Self { pool })
            }
            Err(e) => {
                // Schema init failed - likely corrupted database
                match can_recover_database(path, allow_create).await {
                    Ok(()) => {
                        recover_database(path, &recovery_config).await?;
                        let new_pool = create_connection_pool(&db_url).await?;
                        init_schema(&new_pool).await?;
                        check_schema_version(&new_pool).await?;
                        // Initialize lock manager tables (CRIT-001 fix)
                        let lock_mgr =
                            isolate_core::coordination::locks::LockManager::new(new_pool.clone());
                        lock_mgr.init().await?;
                        Ok(Self { pool: new_pool })
                    }
                    Err(recovery_err) => Err(Error::DatabaseError(format!(
                        "{e}\n\nRecovery check failed: {recovery_err}"
                    ))),
                }
            }
        }
    }

    /// Create a new session with atomic conflict detection
    pub async fn create(&self, name: &str, workspace_path: &str) -> Result<Session> {
        validate_session_name(name)?;

        let max_sessions = get_max_sessions().await;
        check_session_limit(&self.pool, max_sessions).await?;

        let now = get_current_timestamp()?;
        let status = SessionStatus::Creating;
        let state = WorkspaceState::Created;

        let id_opt = insert_session(&self.pool, name, &status, workspace_path, now).await?;

        match id_opt {
            Some(id) => Ok(Session {
                id: Some(id),
                name: name.to_string(),
                status,
                state,
                workspace_path: workspace_path.to_string(),
                branch: None,
                created_at: now,
                updated_at: now,
                last_synced: None,
                metadata: None,
            }),
            None => match query_session_by_name(&self.pool, name).await? {
                Some(existing) => {
                    if existing.workspace_path == workspace_path {
                        Err(Error::DatabaseError(format!(
                            "Session '{name}' already exists"
                        )))
                    } else {
                        Err(Error::DatabaseError(format!(
                            "Session '{name}' already exists with different workspace path.\n\
                                 Existing: {}\n\
                                 Requested: {workspace_path}",
                            existing.workspace_path
                        )))
                    }
                }
                None => Err(Error::DatabaseError(format!(
                    "Session '{name}' creation conflict detected but session not found"
                ))),
            },
        }
    }

    /// Create a new session with optional command idempotency key.
    #[allow(dead_code)]
    pub async fn create_with_command_id(
        &self,
        name: &str,
        workspace_path: &str,
        command_id: Option<&str>,
    ) -> Result<Session> {
        if command_id.is_none() {
            return self.create(name, workspace_path).await;
        }

        validate_session_name(name)?;
        let now = get_current_timestamp()?;

        let mut conn = self.pool.acquire().await.map_err(|e| {
            Error::DatabaseError(format!("Failed to acquire database connection: {e}"))
        })?;

        begin_immediate_with_retry(&mut conn, "create transaction").await?;

        let Some(command_id_value) = command_id else {
            rollback_best_effort(&mut conn).await;
            return Err(Error::DatabaseError("Missing command id".to_string()));
        };

        if is_command_processed_conn(&mut conn, command_id_value).await? {
            let existing = query_session_by_name_conn(&mut conn, name).await?;
            commit_with_retry(&mut conn, "replay transaction").await?;

            return existing.ok_or_else(|| {
                Error::DatabaseError(format!(
                    "Command {command_id_value} already processed but session '{name}' is missing"
                ))
            });
        }

        let insert_result = sqlx::query(
            "INSERT INTO sessions (name, status, state, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(name) DO NOTHING",
        )
        .bind(name)
        .bind(SessionStatus::Creating.to_string())
        .bind(WorkspaceState::Created.to_string())
        .bind(workspace_path)
        .bind(now.to_i64().map_or(i64::MAX, |t| t))
        .bind(now.to_i64().map_or(i64::MAX, |t| t))
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create session: {e}")))?;

        let session = if insert_result.rows_affected() > 0 {
            Session {
                id: Some(insert_result.last_insert_rowid()),
                name: name.to_string(),
                status: SessionStatus::Creating,
                state: WorkspaceState::Created,
                workspace_path: workspace_path.to_string(),
                branch: None,
                created_at: now,
                updated_at: now,
                last_synced: None,
                metadata: None,
            }
        } else {
            let existing = query_session_by_name_conn(&mut conn, name).await?;
            let existing_session = existing.ok_or_else(|| {
                Error::DatabaseError(format!(
                    "Session '{name}' already exists but could not be loaded"
                ))
            })?;
            if existing_session.workspace_path != workspace_path {
                rollback_best_effort(&mut conn).await;
                return Err(Error::DatabaseError(format!(
                    "Session '{name}' already exists with different workspace path"
                )));
            }
            existing_session
        };

        mark_command_processed_conn(&mut conn, command_id_value, None).await?;

        commit_with_retry(&mut conn, "create transaction").await?;

        Ok(session)
    }

    /// Create a session with a specific creation timestamp
    pub async fn create_with_timestamp(
        &self,
        name: &str,
        workspace_path: &str,
        created_at: u64,
    ) -> Result<Session> {
        validate_session_name(name)?;

        let status = SessionStatus::Creating;
        let state = WorkspaceState::Created;

        insert_session(&self.pool, name, &status, workspace_path, created_at)
            .await
            .map(|id| Session {
                id,
                name: name.to_string(),
                status,
                state,
                workspace_path: workspace_path.to_string(),
                branch: None,
                created_at,
                updated_at: created_at,
                last_synced: None,
                metadata: None,
            })
    }

    /// Get a session by name
    pub async fn get(&self, name: &str) -> Result<Option<Session>> {
        query_session_by_name(&self.pool, name).await
    }

    /// Update an existing session
    pub async fn update(&self, name: &str, update: SessionUpdate) -> Result<()> {
        self.update_with_command_id(name, update, None).await
    }

    /// Update the workspace path for an existing session
    pub async fn update_workspace_path(&self, name: &str, workspace_path: &str) -> Result<()> {
        let pool = self.pool.clone();
        let session_name = name.to_string();
        let workspace_path = workspace_path.to_string();

        let rows_affected = run_with_sqlite_busy_retry("update workspace path", move || {
            let pool = pool.clone();
            let workspace_path = workspace_path.clone();
            let session_name = session_name.clone();
            async move {
                sqlx::query("UPDATE sessions SET workspace_path = ? WHERE name = ?")
                    .bind(workspace_path)
                    .bind(session_name)
                    .execute(&pool)
                    .await
            }
        })
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Session '{name}' not found")));
        }

        Ok(())
    }

    /// Update an existing session with optional command idempotency key.
    #[allow(dead_code)]
    pub async fn update_with_command_id(
        &self,
        name: &str,
        update: SessionUpdate,
        command_id: Option<&str>,
    ) -> Result<()> {
        if command_id.is_none() {
            return update_session(&self.pool, name, update).await;
        }

        let mut conn = self.pool.acquire().await.map_err(|e| {
            Error::DatabaseError(format!("Failed to acquire database connection: {e}"))
        })?;

        begin_immediate_with_retry(&mut conn, "update transaction").await?;

        let Some(command_id_value) = command_id else {
            rollback_best_effort(&mut conn).await;
            return Err(Error::DatabaseError("Missing command id".to_string()));
        };

        if command_id_value.trim().is_empty() {
            rollback_best_effort(&mut conn).await;
            return Err(Error::ValidationError {
                message: "Command id cannot be empty".to_string(),
                field: None,
                value: None,
                constraints: Vec::new(),
            });
        }

        let request_fingerprint = update_request_fingerprint(name, &update)?;

        if let Some(existing_fingerprint) =
            query_command_fingerprint_conn(&mut conn, command_id_value).await?
        {
            if existing_fingerprint.as_ref().is_some_and(|stored| {
                normalize_fingerprint_for_compare(stored) != request_fingerprint
            }) {
                rollback_best_effort(&mut conn).await;
                return Err(Error::DatabaseError(format!(
                    "Command {command_id_value} already processed with a different payload"
                )));
            }

            commit_with_retry(&mut conn, "replay transaction").await?;
            return Ok(());
        }

        let has_updates = update.status.is_some()
            || update.state.is_some()
            || update.branch.is_some()
            || update.last_synced.is_some()
            || update.metadata.is_some();

        if !has_updates {
            if query_session_by_name_conn(&mut conn, name).await?.is_none() {
                rollback_best_effort(&mut conn).await;
                return Err(Error::NotFound(format!("Session '{name}' not found")));
            }

            mark_command_processed_conn(
                &mut conn,
                command_id_value,
                Some(request_fingerprint.as_str()),
            )
            .await?;
            commit_with_retry(&mut conn, "update transaction").await?;
            return Ok(());
        }

        let mut query_builder = sqlx::QueryBuilder::new("UPDATE sessions SET ");
        let mut separated = query_builder.separated(", ");

        if let Some(ref status) = update.status {
            separated.push("status = ");
            separated.push_bind_unseparated(status.to_string());
        }
        if let Some(ref state) = update.state {
            separated.push("state = ");
            separated.push_bind_unseparated(state.to_string());
        }
        if let Some(ref branch) = update.branch {
            separated.push("branch = ");
            separated.push_bind_unseparated(branch);
        }
        if let Some(ls) = update.last_synced {
            separated.push("last_synced = ");
            separated.push_bind_unseparated(ls.to_i64().map_or(i64::MAX, |t| t));
        }
        if let Some(ref m) = update.metadata {
            separated.push("metadata = ");
            separated.push_bind_unseparated(
                serde_json::to_string(m).map_err(|e| Error::Unknown(e.to_string()))?,
            );
        }

        query_builder.push(" WHERE name = ");
        query_builder.push_bind(name);

        let result = query_builder
            .build()
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update session: {e}")))?;

        if result.rows_affected() == 0 {
            rollback_best_effort(&mut conn).await;
            return Err(Error::NotFound(format!("Session '{name}' not found")));
        }

        mark_command_processed_conn(
            &mut conn,
            command_id_value,
            Some(request_fingerprint.as_str()),
        )
        .await?;
        commit_with_retry(&mut conn, "update transaction").await?;

        Ok(())
    }

    /// Delete a session by name
    pub async fn delete(&self, name: &str) -> Result<bool> {
        delete_session(&self.pool, name).await
    }

    /// Rename a session
    pub async fn rename(&self, old_name: &str, new_name: &str) -> Result<Session> {
        validate_session_name(new_name)?;

        let _session = self
            .get(old_name)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session '{old_name}' not found")))?;

        if self.get(new_name).await?.is_some() {
            return Err(Error::ValidationError {
                message: format!("Session '{new_name}' already exists"),
                field: Some("name".to_string()),
                value: Some(new_name.to_string()),
                constraints: vec!["must be unique".to_string()],
            });
        }

        let pool = self.pool.clone();
        let old = old_name.to_string();
        let new = new_name.to_string();

        run_with_sqlite_busy_retry("rename session", move || {
            let pool = pool.clone();
            let old = old.clone();
            let new = new.clone();
            async move {
                sqlx::query("UPDATE sessions SET name = ? WHERE name = ?")
                    .bind(&new)
                    .bind(&old)
                    .execute(&pool)
                    .await
            }
        })
        .await?;

        self.get(new_name)
            .await?
            .ok_or_else(|| Error::DatabaseError("Session not found after rename".to_string()))
    }

    /// List all sessions, optionally filtered by status
    pub async fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        query_sessions(&self.pool, status_filter).await
    }

    /// Mark a session as failed removal
    pub async fn mark_removal_failed(&self, name: &str, error: &str) -> Result<()> {
        let now = get_current_timestamp()?;
        run_with_sqlite_busy_retry("mark removal as failed", || async {
            sqlx::query(
                "UPDATE sessions
                 SET removal_status = 'failed',
                     removal_error = ?,
                     removal_attempted_at = ?
                 WHERE name = ?",
            )
            .bind(error)
            .bind(now.to_i64().map_or(i64::MAX, |t| t))
            .bind(name)
            .execute(&self.pool)
            .await
        })
        .await
        .map(|_| ())
    }

    /// Find orphaned workspaces
    #[allow(dead_code)]
    pub async fn find_orphaned_sessions(&self) -> Result<Vec<String>> {
        use futures::stream::{self, StreamExt, TryStreamExt};

        let sessions = self.list(None).await?;

        stream::iter(sessions.into_iter().map(Ok))
            .filter_map(|session_res: Result<Session>| async move {
                match session_res {
                    Ok(session) => {
                        let workspace_path = std::path::Path::new(&session.workspace_path);
                        (!tokio::fs::try_exists(workspace_path).await.is_ok_and(|v| v))
                            .then_some(Ok(session.name))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .try_collect::<Vec<_>>()
            .await
    }

    /// Cleanup orphaned sessions
    #[allow(dead_code)]
    pub async fn cleanup_orphaned_sessions(&self) -> Result<usize> {
        use futures::stream::{self, StreamExt, TryStreamExt};

        let orphans = self.find_orphaned_sessions().await?;

        stream::iter(orphans.into_iter().map(Ok))
            .map(|orphan_name_res: Result<String>| async move {
                let orphan_name = orphan_name_res?;
                self.delete(&orphan_name).await
            })
            .buffered(5)
            .try_collect::<Vec<bool>>()
            .await
            .map(|results| results.into_iter().filter(|&deleted| deleted).count())
    }

    /// Check whether a command id was already processed.
    pub async fn is_command_processed(&self, command_id: &str) -> Result<bool> {
        is_command_processed_pool(&self.pool, command_id).await
    }

    /// Remove an idempotency marker
    pub async fn unmark_command_processed(&self, command_id: &str) -> Result<()> {
        run_with_sqlite_busy_retry("unmark processed command", || async {
            sqlx::query("DELETE FROM processed_commands WHERE command_id = ?")
                .bind(command_id)
                .execute(&self.pool)
                .await
        })
        .await
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_add_operation_journal(
        &self,
        operation_id: &str,
        session_name: &str,
        workspace_path: &str,
        command_id: Option<&str>,
        state: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        run_with_sqlite_busy_retry("upsert add operation journal", || async {
            sqlx::query(
                "INSERT INTO add_operation_journal (operation_id, session_name, workspace_path, command_id, state, last_error, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, strftime('%s', 'now'))
                 ON CONFLICT(operation_id) DO UPDATE SET
                    session_name = excluded.session_name,
                    workspace_path = excluded.workspace_path,
                    command_id = excluded.command_id,
                    state = excluded.state,
                    last_error = excluded.last_error,
                    updated_at = strftime('%s', 'now')",
            )
            .bind(operation_id)
            .bind(session_name)
            .bind(workspace_path)
            .bind(command_id)
            .bind(state)
            .bind(last_error)
            .execute(&self.pool)
            .await
        })
        .await
        .map(|_| ())
    }

    pub async fn list_incomplete_add_operations(&self) -> Result<Vec<AddOperationRecord>> {
        sqlx::query(
            "SELECT operation_id, session_name, workspace_path, command_id, state, last_error
             FROM add_operation_journal
             WHERE state IN ('pending_external', 'compensating', 'failed_compensation')
             ORDER BY updated_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to query add operation journal: {e}")))
        .and_then(|rows| {
            rows.into_iter()
                .map(|row| {
                    Ok(AddOperationRecord {
                        operation_id: row.try_get("operation_id").map_err(|e| {
                            Error::DatabaseError(format!(
                                "Failed to parse add journal operation_id: {e}"
                            ))
                        })?,
                        session_name: row.try_get("session_name").map_err(|e| {
                            Error::DatabaseError(format!(
                                "Failed to parse add journal session_name: {e}"
                            ))
                        })?,
                        workspace_path: row.try_get("workspace_path").map_err(|e| {
                            Error::DatabaseError(format!(
                                "Failed to parse add journal workspace_path: {e}"
                            ))
                        })?,
                        command_id: row.try_get("command_id").map_err(|e| {
                            Error::DatabaseError(format!(
                                "Failed to parse add journal command_id: {e}"
                            ))
                        })?,
                        state: row.try_get("state").map_err(|e| {
                            Error::DatabaseError(format!("Failed to parse add journal state: {e}"))
                        })?,
                        last_error: row.try_get("last_error").map_err(|e| {
                            Error::DatabaseError(format!(
                                "Failed to parse add journal last_error: {e}"
                            ))
                        })?,
                    })
                })
                .collect()
        })
    }
}

/// Get the current recovery policy from environment variables
fn get_recovery_policy() -> RecoveryPolicy {
    if std::env::var("Isolate_STRICT").is_ok() {
        return RecoveryPolicy::FailFast;
    }

    if let Ok(policy_str) = std::env::var("Isolate_RECOVERY_POLICY") {
        return policy_str.parse().map_or(RecoveryPolicy::Warn, |p| p);
    }

    RecoveryPolicy::Warn
}

async fn can_recover_database(path: &Path, allow_create: bool) -> Result<()> {
    if tokio::fs::try_exists(path).await.is_ok_and(|v| v) {
        match tokio::fs::metadata(path).await {
            Ok(_) => {
                use tokio::fs::File;
                match File::open(path).await {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(Error::DatabaseError(format!(
                            "Database file is not accessible: {e}",
                        )));
                    }
                }
            }
            Err(e) => {
                return Err(Error::DatabaseError(format!(
                    "Cannot access database metadata: {e}"
                )));
            }
        }
    }

    if let Some(parent) = path.parent() {
        if tokio::fs::try_exists(parent).await.is_ok_and(|v| v) {
            return Ok(());
        }
    }

    if allow_create {
        Ok(())
    } else {
        Err(Error::DatabaseError(format!(
            "Database file does not exist: {}\n\nRun 'isolate init' to initialize.",
            path.display()
        )))
    }
}

async fn check_wal_integrity(db_path: &Path, config: &RecoveryConfig) -> Result<()> {
    let wal_path = db_path.with_extension("db-wal");

    if !tokio::fs::try_exists(&wal_path).await.is_ok_and(|v| v) {
        return Ok(());
    }

    let Ok(wal_metadata) = tokio::fs::metadata(&wal_path).await else {
        return Ok(());
    };
    if wal_metadata.len() == 0 {
        return Ok(());
    }

    let header = match io::read_exact_bytes::<32>(&wal_path).await {
        Ok(h) => h,
        Err(e) => match config.policy {
            RecoveryPolicy::FailFast => {
                log_recovery(
                    &format!(
                        "WAL file inaccessible or corrupted: {p}. Error: {e}",
                        p = wal_path.display()
                    ),
                    config,
                )
                .await
                .ok();
                return Err(Error::DatabaseError(format!(
                    "WAL file is corrupted or inaccessible: {p}\n\
                         Recovery logged. Run 'isolate doctor' for details.",
                    p = wal_path.display()
                )));
            }
            RecoveryPolicy::Warn | RecoveryPolicy::Silent => {
                if should_log_recovery(config) {
                    log_recovery(
                        &format!(
                            "WAL file temporarily unreadable: {p}. Deferring to SQLite: {e}",
                            p = wal_path.display()
                        ),
                        config,
                    )
                    .await
                    .ok();
                }
                return Ok(());
            }
        },
    };

    let wal_magic = header
        .get(0..4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .map(u32::from_be_bytes)
        .map_or(0, |m| m);

    if wal_magic != 0x377f_0682 {
        let policy = config.policy;
        let should_log = should_log_recovery(config);

        match policy {
            RecoveryPolicy::FailFast => {
                return Err(Error::DatabaseError(format!(
                    "WAL file corrupted: {p}\n\
                     Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682\n\n\
                     Recovery is disabled in strict mode (--strict or Isolate_STRICT=1).\n\n\
                     To recover, either:\n\
                     - Remove --strict flag\n\
                     - Run 'isolate doctor --fix'\n\
                     - Manually delete WAL file: rm {p}",
                    p = wal_path.display()
                )));
            }
            RecoveryPolicy::Warn => {
                eprintln!("⚠  WAL file corrupted: {p}", p = wal_path.display());
                eprintln!("   Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682");
                eprintln!("   SQLite will attempt automatic recovery...");

                if should_log {
                    log_recovery(&format!(
                        "WAL file corrupted: {p}. Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682. SQLite will recover automatically.",
                        p = wal_path.display()
                    ), config).await
                    .ok();
                }
            }
            RecoveryPolicy::Silent => {
                if should_log {
                    log_recovery(&format!(
                        "WAL file corrupted: {p}. Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682. SQLite recovered silently.",
                        p = wal_path.display()
                    ), config).await
                    .ok();
                }
            }
        }
    }

    Ok(())
}

async fn check_database_integrity(
    db_path: &Path,
    config: &isolate_core::config::RecoveryConfig,
) -> Result<()> {
    if !tokio::fs::try_exists(db_path).await.is_ok_and(|v| v) {
        return Ok(());
    }

    let file_size = tokio::fs::metadata(db_path)
        .await
        .map(|m| m.len())
        .map_or(0, |s| s);

    if file_size < 100 {
        log_recovery(
            &format!("Database file too small: {file_size} bytes. Expected at least 100 bytes."),
            config,
        )
        .await
        .ok();
        return Err(Error::DatabaseError(format!(
            "Database file is too small to be valid: {file_size} bytes\n\
             Expected at least 100 bytes. File may be corrupted.\n\
             Recovery logged. Run 'isolate doctor' for details."
        )));
    }

    let header = match io::read_exact_bytes::<16>(db_path).await {
        Ok(h) => h,
        Err(e) => {
            log_recovery(
                &format!(
                    "Database file inaccessible or corrupted: {p}. Error: {e}",
                    p = db_path.display()
                ),
                config,
            )
            .await
            .ok();
            return Err(Error::DatabaseError(format!(
                "Database file is corrupted or inaccessible: {p}\n\
                 Recovery logged. Run 'isolate doctor' for details.",
                p = db_path.display()
            )));
        }
    };

    let expected_magic: &[u8] = &[
        b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ', b'3',
        0x00,
    ];

    let header_prefix = match header.get(0..16) {
        Some(slice) => slice,
        None => &[],
    };
    if header_prefix != expected_magic {
        let policy = config.policy;
        let should_log = should_log_recovery(config);

        let magic_hex: String = header
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");

        match policy {
            RecoveryPolicy::FailFast => {
                return Err(Error::DatabaseError(format!(
                    "Database file corrupted: {p}\n\
                     Magic bytes (hex): {magic_hex}\n\
                     Expected: 53 51 4c 69 74 65 20 66 6f 72 6d 61 74 20 33 00 (SQLite format 3)\n\n\
                     Recovery is disabled in strict mode (--strict or Isolate_STRICT=1).\n\n\
                     To recover, either:\n\
                     - Remove --strict flag\n\
                     - Run 'isolate doctor --fix'\n\
                     - Manually delete database and run 'isolate init'",
                    p = db_path.display()
                )));
            }
            RecoveryPolicy::Warn => {
                eprintln!("⚠  Database file corrupted: {p}", p = db_path.display());
                eprintln!("   Magic bytes (hex): {magic_hex}");
                eprintln!("   Expected: 53 51 4c 69 74 65 20 66 6f 72 6d 61 74 20 33 00 (SQLite format 3)");

                return Err(isolate_core::Error::DatabaseError(format!(
                    "Database file corrupted: {p}\n\n\
                     To prevent data loss, automatic recovery is disabled by default.\n\
                     To recover, please run:\n\
                     isolate doctor --fix\n\n\
                     Or manually delete the database and run 'isolate init':\n\
                     rm {p} && isolate init",
                    p = db_path.display()
                )));
            }
            RecoveryPolicy::Silent => {
                if should_log {
                    log_recovery(&format!(
                        "Database file corrupted: {p}. Magic bytes: {magic_hex}. SQLite recovered silently.",
                        p = db_path.display()
                    ), config).await
                    .ok();
                }
            }
        }
    }

    Ok(())
}

async fn recover_database(
    path: &Path,
    config: &isolate_core::config::RecoveryConfig,
) -> Result<()> {
    let policy = config.policy;
    let should_log = should_log_recovery(config);

    match policy {
        RecoveryPolicy::FailFast => {
            return Err(Error::DatabaseError(format!(
                "Database corruption detected: {p}\n\n\
                 Recovery is disabled in strict mode (--strict or Isolate_STRICT=1).\n\n\
                 To recover, either:\n\
                 - Remove --strict flag\n\
                 - Run 'isolate integrity repair' if applicable\n\
                 - Manually delete the database and run 'isolate init'",
                p = path.display()
            )));
        }
        RecoveryPolicy::Warn => {
            eprintln!("⚠  Database corruption detected: {p}", p = path.display());
            return Err(isolate_core::Error::DatabaseError(format!(
                "Database corruption detected: {p}\n\n\
                 To prevent data loss, automatic recovery is disabled by default.\n\
                 To recover, please run:\n\
                 isolate doctor --fix\n\n\
                 Or manually delete the database and run 'isolate init':\n\
                 rm {p} && isolate init",
                p = path.display()
            )));
        }
        RecoveryPolicy::Silent => {
            if should_log {
                let log_msg = format!(
                    "Database corruption detected at: {p}. Recovered silently.",
                    p = path.display()
                );
                log_recovery(&log_msg, config).await.ok();
            }
        }
    }

    if tokio::fs::try_exists(path).await.is_ok_and(|v| v) {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {}
            Err(e) => {
                log_recovery(
                    &format!(
                        "Failed to remove corrupted database {p}: {e}",
                        p = path.display()
                    ),
                    config,
                )
                .await
                .ok();
                return Err(Error::IoError(format!(
                    "Failed to remove corrupted database: {e}"
                )));
            }
        }
    }

    Ok(())
}

fn get_current_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| Error::Unknown(format!("System time error: {e}")))
}

async fn get_max_sessions() -> usize {
    isolate_core::config::load_config()
        .await
        .map(|config| config.session.max_sessions)
        .unwrap_or(100)
}

async fn check_session_limit(pool: &SqlitePool, max_sessions: usize) -> Result<()> {
    let count: i64 = sqlx::query("SELECT COUNT(*) as count FROM sessions")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to count sessions: {e}")))
        .and_then(|row| {
            row.try_get("count")
                .map_err(|e| Error::DatabaseError(format!("Failed to parse session count: {e}")))
        })?;

    let count_usize = count
        .to_usize()
        .ok_or_else(|| Error::DatabaseError("Session count overflow".to_string()))?;

    if count_usize >= max_sessions {
        return Err(Error::ValidationError {
            message: format!(
                "Cannot create session: maximum limit of {max_sessions} sessions reached\n
                 Current sessions: {count_usize}\n

                 \

                 To increase the limit, set 'session.max_sessions' in your config file.\n

                 Example:\n


                 [session]\n
                 max_sessions = 200"
            ),
            field: Some("max_sessions".to_string()),
            value: Some(count_usize.to_string()),
            constraints: vec![format!("max {}", max_sessions)],
        });
    }

    Ok(())
}

async fn attempt_database_recovery(
    path: &Path,
    allow_create: bool,
    db_url: &str,
    original_error: Error,
    config: &isolate_core::config::RecoveryConfig,
) -> Result<SqlitePool> {
    if let Err(recovery_err) = can_recover_database(path, allow_create).await {
        return Err(Error::DatabaseError(format!(
            "{original_error}\n\nRecovery check failed: {recovery_err}"
        )));
    }

    recover_database(path, config)
        .await
        .map_err(|recovery_err| {
            Error::DatabaseError(format!(
                "{original_error}\n\nRecovery failed: {recovery_err}"
            ))
        })?;

    create_connection_pool(db_url).await
}

async fn create_connection_pool(db_url: &str) -> Result<SqlitePool> {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(Some(std::time::Duration::from_mins(10)))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query(&format!("PRAGMA busy_timeout = {SQLITE_BUSY_TIMEOUT_MS};"))
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA foreign_keys = ON;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA synchronous = NORMAL;")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(db_url)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to connect to database: {e}")))
}

async fn enable_wal_mode(pool: &SqlitePool) -> Result<()> {
    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout;")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to verify busy_timeout: {e}")))?;
    if busy_timeout < SQLITE_BUSY_TIMEOUT_MS {
        return Err(Error::DatabaseError(format!(
            "busy_timeout too low: expected at least {SQLITE_BUSY_TIMEOUT_MS}, got {busy_timeout}"
        )));
    }

    let current_mode: String = sqlx::query_scalar("PRAGMA journal_mode;")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to query current journal_mode: {e}")))?;

    if current_mode.eq_ignore_ascii_case("wal") {
        let auto_checkpoint: i64 = sqlx::query_scalar("PRAGMA wal_autocheckpoint;")
            .fetch_one(pool)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!("Failed to verify wal_autocheckpoint: {e}"))
            })?;
        if auto_checkpoint < SQLITE_WAL_AUTOCHECKPOINT_PAGES {
            return Err(Error::DatabaseError(format!(
                "wal_autocheckpoint too low: expected at least {SQLITE_WAL_AUTOCHECKPOINT_PAGES}, got {auto_checkpoint}"
            )));
        }
        return Ok(());
    }

    let mode: String = run_with_sqlite_busy_retry("enable WAL mode", || async {
        sqlx::query_scalar("PRAGMA journal_mode=WAL;")
            .fetch_one(pool)
            .await
    })
    .await?;

    if !mode.eq_ignore_ascii_case("wal") {
        return Err(Error::DatabaseError(format!(
            "Failed to set journal_mode to WAL (actual: {mode})"
        )));
    }

    run_with_sqlite_busy_retry("set wal_autocheckpoint", || async {
        sqlx::query(&format!(
            "PRAGMA wal_autocheckpoint = {SQLITE_WAL_AUTOCHECKPOINT_PAGES};"
        ))
        .execute(pool)
        .await
    })
    .await?;

    let auto_checkpoint: i64 = sqlx::query_scalar("PRAGMA wal_autocheckpoint;")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to verify wal_autocheckpoint: {e}")))?;
    if auto_checkpoint < SQLITE_WAL_AUTOCHECKPOINT_PAGES {
        return Err(Error::DatabaseError(format!(
            "wal_autocheckpoint too low: expected at least {SQLITE_WAL_AUTOCHECKPOINT_PAGES}, got {auto_checkpoint}"
        )));
    }

    Ok(())
}

async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to initialize schema: {e}")))?;

    ensure_processed_commands_schema(pool).await?;

    sqlx::query("INSERT OR IGNORE INTO schema_version (version) VALUES (?)")
        .bind(CURRENT_SCHEMA_VERSION)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to set schema version: {e}")))?;

    Ok(())
}

async fn ensure_processed_commands_schema(pool: &SqlitePool) -> Result<()> {
    let has_request_fingerprint = sqlx::query("PRAGMA table_info(processed_commands)")
        .fetch_all(pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to inspect processed_commands schema: {e}"))
        })?
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .any(|column| column == "request_fingerprint");

    if !has_request_fingerprint {
        sqlx::query("ALTER TABLE processed_commands ADD COLUMN request_fingerprint TEXT")
            .execute(pool)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!("Failed to migrate processed_commands schema: {e}"))
            })?;
    }

    run_with_sqlite_busy_retry("migrate request fingerprint versions", || async {
        sqlx::query(
            "UPDATE processed_commands
             SET request_fingerprint = ? || request_fingerprint
             WHERE request_fingerprint IS NOT NULL
               AND request_fingerprint <> ''
               AND request_fingerprint NOT GLOB 'v[0-9]*:*'",
        )
        .bind(format!("{REQUEST_FINGERPRINT_VERSION}:"))
        .execute(pool)
        .await
    })
    .await?;

    Ok(())
}

async fn check_schema_version(pool: &SqlitePool) -> Result<()> {
    let version: Option<i64> = sqlx::query("SELECT version FROM schema_version")
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to read schema version: {e}")))?
        .map(|row| {
            row.try_get("version")
                .map_err(|e| Error::DatabaseError(format!("Failed to parse schema version: {e}")))
        })
        .transpose()?;

    match version {
        Some(v) if v == CURRENT_SCHEMA_VERSION => Ok(()),
        Some(v) => Err(Error::DatabaseError(format!(
            "Schema version mismatch: database has version {v}, but isolate expects version {CURRENT_SCHEMA_VERSION}\n\n\
             The database may have been created by a different version of isolate.\n\n\
             To reset: rm .isolate/state.db && isolate init"
        ))),
        None => Err(Error::DatabaseError("Schema version not found in database. The database may be corrupted.\n\n\
             To reset: rm .isolate/state.db && isolate init".to_string())),
    }
}

async fn insert_session(
    pool: &SqlitePool,
    name: &str,
    status: &SessionStatus,
    workspace_path: &str,
    timestamp: u64,
) -> Result<Option<i64>> {
    let result = run_with_sqlite_busy_retry("create session", || async {
        sqlx::query(
            "INSERT INTO sessions (name, status, state, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(name) DO NOTHING",
        )
        .bind(name)
        .bind(status.to_string())
        .bind(WorkspaceState::Created.to_string())
        .bind(workspace_path)
        .bind(timestamp.to_i64().map_or(i64::MAX, |t| t))
        .bind(timestamp.to_i64().map_or(i64::MAX, |t| t))
        .execute(pool)
        .await
    })
    .await?;

    Ok(if result.rows_affected() > 0 {
        Some(result.last_insert_rowid())
    } else {
        None
    })
}

fn sqlite_busy_backoff_duration(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_millis(
        SQLITE_BUSY_RETRY_BASE_MS * u64::from(attempt.saturating_add(1)),
    )
}

fn is_sqlite_busy_or_locked_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("sqlite_busy")
        || lower.contains("sqlite_locked")
        || lower.contains("database is locked")
        || lower.contains("database table is locked")
}

fn is_sqlite_busy_or_locked_error(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            let code_match = db_error.code().is_some_and(|code| {
                code == "5" || code == "6" || code == "SQLITE_BUSY" || code == "SQLITE_LOCKED"
            });
            code_match || is_sqlite_busy_or_locked_message(db_error.message())
        }
        _ => is_sqlite_busy_or_locked_message(&error.to_string()),
    }
}

async fn run_with_sqlite_busy_retry<T, F, Fut>(context: &str, operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, sqlx::Error>>,
{
    run_op_with_retry(operation, context).await
}

async fn run_op_with_retry<T, F, Fut>(mut op: F, context: &str) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, sqlx::Error>>,
{
    for attempt in 0..=SQLITE_BUSY_RETRY_ATTEMPTS {
        match op().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if is_sqlite_busy_or_locked_error(&error) && attempt < SQLITE_BUSY_RETRY_ATTEMPTS {
                    tokio::time::sleep(sqlite_busy_backoff_duration(attempt)).await;
                    continue;
                }

                return Err(Error::DatabaseError(format!(
                    "Failed to {context}: {error}"
                )));
            }
        }
    }

    Err(Error::DatabaseError(format!(
        "Failed to {context} after retry budget"
    )))
}

async fn begin_immediate_with_retry(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    context: &str,
) -> Result<()> {
    for attempt in 0..=SQLITE_BUSY_RETRY_ATTEMPTS {
        match sqlx::query("BEGIN IMMEDIATE").execute(&mut **conn).await {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_sqlite_busy_or_locked_error(&error) && attempt < SQLITE_BUSY_RETRY_ATTEMPTS {
                    tokio::time::sleep(sqlite_busy_backoff_duration(attempt)).await;
                    continue;
                }

                return Err(Error::DatabaseError(format!(
                    "Failed to begin {context}: {error}"
                )));
            }
        }
    }

    Err(Error::DatabaseError(format!(
        "Failed to begin {context} after retry budget"
    )))
}

async fn commit_with_retry(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    context: &str,
) -> Result<()> {
    for attempt in 0..=SQLITE_BUSY_RETRY_ATTEMPTS {
        match sqlx::query("COMMIT").execute(&mut **conn).await {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_sqlite_busy_or_locked_error(&error) && attempt < SQLITE_BUSY_RETRY_ATTEMPTS {
                    tokio::time::sleep(sqlite_busy_backoff_duration(attempt)).await;
                    continue;
                }

                return Err(Error::DatabaseError(format!(
                    "Failed to commit {context}: {error}"
                )));
            }
        }
    }

    Err(Error::DatabaseError(format!(
        "Failed to commit {context} after retry budget"
    )))
}

async fn rollback_best_effort(conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>) {
    for attempt in 0..=SQLITE_BUSY_RETRY_ATTEMPTS {
        if sqlx::query("ROLLBACK").execute(&mut **conn).await.is_ok() {
            return;
        }
        tokio::time::sleep(sqlite_busy_backoff_duration(attempt)).await;
    }
}

async fn query_session_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Session>> {
    sqlx::query(
        "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
         FROM sessions WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to query session: {e}")))
    .and_then(|opt_row| opt_row.map(parse_session_row).transpose())
}

async fn query_sessions(
    pool: &SqlitePool,
    status_filter: Option<SessionStatus>,
) -> Result<Vec<Session>> {
    let rows = match status_filter {
        Some(status) => {
            sqlx::query(
                "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE status = ? ORDER BY created_at"
            )
            .bind(status.to_string())
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions ORDER BY created_at"
            )
            .fetch_all(pool)
            .await
        }
    }.map_err(|e| Error::DatabaseError(format!("Failed to query sessions: {e}")))?;

    rows.into_iter().map(parse_session_row).collect()
}

#[allow(clippy::needless_pass_by_value)]
fn parse_session_row(row: sqlx::sqlite::SqliteRow) -> Result<Session> {
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::DatabaseError(format!("Failed to read id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| Error::DatabaseError(format!("Failed to read name: {e}")))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| Error::DatabaseError(format!("Failed to read status: {e}")))?;
    let status = SessionStatus::from_str(&status_str)?;
    #[allow(clippy::unnecessary_result_map_or_else)]
    let state_str: String = row
        .try_get("state")
        .map_or_else(|_| "created".to_string(), |s| s);
    let state = WorkspaceState::from_str(&state_str).map_or(WorkspaceState::Created, |s| s);
    let workspace_path: String = row
        .try_get("workspace_path")
        .map_err(|e| Error::DatabaseError(format!("Failed to read workspace_path: {e}")))?;
    let branch: Option<String> = row
        .try_get("branch")
        .map_err(|e| Error::DatabaseError(format!("Failed to read branch: {e}")))?;
    let created_at: i64 = row
        .try_get("created_at")
        .map_err(|e| Error::DatabaseError(format!("Failed to read created_at: {e}")))?;
    let updated_at: i64 = row
        .try_get("updated_at")
        .map_err(|e| Error::DatabaseError(format!("Failed to read updated_at: {e}")))?;
    let last_synced: Option<i64> = row
        .try_get("last_synced")
        .map_err(|e| Error::DatabaseError(format!("Failed to read last_synced: {e}")))?;
    let metadata_str: Option<String> = row
        .try_get("metadata")
        .map_err(|e| Error::DatabaseError(format!("Failed to read metadata column: {e}")))?;

    let metadata = metadata_str
        .map(|s| {
            serde_json::from_str(&s).map_err(|e| {
                Error::DatabaseError(format!(
                    "Corrupted metadata JSON in database: {e}\nRaw value: {s}"
                ))
            })
        })
        .transpose()?;

    Ok(Session {
        id: Some(id),
        name,
        status,
        state,
        workspace_path,
        branch,
        created_at: u64::try_from(created_at).unwrap_or(0),
        updated_at: u64::try_from(updated_at).unwrap_or(0),
        last_synced: last_synced.map(|v| u64::try_from(v).unwrap_or(0)),
        metadata,
    })
}

// REST OF FILE (helpers etc)
async fn is_command_processed_pool(pool: &SqlitePool, command_id: &str) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT command_id FROM processed_commands WHERE command_id = ?")
            .bind(command_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to query processed command: {e}")))?;
    Ok(row.is_some())
}

async fn is_command_processed_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    command_id: &str,
) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT command_id FROM processed_commands WHERE command_id = ?")
            .bind(command_id)
            .fetch_optional(&mut **conn)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to query processed command: {e}")))?;
    Ok(row.is_some())
}

async fn query_session_by_name_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    name: &str,
) -> Result<Option<Session>> {
    sqlx::query(
        "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
         FROM sessions WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(&mut **conn)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to query session: {e}")))
    .and_then(|opt_row| opt_row.map(parse_session_row).transpose())
}

async fn mark_command_processed_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    command_id: &str,
    fingerprint: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO processed_commands (command_id, request_fingerprint) VALUES (?, ?)")
        .bind(command_id)
        .bind(fingerprint)
        .execute(&mut **conn)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to mark command as processed: {e}")))?;
    Ok(())
}

#[allow(clippy::useless_let_if_seq)]
async fn update_session(pool: &SqlitePool, name: &str, update: SessionUpdate) -> Result<()> {
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE sessions SET ");
    let mut separated = query_builder.separated(", ");

    let mut has_updates = false;

    if let Some(ref status) = update.status {
        separated.push("status = ");
        separated.push_bind_unseparated(status.to_string());
        has_updates = true;
    }
    if let Some(ref state) = update.state {
        separated.push("state = ");
        separated.push_bind_unseparated(state.to_string());
        has_updates = true;
    }
    if let Some(ref branch) = update.branch {
        separated.push("branch = ");
        separated.push_bind_unseparated(branch);
        has_updates = true;
    }
    if let Some(ls) = update.last_synced {
        separated.push("last_synced = ");
        separated.push_bind_unseparated(ls.to_i64().map_or(i64::MAX, |t| t));
        has_updates = true;
    }
    if let Some(ref m) = update.metadata {
        separated.push("metadata = ");
        separated.push_bind_unseparated(
            serde_json::to_string(m).map_err(|e| Error::Unknown(e.to_string()))?,
        );
        has_updates = true;
    }

    if !has_updates {
        return Ok(());
    }

    query_builder.push(" WHERE name = ");
    query_builder.push_bind(name);

    let result = query_builder
        .build()
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to update session: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Session '{name}' not found")));
    }

    Ok(())
}

async fn delete_session(pool: &SqlitePool, name: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to delete session: {e}")))?;
    Ok(result.rows_affected() > 0)
}

async fn query_command_fingerprint_conn(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    command_id: &str,
) -> Result<Option<Option<String>>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT request_fingerprint FROM processed_commands WHERE command_id = ?")
            .bind(command_id)
            .fetch_optional(&mut **conn)
            .await
            .map_err(|e| {
                Error::DatabaseError(format!("Failed to query command fingerprint: {e}"))
            })?;
    Ok(row.map(|r| r.0))
}

fn update_request_fingerprint(name: &str, update: &SessionUpdate) -> Result<String> {
    let payload = serde_json::json!({
        "name": name,
        "status": update.status,
        "state": update.state,
        "branch": update.branch,
        "last_synced": update.last_synced,
        "metadata": update.metadata,
    });
    Ok(format!(
        "{REQUEST_FINGERPRINT_VERSION}:{}",
        serde_json::to_string(&payload).map_err(|e| Error::Unknown(e.to_string()))?
    ))
}

fn normalize_fingerprint_for_compare(stored: &str) -> String {
    stored.to_string()
}
